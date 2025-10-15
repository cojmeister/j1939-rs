# j1939-rs

A Rust library for working with SAE J1939 protocol messages in embedded and automotive systems.

[![Crates.io](https://img.shields.io/crates/v/j1939-rs.svg)](https://crates.io/crates/j1939-rs)
[![Documentation](https://docs.rs/j1939-rs/badge.svg)](https://docs.rs/j1939-rs)
[![License](https://img.shields.io/crates/l/j1939-rs.svg)](LICENSE)

## Features

- **Type-safe message definitions** using Rust structs with compile-time validation
- **Automatic marshalling/unmarshalling** with zero-cost abstractions
- **Bit-level field packing** with arbitrary bit ranges (e.g., `bits = 5..13`)
- **Scaling and offset support** for physical unit conversions
- **Enum support** with automatic documentation generation
- **`no_std` compatible** - works in bare-metal embedded environments
- **No heap allocations** - all operations use fixed-size stack buffers
- **Comprehensive documentation** automatically generated from your definitions

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
j1939-rs = "0.1"
```

Define a J1939 message:

```rust
use j1939_rs::prelude::*;

#[j1939_message(pgn = 61444, priority = 3)]
pub struct ElectronicEngineController1 {
    /// Engine speed in RPM
    #[j1939(bits = 0..16, scale = 0.125, unit = "rpm")]
    pub engine_speed: f32,

    /// Actual engine torque as percentage
    #[j1939(bits = 16..24, scale = 1.0, offset = -125.0, unit = "%")]
    pub actual_torque: f32,

    #[j1939(bits = 24..64, reserved)]
    pub reserved: (),
}

fn main() {
    // Create and encode a message
    let msg = ElectronicEngineController1 {
        engine_speed: 1850.0,
        actual_torque: 45.0,
        reserved: (),
    };

    let mut j1939_msg = J1939Message::default();
    msg.marshall(&mut j1939_msg).unwrap();

    // Decode a message
    let decoded = ElectronicEngineController1::unmarshall(&j1939_msg).unwrap();
    assert_eq!(decoded.engine_speed, 1850.0);
}
```

## Working with Enums

Define type-safe enums for message fields:

```rust
use j1939_rs::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[j1939_enum]
pub enum TorqueMode {
    LowIdle = 0,
    AcceleratorControl = 1,
    CruiseControl = 2,
    SpeedControl = 3,
}

#[j1939_message(pgn = 61444)]
pub struct EngineController {
    #[j1939(bits = 0..4)]
    pub torque_mode: TorqueMode,

    #[j1939(bits = 4..20, scale = 0.125, unit = "rpm")]
    pub engine_speed: f32,

    #[j1939(bits = 20..64, reserved)]
    pub reserved: (),
}
```

## Encoding Strategies

The library supports multiple encoding strategies for different data types:

| Field Type | Attributes | Encoding | Description |
|------------|-----------|----------|-------------|
| `u8`, `u16`, `u32` | - | UInt | Direct unsigned integer |
| `i8`, `i16`, `i32` | - | SInt | Signed integer with sign extension |
| `f32` | `scale`, `offset?` | Scaled | Linear transformation: `(raw * scale) + offset` |
| `f32` | `encoding = "q9"` | Q9 | Fixed-point format (10 bits: 1 sign + 9 fractional) |
| Custom enum | - | Enum | Type-safe enum (requires `#[repr(u8)]` and `#[j1939_enum]`) |
| `()` | `reserved` | Reserved | Unused bits set to zero |

## Embedded Systems Support

This library is fully `no_std` compatible and designed for embedded systems:

```rust
#![no_std]
use j1939_rs::prelude::*;

#[j1939_message(pgn = 61444)]
pub struct EngineSpeed {
    #[j1939(bits = 0..16, scale = 0.125)]
    pub rpm: f32,

    #[j1939(bits = 16..64, reserved)]
    pub reserved: (),
}

// All of this works without std or heap allocations
fn send_engine_data() {
    let msg = EngineSpeed { rpm: 1850.0, reserved: () };
    let mut buffer = J1939Message::default();
    msg.marshall(&mut buffer).unwrap();
    // Send buffer over CAN bus...
}
```

### Memory Usage

- Message structures: Stack-allocated, fixed size
- `J1939Message` buffer: 1785 bytes maximum (SAE J1939 multi-packet limit)
- No heap allocations during encoding/decoding
- All transformations compile to inline arithmetic

## Documentation

Comprehensive documentation is available at [docs.rs/j1939-rs](https://docs.rs/j1939-rs).

The `#[j1939_message]` macro automatically generates detailed documentation tables for each message, including:
- Field layout with bit positions
- Physical units and scaling factors
- Enum value tables with descriptions
- Example usage

## Examples

Check out the [examples](j1939-rs/examples/) directory for complete working examples:

- `electronic_engine_controller_1.rs` - Standard J1939 EEC1 message with enums and scaling

Run an example with:

```bash
cargo run --example electronic_engine_controller_1
```

## Project Structure

This workspace contains three crates:

- **j1939-core** - Core types and bit manipulation functions (`no_std`)
- **j1939-macros** - Procedural macros for message definitions (compile-time only)
- **j1939-rs** - Main library that combines core and macros

## Testing

The project includes comprehensive tests:

- Unit tests for core functionality
- Integration tests for message encoding/decoding
- Property-based tests using `proptest`
- Compile-fail tests for macro error messages using `trybuild`

Run all tests with:

```bash
cargo test --all
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

This library implements the SAE J1939 protocol as specified in the SAE J1939 standards for vehicle networking.
