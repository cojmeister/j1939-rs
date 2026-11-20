//! # j1939-macros
//!
//! Procedural macros for the j1939-rs library.
//!
//! ## Note on `std` Usage
//!
//! This crate uses the standard library (`std`) and does **not** have `#![no_std]`.
//! This is intentional and correct:
//!
//! - **Proc macros run at compile time** on the developer's machine, not on the embedded target
//! - The Rust compiler **requires** proc macro crates to use `std`
//! - Dynamic allocations (Vec, String, HashMap) are used during code generation
//! - This code **never runs** on the embedded system
//!
//! The **generated code** from these macros is fully `no_std` compatible and runs on embedded
//! targets without heap allocation. See the `j1939-rs` crate documentation for details on
//! embedded system support.

#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use quote::quote;
use syn::Expr::Lit;
use syn::Meta::NameValue;
use syn::{Data, DeriveInput, ExprLit, Fields, parse_macro_input};

// Internal modules for macro implementation
mod codegen;
mod documentation_generation;
mod field_info;
mod mux;
mod parse;

use codegen::*;
use field_info::*;
use parse::*;

/// Attribute macro for defining J1939 message structures with automatic marshalling/unmarshalling.
///
/// This macro transforms a struct definition into a complete J1939 message with serialization
/// and deserialization capabilities. It generates implementations of the `Marshall` and `Unmarshall`
/// traits, as well as associated constants for message metadata.
///
/// # Parameters
///
/// - `pgn`: (required) Parameter Group Number, at most `0x3FFFF` (18 bits). A PDU1 PGN
///   (PF byte `(pgn >> 8) & 0xFF` below 240) must have a zero low byte, because the destination
///   address travels separately; anything else is a compile error.
/// - `priority`: (optional) Message priority (0-7, where 0 is highest; larger values are a
///   compile error). Default: 6
/// - `length`: (optional) Message length in **bytes**, 1..=223. Default: 8, growing to fit the fields
/// - `single_frame`: (optional flag) Compile error if the message is longer than 8 bytes
/// - `defmt`: (optional flag) Adds `#[derive(defmt::Format)]` to the generated struct. The derive
///   expands to `defmt::` paths, so the crate using this flag must depend on `defmt` itself.
///   Enum fields must implement `defmt::Format` too.
///
/// # Field Attributes
///
/// Each field must have a `#[j1939(...)]` attribute with the following parameters:
///
/// - `bits = start..end`: (required) Bit range for this field within the message
/// - `scale = f32`: (optional) Scaling factor for float conversion. Use with `f32` fields
/// - `offset = f32`: (optional) Offset applied to scaled values. Default: 0.0
///   - Formula: `raw = (value - offset) / scale`
/// - `encoding = "type"`: (optional) Special encoding type (e.g., "q9" for fixed-point)
/// - `unit = "string"`: (optional) Physical unit for documentation (e.g., "km/h", "°C")
/// - `byte_order = "big" | "little"`: (optional) Wire byte order of a multi-byte field. Default:
///   little. Big-endian fields must be byte-aligned with a width that is a multiple of 8
/// - `reserved`: (optional flag) Marks bits as reserved/unused
///
/// ## Encoding Strategies
///
/// The encoding strategy is automatically determined based on field type and attributes:
///
/// | Field Type | Attributes | Encoding | Description |
/// |------------|-----------|----------|-------------|
/// | `u8`..`u64` | - | UInt | Direct unsigned integer |
/// | `i8`..`i64` | - | SInt | Signed integer with sign extension |
/// | `f32` | `scale`, `offset?`, `signed?` | Scaled | `(raw * scale) + offset`; raw is unsigned unless `signed` |
/// | `Signal<T>` | - or `scale` | Signal | Value / not available / error / reserved (unsigned only) |
/// | `f32` | `encoding = "q9"` | Q9 | Fixed-point format (10 bits: 1 sign + 9 fractional) |
/// | Custom enum | - | Enum | Type-safe enum (requires `#[repr(u8)]` and `#[j1939_enum]`) |
/// | `()` | `reserved` | Reserved | Unused bits set to zero |
///
/// ### Signedness
///
/// Scaled raw values are **unsigned** unless the field has the `signed` flag; the signedness
/// is never inferred from `offset`.
///
/// # Generated Items
///
/// The macro generates:
/// - Constants: `PGN`, `PRIORITY`, `LENGTH`
/// - `Marshall` trait implementation for encoding
/// - `Unmarshall` trait implementation for decoding
/// - Comprehensive documentation table with field layout
///
/// # Examples
///
/// ## Basic Message with Scaled Values
///
/// ```ignore
/// use j1939_rs::prelude::*;
///
/// #[j1939_message(pgn = 61444, priority = 3, length = 8)]
/// pub struct EngineSpeed {
///     /// Engine RPM
///     #[j1939(bits = 0..16, scale = 0.125, unit = "rpm")]
///     pub engine_speed: f32,
///
///     /// Coolant temperature with offset
///     #[j1939(bits = 16..24, scale = 1.0, offset = -40.0, unit = "°C")]
///     pub coolant_temp: f32,
///
///     /// Reserved bits
///     #[j1939(bits = 24..64, reserved)]
///     pub reserved: (),
/// }
///
/// // Usage
/// let msg = EngineSpeed {
///     engine_speed: 1850.0,
///     coolant_temp: 85.0,
///     reserved: (),
/// };
///
/// let mut j1939_msg = J1939Message::default();
/// msg.marshall(&mut j1939_msg).unwrap();
///
/// let decoded = EngineSpeed::unmarshall(&j1939_msg).unwrap();
/// assert_eq!(decoded.engine_speed, 1850.0);
/// ```
///
/// ## Message with Enums
///
/// ```ignore
/// use j1939_rs::prelude::*;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// #[repr(u8)]
/// #[j1939_enum]
/// pub enum GearPosition {
///     Park = 0,
///     Reverse = 1,
///     Neutral = 2,
///     Drive = 3,
/// }
///
/// #[j1939_message(pgn = 0xF039, priority = 6)]
/// pub struct TransmissionStatus {
///     #[j1939(bits = 0..2)]
///     pub gear: GearPosition,
///
///     #[j1939(bits = 2..18, scale = 0.1, unit = "km/h")]
///     pub speed: f32,
///
///     #[j1939(bits = 18..64, reserved)]
///     pub reserved: (),
/// }
/// ```
///
/// ## Message with Q9 Fixed-Point Encoding
///
/// ```ignore
/// use j1939_rs::prelude::*;
///
/// #[j1939_message(pgn = 65400)]
/// pub struct ControlData {
///     /// High-precision control value using Q9 fixed-point
///     #[j1939(bits = 0..10, encoding = "q9")]
///     pub control_value: f32,
///
///     #[j1939(bits = 10..64, reserved)]
///     pub reserved: (),
/// }
/// ```
///
/// # Notes
///
/// - All bit ranges must be non-overlapping and within the message length
/// - Float fields require either `scale` or `encoding` attribute
/// - Scaled raw values are unsigned unless `signed` is given
/// - Reserved bits are automatically set to zero during marshalling
/// - Encoding types are inferred automatically from field types and attributes
///
/// # See Also
///
/// - [`j1939_enum`] - Attribute macro for registering enums used in messages
/// - [`j1939_mux`] - Attribute macro for multiplexed messages (an enum of `#[j1939_message]` structs)
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

/// Attribute macro for registering enums used in J1939 messages.
///
/// This macro registers enum information for automatic documentation generation. When an enum
/// marked with `#[j1939_enum]` is used as a field type in a message, the generated documentation
/// will include a comprehensive table showing all enum variants, their values, and descriptions.
///
/// # Requirements
///
/// - The enum must have `#[repr(u8)]` to ensure proper memory layout
/// - Variants can have explicit discriminant values or use implicit incrementing values
/// - Each variant can have doc comments which will appear in the generated documentation
///
/// # Benefits
///
/// - Automatic documentation table generation for enum fields in J1939 messages
/// - Variant values shown in both decimal and binary format
/// - Integration with the message field layout documentation
/// - Type-safe enum encoding/decoding
///
/// # Examples
///
/// ## Basic Enum Registration
///
/// ```ignore
/// use j1939_rs::prelude::*;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// #[repr(u8)]
/// #[j1939_enum]
/// /// Engine operating mode
/// pub enum EngineMode {
///     /// Engine is stopped
///     Stopped = 0,
///     /// Engine is starting
///     Starting = 1,
///     /// Engine running normally
///     Running = 2,
///     /// Engine has critical fault
///     Fault = 7,
/// }
/// ```
///
/// ## Enum with Implicit Values
///
/// ```ignore
/// use j1939_rs::prelude::*;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// #[repr(u8)]
/// #[j1939_enum]
/// /// Gear selection
/// pub enum GearPosition {
///     /// Park - vehicle locked
///     Park,      // = 0
///     /// Reverse gear
///     Reverse,   // = 1
///     /// Neutral - no gear engaged
///     Neutral,   // = 2
///     /// Drive mode
///     Drive,     // = 3
///     /// Low gear for climbing
///     Low,       // = 4
/// }
/// ```
///
/// ## Using Enums in Messages
///
/// ```ignore
/// use j1939_rs::prelude::*;
///
/// #[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// #[repr(u8)]
/// #[j1939_enum]
/// pub enum TorqueMode {
///     LowIdle = 0,
///     AcceleratorControl = 1,
///     CruiseControl = 2,
/// }
///
/// #[j1939_message(pgn = 61444, priority = 3)]
/// pub struct EngineController {
///     /// Current torque control mode
///     #[j1939(bits = 0..4)]
///     pub torque_mode: TorqueMode,
///
///     #[j1939(bits = 4..20, scale = 0.125, unit = "rpm")]
///     pub engine_speed: f32,
///
///     #[j1939(bits = 20..64, reserved)]
///     pub reserved: (),
/// }
/// ```
///
/// # Notes
///
/// - The macro preserves the original enum definition unchanged
/// - Enum information is stored in a compile-time registry for documentation generation
/// - Enums are transmitted as their underlying `u8` representation
/// - Use `#[repr(u8)]` to ensure consistent memory layout across platforms
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
            let value = if let Some((_, expr)) = &variant.discriminant
                && let Lit(ExprLit {
                    lit: syn::Lit::Int(lit_int),
                    ..
                }) = expr
            {
                current_value = lit_int.base10_parse().unwrap_or(current_value);
                current_value
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

    // Return the original enum plus a checked `TryFrom<u8>` (wire data is untrusted, so
    // unknown values must be reported, never transmuted).
    let enum_name = &input.ident;
    let vis = &input.vis;
    let attrs = &input.attrs;

    if let Data::Enum(enum_data) = &input.data {
        let variants = &enum_data.variants;

        if let Some(bad) = variants.iter().find(|v| !matches!(v.fields, Fields::Unit)) {
            return syn::Error::new_spanned(
                bad,
                "j1939_enum only supports fieldless (unit) variants",
            )
            .to_compile_error()
            .into();
        }
        let idents: Vec<_> = variants.iter().map(|v| &v.ident).collect();

        quote! {
            #(#attrs)*
            #vis enum #enum_name {
                #variants
            }

            impl core::convert::TryFrom<u8> for #enum_name {
                /// The unrecognised raw value.
                type Error = u8;

                fn try_from(raw: u8) -> core::result::Result<Self, u8> {
                    #( if raw == (#enum_name::#idents as u8) {
                        return core::result::Result::Ok(#enum_name::#idents);
                    } )*
                    core::result::Result::Err(raw)
                }
            }
        }
        .into()
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
            if attr.path().is_ident("doc")
                && let NameValue(nv) = &attr.meta
                && let Lit(ExprLit {
                    lit: syn::Lit::Str(s),
                    ..
                }) = &nv.value
            {
                return Some(s.value().trim().to_string());
            }
            None
        })
        .collect()
}

/// Attribute macro for multiplexed J1939 messages: one PGN whose layout depends on a selector.
///
/// Apply it to an enum of single-field tuple variants, each wrapping a `#[j1939_message]` struct
/// with the same PGN. `selector = a..b` gives the selector bit range; each variant carries
/// `#[j1939(selector = N)]`. The variant structs should mark the selector bits `reserved`.
///
/// `Marshall` delegates to the variant and then writes the selector, overwriting those bits.
/// `Unmarshall` reads the selector and delegates; an unregistered value gives
/// `Error::UnknownSelector`.
///
/// ```ignore
/// #[j1939_mux(selector = 0..8)]
/// pub enum Foo {
///     #[j1939(selector = 1)]
///     A(MsgA),
///     #[j1939(selector = 2)]
///     B(MsgB),
/// }
/// ```
#[proc_macro_attribute]
pub fn j1939_mux(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    match mux::expand(attr.into(), input) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
