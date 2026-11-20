//! `core`-only float → integer conversion helpers.
//!
//! `f32::round`/`trunc` are not available in `core` on stable Rust, so everything here is built on
//! `as` casts, which are available everywhere and *saturate* (NaN becomes 0).

/// How a fractional raw value is converted to an integer.
///
/// The default is [`RoundingMode::HalfAwayFromZero`] (ordinary "round to nearest").
/// Many C J1939 stacks compute `(int)(value * (1 << n))`, which **truncates toward zero**; select
/// [`RoundingMode::Truncate`] to be bit-for-bit compatible with those.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum RoundingMode {
    /// Drop the fractional part (round toward zero). Matches the C `(int)` cast.
    Truncate,
    /// Round to nearest; exact halves round away from zero (2.5 → 3, -2.5 → -3).
    #[default]
    HalfAwayFromZero,
}

/// Converts `x` to `i64` with the given rounding, saturating at the `i64` limits (NaN → 0).
pub fn f32_to_i64(x: f32, mode: RoundingMode) -> i64 {
    let t = x as i64; // truncates toward zero, saturating
    match mode {
        RoundingMode::Truncate => t,
        RoundingMode::HalfAwayFromZero => {
            let frac = x - t as f32;
            if frac >= 0.5 {
                t.saturating_add(1)
            } else if frac <= -0.5 {
                t.saturating_sub(1)
            } else {
                t
            }
        }
    }
}

/// Converts a physical-derived raw value to an **unsigned** field of `bits` bits (1..=64),
/// saturating to `[0, 2^bits - 1]`. Negative values and NaN give 0.
///
/// # Examples
///
/// ```
/// use j1939_core::{f32_to_raw_unsigned, RoundingMode};
/// // 6000 rpm at 0.125 rpm/bit
/// assert_eq!(f32_to_raw_unsigned(6000.0 / 0.125, 16, RoundingMode::default()), 48000);
/// assert_eq!(f32_to_raw_unsigned(1.0e9, 16, RoundingMode::default()), 65535);
/// ```
pub fn f32_to_raw_unsigned(x: f32, bits: u32, mode: RoundingMode) -> u64 {
    let max = if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    };
    if x.is_nan() || x <= 0.0 {
        // `x <= 0.0` can still round to 0 only; anything negative saturates to 0
        return 0;
    }
    // Values above i64::MAX only matter for 64-bit fields; `as u64` saturates correctly.
    let raw = match mode {
        RoundingMode::Truncate => x as u64,
        RoundingMode::HalfAwayFromZero => {
            let t = x as u64;
            if x - t as f32 >= 0.5 {
                t.saturating_add(1)
            } else {
                t
            }
        }
    };
    if raw > max { max } else { raw }
}

/// Converts a physical-derived raw value to a **signed** field of `bits` bits (1..=64),
/// saturating to `[-2^(bits-1), 2^(bits-1) - 1]`. NaN gives 0.
///
/// # Examples
///
/// ```
/// use j1939_core::{f32_to_raw_signed, RoundingMode};
/// assert_eq!(f32_to_raw_signed(-40000.0, 24, RoundingMode::default()), -40000);
/// assert_eq!(f32_to_raw_signed(-1.0e9, 16, RoundingMode::default()), -32768);
/// ```
pub fn f32_to_raw_signed(x: f32, bits: u32, mode: RoundingMode) -> i64 {
    let (min, max) = if bits >= 64 {
        (i64::MIN, i64::MAX)
    } else {
        (-(1i64 << (bits - 1)), (1i64 << (bits - 1)) - 1)
    };
    f32_to_i64(x, mode).clamp(min, max)
}

/// Rounds an `f32` to the nearest integer value (halves away from zero) without `std`.
///
/// # Examples
///
/// ```
/// use j1939_core::fixed_point::round_f32;
///
/// assert_eq!(round_f32(123.123), 123.0f32);
/// assert_eq!(round_f32(10.623), 11.0f32);
/// assert_eq!(round_f32(-123.5), -124.0f32);
/// assert_eq!(round_f32(-10.4), -10.0f32);
/// ```
pub fn round_f32(x: f32) -> f32 {
    f32_to_i64(x, RoundingMode::HalfAwayFromZero) as f32
}
