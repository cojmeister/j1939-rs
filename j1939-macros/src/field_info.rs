use std::fmt::Display;
use syn::{Ident, Type};

#[derive(Debug, Clone)]
pub struct FieldInfo {
    pub name: Ident,
    pub ty: Type,
    pub bit_start: usize,
    pub bit_end: usize,
    pub encoding: Encoding,
    pub units: Option<String>,
    pub doc: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Encoding {
    /// Direct unsigned integer
    UInt,
    /// Direct signed integer (with sign extension)
    SInt,
    /// Scaled float: scale factor for conversion
    Scaled(f32),
    /// Q9 fixed-point
    Q9,
    /// Enum type with the actual enum type for variant extraction
    Enum(Type),
}

impl Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Encoding::UInt => { "UINT".to_string() }
            Encoding::SInt => { "Scaled INT".to_string() }
            Encoding::Scaled(_) => { "Scaled".to_string() }
            Encoding::Q9 => { "Q9".to_string() }
            Encoding::Enum(_) => { "Enum".to_string() }
        };
        write!(f, "{}", str)
    }
}

impl FieldInfo {
    pub fn bit_length(&self) -> usize {
        self.bit_end - self.bit_start
    }
}

#[derive(Debug, Clone)]
pub struct EnumVariant {
    pub name: String,
    pub value: u64,
    pub doc_comments: Vec<String>,
}

pub fn extract_enum_variants_from_type(enum_type: &Type) -> Option<Vec<EnumVariant>> {
    // For now, we'll need to implement this using a different approach
    // since we don't have access to the actual enum definition here.
    // This will be handled in the macro expansion phase.
    None
}

pub fn format_binary_value(value: u64, bit_width: usize) -> String {
    format!("{:0width$b}b", value, width = bit_width)
}