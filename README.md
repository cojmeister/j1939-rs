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
- **Not-available / error signals**, **big-endian fields**, **multiplexed messages**, **DM1/DM2** and
  **multi-packet transport (BAM, RTS/CTS)**
- **`no_std` compatible** - works in bare-metal embedded environments
- **No heap allocations** - all operations use fixed-size stack buffers
- **Comprehensive documentation** automatically generated from your definitions

## Quick Start

Add this to your `Cargo.toml`:

```toml
[dependencies]
j1939-rs = "0.2"
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
| `u8`..`u64` | - | UInt | Direct unsigned integer, 1–64 bits, any bit offset |
| `i8`..`i64` | - | SInt | Two's-complement signed integer with sign extension |
| `f32` | `scale`, `offset?`, `signed?` | Scaled | `physical = raw * scale + offset`. Raw is **unsigned** unless `signed` is given (never inferred from `offset`) |
| `f32` | `q = N` | Q*N* | Signed fixed point, 1 sign bit + *N* fractional bits (`bits = N + 1`), range `[-1.0, 1.0 - 2^-N]` |
| `f32` | `uq = N` | UQ*N* | Unsigned full-scale fixed point, `raw / (2^N - 1)`, range `[0.0, 1.0]` (`bits = N`) |
| `f32` | `encoding = "q9"` | Q9 | Same as `q = 9` |
| `f32` | `q`/`uq` + `range = R` | ranged Q | Physical value is `q_value * R` (e.g. `q = 9, range = 90.0` for ±90°) |
| `Signal<u8>`..`Signal<u64>`, `Signal<f32>` | `scale`, `offset?` (f32) | Signal | Unsigned value, or not available / error / reserved (see below), at least 2 bits |
| Custom enum | - | Enum | Type-safe enum (requires `#[repr(u8)]` and `#[j1939_enum]`, at most 8 bits) |
| `()` | `reserved` | Reserved | Written as 0, ignored on decode |

### Message attributes

- `pgn = N` (required), `priority = N` (default 6).
- `length = N` is the message length in **bytes** (1..=223). Without it the message is 8 bytes, growing
  to fit the fields. Every bit up to `length * 8` must be covered by a field (use `reserved`).
- `single_frame`: compile error if the message is longer than 8 bytes. Also generates
  `Msg::SINGLE_FRAME: bool`.
- `defmt`: derives `defmt::Format` on the generated struct (see [defmt](#defmt)).
- `strict`: decoding requires `msg.length == LENGTH`. By default decoding accepts
  `msg.length >= LENGTH` and ignores trailing bytes, so a 6-byte PGN in a padded 8-byte frame decodes.

### Conversion rules

- **Saturation:** converting a float to a raw value clamps to the field's own raw range
  (`[0, 2^bits - 1]`, signed `[-2^(bits-1), 2^(bits-1) - 1]`, Q `[-2^N, 2^N - 1]`); NaN becomes 0.
  Nothing wraps. Integer fields are masked to their width.
- **Rounding:** the default is round-to-nearest, halves away from zero. Many C stacks truncate toward
  zero (`(int)(v * (1 << n))`); add `rounding = "truncate"` to a field to match them bit for bit.
- **Enums:** an undefined wire value makes `unmarshall` return `Error::InvalidEnum { field, raw }`.
  `#[j1939_enum]` also generates `TryFrom<u8>` (error type `u8`, the unknown raw value).
- `#![forbid(unsafe_code)]` is set in all crates.
- Minimum supported Rust version: 1.88.

## CAN bus integration

Enable the `embedded-can` feature to convert to and from any `embedded_can::Frame` implementation:

```rust,ignore
let id = J1939Id::new(3, 0x1F004, None, 0x10)?;   // keeps the DP bit: PGN 0x1F004 != 0xF004
let ext: embedded_can::ExtendedId = id.into();

let msg = J1939Message::from_frame(&rx_frame)?;     // 29-bit data frames only
let tx_frame: MyFrame = msg.to_frame()?;            // payloads up to 8 bytes
```

`J1939Id` packs/unpacks priority, EDP, DP, PF, PS and source address. The PGN is 18 bits including EDP
and DP; for PDU1 PGNs (PF < 240) the PS byte is the destination address (`J1939Message::destination_address`),
not part of the PGN. Multi-packet transport is described in [Transport protocol (TP)](#transport-protocol-tp);
`j1939_rs::tp::CanFrame` has `from_frame` / `to_frame` conversions with the same feature.

## Signals (not-available / error)

J1939 reserves the top raw values of a signal for special states. Declare the field as `Signal<T>`
to keep them apart from measurements:

```rust,ignore
use j1939_rs::prelude::*;

#[j1939_message(pgn = 0x2001, length = 3)]
struct Sensor {
    #[j1939(bits = 0..8)]
    a: Signal<u8>,
    #[j1939(bits = 8..24, scale = 0.125)]
    rpm: Signal<f32>,
}

// Signal<T> has the variants Value(T), NotAvailable, Error and Reserved(u64 raw).
let tx = Sensor { a: Signal::NotAvailable, rpm: Signal::Value(1850.0) };
match Sensor::unmarshall(&raw)?.a {
    Signal::Value(v) => { /* use v */ }
    Signal::NotAvailable | Signal::Error => { /* no measurement */ }
    Signal::Reserved(raw) => { /* preserved so it can be re-encoded unchanged */ }
}
// Signal::value() returns Some(v) only for Signal::Value(v).
```

- Supported: unsigned integers (`Signal<u8>`..`Signal<u64>`) and unsigned scaled `Signal<f32>`
  (`scale`, optional `offset`). No `signed`, `q`, `uq`, `encoding` or `reserved`, and the field needs at least 2 bits.
- Fields of 8 or more bits: the top byte (relative to the field width) selects the class:
  `0x00..=0xFA` valid, `0xFB..=0xFD` reserved, `0xFE` error, `0xFF` not available.
- Fields of 2 to 7 bits have no reserved class: all ones is not available, all ones minus one is error.
- Encoding a `Value` saturates at the largest valid raw value, so a measurement never collides with a
  special code. `NotAvailable` is written as all ones, `Error` as the first error code.

## Big-endian fields

Fields are little-endian by default. Add `byte_order = "big"` for a multi-byte field sent most
significant byte first:

```rust,ignore
#[j1939_message(pgn = 0x3001, length = 8)]
struct Be {
    #[j1939(bits = 0..8)]
    le_a: u8,
    #[j1939(bits = 8..24, byte_order = "big")]
    v: u16,
    #[j1939(bits = 24..56, byte_order = "big")]
    s: i32,
    #[j1939(bits = 56..64, byte_order = "little")]
    le_b: u8,
}
```

A big-endian field must start on a byte boundary and its width must be a multiple of 8
(otherwise the macro reports a compile error).

## Multiplexed messages

A multiplexed PGN has one selector field that decides the layout of the rest. Define one
`#[j1939_message]` struct per layout (all with the same PGN, selector bits marked `reserved`) and
tie them together with `#[j1939_mux]`:

```rust,ignore
#[j1939_message(pgn = 0xFF10, length = 8)]
struct MsgA {
    #[j1939(bits = 0..8, reserved)]
    _sel: (),
    #[j1939(bits = 8..24)]
    value: u16,
    #[j1939(bits = 24..64, reserved)]
    _r: (),
}

#[j1939_message(pgn = 0xFF10, length = 8)]
struct MsgB {
    #[j1939(bits = 0..8, reserved)]
    _sel: (),
    #[j1939(bits = 8..16)]
    count: u8,
    #[j1939(bits = 16..64, reserved)]
    _r: (),
}

#[j1939_mux(selector = 0..8)]
enum Foo {
    #[j1939(selector = 1)]
    A(MsgA),
    #[j1939(selector = 2)]
    B(MsgB),
}
```

`Foo` implements `Marshall` and `Unmarshall`. Marshalling delegates to the variant and then writes the
selector value into the selector bits; unmarshalling reads the selector and delegates. The selector
range is half-open (`a..b`) and each variant is a one-field tuple variant. An unregistered selector value
makes `unmarshall` return `Error::UnknownSelector`.

## DM1/DM2

`Dm1<N>` (active diagnostic trouble codes, PGN `0xFECA`) and `Dm2<N>` (previously active, PGN `0xFECB`,
a newtype around `Dm1<N>`) hold up to `N` DTCs without allocating:

```rust,ignore
use j1939_rs::prelude::*;

let mut dm = Dm1::<4>::new(LampStatus::default(), LampStatus::default());
dm.push(Dtc { spn: 110, fmi: 3, conversion_method: false, occurrence_count: 1 })?;

let mut msg = J1939Message::default();
dm.marshall(&mut msg)?;                  // priority 6, at least 8 bytes
let back = Dm1::<4>::unmarshall(&msg)?;
assert_eq!(back.dtcs().len(), 1);
```

- `LampStatus { malfunction, red_stop, amber_warning, protect }`: four 2-bit values (0..=3).
  `lamps` and `flash` are public fields of `Dm1`.
- `Dtc`: `spn` at most `0x7FFFF`, `fmi` at most 31, `occurrence_count` 0..=127 (127 means not available);
  out-of-range values make `encode` and `marshall` return `Error::InvalidLength`.
- A lone all-zero DTC is the wire's "no active DTC" marker: it decodes to an empty `dtcs()`.
  An empty `Dm1` is sent as that marker.
- `push` returns `Error::BufferTooSmall` when `N` DTCs are stored; decoding more than `N` DTCs does too.
  Messages longer than 8 bytes need the transport protocol below.

## Transport protocol (TP)

Messages of 9..=223 bytes travel as several CAN frames. `TpTx` and `TpRx` are sans-IO state machines:
you pass frames and a millisecond clock (`now_ms: u32`, wrapping is handled) and send whatever they return.

```rust,ignore
use j1939_rs::prelude::*;
use j1939_rs::tp::{TpRx, TpTx, TxState};

// Sender: BAM (broadcast) or RTS/CTS (to msg.destination_address).
let mut tx = TpTx::bam(&msg)?;                       // or TpTx::rts(&msg)?.with_max_per_cts(8)
while tx.state() == TxState::Active {
    if let Some(frame) = tx.poll(now_ms()) {         // frame: tp::CanFrame
        can_send(&frame);
    }
    // RTS/CTS only: feed the receiver's CTS/ACK/abort frames to tx.on_frame(&frame, now_ms())?
}

// Receiver: up to SESSIONS concurrent sessions; a source may run one BAM and one RTS/CTS at once.
let mut rx = TpRx::<2>::new(local_address);
let result = rx.on_frame(&frame, now_ms());          // RxResult { reply, message }
if let Some(reply) = result.reply { can_send(&reply); }
if let Some(msg) = result.message { /* reassembled J1939Message */ }
if let Some(abort) = rx.poll_timeout(now_ms()) { can_send(&abort); }  // call periodically
```

- `TpTx::bam` and `TpTx::rts` require `9 <= length <= 223`. `rts` needs a non-global destination
  (`Error::InvalidId` for 0xFF). `with_max_per_cts(n)` limits the packets the receiver may request per CTS.
- Both reject (with `Error::InvalidPgn`) PGNs the receiver cannot rebuild: a PDU1 PGN (PF < 240)
  must have a zero low byte.
- The reassembly buffer is `J1939Message`'s 223 bytes. A receiver answers an RTS announcing more with
  an abort (Resources); a BAM announcing more is dropped. Extended transport (ETP) is not implemented.
- The reassembled message takes the priority of the connection management frame, which is 7 for
  `TpTx`; the original message priority is not carried on the wire. An RTS with a size below 9 is ignored.
- `poll_timeout` drops sessions that went silent (T1/T2, constants in `tp::timing`); RTS/CTS sessions
  get an abort frame back, BAM sessions are dropped silently. It expires at most one session per call.
- Enable `embedded-can` to convert `tp::CanFrame` with `CanFrame::from_frame` / `to_frame`.

## defmt

Enable the `defmt` feature (on `j1939-rs` or `j1939-core`) for `defmt::Format` on the public types
(ids, messages, `Signal`, DM1/DM2, TP types, ...). `J1939Message` logs only its valid bytes.
To log your own messages add the `defmt` message flag:

```rust,ignore
#[j1939_message(pgn = 1, defmt)]
struct Logged {
    #[j1939(bits = 0..8)]
    a: u8,
    #[j1939(bits = 8..64, reserved)]
    _r: (),
}
```

The derive expands to `defmt::` paths, so your crate must depend on `defmt` itself, and enum
fields must implement `defmt::Format` too.

## Limitations

- Transport protocol messages are limited to 223 bytes; ETP (up to 1785 bytes) is not implemented.
- `byte_order = "big"` fields must be byte-aligned with a width that is a multiple of 8.
- `Signal<T>` supports unsigned integers and unsigned scaled `f32` only.
- Multiplexed selectors use a half-open bit range and tuple variants only.

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
- `J1939Message` buffer: 223 bytes maximum (`J1939Message::MAX_LENGTH`), `length` is a `u16` byte count
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
