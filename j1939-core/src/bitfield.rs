/// Encodes a value into a bitfield at the specified bit offset.
///
/// This function takes a value and writes it into a byte array at a specific bit position,
/// allowing for precise bit-level data packing. The function handles encoding across byte
/// boundaries and properly masks the input value to fit within the specified bit length.
///
/// # Arguments
///
/// * `data` - A mutable slice of bytes where the value will be encoded
/// * `bit_offset` - The starting bit position (0-based) where encoding begins
/// * `bit_length` - The number of bits to use for encoding (1-16)
/// * `value` - The value to encode (will be masked to fit in bit_length bits)
///
/// # Examples
///
/// ```
/// use j1939_core::bitfield::encode_bitfield;
///
/// // Encode a single byte value
/// let mut data = [0u8; 4];
/// encode_bitfield(&mut data, 0, 8, 0xAB);
/// assert_eq!(data[0], 0xAB);
///
/// // Encode a value across byte boundaries
/// let mut data = [0u8; 4];
/// encode_bitfield(&mut data, 6, 10, 0x3FF); // 10 bits: 1111111111
/// // This spans from bit 6 of byte 0 to bit 0 of byte 2
///
/// // Encode multiple values in the same array
/// let mut data = [0u8; 4];
/// encode_bitfield(&mut data, 0, 4, 0xA);    // First 4 bits: 1010
/// encode_bitfield(&mut data, 4, 4, 0x5);    // Next 4 bits: 0101
/// assert_eq!(data[0], 0x5A); // Combined: 01011010
/// ```
pub fn encode_bitfield(data: &mut [u8], bit_offset: usize, bit_length: usize, value: u16) {
    let mask = if bit_length >= 16 {
        0xFFFF
    } else {
        (1u16 << bit_length) - 1
    };
    let masked_value = value & mask;

    for i in 0..bit_length {
        let bit_pos = bit_offset + i;
        let byte_idx = bit_pos / 8;
        let bit_in_byte = bit_pos % 8;

        if (masked_value >> i) & 1 == 1 {
            data[byte_idx] |= 1 << bit_in_byte;
        } else {
            data[byte_idx] &= !(1 << bit_in_byte);
        }
    }
}

/// Decodes a value from a bitfield at the specified bit offset.
///
/// This function extracts a value from a byte array starting at a specific bit position,
/// allowing for precise bit-level data unpacking. The function handles decoding across byte
/// boundaries and safely handles out-of-bounds access by treating missing bytes as zero.
///
/// # Arguments
///
/// * `data` - A slice of bytes to read from
/// * `bit_offset` - The starting bit position (0-based) where decoding begins
/// * `bit_length` - The number of bits to decode (1-16)
///
/// # Returns
///
/// Returns the decoded value as a `u16`. If `bit_length` is 0 or greater than 16,
/// the behavior is implementation-defined but safe.
///
/// # Examples
///
/// ```
/// use j1939_core::bitfield::decode_bitfield;
///
/// // Decode a single byte value
/// let data = [0xAB, 0x00, 0x00, 0x00];
/// let value = decode_bitfield(&data, 0, 8);
/// assert_eq!(value, 0xAB);
///
/// // Decode a value across byte boundaries
/// let data = [0xFF, 0xFF, 0x03, 0x00]; // Contains 0x3FF at bit offset 6
/// let value = decode_bitfield(&data, 6, 10);
/// assert_eq!(value, 0x3FF);
///
/// // Decode from the middle of a byte
/// let data = [0x5A, 0x00, 0x00, 0x00]; // 01011010
/// let lower_nibble = decode_bitfield(&data, 0, 4);  // Gets 1010 = 0xA
/// let upper_nibble = decode_bitfield(&data, 4, 4);  // Gets 0101 = 0x5
/// assert_eq!(lower_nibble, 0xA);
/// assert_eq!(upper_nibble, 0x5);
///
/// // Safe handling of out-of-bounds access
/// let data = [0xFF];
/// let value = decode_bitfield(&data, 4, 8); // Partially out of bounds
/// assert_eq!(value, 0x0F); // Only the available 4 bits are set
/// ```
pub fn decode_bitfield(data: &[u8], bit_offset: usize, bit_length: usize) -> u16 {
    let mut value = 0u16;

    for i in 0..bit_length {
        let bit_pos = bit_offset + i;
        let byte_idx = bit_pos / 8;
        let bit_in_byte = bit_pos % 8;

        if byte_idx < data.len() && (data[byte_idx] >> bit_in_byte) & 1 == 1 {
            value |= 1 << i;
        }
    }

    value
}

/// Sign-extends a value based on its bit length.
///
/// This function converts an unsigned value to a signed value by interpreting the most
/// significant bit as a sign bit and extending it to fill the remaining bits of an `i16`.
/// This is commonly used when decoding signed integers from bitfields where the original
/// value was stored in fewer than 16 bits.
///
/// # Arguments
///
/// * `value` - The unsigned value to sign-extend
/// * `bit_length` - The number of bits that were used to represent the original signed value (1-16)
///
/// # Returns
///
/// Returns the sign-extended value as an `i16`. For edge cases:
/// * If `bit_length` is 0 or greater than 16, returns `value` cast to `i16` without modification
/// * If `bit_length` is 1, treats bit 0 as the sign bit
///
/// # Examples
///
/// ```
/// use j1939_core::bitfield::sign_extend;
///
/// // 8-bit signed values
/// assert_eq!(sign_extend(0x7F, 8), 127);   // Positive: 01111111
/// assert_eq!(sign_extend(0x80, 8), -128);  // Negative: 10000000
/// assert_eq!(sign_extend(0xFF, 8), -1);    // Negative: 11111111
///
/// // 4-bit signed values
/// assert_eq!(sign_extend(0x7, 4), 7);      // Positive: 0111
/// assert_eq!(sign_extend(0x8, 4), -8);     // Negative: 1000
/// assert_eq!(sign_extend(0xF, 4), -1);     // Negative: 1111
///
/// // 12-bit signed values (common in J1939)
/// assert_eq!(sign_extend(0x7FF, 12), 2047);    // Positive: 011111111111
/// assert_eq!(sign_extend(0x800, 12), -2048);   // Negative: 100000000000
/// assert_eq!(sign_extend(0xFFF, 12), -1);      // Negative: 111111111111
///
/// // Edge case: 1-bit signed (just sign bit)
/// assert_eq!(sign_extend(0x0, 1), 0);      // Positive: 0
/// assert_eq!(sign_extend(0x1, 1), -1);     // Negative: 1
/// ```
pub fn sign_extend(value: u16, bit_length: usize) -> i16 {
    if bit_length == 0 || bit_length > 16 {
        return value as i16;
    }

    // Handle 16-bit case specially to avoid shift overflow
    if bit_length == 16 {
        return value as i16;
    }

    let sign_bit = 1u16 << (bit_length - 1);

    if value & sign_bit != 0 {
        // Negative number - extend with 1s
        let extension_mask = !((1u16 << bit_length) - 1);
        (value | extension_mask) as i16
    } else {
        // Positive number
        value as i16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_single_byte() {
        let mut data = [0u8; 8];
        encode_bitfield(&mut data, 0, 8, 0xAB);
        assert_eq!(decode_bitfield(&data, 0, 8), 0xAB);
    }

    #[test]
    fn test_encode_decode_across_boundary() {
        let mut data = [0u8; 8];
        encode_bitfield(&mut data, 6, 10, 0x3FF);
        assert_eq!(decode_bitfield(&data, 6, 10), 0x3FF);
    }

    #[test]
    fn test_sign_extend_positive() {
        assert_eq!(sign_extend(0b0_1111_1111, 10), 255);
    }

    #[test]
    fn test_sign_extend_negative() {
        // 10-bit: 0b11_1111_1111 = -1
        assert_eq!(sign_extend(0b11_1111_1111, 10), -1);
        // 10-bit: 0b10_0000_0000 = -512
        assert_eq!(sign_extend(0b10_0000_0000, 10), -512);
    }

    #[test]
    fn test_encode_bitfield_single_bit() {
        let mut data = [0u8; 4];

        // Set bit 0
        encode_bitfield(&mut data, 0, 1, 1);
        assert_eq!(data[0], 0b00000001);

        // Set bit 7
        encode_bitfield(&mut data, 7, 1, 1);
        assert_eq!(data[0], 0b10000001);

        // Clear bit 0
        encode_bitfield(&mut data, 0, 1, 0);
        assert_eq!(data[0], 0b10000000);
    }

    #[test]
    fn test_encode_bitfield_nibbles() {
        let mut data = [0u8; 4];

        // Encode lower nibble
        encode_bitfield(&mut data, 0, 4, 0xA);
        assert_eq!(data[0], 0x0A);

        // Encode upper nibble
        encode_bitfield(&mut data, 4, 4, 0x5);
        assert_eq!(data[0], 0x5A);

        // Overwrite lower nibble
        encode_bitfield(&mut data, 0, 4, 0xF);
        assert_eq!(data[0], 0x5F);
    }

    #[test]
    fn test_encode_bitfield_value_masking() {
        let mut data = [0u8; 4];

        // Value larger than bit_length allows should be masked
        encode_bitfield(&mut data, 0, 4, 0xFF); // Only lower 4 bits should be used
        assert_eq!(data[0], 0x0F);

        // 16-bit value in 8-bit field
        encode_bitfield(&mut data, 0, 8, 0x1234); // Only 0x34 should be used
        assert_eq!(data[0], 0x34);
    }

    #[test]
    fn test_encode_bitfield_cross_byte_boundary() {
        let mut data = [0u8; 4];

        // Encode 12 bits starting at bit 4 (spans 3 bytes)
        encode_bitfield(&mut data, 4, 12, 0xFFF);

        // Let's verify by decoding first
        let decoded = decode_bitfield(&data, 4, 12);
        assert_eq!(decoded, 0xFFF);
    }

    #[test]
    fn test_encode_bitfield_multiple_values() {
        let mut data = [0u8; 8];

        // Pack multiple values into the same byte array
        encode_bitfield(&mut data, 0, 3, 0x5);   // 3 bits: 101
        encode_bitfield(&mut data, 3, 5, 0x1A);  // 5 bits: 11010
        encode_bitfield(&mut data, 8, 4, 0xC);   // 4 bits: 1100
        encode_bitfield(&mut data, 12, 4, 0x3);  // 4 bits: 0011

        // Verify the packed data
        assert_eq!(data[0], 0b11010101); // 0xD5
        assert_eq!(data[1], 0b00111100); // 0x3C
    }

    #[test]
    fn test_encode_bitfield_edge_case_16_bits() {
        let mut data = [0u8; 4];

        // Encode maximum 16-bit value
        encode_bitfield(&mut data, 0, 16, 0xFFFF);
        assert_eq!(data[0], 0xFF);
        assert_eq!(data[1], 0xFF);

        // Encode 16-bit value at offset
        data.fill(0);
        encode_bitfield(&mut data, 8, 16, 0x1234);
        assert_eq!(data[1], 0x34);
        assert_eq!(data[2], 0x12);
    }

    #[test]
    fn test_encode_bitfield_overwrite_existing() {
        let mut data = [0xFF; 4]; // Start with all bits set

        // Clear specific bits
        encode_bitfield(&mut data, 0, 4, 0x0);
        assert_eq!(data[0], 0xF0);

        // Set different bits
        encode_bitfield(&mut data, 4, 4, 0x5);
        assert_eq!(data[0], 0x50);
    }

    #[test]
    fn test_decode_bitfield_single_bit() {
        let data = [0b10000001, 0x00, 0x00, 0x00];

        assert_eq!(decode_bitfield(&data, 0, 1), 1); // Bit 0
        assert_eq!(decode_bitfield(&data, 1, 1), 0); // Bit 1
        assert_eq!(decode_bitfield(&data, 7, 1), 1); // Bit 7
    }

    #[test]
    fn test_decode_bitfield_out_of_bounds() {
        let data = [0xFF];

        // Partially out of bounds
        assert_eq!(decode_bitfield(&data, 4, 8), 0x0F); // Only 4 bits available

        // Completely out of bounds
        assert_eq!(decode_bitfield(&data, 8, 8), 0x00);
    }

    #[test]
    fn test_decode_bitfield_zero_length() {
        let data = [0xFF, 0xFF, 0xFF, 0xFF];

        assert_eq!(decode_bitfield(&data, 0, 0), 0);
        assert_eq!(decode_bitfield(&data, 10, 0), 0);
    }

    #[test]
    fn test_decode_bitfield_16_bits() {
        let data = [0x34, 0x12, 0x00, 0x00];

        assert_eq!(decode_bitfield(&data, 0, 16), 0x1234);
    }

    #[test]
    fn test_decode_bitfield_sparse_bits() {
        let data = [0b10101010, 0x00, 0x00, 0x00];

        // Extract every other bit
        assert_eq!(decode_bitfield(&data, 0, 2), 0b10); // Bits 0-1: 10
        assert_eq!(decode_bitfield(&data, 2, 2), 0b10); // Bits 2-3: 10
        assert_eq!(decode_bitfield(&data, 4, 2), 0b10); // Bits 4-5: 10
        assert_eq!(decode_bitfield(&data, 6, 2), 0b10); // Bits 6-7: 10
    }

    // Additional comprehensive tests for sign_extend

    #[test]
    fn test_sign_extend_various_bit_lengths() {
        // 1-bit signed
        assert_eq!(sign_extend(0x0, 1), 0);
        assert_eq!(sign_extend(0x1, 1), -1);

        // 2-bit signed
        assert_eq!(sign_extend(0x0, 2), 0);
        assert_eq!(sign_extend(0x1, 2), 1);
        assert_eq!(sign_extend(0x2, 2), -2);
        assert_eq!(sign_extend(0x3, 2), -1);

        // 3-bit signed
        assert_eq!(sign_extend(0x3, 3), 3);   // 011 = +3
        assert_eq!(sign_extend(0x4, 3), -4);  // 100 = -4
        assert_eq!(sign_extend(0x7, 3), -1);  // 111 = -1
    }

    #[test]
    fn test_sign_extend_8_bit() {
        // Full 8-bit range
        assert_eq!(sign_extend(0x00, 8), 0);
        assert_eq!(sign_extend(0x7F, 8), 127);  // Maximum positive
        assert_eq!(sign_extend(0x80, 8), -128); // Minimum negative
        assert_eq!(sign_extend(0xFF, 8), -1);   // -1 in 8-bit

        // Some middle values
        assert_eq!(sign_extend(0x01, 8), 1);
        assert_eq!(sign_extend(0xFE, 8), -2);
    }

    #[test]
    fn test_sign_extend_16_bit() {
        // 16-bit values (should pass through unchanged)
        assert_eq!(sign_extend(0x0000, 16), 0);
        assert_eq!(sign_extend(0x7FFF, 16), 32767);
        assert_eq!(sign_extend(0x8000, 16), -32768);
        assert_eq!(sign_extend(0xFFFF, 16), -1);
    }

    #[test]
    fn test_sign_extend_edge_cases() {
        // Zero bit length (should return value as-is)
        assert_eq!(sign_extend(0x1234, 0), 0x1234_u16 as i16);

        // Greater than 16 bits (should return value as-is)
        assert_eq!(sign_extend(0x1234, 17), 0x1234_u16 as i16);
        assert_eq!(sign_extend(0x1234, 32), 0x1234_u16 as i16);
    }

    #[test]
    fn test_sign_extend_j1939_common_sizes() {
        // Common J1939 signal sizes

        // 12-bit signed (common for temperatures, angles)
        assert_eq!(sign_extend(0x000, 12), 0);      // 0
        assert_eq!(sign_extend(0x7FF, 12), 2047);   // Maximum positive
        assert_eq!(sign_extend(0x800, 12), -2048);  // Minimum negative
        assert_eq!(sign_extend(0xFFF, 12), -1);     // -1

        // 13-bit signed
        assert_eq!(sign_extend(0x0FFF, 13), 4095);
        assert_eq!(sign_extend(0x1000, 13), -4096);

        // 14-bit signed
        assert_eq!(sign_extend(0x1FFF, 14), 8191);
        assert_eq!(sign_extend(0x2000, 14), -8192);
    }

    // Integration tests combining encode/decode with sign extension

    #[test]
    fn test_encode_decode_roundtrip_signed() {
        let mut data = [0u8; 8];

        // Test signed 8-bit values
        let test_values = [-128i16, -1, 0, 1, 127];

        for &value in &test_values {
            data.fill(0);
            let unsigned_value = value as u16;
            encode_bitfield(&mut data, 0, 8, unsigned_value);
            let decoded = decode_bitfield(&data, 0, 8);
            let sign_extended = sign_extend(decoded, 8);
            assert_eq!(sign_extended, value, "Failed roundtrip for value {}", value);
        }
    }

    #[test]
    fn test_encode_decode_roundtrip_12_bit_signed() {
        let mut data = [0u8; 8];

        // Test 12-bit signed values
        let test_values = [-2048i16, -1, 0, 1, 2047];

        for &value in &test_values {
            data.fill(0);
            let unsigned_value = value as u16;
            encode_bitfield(&mut data, 4, 12, unsigned_value); // Test with offset
            let decoded = decode_bitfield(&data, 4, 12);
            let sign_extended = sign_extend(decoded, 12);
            assert_eq!(sign_extended, value, "Failed 12-bit roundtrip for value {}", value);
        }
    }
}