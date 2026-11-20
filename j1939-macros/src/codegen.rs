// j1939-macros/src/codegen.rs
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::documentation_generation::generate_documentation_table;
use crate::extract_doc_comments;
use crate::field_info::*;
use crate::parse::MessageAttributes;

pub fn generate_message_impl(
    input: &DeriveInput,
    attr: &MessageAttributes,
    fields: &[FieldInfo],
) -> TokenStream {
    let struct_name = &input.ident;
    let pgn = attr.pgn;
    let priority = attr.priority;

    // Message length in bytes
    let length = attr.length;
    let length_check = if attr.strict {
        quote! { msg.length != Self::LENGTH }
    } else {
        // Accept padded frames (e.g. a 6-byte PGN in an 8-byte CAN frame): trailing bytes ignored
        quote! { msg.length < Self::LENGTH }
    };

    let marshall_fields = generate_marshall_fields(fields);
    let unmarshall_fields = generate_unmarshall_fields(fields);

    let vis = &input.vis;

    // Extract struct-level doc comments
    let original_struct_docs: Vec<String> = extract_doc_comments(&input.attrs);

    // Generate documentation table
    let field_table = generate_documentation_table(&fields.to_vec());

    // Combine original docs with generated table
    let mut combined_docs = original_struct_docs.clone();

    // Add field layout section if we have fields
    if !fields.is_empty() {
        if !combined_docs.is_empty() {
            combined_docs.push("".to_string()); // Empty line separator
        }
        combined_docs.push("## Field Layout".to_string());
        combined_docs.push("".to_string()); // Empty line before table

        // Add each line of the table as a separate doc comment
        for line in field_table.lines() {
            combined_docs.push(line.to_string());
        }
    }

    // Generate field tokens for the struct definition
    let fields_tokens: Vec<_> = fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let ty = &f.ty;
            let ty = if f.signal {
                quote! { j1939_core::Signal<#ty> }
            } else {
                quote! { #ty }
            };
            let docs = &f.doc;
            quote! {
                #(#[doc = #docs])*
                #vis #name: #ty
            }
        })
        .collect();

    let expanded = quote! {
        // Re-emit the struct with derives and combined documentation
        #(#[doc = #combined_docs])*
        #[derive(Debug, Clone, Copy)]
        #vis struct #struct_name {
            #(#fields_tokens),*
        }

        impl #struct_name {
            pub const PGN: u32 = #pgn;
            pub const PRIORITY: u8 = #priority;
            pub const LENGTH: u16 = #length;
            /// `true` if the message fits in a single CAN frame (`LENGTH <= 8`).
            pub const SINGLE_FRAME: bool = Self::LENGTH <= 8;
        }

        impl j1939_core::Marshall for #struct_name {
            fn marshall(&self, msg: &mut j1939_core::J1939Message) -> j1939_core::Result<()> {
                msg.priority = Self::PRIORITY;
                msg.pgn = Self::PGN;
                msg.length = Self::LENGTH;
                // Zero the whole buffer so reserved and unused bits are always 0
                msg.data.fill(0);

                #(#marshall_fields)*

                Ok(())
            }
        }

        impl j1939_core::Unmarshall for #struct_name {
            fn unmarshall(msg: &j1939_core::J1939Message) -> j1939_core::Result<Self> {
                if msg.pgn != Self::PGN {
                    return Err(j1939_core::Error::InvalidPgn);
                }
                if #length_check {
                    return Err(j1939_core::Error::InvalidLength);
                }

                Ok(Self {
                    #(#unmarshall_fields),*
                })
            }
        }
    };

    expanded
}

fn generate_marshall_fields(fields: &[FieldInfo]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = &field.name;
            let bit_start = field.bit_start;
            let bit_length = field.bit_length();
            let bits32 = bit_length as u32;

            // Expression producing the raw `u64` to write (masked to `bit_length` bits by the
            // bitfield encoder; negative values are two's complement via sign-extending casts).
            let raw: TokenStream = match &field.encoding {
                Encoding::UInt | Encoding::Scaled { signed: false, .. } if field.signal => {
                    // `signal_to_raw` clamps valid values below the special range.
                    let valid = match &field.encoding {
                        Encoding::Scaled {
                            scale,
                            offset,
                            rounding,
                            ..
                        } => {
                            let mode = rounding.tokens();
                            quote! {
                                j1939_core::f32_to_raw_unsigned(
                                    (*v - #offset) / #scale, #bits32, #mode
                                )
                            }
                        }
                        _ => quote! { *v as u64 },
                    };
                    quote! { j1939_core::signal_to_raw(&self.#name, #bits32, |v| #valid) }
                }
                Encoding::UInt | Encoding::SInt => quote! { self.#name as u64 },
                Encoding::Enum(_) => quote! { (self.#name as u8) as u64 },
                Encoding::Scaled {
                    scale,
                    offset,
                    signed,
                    rounding,
                } => {
                    let mode = rounding.tokens();
                    if *signed {
                        quote! {
                            j1939_core::f32_to_raw_signed(
                                (self.#name - #offset) / #scale, #bits32, #mode
                            ) as u64
                        }
                    } else {
                        quote! {
                            j1939_core::f32_to_raw_unsigned(
                                (self.#name - #offset) / #scale, #bits32, #mode
                            )
                        }
                    }
                }
                Encoding::Q {
                    frac_bits,
                    range,
                    rounding,
                } => {
                    let mode = rounding.tokens();
                    let value = match range {
                        Some(r) => quote! { self.#name / #r },
                        None => quote! { self.#name },
                    };
                    quote! {
                        (j1939_core::Q::<#frac_bits>::from_float_with(#value, #mode).to_raw() as i64)
                            as u64
                    }
                }
                Encoding::UQ {
                    bits,
                    range,
                    rounding,
                } => {
                    let mode = rounding.tokens();
                    let value = match range {
                        Some(r) => quote! { self.#name / #r },
                        None => quote! { self.#name },
                    };
                    quote! {
                        j1939_core::UQ::<#bits>::from_float_with(#value, #mode).to_raw() as u64
                    }
                }
                Encoding::Reserved => {
                    // Reserved bits stay zero: the buffer is cleared before encoding.
                    return quote! {};
                }
            };

            let raw = if field.big_endian {
                quote! { j1939_core::swap_field_bytes(#raw, #bit_length) }
            } else {
                raw
            };

            quote! {
                j1939_core::encode_bitfield(&mut msg.data, #bit_start, #bit_length, #raw)?;
            }
        })
        .collect()
}

fn generate_unmarshall_fields(fields: &[FieldInfo]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = &field.name;
            let name_str = name.to_string();
            let ty = &field.ty;
            let bit_start = field.bit_start;
            let bit_length = field.bit_length();
            let raw = quote! { j1939_core::decode_bitfield(&msg.data, #bit_start, #bit_length)? };
            // Big-endian fields are byte-swapped back before any sign extension or scaling.
            let raw = if field.big_endian {
                quote! { j1939_core::swap_field_bytes(#raw, #bit_length) }
            } else {
                raw
            };

            if field.signal {
                let valid = match &field.encoding {
                    Encoding::Scaled { scale, offset, .. } => {
                        quote! { |r| (r as f32 * #scale) + #offset }
                    }
                    _ => quote! { |r| r as #ty },
                };
                return quote! {
                    #name: j1939_core::raw_to_signal(#raw, #bit_length as u32, #valid)
                };
            }

            match &field.encoding {
                Encoding::UInt => quote! { #name: #raw as #ty },
                Encoding::SInt => quote! {
                    #name: j1939_core::sign_extend(#raw, #bit_length) as #ty
                },
                Encoding::Scaled {
                    scale,
                    offset,
                    signed,
                    ..
                } => {
                    if *signed {
                        quote! {
                            #name: (j1939_core::sign_extend(#raw, #bit_length) as f32 * #scale)
                                + #offset
                        }
                    } else {
                        quote! { #name: (#raw as f32 * #scale) + #offset }
                    }
                }
                Encoding::Q {
                    frac_bits, range, ..
                } => {
                    let q = quote! {
                        j1939_core::Q::<#frac_bits>::from_raw(
                            j1939_core::sign_extend(#raw, #bit_length) as i32
                        ).to_float()
                    };
                    match range {
                        Some(r) => quote! { #name: #q * #r },
                        None => quote! { #name: #q },
                    }
                }
                Encoding::UQ { bits, range, .. } => {
                    let uq = quote! { j1939_core::UQ::<#bits>::from_raw(#raw as u32).to_float() };
                    match range {
                        Some(r) => quote! { #name: #uq * #r },
                        None => quote! { #name: #uq },
                    }
                }
                Encoding::Enum(_) => quote! {
                    #name: <#ty as core::convert::TryFrom<u8>>::try_from(#raw as u8).map_err(
                        |raw| j1939_core::Error::InvalidEnum { field: #name_str, raw }
                    )?
                },
                Encoding::Reserved => {
                    // Reserved bits are ignored on decode
                    quote! { #name: () }
                }
            }
        })
        .collect()
}
