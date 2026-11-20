//! Generic fixed-point formats: [`Q`] (signed) and [`UQ`] (unsigned, full-scale).
use super::rounding::{RoundingMode, f32_to_raw_signed, f32_to_raw_unsigned};

/// Signed fixed-point number with `N` fractional bits and one sign bit (`N + 1` bits total).
///
/// `value = raw / 2^N`, with `raw` in `[-2^N, 2^N - 1]`, i.e. a range of `[-1.0, 1.0 - 2^-N]`.
/// Conversion from floating point **saturates** to that range (it never wraps), so `1.0`
/// encodes as the largest positive raw value, not as `-1.0`.
///
/// `N` must be between 1 and 31.
///
/// # Examples
///
/// ```
/// use j1939_core::{Q, Q9, RoundingMode};
///
/// assert_eq!(Q9::from_float(1.0).to_raw(), 511);        // saturates
/// assert_eq!(Q9::from_float(-1.0).to_raw(), -512);
/// assert_eq!(Q::<7>::from_float(0.5).to_raw(), 64);      // Q7: 8 bits
/// assert_eq!(Q::<15>::BITS, 16);
///
/// // Rounding policy: default rounds to nearest, `Truncate` matches C `(int)` casts.
/// assert_eq!(Q9::from_float(0.0039).to_raw(), 2);
/// assert_eq!(Q9::from_float_with(0.0039, RoundingMode::Truncate).to_raw(), 1);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Q<const N: u32>(i32);

/// The Q9 format: 1 sign bit + 9 fractional bits (10 bits), range `[-1.0, 0.998046875]`.
pub type Q9 = Q<9>;

impl<const N: u32> Q<N> {
    const VALID: () = assert!(N >= 1 && N <= 31, "Q<N> requires 1 <= N <= 31");

    /// Number of bits the raw value occupies on the wire (`N + 1`).
    pub const BITS: u32 = {
        let () = Self::VALID;
        N + 1
    };
    const SCALE: f32 = (1u64 << N) as f32;

    /// Converts from floating point using the default rounding ([`RoundingMode::default`]),
    /// saturating to the representable range.
    pub fn from_float(value: f32) -> Self {
        Self::from_float_with(value, RoundingMode::default())
    }

    /// Converts from floating point with an explicit rounding mode, saturating to
    /// `[-2^N, 2^N - 1]`.
    pub fn from_float_with(value: f32, mode: RoundingMode) -> Self {
        Q(f32_to_raw_signed(value * Self::SCALE, Self::BITS, mode) as i32)
    }

    /// Converts back to floating point.
    pub fn to_float(self) -> f32 {
        self.0 as f32 / Self::SCALE
    }

    /// Wraps an already-scaled raw value (e.g. sign-extended from the wire).
    pub fn from_raw(raw: i32) -> Self {
        Q(raw)
    }

    /// Returns the raw scaled value.
    pub fn to_raw(self) -> i32 {
        self.0
    }
}

/// Unsigned full-scale fixed-point number with `N` bits: `value = raw / (2^N - 1)`.
///
/// The range is `[0.0, 1.0]` and raw is `[0, 2^N - 1]`; conversion saturates. `N` must be
/// between 1 and 32.
///
/// # Examples
///
/// ```
/// use j1939_core::UQ;
///
/// assert_eq!(UQ::<10>::from_float(1.0).to_raw(), 1023);
/// assert_eq!(UQ::<10>::from_float(5.0).to_raw(), 1023);
/// assert_eq!(UQ::<8>::from_raw(255).to_float(), 1.0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct UQ<const N: u32>(u32);

impl<const N: u32> UQ<N> {
    const VALID: () = assert!(N >= 1 && N <= 32, "UQ<N> requires 1 <= N <= 32");

    /// Number of bits the raw value occupies on the wire (`N`).
    pub const BITS: u32 = {
        let () = Self::VALID;
        N
    };
    const MAX_RAW: u32 = if N == 32 { u32::MAX } else { (1u32 << N) - 1 };

    /// Converts from floating point using the default rounding, saturating to `[0, 2^N - 1]`.
    pub fn from_float(value: f32) -> Self {
        Self::from_float_with(value, RoundingMode::default())
    }

    /// Converts from floating point with an explicit rounding mode.
    pub fn from_float_with(value: f32, mode: RoundingMode) -> Self {
        UQ(f32_to_raw_unsigned(value * Self::MAX_RAW as f32, Self::BITS, mode) as u32)
    }

    /// Converts back to floating point.
    pub fn to_float(self) -> f32 {
        self.0 as f32 / Self::MAX_RAW as f32
    }

    /// Wraps a raw value from the wire (masked to `N` bits).
    pub fn from_raw(raw: u32) -> Self {
        UQ(raw & Self::MAX_RAW)
    }

    /// Returns the raw value.
    pub fn to_raw(self) -> u32 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn q9_roundtrip() {
        for &v in &[0.0, 0.5, -0.5, 0.998, -1.0] {
            let recovered = Q9::from_float(v).to_float();
            assert!((recovered - v).abs() < 0.002, "Failed for {}", v);
        }
    }

    #[test]
    fn q9_boundary_values() {
        let max_positive = Q9::from_float(0.998_046_9);
        assert_eq!(max_positive.to_float(), 0.998_046_9);
        assert_eq!(max_positive.to_raw(), 511);
        assert_eq!(Q9::from_float(-1.0).to_raw(), -512);
        assert_eq!(Q9::from_float(0.0).to_raw(), 0);
    }

    #[test]
    fn q9_common_fractions() {
        for &(v, raw) in &[
            (0.5, 256),
            (0.25, 128),
            (0.125, 64),
            (-0.5, -256),
            (-0.25, -128),
        ] {
            let q = Q9::from_float(v);
            assert_eq!(q.to_raw(), raw);
            assert_eq!(q.to_float(), v);
        }
    }

    #[test]
    fn q9_resolution() {
        assert_eq!(Q9::from_raw(1).to_float(), 1.0 / 512.0);
        assert_eq!(Q9::from_raw(-1).to_float(), -1.0 / 512.0);
    }

    #[test]
    fn q9_out_of_range_saturates() {
        assert_eq!(Q9::from_float(100.0).to_raw(), 511);
        assert_eq!(Q9::from_float(-100.0).to_raw(), -512);
        assert_eq!(Q9::from_float(f32::NAN).to_raw(), 0);
    }

    #[test]
    fn q_equality_and_copy() {
        let a = Q9::from_float(0.5);
        let b = a;
        assert_eq!(a, b);
        assert_eq!(a, Q9::from_raw(256));
        assert_ne!(a, Q9::from_raw(257));
    }
}
