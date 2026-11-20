use std::collections::HashMap;
use std::fmt::Display;
use std::sync::Mutex;
use syn::{Ident, Type};

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: Ident,
    /// The Rust type used for casts and width checks. For a `Signal<T>` field this is `T`.
    pub ty: Type,
    /// `true` if the field is declared as `Signal<T>` (special-value aware).
    pub signal: bool,
    pub bit_start: usize,
    pub bit_end: usize,
    pub encoding: Encoding,
    /// `true` for `byte_order = "big"` (bytes of the field reversed on the wire).
    pub big_endian: bool,
    pub units: Option<String>,
    pub doc: Vec<String>,
}

/// Rounding applied when a floating-point value is converted to a raw integer on encode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rounding {
    /// Round to nearest, halves away from zero (the default).
    #[default]
    Round,
    /// Truncate toward zero (matches a C `(int)` cast).
    Truncate,
}

impl Rounding {
    pub fn tokens(self) -> proc_macro2::TokenStream {
        match self {
            Rounding::Round => quote::quote!(j1939_core::RoundingMode::HalfAwayFromZero),
            Rounding::Truncate => quote::quote!(j1939_core::RoundingMode::Truncate),
        }
    }
}

/// Represents the encoding strategy for a J1939 message field.
///
/// * `UInt` / `SInt` – `u8..u64` / `i8..i64` fields (1..=64 bits, must fit the Rust type).
/// * `Scaled` – `f32` with `physical = raw * scale + offset`. The raw value is **unsigned**
///   unless the field is marked `signed`; signedness is never inferred from the offset.
///   Encoding saturates to the field's own raw range.
/// * `Q` – signed fixed point, `frac_bits` fractional bits plus a sign bit
///   (`bits = frac_bits + 1`), optionally scaled to a physical `range`.
/// * `UQ` – unsigned full-scale fixed point, `raw / (2^bits - 1)`, optionally scaled by `range`.
/// * `Enum` – a `#[j1939_enum]` type, decoded through its generated `TryFrom<u8>`.
/// * `Reserved` – bits written as zero and ignored on decode.
#[derive(Debug, Clone, PartialEq)]
pub enum Encoding {
    UInt,
    SInt,
    Scaled {
        scale: f32,
        offset: f32,
        signed: bool,
        rounding: Rounding,
    },
    Q {
        frac_bits: u32,
        range: Option<f32>,
        rounding: Rounding,
    },
    UQ {
        bits: u32,
        range: Option<f32>,
        rounding: Rounding,
    },
    Enum(Box<Type>),
    Reserved,
}

impl Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Encoding::UInt => "UINT".to_string(),
            Encoding::SInt => "INT".to_string(),
            Encoding::Scaled {
                scale,
                offset,
                signed,
                ..
            } => {
                let sign = if *signed { "signed " } else { "" };
                if *offset != 0.0 {
                    format!("Scaled {sign}(scale: {scale}, offset: {offset})")
                } else {
                    format!("Scaled {sign}(scale: {scale})")
                }
            }
            Encoding::Q {
                frac_bits, range, ..
            } => match range {
                Some(r) => format!("Q{frac_bits} (range: ±{r})"),
                None => format!("Q{frac_bits}"),
            },
            Encoding::UQ { bits, range, .. } => match range {
                Some(r) => format!("UQ{bits} (range: {r})"),
                None => format!("UQ{bits}"),
            },
            Encoding::Enum(_) => "Enum".to_string(),
            Encoding::Reserved => "Reserved".to_string(),
        };
        write!(f, "{}", str)
    }
}

impl FieldInfo {
    pub fn bit_length(&self) -> usize {
        self.bit_end - self.bit_start
    }

    /// Encoding description for the generated docs table (` BE` marks big-endian fields).
    pub fn encoding_label(&self) -> String {
        if self.big_endian {
            format!("{} BE", self.encoding)
        } else {
            self.encoding.to_string()
        }
    }
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub value: u64,
    pub doc_comments: Vec<String>,
}

pub fn format_binary_value(value: u64, bit_width: usize) -> String {
    format!("{:0width$b}b", value, width = bit_width)
}

#[derive(Debug, Clone)]
pub struct EnumInfo {
    pub name: String,
    pub doc_comments: Vec<String>,
    pub variants: Vec<EnumVariant>,
}

// Global registry for storing enum information during compilation
static ENUM_REGISTRY: Mutex<Option<HashMap<String, EnumInfo>>> = Mutex::new(None);

pub fn register_enum(enum_info: EnumInfo) {
    let mut registry = ENUM_REGISTRY.lock().unwrap();
    if registry.is_none() {
        *registry = Some(HashMap::new());
    }
    if let Some(ref mut map) = *registry {
        map.insert(enum_info.name.clone(), enum_info);
    }
}

pub fn get_enum_info(enum_name: &str) -> Option<EnumInfo> {
    let registry = ENUM_REGISTRY.lock().unwrap();
    if let Some(ref map) = *registry {
        map.get(enum_name).cloned()
    } else {
        None
    }
}
