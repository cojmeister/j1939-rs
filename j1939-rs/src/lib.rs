#![no_std]
//! # j1939-rs
//!
//! A Rust library for working with SAE J1939 protocol messages in embedded and automotive systems.
//!
//! This library provides a powerful macro-based approach to defining, encoding, and decoding J1939
//! messages with compile-time validation and automatic documentation generation.
//!
//! ## Features
//!
//! - **Type-safe message definitions** using Rust structs
//! - **Automatic marshalling/unmarshalling** with the `Marshall` and `Unmarshall` traits
//! - **Bit-level field packing** with arbitrary bit ranges
//! - **Scaling and offset support** for physical unit conversions
//! - **Enum support** with automatic documentation generation
//! - **Compile-time validation** of message layouts
//! - **Comprehensive documentation** automatically generated from your definitions
//! - **`no_std` compatible** for embedded systems
//!
//! ## Quick Start
//!
//! Add this to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! j1939-rs = "0.1"
//! ```
//!
//! Define a J1939 message:
//!
//! ```rust
//! use j1939_rs::prelude::*;
//!
//! #[j1939_message(pgn = 61444, priority = 3)]
//! pub struct EngineController {
//!     /// Engine speed in RPM
//!     #[j1939(bits = 0..16, scale = 0.125, unit = "rpm")]
//!     pub engine_speed: f32,
//!
//!     /// Actual engine torque as percentage
//!     #[j1939(bits = 16..24, scale = 1.0, offset = -125.0, unit = "%")]
//!     pub actual_torque: f32,
//!
//!     #[j1939(bits = 24..64, reserved)]
//!     pub reserved: (),
//! }
//!
//! // Create and encode a message
//! let msg = EngineController {
//!     engine_speed: 1850.0,
//!     actual_torque: 45.0,
//!     reserved: (),
//! };
//!
//! let mut j1939_msg = J1939Message::default();
//! msg.marshall(&mut j1939_msg).unwrap();
//!
//! // Decode a message
//! let decoded = EngineController::unmarshall(&j1939_msg).unwrap();
//! assert_eq!(decoded.engine_speed, 1850.0);
//! ```
//!
//! ## Working with Enums
//!
//! ```rust
//! use j1939_rs::prelude::*;
//!
//! #[derive(Debug, Clone, Copy, PartialEq, Eq)]
//! #[repr(u8)]
//! #[j1939_enum]
//! /// Transmission gear selection
//! pub enum GearPosition {
//!     Park = 0,
//!     Reverse = 1,
//!     Neutral = 2,
//!     Drive = 3,
//! }
//!
//! #[j1939_message(pgn = 65265)]
//! pub struct TransmissionStatus {
//!     #[j1939(bits = 0..2)]
//!     pub current_gear: GearPosition,
//!
//!     #[j1939(bits = 2..64, reserved)]
//!     pub reserved: (),
//! }
//! ```
//!
//! ## Core Concepts
//!
//! ### Messages
//!
//! Messages are defined using the `#[j1939_message]` attribute macro on structs. Each message has:
//! - A **PGN (Parameter Group Number)** - unique identifier
//! - A **priority** level (0-7, where 0 is highest)
//! - A **length** in bits (defaults to 64 bits / 8 bytes)
//!
//! ### Fields
//!
//! Each field in a message struct must specify its bit range using `#[j1939(bits = start..end)]`.
//! Additional attributes control encoding:
//! - `scale`: Multiply/divide factor for float values
//! - `offset`: Additive offset for values (e.g., temperature with -40°C offset)
//! - `encoding`: Special encoding schemes like "q9" for fixed-point
//! - `unit`: Documentation string for physical units
//! - `reserved`: Mark unused bits
//!
//! ### Marshalling
//!
//! The `Marshall` trait encodes your struct into a `J1939Message`:
//!
//! ```rust
//! # use j1939_rs::prelude::*;
//! # #[j1939_message(pgn = 61444)]
//! # pub struct EngineController {
//! #     #[j1939(bits = 0..16, scale = 0.125)]
//! #     pub engine_speed: f32,
//! #     #[j1939(bits = 16..64, reserved)]
//! #     pub reserved: (),
//! # }
//! let msg = EngineController { engine_speed: 1850.0, reserved: () };
//! let mut j1939_msg = J1939Message::default();
//! msg.marshall(&mut j1939_msg)?;
//! # Ok::<(), j1939_core::Error>(())
//! ```
//!
//! ### Unmarshalling
//!
//! The `Unmarshall` trait decodes a `J1939Message` back into your struct:
//!
//! ```rust
//! # use j1939_rs::prelude::*;
//! # #[j1939_message(pgn = 61444)]
//! # pub struct EngineController {
//! #     #[j1939(bits = 0..16, scale = 0.125)]
//! #     pub engine_speed: f32,
//! #     #[j1939(bits = 16..64, reserved)]
//! #     pub reserved: (),
//! # }
//! # let j1939_msg = J1939Message::default();
//! let decoded = EngineController::unmarshall(&j1939_msg)?;
//! # Ok::<(), j1939_core::Error>(())
//! ```
//!
//! ## Prelude
//!
//! For convenience, import everything you need with:
//!
//! ```rust
//! use j1939_rs::prelude::*;
//! ```
//!
//! This brings in all macros, traits, and core types.
//!
//! ## See Also
//!
//! - [`j1939_message`] - Define J1939 message structures
//! - [`j1939_enum`] - Register enums for documentation
//! - [`Marshall`] - Trait for encoding messages
//! - [`Unmarshall`] - Trait for decoding messages

pub use j1939_core::*;
pub use j1939_macros::*;

/// Convenient prelude that imports all commonly used items.
///
/// # Examples
///
/// ```rust
/// use j1939_rs::prelude::*;
///
/// // Now you have access to:
/// // - j1939_message macro
/// // - j1939_enum macro
/// // - Marshall and Unmarshall traits
/// // - J1939Message type
/// // - Error and Result types
/// ```
pub mod prelude {
    pub use j1939_core::*;
    pub use j1939_macros::*;
}