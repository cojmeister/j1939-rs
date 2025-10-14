use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

mod field_info;
mod parse;
mod codegen;
mod documentation_generation;

use codegen::*;
use field_info::*;
use parse::*;

#[proc_macro_attribute]
pub fn j1939_message(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let attr_args = parse_macro_input!(attr as MessageAttributes);

    // Parse the struct
    let struct_data = match &input.data {
        Data::Struct(data) => data,
        _ => {
            return syn::Error::new_spanned(&input, "j1939_message can only be used on structs")
                .to_compile_error()
                .into();
        }
    };

    let fields = match &struct_data.fields {
        Fields::Named(fields) => fields,
        _ => {
            return syn::Error::new_spanned(&input, "j1939_message requires named fields")
                .to_compile_error()
                .into();
        }
    };

    // Parse field attributes
    let field_infos: Vec<FieldInfo> = match parse_fields(fields) {
        Ok(infos) => infos,
        Err(e) => return e.to_compile_error().into(),
    };

    // Validate fields and auto-calculate length if needed
    let mut attr_args = attr_args;
    if let Err(e) = validate_fields(&field_infos, &mut attr_args) {
        return e.to_compile_error().into();
    }

    // Generate code
    let expanded = generate_message_impl(&input, &attr_args, &field_infos);

    TokenStream::from(expanded)
}

#[proc_macro_attribute]
pub fn j1939_enum(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    // Parse and register enum information
    if let Data::Enum(enum_data) = &input.data {
        let enum_name = input.ident.to_string();

        // Extract enum-level documentation
        let enum_docs = extract_doc_comments(&input.attrs);

        // Extract variant information
        let mut variants = Vec::new();
        let mut current_value = 0u64;

        for variant in &enum_data.variants {
            let variant_name = variant.ident.to_string();
            let variant_docs = extract_doc_comments(&variant.attrs);

            // Handle explicit discriminant values
            let value = if let Some((_, expr)) = &variant.discriminant {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(lit_int), .. }) = expr {
                    current_value = lit_int.base10_parse().unwrap_or(current_value);
                    current_value
                } else {
                    current_value
                }
            } else {
                current_value
            };

            variants.push(EnumVariant {
                name: variant_name,
                value,
                doc_comments: variant_docs,
            });

            current_value += 1;
        }

        // Register the enum
        let enum_info = EnumInfo {
            name: enum_name,
            doc_comments: enum_docs,
            variants,
        };

        register_enum(enum_info);
    }

    // Return the original enum unchanged
    let enum_name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;

    if let Data::Enum(enum_data) = &input.data {
        let variants = &enum_data.variants;

        quote! {
            #(#attrs)*
            #vis enum #enum_name {
                #variants
            }
        }.into()
    } else {
        syn::Error::new_spanned(&input, "j1939_enum can only be used on enums")
            .to_compile_error()
            .into()
    }
}

fn extract_doc_comments(attrs: &[syn::Attribute]) -> Vec<String> {
    attrs
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
        .collect()
}