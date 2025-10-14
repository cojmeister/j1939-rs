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
    /// Enum type
    /// Value is the bit size of the enum
    #[allow(dead_code)]
    Enum,
}

impl Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Encoding::UInt => { "UINT".to_string() }
            Encoding::SInt => { "Scaled INT".to_string() }
            Encoding::Scaled(_) => { "Scaled".to_string() }
            Encoding::Q9 => { "Q9".to_string() }
            Encoding::Enum => { "Enum".to_string() }
        };
        write!(f, "{}", str)
    }
}

impl FieldInfo {
    pub fn bit_length(&self) -> usize {
        self.bit_end - self.bit_start
    }
}