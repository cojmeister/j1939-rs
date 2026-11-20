use syn::{Attribute, Expr, ExprLit, ExprRange, FieldsNamed, Lit, Meta};

use crate::field_info::*;

/// Maximum message payload in bytes (must match `j1939_core::J1939Message::MAX_LENGTH`).
pub const MAX_MESSAGE_BYTES: u16 = 223;
/// Default message length in bytes when `length` is not given (a classic CAN frame).
const DEFAULT_MESSAGE_BYTES: u16 = 8;

pub struct MessageAttributes {
    pub pgn: u32,
    pub priority: u8,
    /// Message length in **bytes**.
    pub length: u16,
    /// `true` if `length` was given explicitly (otherwise it grows to fit the fields).
    pub length_explicit: bool,
    /// Require `msg.length == LENGTH` when decoding (default: `>=`, ignoring trailing bytes).
    pub strict: bool,
    /// Message must fit in one CAN frame (`length <= 8`); checked at macro-expansion time.
    pub single_frame: bool,
    pub defmt: bool,
}

impl syn::parse::Parse for MessageAttributes {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut pgn = None;
        let mut priority = 6; // Default priority
        let mut length = DEFAULT_MESSAGE_BYTES;
        let mut length_explicit = false;
        let mut strict = false;
        let mut single_frame = false;
        let mut defmt = false;

        while !input.is_empty() {
            let ident: syn::Ident = input.parse()?;

            if ident == "strict" {
                strict = true;
            } else if ident == "single_frame" {
                single_frame = true;
            } else if ident == "defmt" {
                defmt = true;
            } else {
                let _: syn::Token![=] = input.parse()?;
                match ident.to_string().as_str() {
                    "pgn" => {
                        let lit: syn::LitInt = input.parse()?;
                        let value: u32 = lit.base10_parse()?;
                        validate_pgn(value, &lit)?;
                        pgn = Some(value);
                    }
                    "priority" => {
                        let lit: syn::LitInt = input.parse()?;
                        priority = lit.base10_parse()?;
                        if priority > 7 {
                            return Err(syn::Error::new_spanned(lit, "priority must be in 0..=7"));
                        }
                    }
                    "length" => {
                        let lit: syn::LitInt = input.parse()?;
                        length = lit.base10_parse()?;
                        length_explicit = true;
                        if length == 0 || length > MAX_MESSAGE_BYTES {
                            return Err(syn::Error::new_spanned(
                                lit,
                                format!(
                                    "Message length is in bytes and must be 1..={MAX_MESSAGE_BYTES}"
                                ),
                            ));
                        }
                    }
                    _ => {
                        return Err(syn::Error::new_spanned(
                            ident,
                            "Unknown attribute, expected 'pgn', 'priority', 'length', 'strict', 'single_frame' or 'defmt'",
                        ));
                    }
                }
            }

            // Parse comma if there's more input
            if !input.is_empty() && input.peek(syn::Token![,]) {
                let _: syn::Token![,] = input.parse()?;
            }
        }

        let pgn = pgn.ok_or_else(|| input.error("Missing required 'pgn' attribute"))?;

        Ok(MessageAttributes {
            pgn,
            priority,
            length,
            length_explicit,
            strict,
            single_frame,
            defmt,
        })
    }
}

/// Rejects PGNs that cannot appear in a J1939 identifier.
fn validate_pgn(pgn: u32, lit: &syn::LitInt) -> syn::Result<()> {
    if pgn > 0x3FFFF {
        return Err(syn::Error::new_spanned(
            lit,
            "pgn must fit in 18 bits (0..=0x3FFFF)",
        ));
    }
    let pdu_format = (pgn >> 8) & 0xFF;
    if pdu_format < 240 && pgn & 0xFF != 0 {
        return Err(syn::Error::new_spanned(
            lit,
            "PDU1 PGNs (PF < 240) must have a zero low byte: the destination address is carried separately",
        ));
    }
    Ok(())
}

pub fn parse_fields(fields: &FieldsNamed) -> syn::Result<Vec<FieldInfo>> {
    let mut field_infos = Vec::new();

    for field in &fields.named {
        let name = field.ident.clone().unwrap();
        let (ty, signal) = match signal_inner_type(&field.ty) {
            Some(inner) => (inner.clone(), true),
            None => (field.ty.clone(), false),
        };

        // Extract doc comments
        let doc = extract_doc_comments(&field.attrs);

        // Find the #[j1939(...)] attribute
        let j1939_attr = field
            .attrs
            .iter()
            .find(|attr| attr.path().is_ident("j1939"))
            .ok_or_else(|| syn::Error::new_spanned(field, "Missing #[j1939(...)] attribute"))?;

        let (bit_start, bit_end, encoding, units, big_endian) =
            parse_j1939_attr(j1939_attr, &ty, signal)?;

        field_infos.push(FieldInfo {
            name,
            ty,
            signal,
            bit_start,
            bit_end,
            encoding,
            big_endian,
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
                && let Expr::Lit(ExprLit {
                    lit: Lit::Str(s), ..
                }) = &nv.value
            {
                return Some(s.value().trim().to_string());
            }
            None
        })
        .collect()
}

/// Parses an integer or float literal (optionally negative) as `f32`.
fn parse_f32(value: syn::parse::ParseStream) -> syn::Result<f32> {
    let expr: Expr = value.parse()?;
    let (neg, inner) = match &expr {
        Expr::Unary(u) if matches!(u.op, syn::UnOp::Neg(_)) => (true, u.expr.as_ref()),
        e => (false, e),
    };
    let v: f32 = match inner {
        Expr::Lit(ExprLit {
            lit: Lit::Float(f), ..
        }) => f.base10_parse()?,
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => i.base10_parse()?,
        _ => return Err(syn::Error::new_spanned(&expr, "expected a numeric literal")),
    };
    Ok(if neg { -v } else { v })
}

/// Width in bits of a primitive integer type, or `None` if `ty` is not one.
fn int_type_info(ty: &syn::Type) -> Option<(bool, u32)> {
    let syn::Type::Path(p) = ty else { return None };
    if p.qself.is_some() || p.path.segments.len() != 1 {
        return None;
    }
    match p.path.segments[0].ident.to_string().as_str() {
        "u8" => Some((false, 8)),
        "u16" => Some((false, 16)),
        "u32" => Some((false, 32)),
        "u64" => Some((false, 64)),
        "i8" => Some((true, 8)),
        "i16" => Some((true, 16)),
        "i32" => Some((true, 32)),
        "i64" => Some((true, 64)),
        _ => None,
    }
}

/// If `ty` is `Signal<T>` (any path ending in `Signal`), returns `T`.
fn signal_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(p) = ty else { return None };
    let last = p.path.segments.last()?;
    if last.ident != "Signal" {
        return None;
    }
    let syn::PathArguments::AngleBracketed(args) = &last.arguments else {
        return None;
    };
    match args.args.first()? {
        syn::GenericArgument::Type(inner) if args.args.len() == 1 => Some(inner),
        _ => None,
    }
}

fn is_float_type(ty: &syn::Type) -> bool {
    matches!(ty, syn::Type::Path(p) if p.path.is_ident("f32") || p.path.is_ident("f64"))
}

fn parse_j1939_attr(
    attr: &Attribute,
    ty: &syn::Type,
    signal: bool,
) -> syn::Result<(usize, usize, Encoding, Option<String>, bool)> {
    let mut bit_start = None;
    let mut bit_end = None;
    let mut scale = None;
    let mut offset = None;
    let mut encoding_type = None;
    let mut units = None;
    let mut is_reserved = false;
    let mut signed = false;
    let mut rounding = None;
    let mut q = None;
    let mut uq = None;
    let mut range = None;
    let mut big_endian = false;

    attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("bits") {
            let value = meta.value()?;
            let range: ExprRange = value.parse()?;

            // Parse start
            if let Some(start_expr) = &range.start
                && let Expr::Lit(ExprLit {
                    lit: Lit::Int(lit), ..
                }) = start_expr.as_ref()
            {
                bit_start = Some(lit.base10_parse()?);
            }

            // Parse end
            if let Some(end_expr) = &range.end
                && let Expr::Lit(ExprLit {
                    lit: Lit::Int(lit), ..
                }) = end_expr.as_ref()
            {
                bit_end = Some(lit.base10_parse()?);
            }

            Ok(())
        } else if meta.path.is_ident("scale") {
            scale = Some(parse_f32(meta.value()?)?);
            Ok(())
        } else if meta.path.is_ident("offset") {
            offset = Some(parse_f32(meta.value()?)?);
            Ok(())
        } else if meta.path.is_ident("range") {
            range = Some(parse_f32(meta.value()?)?);
            Ok(())
        } else if meta.path.is_ident("q") {
            let lit: syn::LitInt = meta.value()?.parse()?;
            q = Some(lit.base10_parse::<u32>()?);
            Ok(())
        } else if meta.path.is_ident("uq") {
            let lit: syn::LitInt = meta.value()?.parse()?;
            uq = Some(lit.base10_parse::<u32>()?);
            Ok(())
        } else if meta.path.is_ident("encoding") {
            let value = meta.value()?;
            let lit: syn::LitStr = value.parse()?;
            encoding_type = Some(lit.value());
            Ok(())
        } else if meta.path.is_ident("rounding") {
            let lit: syn::LitStr = meta.value()?.parse()?;
            rounding = Some(match lit.value().as_str() {
                "truncate" => Rounding::Truncate,
                "round" => Rounding::Round,
                _ => {
                    return Err(syn::Error::new_spanned(
                        lit,
                        "rounding must be \"truncate\" or \"round\"",
                    ));
                }
            });
            Ok(())
        } else if meta.path.is_ident("byte_order") {
            let lit: syn::LitStr = meta.value()?.parse()?;
            big_endian = match lit.value().as_str() {
                "big" => true,
                "little" => false,
                _ => {
                    return Err(syn::Error::new_spanned(
                        lit,
                        "byte_order must be \"big\" or \"little\"",
                    ));
                }
            };
            Ok(())
        } else if meta.path.is_ident("unit") {
            let value = meta.value()?;
            let lit: syn::LitStr = value.parse()?;
            units = Some(lit.value());
            Ok(())
        } else if meta.path.is_ident("reserved") {
            is_reserved = true;
            Ok(())
        } else if meta.path.is_ident("signed") {
            signed = true;
            Ok(())
        } else {
            Err(meta.error("Unsupported attribute"))
        }
    })?;

    let bit_start: usize =
        bit_start.ok_or_else(|| syn::Error::new_spanned(attr, "Missing 'bits' range start"))?;
    let bit_end: usize =
        bit_end.ok_or_else(|| syn::Error::new_spanned(attr, "Missing 'bits' range end"))?;
    if bit_end <= bit_start {
        return Err(syn::Error::new_spanned(
            attr,
            "'bits' range must not be empty",
        ));
    }
    let width = bit_end - bit_start;
    if width > 64 && !is_reserved {
        return Err(syn::Error::new_spanned(
            attr,
            "A field can be at most 64 bits wide",
        ));
    }

    if signal {
        if is_reserved || signed || q.is_some() || uq.is_some() || encoding_type.is_some() {
            return Err(syn::Error::new_spanned(
                attr,
                "'Signal<T>' fields cannot be 'reserved', 'signed', 'q', 'uq' or use 'encoding'; \
                 use Signal<u8..u64> or Signal<f32> with 'scale'",
            ));
        }
        if width < 2 {
            return Err(syn::Error::new_spanned(
                attr,
                "'Signal<T>' fields need at least 2 bits (all-ones is 'not available', all-ones \
                 minus one is 'error')",
            ));
        }
        let inner_ok = if scale.is_some() {
            matches!(ty, syn::Type::Path(p) if p.path.is_ident("f32"))
        } else {
            matches!(int_type_info(ty), Some((false, _)))
        };
        if !inner_ok {
            return Err(syn::Error::new_spanned(
                ty,
                "'Signal<T>' supports unsigned integers (Signal<u8..u64>) and Signal<f32> with 'scale'",
            ));
        }
    }

    // Modifiers that only make sense for some encodings.
    let is_q_like = q.is_some() || uq.is_some() || encoding_type.is_some();
    if range.is_some() && !is_q_like {
        return Err(syn::Error::new_spanned(
            attr,
            "'range' requires 'q = N', 'uq = N' or encoding = \"q9\"",
        ));
    }
    if signed && (scale.is_none() || is_q_like || is_reserved) {
        return Err(syn::Error::new_spanned(
            attr,
            "'signed' only applies to fields with 'scale' (integer fields take their signedness \
             from the Rust type)",
        ));
    }
    if rounding.is_some() && !(scale.is_some() || is_q_like) {
        return Err(syn::Error::new_spanned(
            attr,
            "'rounding' only applies to scaled or fixed-point float fields",
        ));
    }
    if [
        q.is_some(),
        uq.is_some(),
        encoding_type.is_some(),
        scale.is_some(),
    ]
    .iter()
    .filter(|b| **b)
    .count()
        > 1
    {
        return Err(syn::Error::new_spanned(
            attr,
            "Use only one of 'scale', 'q', 'uq' or 'encoding'",
        ));
    }
    let rounding = rounding.unwrap_or_default();

    // Determine encoding based on attributes and type
    let encoding = if is_reserved {
        // Reserved fields don't need type checking
        Encoding::Reserved
    } else if let Some(enc_str) = encoding_type {
        match enc_str.as_str() {
            "q9" => Encoding::Q {
                frac_bits: 9,
                range,
                rounding,
            },
            _ => {
                return Err(syn::Error::new_spanned(
                    attr,
                    format!("Unknown encoding: {}", enc_str),
                ));
            }
        }
    } else if let Some(frac_bits) = q {
        if !(1..=31).contains(&frac_bits) {
            return Err(syn::Error::new_spanned(attr, "'q' must be in 1..=31"));
        }
        Encoding::Q {
            frac_bits,
            range,
            rounding,
        }
    } else if let Some(bits) = uq {
        if !(1..=32).contains(&bits) {
            return Err(syn::Error::new_spanned(attr, "'uq' must be in 1..=32"));
        }
        Encoding::UQ {
            bits,
            range,
            rounding,
        }
    } else if let Some(scale_val) = scale {
        if scale_val == 0.0 || !scale_val.is_finite() {
            return Err(syn::Error::new_spanned(
                attr,
                "'scale' must be finite and non-zero",
            ));
        }
        // Default offset to 0.0 if not specified
        Encoding::Scaled {
            scale: scale_val,
            offset: offset.unwrap_or(0.0),
            signed,
            rounding,
        }
    } else if is_float_type(ty) {
        return Err(syn::Error::new_spanned(
            ty,
            "Float fields require 'scale', 'q', 'uq' or 'encoding'",
        ));
    } else if let Some((is_signed, _)) = int_type_info(ty) {
        if is_signed {
            Encoding::SInt
        } else {
            Encoding::UInt
        }
    } else {
        // Assume it's an enum if it's not a primitive type
        Encoding::Enum(Box::from(ty.clone()))
    };

    // Field width must agree with the encoding / Rust type.
    match &encoding {
        Encoding::Q { frac_bits, .. } if width as u32 != frac_bits + 1 => {
            return Err(syn::Error::new_spanned(
                attr,
                format!(
                    "Q{frac_bits} needs exactly {} bits (1 sign + {frac_bits} fractional), found {width}",
                    frac_bits + 1
                ),
            ));
        }
        Encoding::UQ { bits, .. } if width as u32 != *bits => {
            return Err(syn::Error::new_spanned(
                attr,
                format!("UQ{bits} needs exactly {bits} bits, found {width}"),
            ));
        }
        Encoding::UInt | Encoding::SInt => {
            let (_, ty_bits) = int_type_info(ty).expect("integer encoding implies integer type");
            if width as u32 > ty_bits {
                return Err(syn::Error::new_spanned(
                    ty,
                    format!("A {width}-bit field does not fit in a {ty_bits}-bit Rust type"),
                ));
            }
        }
        Encoding::Enum(_) if width > 8 => {
            return Err(syn::Error::new_spanned(
                ty,
                "Enum fields are limited to 8 bits",
            ));
        }
        _ => {}
    }

    if big_endian {
        if matches!(encoding, Encoding::Reserved | Encoding::Enum(_)) {
            return Err(syn::Error::new_spanned(
                attr,
                "'byte_order' does not apply to enum or reserved fields",
            ));
        }
        if (bit_start | width) & 7 != 0 {
            return Err(syn::Error::new_spanned(
                attr,
                "byte_order = \"big\" needs a byte-aligned field whose width is a multiple of 8",
            ));
        }
    }

    Ok((bit_start, bit_end, encoding, units, big_endian))
}

pub fn validate_fields(
    fields: &[FieldInfo],
    message_attributes: &mut MessageAttributes,
) -> syn::Result<()> {
    // Check for overlapping fields
    for (i, field1) in fields.iter().enumerate() {
        for field2 in fields.iter().skip(i + 1) {
            if field1.bit_end > field2.bit_start && field1.bit_start < field2.bit_end {
                return Err(syn::Error::new_spanned(
                    &field2.name,
                    format!(
                        "Field '{}' overlaps with field '{}'. Bits [{}..{}) overlap with [{}..{})",
                        field2.name,
                        field1.name,
                        field2.bit_start,
                        field2.bit_end,
                        field1.bit_start,
                        field1.bit_end
                    ),
                ));
            }
        }
    }

    // Calculate the minimum required length from field bit ranges
    let max_bit_used = fields.iter().map(|f| f.bit_end).max().unwrap_or(0);

    // If no explicit length was given, grow the default to fit the fields
    if !message_attributes.length_explicit {
        let needed = max_bit_used.div_ceil(8);
        if needed > MAX_MESSAGE_BYTES as usize {
            return Err(syn::Error::new(
                proc_macro2::Span::call_site(),
                format!(
                    "Fields need {needed} bytes but the maximum message length is {MAX_MESSAGE_BYTES} bytes"
                ),
            ));
        }
        message_attributes.length = message_attributes.length.max(needed as u16);
    }
    if message_attributes.single_frame && message_attributes.length > 8 {
        return Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "single_frame messages must be at most 8 bytes",
        ));
    }
    let length_bits = message_attributes.length as usize * 8;

    // Check that all fields fit within the specified message length
    for field in fields {
        if field.bit_end > length_bits {
            return Err(syn::Error::new_spanned(
                &field.name,
                format!(
                    "Field '{}' extends beyond message length of {} bytes ({} bits; field ends at bit {})",
                    field.name, message_attributes.length, length_bits, field.bit_end
                ),
            ));
        }
    }

    // Check for gaps (uncovered bit ranges) and warn about them
    validate_bit_coverage(fields, length_bits)?;

    Ok(())
}

fn validate_bit_coverage(fields: &[FieldInfo], message_length: usize) -> syn::Result<()> {
    // Create a sorted list of all field ranges
    let mut field_ranges: Vec<(usize, usize, bool)> = fields
        .iter()
        .map(|f| {
            (
                f.bit_start,
                f.bit_end,
                matches!(f.encoding, Encoding::Reserved),
            )
        })
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
            format!("Message has uncovered bit ranges:\n{}", warning_message),
        ));
    }

    Ok(())
}
