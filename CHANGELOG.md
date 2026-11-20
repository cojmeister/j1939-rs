# Changelog

## 0.2.0

Correctness release. **Breaking changes** are marked ⚠️.

### Fixed
- Fields wider than 16 bits were silently truncated; the bitfield API now handles 1–64 bit,
  unaligned fields spanning up to 9 bytes, and `u64`/`i64` fields are supported.
- Scaled encode saturated through `i16` (6000 rpm at 0.125/bit gave raw 32767). It now converts in
  the field's own type and saturates to the field's raw range.
- Generated code used `f32::round`, which does not exist in `core`, so `#[j1939_message]` structs
  with scaled fields did not build for `no_std` targets. All rounding goes through `core`-only
  helpers in `j1939-core` (verified with a `thumbv6m-none-eabi` build, `j1939-nostd-check`).
- Scaled decode inferred signedness from `offset`. It is now explicit (`signed`).
- Message length was a bit count capped at 223 bits (28 bytes). It is now a byte count up to 223.
- `Q9::from_float(1.0)` wrapped to −1.0 on the wire. Q conversion now saturates.
- Enum decode used `unsafe { transmute }`, which is UB for undefined wire values. It now returns an error.
- `unmarshall` rejected padded frames (e.g. a 6-byte PGN in an 8-byte frame).
- Signed integer fields wider than 16 bits (e.g. `i32` in 20 bits) overflowed.
- `#![forbid(unsafe_code)]` on all crates; `rust-version = "1.88"` declared.

### Added
- `Q<N>` (signed) and `UQ<N>` (unsigned, `2^N − 1` denominator) generic fixed-point types; `Q9 = Q<9>`.
- Field attributes `signed`, `q = N`, `uq = N`, `range = R`, `rounding = "round" | "truncate"`;
  message attribute `strict`.
- `RoundingMode`, `f32_to_raw_signed`, `f32_to_raw_unsigned`, `f32_to_i64`.
- `#[j1939_enum]` generates `TryFrom<u8>` (`Error = u8`).
- `Error::InvalidEnum { field, raw }`, `J1939Message::MAX_LENGTH`.
- `J1939Id`: 29-bit identifier (priority, 18-bit PGN **including EDP and DP**, PDU1 destination, source
  address) with `from_raw`/`to_raw`; `J1939Message::id()` and `J1939Message::from_id_and_data()`.
- `embedded-can` feature (also on `j1939-rs`): `J1939Message::from_frame` / `to_frame`, and conversions between
  `J1939Id` and `embedded_can::ExtendedId` / `Id` (11-bit ids and remote frames are rejected).
- `J1939Message::destination_address` (PDU1) and `Error::InvalidId`.
- `j1939-nostd-check` workspace crate: a build-only `no_std` consumer of the macros.

### ⚠️ Breaking
- `encode_bitfield` / `decode_bitfield` take/return `u64` and return `Result` (`InvalidLength` for a
  width of 0 or > 64, `BufferTooSmall` for out-of-bounds fields) instead of panicking or reading zeros.
- `sign_extend` takes `u64` and returns `i64`.
- `#[j1939_message(length = N)]` is now **bytes** (was bits). `LENGTH` is `u16`, and `J1939Message::length` is `u16`.
- Scaled fields are **unsigned by default**; add `signed` for signed raw values. (Previously a
  zero-offset scaled field was always sign-extended.)
- `Q9` is now an alias of `Q<9>`: `from_raw`/`to_raw` use `i32`; conversion saturates instead of wrapping.
  `round_f32` now handles the full range and `from_float` no longer wraps.
- Enum fields: unknown wire values yield `Err(Error::InvalidEnum { .. })`. `Error` has a new variant.
- Default decoding accepts `msg.length >= LENGTH`; use `strict` for the old exact match.
- Fields wider than their Rust type (e.g. 20 bits in a `u16`) and Q/UQ fields whose bit width does
  not match are now compile errors.
- `J1939Message` buffer is documented as 223 bytes (the README previously said 1785).
- `J1939Message` gained the public field `destination_address` (default 0xFF) and now derives `PartialEq, Eq`.
- `j1939-rs` depends on `j1939-core`/`j1939-macros` via the workspace path.
