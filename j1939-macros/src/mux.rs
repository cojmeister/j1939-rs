//! Implementation of the `#[j1939_mux]` attribute macro.

use std::collections::BTreeSet;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Expr, ExprLit, ExprRange, Fields, Lit};

/// Parses an integer literal bound of a `a..b` range.
fn range_bound(expr: &Option<Box<Expr>>, span: &ExprRange) -> syn::Result<u64> {
    if let Some(e) = expr
        && let Expr::Lit(ExprLit {
            lit: Lit::Int(lit), ..
        }) = e.as_ref()
    {
        return lit.base10_parse();
    }
    Err(syn::Error::new_spanned(
        span,
        "selector range must use integer literals, e.g. `selector = 0..8`",
    ))
}

fn parse_selector_range(attr: TokenStream) -> syn::Result<(usize, usize)> {
    let mut range = None;
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("selector") {
            let r: ExprRange = meta.value()?.parse()?;
            range = Some(r);
            Ok(())
        } else {
            Err(meta.error("unknown j1939_mux attribute, expected `selector = a..b`"))
        }
    });
    syn::parse::Parser::parse2(parser, attr)?;
    let range = range.ok_or_else(|| {
        syn::Error::new(
            proc_macro2::Span::call_site(),
            "j1939_mux requires `selector = a..b`",
        )
    })?;
    if matches!(range.limits, syn::RangeLimits::Closed(_)) {
        return Err(syn::Error::new_spanned(
            &range,
            "selector range must be half-open (a..b)",
        ));
    }
    let start = range_bound(&range.start, &range)?;
    let end = range_bound(&range.end, &range)?;
    if end <= start || end - start > 64 {
        return Err(syn::Error::new_spanned(
            &range,
            "selector width must be between 1 and 64 bits",
        ));
    }
    Ok((start as usize, (end - start) as usize))
}

fn parse_variant_selector(variant: &syn::Variant) -> syn::Result<u64> {
    let mut value = None;
    for attr in &variant.attrs {
        if attr.path().is_ident("j1939") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("selector") {
                    let lit: syn::LitInt = meta.value()?.parse()?;
                    value = Some(lit.base10_parse::<u64>()?);
                    Ok(())
                } else {
                    Err(meta.error("unknown variant attribute, expected `selector = N`"))
                }
            })?;
        }
    }
    value.ok_or_else(|| {
        syn::Error::new_spanned(variant, "mux variant requires `#[j1939(selector = N)]`")
    })
}

pub fn expand(attr: TokenStream, input: DeriveInput) -> syn::Result<TokenStream> {
    let (sel_start, sel_len) = parse_selector_range(attr)?;
    let Data::Enum(data) = &input.data else {
        return Err(syn::Error::new_spanned(
            &input,
            "j1939_mux can only be used on enums",
        ));
    };

    let mut seen = BTreeSet::new();
    let mut variants_out = Vec::new();
    let mut idents = Vec::new();
    let mut selectors = Vec::new();
    for variant in &data.variants {
        let single = matches!(&variant.fields, Fields::Unnamed(f) if f.unnamed.len() == 1);
        if !single {
            return Err(syn::Error::new_spanned(
                variant,
                "mux variants must be single-field tuple variants",
            ));
        }
        let sel = parse_variant_selector(variant)?;
        if sel_len < 64 && sel >> sel_len != 0 {
            return Err(syn::Error::new_spanned(
                variant,
                format!("selector value {sel} does not fit in {sel_len} bits"),
            ));
        }
        if !seen.insert(sel) {
            return Err(syn::Error::new_spanned(
                variant,
                format!("duplicate selector value {sel}"),
            ));
        }
        let mut v = variant.clone();
        v.attrs.retain(|a| !a.path().is_ident("j1939"));
        variants_out.push(v);
        idents.push(&variant.ident);
        selectors.push(sel);
    }

    let name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;
    let generics = &input.generics;
    if !generics.params.is_empty() {
        return Err(syn::Error::new_spanned(
            generics,
            "j1939_mux does not support generics",
        ));
    }

    Ok(quote! {
        #(#attrs)*
        #[derive(Debug, Clone, Copy)]
        #vis enum #name {
            #(#variants_out),*
        }

        impl j1939_core::Marshall for #name {
            fn marshall(&self, msg: &mut j1939_core::J1939Message) -> j1939_core::Result<()> {
                match self {
                    #( #name::#idents(inner) => {
                        j1939_core::Marshall::marshall(inner, msg)?;
                        j1939_core::encode_bitfield(
                            &mut msg.data, #sel_start, #sel_len, #selectors,
                        )?;
                    } )*
                }
                Ok(())
            }
        }

        impl j1939_core::Unmarshall for #name {
            fn unmarshall(msg: &j1939_core::J1939Message) -> j1939_core::Result<Self> {
                let raw = j1939_core::decode_bitfield(&msg.data, #sel_start, #sel_len)?;
                match raw {
                    #( #selectors => Ok(#name::#idents(
                        <_ as j1939_core::Unmarshall>::unmarshall(msg)?
                    )), )*
                    _ => Err(j1939_core::Error::UnknownSelector { raw }),
                }
            }
        }
    })
}
