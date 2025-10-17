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

/// Represents different encoding strategies for J1939 message fields.
///
/// This enum defines how field values are encoded into and decoded from the raw bit stream
/// of a J1939 message. Each encoding type has specific behavior for marshalling (encoding)
/// and unmarshalling (decoding) operations.
///
/// # Encoding Types
///
/// ## `UInt` - Unsigned Integer
///
/// Direct unsigned integer encoding with no transformation.
/// - **Used for**: `u8`, `u16`, `u32` field types
/// - **Marshall**: Direct bit packing as unsigned value
/// - **Unmarshall**: Direct bit extraction as unsigned value
/// - **Example**:
///   ```ignore
///   #[j1939(bits = 0..8)]
///   pub counter: u8,  // Automatically inferred as UInt
///   ```
///
/// ## `SInt` - Signed Integer
///
/// Signed integer encoding with sign extension for negative values.
/// - **Used for**: `i8`, `i16`, `i32` field types
/// - **Marshall**: Two's complement representation
/// - **Unmarshall**: Sign extension applied to extract signed value
/// - **Example**:
///   ```ignore
///   #[j1939(bits = 0..16)]
///   pub temperature_delta: i16,  // Automatically inferred as SInt
///   ```
///
/// ## `Scaled` - Scaled Float with Optional Offset
///
/// Float encoding with linear transformation: `physical_value = (raw * scale) + offset`
/// - **Used for**: `f32` fields with `scale` attribute
/// - **Parameters**:
///   - `scale`: Multiplicative scaling factor (required)
///   - `offset`: Additive offset (optional, defaults to 0.0)
/// - **Marshall formula**: `raw = (physical_value - offset) / scale`
/// - **Unmarshall formula**: `physical_value = (raw * scale) + offset`
/// - **Signedness**:
///   - When `offset != 0.0`: Raw values treated as unsigned
///   - When `offset == 0.0`: Sign extension applied for negative values
/// - **Examples**:
///   ```ignore
///   // Simple scaling without offset
///   #[j1939(bits = 0..16, scale = 0.125, unit = "rpm")]
///   pub engine_speed: f32,  // 0.125 RPM per bit
///
///   // Scaling with offset (e.g., temperature)
///   #[j1939(bits = 16..24, scale = 1.0, offset = -40.0, unit = "°C")]
///   pub coolant_temp: f32,  // -40°C to +215°C range
///
///   // Torque percentage with negative offset
///   #[j1939(bits = 8..16, scale = 1.0, offset = -125.0, unit = "%")]
///   pub torque: f32,  // -125% to +125% range
///   ```
///
/// ## `Q9` - Fixed-Point Q9 Format
///
/// Q9 fixed-point encoding for high-precision fractional values in limited bit space.
/// - **Used for**: `f32` fields with `encoding = "q9"` attribute
/// - **Format**: 1 sign bit + 9 fractional bits (10 bits total)
/// - **Range**: Approximately -1.0 to +1.0
/// - **Precision**: 1/512 ≈ 0.00195
/// - **Example**:
///   ```ignore
///   #[j1939(bits = 0..10, encoding = "q9")]
///   pub control_gain: f32,  // High precision control value
///   ```
///
/// ## `Enum` - Enumeration Type
///
/// Type-safe enum encoding using the enum's discriminant value.
/// - **Used for**: Custom enum types (automatically detected)
/// - **Requirements**: Enum must be `#[repr(u8)]` and marked with `#[j1939_enum]`
/// - **Marshall**: Uses enum discriminant as raw value
/// - **Unmarshall**: Transmutes raw value back to enum (unsafe but fast)
/// - **Example**:
///   ```ignore
///   #[repr(u8)]
///   #[j1939_enum]
///   pub enum GearPosition {
///       Park = 0,
///       Drive = 1,
///   }
///
///   #[j1939(bits = 0..2)]
///   pub gear: GearPosition,  // Automatically inferred as Enum
///   ```
///
/// ## `Reserved` - Reserved/Unused Bits
///
/// Marks bits as reserved or unused in the message.
/// - **Used for**: Field types with `reserved` flag
/// - **Marshall**: Sets bits to zero
/// - **Unmarshall**: Returns unit type `()`
/// - **Purpose**: Ensures complete bit coverage and future compatibility
/// - **Example**:
///   ```ignore
///   #[j1939(bits = 24..64, reserved)]
///   pub reserved: (),
///   ```
///
/// # Selection Logic
///
/// The encoding type is determined automatically based on:
/// 1. If `reserved` flag is present → `Reserved`
/// 2. If `encoding = "q9"` → `Q9`
/// 3. If `scale` is present → `Scaled { scale, offset }`
/// 4. Otherwise inferred from Rust type:
///    - `f32`/`f64` without scale → Error (must specify scale or encoding)
///    - `i8`/`i16`/`i32` → `SInt`
///    - `u8`/`u16`/`u32` → `UInt`
///    - Other types → `Enum`
///
/// # See Also
///
/// - [`j1939_message`](crate::j1939_message) - The attribute macro that uses these encodings
/// - [`j1939_enum`](crate::j1939_enum) - For registering enums used with `Encoding::Enum`
#[derive(Debug, Clone, PartialEq)]
pub enum Encoding {
    /// Direct unsigned integer encoding (u8, u16, u32)
    UInt,
    /// Signed integer with sign extension (i8, i16, i32)
    SInt,
    /// Scaled float with linear transformation: `value = (raw * scale) + offset`
    Scaled { scale: f32, offset: f32 },
    /// Q9 fixed-point format (10 bits: 1 sign + 9 fractional)
    Q9,
    /// Enum type with discriminant-based encoding
    Enum(Box<Type>),
    /// Reserved/unused bits (set to zero)
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