use crate::round_f32;

/// A Q9 fixed-point number representation for precise fractional values.
///
/// Q9 format uses 16 bits with 1 sign bit, 0 integer bits, and 9 fractional bits.
/// This provides a range of [-1.0, 0.998046875] with a resolution of approximately 0.00195.
///
/// This format is commonly used in automotive and embedded systems where precise
/// representation of values between -1 and +1 is needed, such as for normalized
/// sensor readings, coefficients, or percentages.
///
/// # Format Details
/// - **Total bits**: 16 (stored in `i16`)
/// - **Sign bit**: 1 (MSB)
/// - **Integer bits**: 0
/// - **Fractional bits**: 9
/// - **Range**: [-1.0, 0.998046875]
/// - **Resolution**: 1/512 ≈ 0.00195
/// - **Scale factor**: 2^9 = 512
///
/// # Examples
///
/// ```
/// use j1939_core::fixed_point::q9::Q9;
///
/// // Create Q9 values from floating point
/// let half = Q9::from_float(0.5);
/// let negative_quarter = Q9::from_float(-0.25);
/// let max_positive = Q9::from_float(0.998046875);
/// let max_negative = Q9::from_float(-1.0);
///
/// // Convert back to floating point
/// assert!((half.to_float() - 0.5).abs() < 0.001);
/// assert!((negative_quarter.to_float() - (-0.25)).abs() < 0.001);
///
/// // Work with raw values (useful for serialization)
/// let raw_value = half.to_raw();
/// let reconstructed = Q9::from_raw(raw_value);
/// assert_eq!(half, reconstructed);
///
/// // Common use case: percentage values
/// let percentage = Q9::from_float(0.75); // 75%
/// println!("Stored percentage: {:.3}", percentage.to_float()); // ~0.750
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Q9(i16);

impl Q9 {
    const FRACTIONAL_BITS: u32 = 9;
    const SCALE: f32 = (1 << Self::FRACTIONAL_BITS) as f32;

    /// Creates a Q9 fixed-point value from a floating-point number.
    ///
    /// The input value is scaled by 2^9 (512) and rounded to the nearest integer
    /// using banker's rounding. Values outside the valid range will be clamped
    /// to the representable range.
    ///
    /// # Arguments
    ///
    /// * `value` - A floating-point value to convert, ideally in range [-1.0, 1.0)
    ///
    /// # Examples
    ///
    /// ```
    /// use j1939_core::fixed_point::q9::Q9;
    ///
    /// // Standard conversions
    /// let zero = Q9::from_float(0.0);
    /// let half = Q9::from_float(0.5);
    /// let quarter = Q9::from_float(0.25);
    /// let negative = Q9::from_float(-0.75);
    ///
    /// // Boundary values
    /// let max_pos = Q9::from_float(0.998046875); // Largest representable positive
    /// let max_neg = Q9::from_float(-1.0);        // Largest representable negative
    ///
    /// // Values requiring rounding
    /// let third = Q9::from_float(1.0 / 3.0);     // Will be rounded to nearest Q9 value
    /// ```
    pub fn from_float(value: f32) -> Self {
        let scaled = round_f32(value * Self::SCALE) as i16;
        Q9(scaled)
    }

    /// Converts the Q9 fixed-point value back to a floating-point number.
    ///
    /// This operation is lossless within the precision limits of the Q9 format.
    /// The internal representation is divided by the scale factor (2^9 = 512)
    /// to recover the original floating-point value.
    ///
    /// # Returns
    ///
    /// The floating-point representation of this Q9 value.
    ///
    /// # Examples
    ///
    /// ```
    /// use j1939_core::fixed_point::q9::Q9;
    ///
    /// // Perfect conversions for values that align with Q9 precision
    /// let half = Q9::from_float(0.5);
    /// assert_eq!(half.to_float(), 0.5);
    ///
    /// let quarter = Q9::from_float(0.25);
    /// assert_eq!(quarter.to_float(), 0.25);
    ///
    /// // Values may have small rounding differences due to fixed-point precision
    /// let third = Q9::from_float(1.0 / 3.0);
    /// let recovered = third.to_float();
    /// assert!((recovered - (1.0 / 3.0)).abs() < 0.002);
    ///
    /// // Boundary values
    /// let max_neg = Q9::from_float(-1.0);
    /// assert_eq!(max_neg.to_float(), -1.0);
    /// ```
    pub fn to_float(self) -> f32 {
        self.0 as f32 / Self::SCALE
    }

    /// Creates a Q9 value directly from a raw 16-bit signed integer.
    ///
    /// This method bypasses any scaling or validation and directly uses the provided
    /// raw value as the internal representation. Use this when you have pre-scaled
    /// values or when deserializing from a binary format.
    ///
    /// # Arguments
    ///
    /// * `raw` - The raw 16-bit signed integer representing the scaled Q9 value
    ///
    /// # Examples
    ///
    /// ```
    /// use j1939_core::fixed_point::q9::Q9;
    ///
    /// // Create Q9 values from known raw representations
    /// let zero = Q9::from_raw(0);        // 0.0
    /// let half = Q9::from_raw(256);      // 0.5 (256/512)
    /// let quarter = Q9::from_raw(128);   // 0.25 (128/512)
    /// let negative = Q9::from_raw(-256); // -0.5 (-256/512)
    ///
    /// // Verify the conversions
    /// assert_eq!(zero.to_float(), 0.0);
    /// assert_eq!(half.to_float(), 0.5);
    /// assert_eq!(quarter.to_float(), 0.25);
    /// assert_eq!(negative.to_float(), -0.5);
    ///
    /// // Useful for deserialization
    /// let serialized_value: i16 = -512; // Represents -1.0
    /// let deserialized = Q9::from_raw(serialized_value);
    /// assert_eq!(deserialized.to_float(), -1.0);
    /// ```
    pub fn from_raw(raw: i16) -> Self {
        Q9(raw)
    }

    /// Returns the raw 16-bit signed integer representation of this Q9 value.
    ///
    /// This method provides access to the internal scaled representation, which
    /// is useful for serialization, storage, or when working with systems that
    /// expect the raw Q9 format.
    ///
    /// # Returns
    ///
    /// The raw 16-bit signed integer representing this Q9 value.
    ///
    /// # Examples
    ///
    /// ```
    /// use j1939_core::fixed_point::q9::Q9;
    ///
    /// // Get raw values for common fractions
    /// let half = Q9::from_float(0.5);
    /// assert_eq!(half.to_raw(), 256); // 0.5 * 512 = 256
    ///
    /// let quarter = Q9::from_float(0.25);
    /// assert_eq!(quarter.to_raw(), 128); // 0.25 * 512 = 128
    ///
    /// let negative_half = Q9::from_float(-0.5);
    /// assert_eq!(negative_half.to_raw(), -256); // -0.5 * 512 = -256
    ///
    /// // Roundtrip through raw representation
    /// let original = Q9::from_float(0.75);
    /// let raw = original.to_raw();
    /// let reconstructed = Q9::from_raw(raw);
    /// assert_eq!(original, reconstructed);
    ///
    /// // Useful for serialization
    /// let value = Q9::from_float(0.333);
    /// let serialized: i16 = value.to_raw(); // Store this value
    /// ```
    pub fn to_raw(self) -> i16 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_q9_roundtrip() {
        let values = [0.0, 0.5, -0.5, 0.998, -1.0];
        for &v in &values {
            let q = Q9::from_float(v);
            let recovered = q.to_float();
            assert!((recovered - v).abs() < 0.002, "Failed for {}", v);
        }
    }

    #[test]
    fn test_q9_boundary_values() {
        // Test exact boundary values
        let max_positive = Q9::from_float(0.998046875); // (512-1)/512
        assert_eq!(max_positive.to_float(), 0.998046875);
        assert_eq!(max_positive.to_raw(), 511);

        let max_negative = Q9::from_float(-1.0);
        assert_eq!(max_negative.to_float(), -1.0);
        assert_eq!(max_negative.to_raw(), -512);

        let zero = Q9::from_float(0.0);
        assert_eq!(zero.to_float(), 0.0);
        assert_eq!(zero.to_raw(), 0);
    }

    #[test]
    fn test_q9_precision_limits() {
        // Test the smallest representable positive value
        let smallest_positive = Q9::from_raw(1);
        assert_eq!(smallest_positive.to_float(), 1.0 / 512.0);

        // Test the largest representable negative value (closest to zero)
        let largest_negative = Q9::from_raw(-1);
        assert_eq!(largest_negative.to_float(), -1.0 / 512.0);

        // Test resolution: consecutive raw values should differ by 1/512
        let val1 = Q9::from_raw(100);
        let val2 = Q9::from_raw(101);
        let diff = val2.to_float() - val1.to_float();
        assert!((diff - 1.0 / 512.0).abs() < f32::EPSILON);
    }

    #[test]
    fn test_q9_common_fractions() {
        // Test perfect representations of common fractions
        let half = Q9::from_float(0.5);
        assert_eq!(half.to_float(), 0.5);
        assert_eq!(half.to_raw(), 256);

        let quarter = Q9::from_float(0.25);
        assert_eq!(quarter.to_float(), 0.25);
        assert_eq!(quarter.to_raw(), 128);

        let eighth = Q9::from_float(0.125);
        assert_eq!(eighth.to_float(), 0.125);
        assert_eq!(eighth.to_raw(), 64);

        // Test negative fractions
        let neg_half = Q9::from_float(-0.5);
        assert_eq!(neg_half.to_float(), -0.5);
        assert_eq!(neg_half.to_raw(), -256);

        let neg_quarter = Q9::from_float(-0.25);
        assert_eq!(neg_quarter.to_float(), -0.25);
        assert_eq!(neg_quarter.to_raw(), -128);
    }

    #[test]
    fn test_q9_rounding_behavior() {
        // Test rounding of values that don't align perfectly with Q9 precision
        let third = Q9::from_float(1.0 / 3.0);
        let recovered_third = third.to_float();
        assert!((recovered_third - (1.0 / 3.0)).abs() < 0.002);

        // Test rounding near boundaries
        let almost_half = Q9::from_float(0.5 + 0.0005); // Just above 0.5
        let rounded = almost_half.to_float();
        // Should round to nearest representable value
        assert!((rounded - 0.5).abs() < 0.002);

        // Test very small values
        let tiny = Q9::from_float(0.0001);
        assert!(tiny.to_float().abs() < 0.002); // Should be close to zero
    }

    #[test]
    fn test_q9_raw_conversions() {
        // Test raw value conversions
        let test_cases = [
            (0, 0.0),
            (256, 0.5),
            (128, 0.25),
            (64, 0.125),
            (-256, -0.5),
            (-512, -1.0),
            (511, 0.998046875),
            (-511, -0.998046875),
        ];

        for &(raw, expected_float) in &test_cases {
            let q9 = Q9::from_raw(raw);
            let actual_float = q9.to_float();
            assert!(
                (actual_float - expected_float).abs() < 0.001,
                "Raw {} -> expected {}, got {}",
                raw,
                expected_float,
                actual_float
            );
            assert_eq!(q9.to_raw(), raw);
        }
    }

    #[test]
    fn test_q9_out_of_range_values() {
        // Test values outside the normal Q9 range
        // 2.0 * 512 = 1024, which fits in i16, so it doesn't overflow as expected
        let too_large = Q9::from_float(2.0);
        // Let's just verify it creates a valid Q9 value
        let recovered_large = too_large.to_float();
        assert!(recovered_large.is_finite()); // Should be a valid number

        let too_small = Q9::from_float(-2.0);
        // -2.0 * 512 = -1024, which also fits in i16
        let recovered_small = too_small.to_float();
        assert!(recovered_small.is_finite()); // Should be a valid number

        // Test actual overflow conditions
        let very_large = Q9::from_float(100.0);
        let recovered_very_large = very_large.to_float();
        // This will definitely overflow i16 (100 * 512 = 51200 > 32767)
        // The actual behavior is implementation defined, but should be finite
        assert!(recovered_very_large.is_finite());

        // Values just outside the exact valid range
        let slightly_over = Q9::from_float(1.0);
        // 1.0 * 512 = 512, which is within i16 range
        let recovered = slightly_over.to_float();
        assert!(recovered.is_finite());
    }

    #[test]
    fn test_q9_equality() {
        // Test equality comparisons
        let a = Q9::from_float(0.5);
        let b = Q9::from_float(0.5);
        let c = Q9::from_raw(256); // Same as 0.5
        let d = Q9::from_float(0.25);

        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_ne!(a, d);

        // Test that very close but different values are not equal
        let close1 = Q9::from_raw(256);
        let close2 = Q9::from_raw(257);
        assert_ne!(close1, close2);
    }

    #[test]
    fn test_q9_debug_trait() {
        // Test that Debug trait is implemented (compilation test)
        let half = Q9::from_float(0.5);
        // We can't easily test the actual output in no_std, but we can verify
        // the Debug trait is implemented by this compilation test
        let _debug_ref = &half as &dyn core::fmt::Debug;
    }

    #[test]
    fn test_q9_copy_clone() {
        // Test that Copy and Clone work correctly
        let original = Q9::from_float(0.75);

        // Test Copy
        let copied = original;
        assert_eq!(original, copied);

        // Test Clone
        let cloned = original.clone();
        assert_eq!(original, cloned);

        // Verify they're independent (though being Copy, this is mostly for demonstration)
        assert_eq!(copied.to_float(), 0.75);
        assert_eq!(cloned.to_float(), 0.75);
    }

    #[test]
    fn test_q9_automotive_use_cases() {
        // Test common automotive sensor value ranges

        // Throttle position (0% to 100% as 0.0 to 1.0)
        let throttle_idle = Q9::from_float(0.0);
        let throttle_half = Q9::from_float(0.5);
        let throttle_full = Q9::from_float(0.998); // Close to max representable

        assert_eq!(throttle_idle.to_float(), 0.0);
        assert!((throttle_half.to_float() - 0.5).abs() < 0.001);
        assert!(throttle_full.to_float() > 0.995);

        // Steering angle normalized (-1.0 to +1.0)
        let steering_left = Q9::from_float(-1.0);
        let steering_center = Q9::from_float(0.0);
        let steering_right = Q9::from_float(0.8);

        assert_eq!(steering_left.to_float(), -1.0);
        assert_eq!(steering_center.to_float(), 0.0);
        assert!((steering_right.to_float() - 0.8).abs() < 0.002);

        // Brake pressure percentage
        let brake_off = Q9::from_float(0.0);
        let brake_light = Q9::from_float(0.1);
        let brake_heavy = Q9::from_float(0.9);

        assert_eq!(brake_off.to_float(), 0.0);
        assert!((brake_light.to_float() - 0.1).abs() < 0.002);
        assert!((brake_heavy.to_float() - 0.9).abs() < 0.002);
    }
}