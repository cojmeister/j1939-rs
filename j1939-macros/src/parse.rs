use syn::{Attribute, Expr, ExprLit, ExprRange, FieldsNamed, Lit, Meta};

use crate::field_info::*;

pub struct MessageAttributes {
    pub pgn: u32,
    pub priority: u8,
    pub length: u8,
}

impl syn::parse::Parse for MessageAttributes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut pgn = None;
        let mut priority = 6; // Default priority
        let mut length = 64u8;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;
            let _: syn::Token![=] = input.parse()?;

            match ident.to_string().as_str() {
                "pgn" => {
                    let lit: syn::LitInt = input.parse()?;
                    pgn = Some(lit.base10_parse()?);
                }
                "priority" => {
                    let lit: syn::LitInt = input.parse()?;
                    priority = lit.base10_parse()?;
                }
                "length" => {
                    let lit: syn::LitInt = input.parse()?;
                    length = lit.base10_parse()?;
                    if length > 223 {
                        return Err(syn::Error::new_spanned(
                            lit,
                            "Message length cannot exceed 223 bits (J1939 multi-packet limit)",
                        ));
                    }
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        ident,
                        "Unknown attribute, expected 'pgn', 'priority', or 'length'",
                    ));
                }
            }

            // Parse comma if there's more input
            if !input.is_empty() {
                if input.peek(syn::Token![,]) {
                    let _: syn::Token![,] = input.parse()?;
                }
            }
        }

        let pgn = pgn.ok_or_else(|| input.error("Missing required 'pgn' attribute"))?;

        Ok(MessageAttributes { pgn, priority, length })
    }
}

pub fn parse_fields(fields: &FieldsNamed) -> syn::Result<Vec<FieldInfo>> {
    let mut field_infos = Vec::new();

    for field in &fields.named {
        let name = field.ident.clone().unwrap();
        let ty = field.ty.clone();

        // Extract doc comments
        let doc = extract_doc_comments(&field.attrs);

        // Find the #[j1939(...)] attribute
        let j1939_attr = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("j1939"))
            .ok_or_else(|| {
                syn::Error::new_spanned(field, "Missing #[j1939(...)] attribute")
            })?;

        let (bit_start, bit_end, encoding, units) = parse_j1939_attr(j1939_attr, &ty)?;

        field_infos.push(FieldInfo {
            name,
            ty,
            bit_start,
            bit_end,
            encoding,
            units,
            doc,
        });
    }

    Ok(field_infos)
}

fn extract_doc_comments(attrs: &[Attribute]) -> Vec<String> {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc")
                && let Meta::NameValue(nv) = &attr.meta
                    && let Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) = &nv.value {
                        return Some(s.value().trim().to_string());
            }
            None
        })
        .collect()
}

fn parse_j1939_attr(attr: &Attribute, ty: &syn::Type) -> syn::Result<(usize, usize, Encoding, Option<String>)> {
    let mut bit_start = None;
    let mut bit_end = None;
    let mut scale = None;
    let mut offset = None;
    let mut encoding_type = None;
    let mut units = None;
    let mut is_reserved = false;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("bits") {
            let value = meta.value()?;
            let range: ExprRange = value.parse()?;

            // Parse start
            if let Some(start_expr) = &range.start
                && let Expr::Lit(ExprLit { lit: Lit::Int(lit), .. }) = start_expr.as_ref() {
                    bit_start = Some(lit.base10_parse()?);
            }

            // Parse end
            if let Some(end_expr) = &range.end &&
                let Expr::Lit(ExprLit { lit: Lit::Int(lit), .. }) = end_expr.as_ref() {
                    bit_end = Some(lit.base10_parse()?);
            }

            Ok(())
        } else if meta.path.is_ident("scale") {
            let value = meta.value()?;
            let lit: syn::LitFloat = value.parse()?;
            scale = Some(lit.base10_parse()?);
            Ok(())
        } else if meta.path.is_ident("offset") {
            let value = meta.value()?;
            let lit: syn::LitFloat = value.parse()?;
            offset = Some(lit.base10_parse()?);
            Ok(())
        } else if meta.path.is_ident("encoding") {
            let value = meta.value()?;
            let lit: syn::LitStr = value.parse()?;
            encoding_type = Some(lit.value());
            Ok(())
        } else if meta.path.is_ident("unit") {
            let value = meta.value()?;
            let lit: syn::LitStr = value.parse()?;
            units = Some(lit.value());
            Ok(())
        } else if meta.path.is_ident("reserved") {
            is_reserved = true;
            Ok(())
        } else {
            Err(meta.error("Unsupported attribute"))
        }
    })?;

    let bit_start = bit_start.ok_or_else(|| {
        syn::Error::new_spanned(attr, "Missing 'bits' range start")
    })?;
    let bit_end = bit_end.ok_or_else(|| {
        syn::Error::new_spanned(attr, "Missing 'bits' range end")
    })?;

    // Determine encoding based on attributes and type
    let encoding = if is_reserved {
        // Reserved fields don't need type checking
        Encoding::Reserved
    } else if let Some(enc_str) = encoding_type {
        match enc_str.as_str() {
            "q9" => Encoding::Q9,
            _ => return Err(syn::Error::new_spanned(attr, format!("Unknown encoding: {}", enc_str))),
        }
    } else if let Some(scale_val) = scale {
        // Default offset to 0.0 if not specified
        let offset_val = offset.unwrap_or(0.0);
        Encoding::Scaled { scale: scale_val, offset: offset_val }
    } else {
        // Infer from type
        let ty_str = quote::quote!(#ty).to_string();
        if ty_str.contains("f32") || ty_str.contains("f64") {
            return Err(syn::Error::new_spanned(
                ty,
                "Float fields require either 'scale' or 'encoding' attribute",
            ));
        } else if ty_str.contains("i8") || ty_str.contains("i16") || ty_str.contains("i32") {
            Encoding::SInt
        } else if ty_str.contains("u8") || ty_str.contains("u16") || ty_str.contains("u32") {
            Encoding::UInt
        } else {
            // Assume it's an enum if it's not a primitive type
            Encoding::Enum(ty.clone())
        }
    };

    Ok((bit_start, bit_end, encoding, units))
}

pub fn validate_fields(fields: &[FieldInfo], message_attributes: &mut MessageAttributes) -> syn::Result<()> {
    // Check for overlapping fields
    for (i, field1) in fields.iter().enumerate() {
        for field2 in fields.iter().skip(i + 1) {
            if field1.bit_end > field2.bit_start && field1.bit_start < field2.bit_end {
                return Err(syn::Error::new_spanned(
                    &field2.name,
                    format!(
                        "Field '{}' overlaps with field '{}'. Bits [{}..{}) overlap with [{}..{})",
                        field2.name, field1.name, field2.bit_start, field2.bit_end, field1.bit_start, field1.bit_end
                    ),
                ));
            }
        }
    }

    // Calculate the minimum required length from field bit ranges
    let max_bit_used = fields.iter().map(|f| f.bit_end).max().unwrap_or(0);

    // If message length is still default (64), auto-calculate from fields
    if message_attributes.length == 64 && max_bit_used > 64 {
        message_attributes.length = max_bit_used as u8;
    }

    // Check that all fields fit within the specified message length
    for field in fields {
        if field.bit_end > message_attributes.length as usize {
            return Err(syn::Error::new_spanned(
                &field.name,
                format!(
                    "Field '{}' extends beyond message length of {} bits (ends at bit {})",
                    field.name,
                    message_attributes.length,
                    field.bit_end
                ),
            ));
        }
    }

    // Check for gaps (uncovered bit ranges) and warn about them
    validate_bit_coverage(fields, message_attributes.length as usize)?;

    // Note: J1939 supports up to 223 bytes (1784 bits) for multi-packet messages
    // The validation for max length is already handled above

    Ok(())
}

fn validate_bit_coverage(fields: &[FieldInfo], message_length: usize) -> syn::Result<()> {
    // Create a sorted list of all field ranges
    let mut field_ranges: Vec<(usize, usize, bool)> = fields
        .iter()
        .map(|f| (f.bit_start, f.bit_end, matches!(f.encoding, Encoding::Reserved)))
        .collect();

    // Sort by start position
    field_ranges.sort_by_key(|&(start, _, _)| start);

    let mut warnings = Vec::new();
    let mut current_pos = 0;

    for (start, end, _is_reserved) in field_ranges {
        // Check for gap before this field
        if current_pos < start {
            warnings.push(format!(
                "Bits {}..{} are not covered by any field. Use #[j1939(bits = {}..{}, reserved)] to explicitly mark as reserved.",
                current_pos, start, current_pos, start
            ));
        }

        current_pos = end;
    }

    // Check for gap at the end
    if current_pos < message_length {
        warnings.push(format!(
            "Bits {}..{} are not covered by any field. Use #[j1939(bits = {}..{}, reserved)] to explicitly mark as reserved.",
            current_pos, message_length, current_pos, message_length
        ));
    }

    // Generate compilation warnings for uncovered gaps
    if !warnings.is_empty() {
        let warning_message = warnings.join("\n");
        // Note: In a real implementation, we'd want to generate proper compiler warnings
        // For now, we'll return an error to make gaps visible during development
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            format!("Message has uncovered bit ranges:\n{}", warning_message)
        ));
    }

    Ok(())
}