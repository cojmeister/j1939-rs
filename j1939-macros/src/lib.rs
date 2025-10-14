use proc_macro::TokenStream;
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
    // For now, just pass through - we'll implement enum handling later
    item
}