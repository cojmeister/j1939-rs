pub mod q9;

/// Round f32s in a `no_std` environment.
///
/// # Examples
///
/// ```
/// use j1939_core::fixed_point::round_f32;
///
/// // Positive numbers:
/// assert_eq!(round_f32(123.123), 123.0f32);
/// assert_eq!(round_f32(10.623), 11.0f32);
///
/// // Negative number
/// assert_eq!(round_f32(-123.5), -124.0f32);
/// assert_eq!(round_f32(-10.4), -10.0f32);
/// ```
pub fn round_f32(x: f32) -> f32 {
    if x >= 0.0 {
        // For positive numbers, add 0.5 and truncate
        (x + 0.5) as i32 as f32
    } else {
        // For negative numbers, subtract 0.5 and truncate
        (x - 0.5) as i32 as f32
    }
}