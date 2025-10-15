use std::fmt::Display;
use std::collections::HashMap;
use std::sync::Mutex;
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
    /// Scaled float: scale factor and offset for conversion
    Scaled { scale: f32, offset: f32 },
    /// Q9 fixed-point
    Q9,
    /// Enum type with the actual enum type for variant extraction
    Enum(Type),
    /// Reserved bits - not used for data
    Reserved,
}

impl Display for Encoding {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let str = match self {
            Encoding::UInt => { "UINT".to_string() }
            Encoding::SInt => { "Scaled INT".to_string() }
            Encoding::Scaled { scale, offset } => {
                if *offset != 0.0 {
                    format!("Scaled (scale: {}, offset: {})", scale, offset)
                } else {
                    format!("Scaled (scale: {})", scale)
                }
            }
            Encoding::Q9 => { "Q9".to_string() }
            Encoding::Enum(_) => { "Enum".to_string() }
            Encoding::Reserved => { "Reserved".to_string() }
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