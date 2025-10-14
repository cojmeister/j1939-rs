// j1939-macros/src/codegen.rs
use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::field_info::*;
use crate::parse::MessageAttributes;
use crate::documentation_generation::generate_documentation_table;

pub fn generate_message_impl(
    input: &DeriveInput,
    attr: &MessageAttributes,
    fields: &[FieldInfo],
) -> TokenStream {
    let struct_name = &input.ident;
    let pgn = attr.pgn;
    let priority = attr.priority;

    // Use the specified message length in bytes
    let length = (attr.length + 7) / 8; // Convert bits to bytes, round up

    let marshall_fields = generate_marshall_fields(fields);
    let unmarshall_fields = generate_unmarshall_fields(fields);

    let vis = &input.vis;

    // Extract struct-level doc comments
    let original_struct_docs: Vec<String> = input.attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(nv) = &attr.meta {
                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &nv.value {
                        return Some(s.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect();

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
    let fields_tokens: Vec<_> = fields.iter().map(|f| {
        let name = &f.name;
        let ty = &f.ty;
        let docs = &f.doc;
        quote! {
            #(#[doc = #docs])*
            #vis #name: #ty
        }
    }).collect();

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
            pub const LENGTH: u8 = #length;
        }

        impl j1939_core::Marshall for #struct_name {
            fn marshall(&self, msg: &mut j1939_core::J1939Message) -> j1939_core::Result<()> {
                msg.priority = Self::PRIORITY;
                msg.pgn = Self::PGN;
                msg.length = Self::LENGTH;
                // Zero out the data buffer for the message length
                for i in 0..msg.data.len() {
                    msg.data[i] = 0;
                }

                #(#marshall_fields)*

                Ok(())
            }
        }

        impl j1939_core::Unmarshall for #struct_name {
            fn unmarshall(msg: &j1939_core::J1939Message) -> j1939_core::Result<Self> {
                if msg.pgn != Self::PGN {
                    return Err(j1939_core::Error::InvalidPgn);
                }
                if msg.length != Self::LENGTH {
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

            match &field.encoding {
                Encoding::UInt => {
                    quote! {
                        j1939_core::encode_bitfield(
                            &mut msg.data,
                            #bit_start,
                            #bit_length,
                            self.#name as u16
                        );
                    }
                }
                Encoding::SInt => {
                    quote! {
                        j1939_core::encode_bitfield(
                            &mut msg.data,
                            #bit_start,
                            #bit_length,
                            self.#name as u16
                        );
                    }
                }
                Encoding::Scaled(scale) => {
                    quote! {
                        {
                            let scaled = (self.#name / #scale).round() as i16;
                            j1939_core::encode_bitfield(
                                &mut msg.data,
                                #bit_start,
                                #bit_length,
                                scaled as u16
                            );
                        }
                    }
                }
                Encoding::Q9 => {
                    quote! {
                        {
                            let q9 = j1939_core::Q9::from_float(self.#name);
                            j1939_core::encode_bitfield(
                                &mut msg.data,
                                #bit_start,
                                #bit_length,
                                q9.to_raw() as u16
                            );
                        }
                    }
                }
                Encoding::Enum => {
                    quote! {
                        j1939_core::encode_bitfield(
                            &mut msg.data,
                            #bit_start,
                            #bit_length,
                            self.#name as u16
                        );
                    }
                }
            }
        })
        .collect()
}

fn generate_unmarshall_fields(fields: &[FieldInfo]) -> Vec<TokenStream> {
    fields
        .iter()
        .map(|field| {
            let name = &field.name;
            let ty = &field.ty;
            let bit_start = field.bit_start;
            let bit_length = field.bit_length();

            match &field.encoding {
                Encoding::UInt => {
                    quote! {
                        #name: j1939_core::decode_bitfield(&msg.data, #bit_start, #bit_length) as #ty
                    }
                }
                Encoding::SInt => {
                    quote! {
                        #name: j1939_core::sign_extend(
                            j1939_core::decode_bitfield(&msg.data, #bit_start, #bit_length),
                            #bit_length
                        ) as #ty
                    }
                }
                Encoding::Scaled(scale) => {
                    quote! {
                        #name: {
                            let raw = j1939_core::decode_bitfield(&msg.data, #bit_start, #bit_length);
                            let signed = j1939_core::sign_extend(raw, #bit_length);
                            signed as f32 * #scale
                        }
                    }
                }
                Encoding::Q9 => {
                    quote! {
                        #name: {
                            let raw = j1939_core::decode_bitfield(&msg.data, #bit_start, #bit_length);
                            let signed = j1939_core::sign_extend(raw, #bit_length);
                            j1939_core::Q9::from_raw(signed).to_float()
                        }
                    }
                }
                Encoding::Enum => {
                    quote! {
                        #name: {
                            let raw_value = j1939_core::decode_bitfield(&msg.data, #bit_start, #bit_length) as u8;
                            unsafe { core::mem::transmute::<u8, #ty>(raw_value) }
                        }
                    }
                }
            }
        })
        .collect()
}