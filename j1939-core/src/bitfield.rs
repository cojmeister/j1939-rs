use crate::{Error, Result};

/// Maximum width, in bits, of a single bitfield.
pub const MAX_BITFIELD_BITS: usize = 64;

fn check_bounds(len: usize, bit_offset: usize, bit_length: usize) -> Result<()> {
    if bit_length == 0 || bit_length > MAX_BITFIELD_BITS {
        return Err(Error::InvalidLength);
    }
    match bit_offset.checked_add(bit_length) {
        Some(end) if end <= len.saturating_mul(8) => Ok(()),
        _ => Err(Error::BufferTooSmall),
    }
}

/// Encodes `value` into a bitfield of `bit_length` bits (1..=64) starting at `bit_offset`.
///
/// Bit numbering is little-endian (J1939 / Intel): bit 0 is the least significant bit of byte 0.
/// The value is masked to `bit_length` bits (so a negative value cast with `as u64` is stored in
/// two's complement). Bits outside the field are left untouched. The field may be unaligned and
/// may span up to 9 bytes.
///
/// # Errors
///
/// * [`Error::InvalidLength`] if `bit_length` is 0 or greater than 64.
/// * [`Error::BufferTooSmall`] if the field does not fit inside `data`.
///
/// # Examples
///
/// ```
/// use j1939_core::bitfield::encode_bitfield;
///
/// let mut data = [0u8; 4];
/// encode_bitfield(&mut data, 0, 4, 0xA).unwrap();
/// encode_bitfield(&mut data, 4, 4, 0x5).unwrap();
/// assert_eq!(data[0], 0x5A);
///
/// // 32-bit field
/// let mut data = [0u8; 4];
/// encode_bitfield(&mut data, 0, 32, 0xDEAD_BEEF).unwrap();
/// assert_eq!(data, [0xEF, 0xBE, 0xAD, 0xDE]);
///
/// // Out of bounds is an error, not a panic
/// assert!(encode_bitfield(&mut data, 30, 8, 1).is_err());
/// ```
pub fn encode_bitfield(
    data: &mut [u8],
    bit_offset: usize,
    bit_length: usize,
    value: u64,
) -> Result<()> {
    check_bounds(data.len(), bit_offset, bit_length)?;

    let mut value = if bit_length == 64 {
        value
    } else {
        value & ((1u64 << bit_length) - 1)
    };
    let mut pos = bit_offset;
    let mut remaining = bit_length;

    while remaining > 0 {
        let byte_idx = pos / 8;
        let bit_in_byte = pos % 8;
        let n = core::cmp::min(8 - bit_in_byte, remaining);
        let mask = (((1u32 << n) - 1) as u8) << bit_in_byte;
        let chunk = ((value as u8) << bit_in_byte) & mask;
        data[byte_idx] = (data[byte_idx] & !mask) | chunk;
        value = if n == 64 { 0 } else { value >> n };
        pos += n;
        remaining -= n;
    }
    Ok(())
}

/// Decodes an unsigned bitfield of `bit_length` bits (1..=64) starting at `bit_offset`.
///
/// # Errors
///
/// * [`Error::InvalidLength`] if `bit_length` is 0 or greater than 64.
/// * [`Error::BufferTooSmall`] if the field does not fit inside `data`.
///
/// # Examples
///
/// ```
/// use j1939_core::bitfield::decode_bitfield;
///
/// let data = [0x5A, 0x00];
/// assert_eq!(decode_bitfield(&data, 0, 4).unwrap(), 0xA);
/// assert_eq!(decode_bitfield(&data, 4, 4).unwrap(), 0x5);
/// assert!(decode_bitfield(&data, 8, 16).is_err());
/// ```
pub fn decode_bitfield(data: &[u8], bit_offset: usize, bit_length: usize) -> Result<u64> {
    check_bounds(data.len(), bit_offset, bit_length)?;

    let mut value = 0u64;
    let mut pos = bit_offset;
    let mut done = 0usize;

    while done < bit_length {
        let byte_idx = pos / 8;
        let bit_in_byte = pos % 8;
        let n = core::cmp::min(8 - bit_in_byte, bit_length - done);
        let chunk = ((data[byte_idx] >> bit_in_byte) as u32) & ((1u32 << n) - 1);
        value |= (chunk as u64) << done;
        pos += n;
        done += n;
    }
    Ok(value)
}

/// Sign-extends the low `bit_length` bits of `value` to an `i64`.
///
/// If `bit_length` is 0 or greater than 64 the value is returned reinterpreted as `i64`.
///
/// # Examples
///
/// ```
/// use j1939_core::bitfield::sign_extend;
///
/// assert_eq!(sign_extend(0x80, 8), -128);
/// assert_eq!(sign_extend(0xFF_FFFF, 24), -1);
/// assert_eq!(sign_extend(0x7FF, 12), 2047);
/// assert_eq!(sign_extend(0x1, 1), -1);
/// ```
pub fn sign_extend(value: u64, bit_length: usize) -> i64 {
    if bit_length == 0 || bit_length >= 64 {
        return value as i64;
    }
    let shift = 64 - bit_length as u32;
    ((value << shift) as i64) >> shift
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_single_byte() {
        let mut data = [0u8; 8];
        encode_bitfield(&mut data, 0, 8, 0xAB).unwrap();
        assert_eq!(decode_bitfield(&data, 0, 8).unwrap(), 0xAB);
    }

    #[test]
    fn test_encode_decode_across_boundary() {
        let mut data = [0u8; 8];
        encode_bitfield(&mut data, 6, 10, 0x3FF).unwrap();
        assert_eq!(decode_bitfield(&data, 6, 10).unwrap(), 0x3FF);
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
        encode_bitfield(&mut data, 0, 1, 1).unwrap();
        assert_eq!(data[0], 0b00000001);

        // Set bit 7
        encode_bitfield(&mut data, 7, 1, 1).unwrap();
        assert_eq!(data[0], 0b10000001);

        // Clear bit 0
        encode_bitfield(&mut data, 0, 1, 0).unwrap();
        assert_eq!(data[0], 0b10000000);
    }

    #[test]
    fn test_encode_bitfield_nibbles() {
        let mut data = [0u8; 4];

        // Encode lower nibble
        encode_bitfield(&mut data, 0, 4, 0xA).unwrap();
        assert_eq!(data[0], 0x0A);

        // Encode upper nibble
        encode_bitfield(&mut data, 4, 4, 0x5).unwrap();
        assert_eq!(data[0], 0x5A);

        // Overwrite lower nibble
        encode_bitfield(&mut data, 0, 4, 0xF).unwrap();
        assert_eq!(data[0], 0x5F);
    }

    #[test]
    fn test_encode_bitfield_value_masking() {
        let mut data = [0u8; 4];

        // Value larger than bit_length allows should be masked
        encode_bitfield(&mut data, 0, 4, 0xFF).unwrap(); // Only lower 4 bits should be used
        assert_eq!(data[0], 0x0F);

        // 16-bit value in 8-bit field
        encode_bitfield(&mut data, 0, 8, 0x1234).unwrap(); // Only 0x34 should be used
        assert_eq!(data[0], 0x34);
    }

    #[test]
    fn test_encode_bitfield_cross_byte_boundary() {
        let mut data = [0u8; 4];

        // Encode 12 bits starting at bit 4 (spans 3 bytes)
        encode_bitfield(&mut data, 4, 12, 0xFFF).unwrap();

        // Let's verify by decoding first
        let decoded = decode_bitfield(&data, 4, 12).unwrap();
        assert_eq!(decoded, 0xFFF);
    }

    #[test]
    fn test_encode_bitfield_multiple_values() {
        let mut data = [0u8; 8];

        // Pack multiple values into the same byte array
        encode_bitfield(&mut data, 0, 3, 0x5).unwrap(); // 3 bits: 101
        encode_bitfield(&mut data, 3, 5, 0x1A).unwrap(); // 5 bits: 11010
        encode_bitfield(&mut data, 8, 4, 0xC).unwrap(); // 4 bits: 1100
        encode_bitfield(&mut data, 12, 4, 0x3).unwrap(); // 4 bits: 0011

        // Verify the packed data
        assert_eq!(data[0], 0b11010101); // 0xD5
        assert_eq!(data[1], 0b00111100); // 0x3C
    }

    #[test]
    fn test_encode_bitfield_edge_case_16_bits() {
        let mut data = [0u8; 4];

        // Encode maximum 16-bit value
        encode_bitfield(&mut data, 0, 16, 0xFFFF).unwrap();
        assert_eq!(data[0], 0xFF);
        assert_eq!(data[1], 0xFF);

        // Encode 16-bit value at offset
        data.fill(0);
        encode_bitfield(&mut data, 8, 16, 0x1234).unwrap();
        assert_eq!(data[1], 0x34);
        assert_eq!(data[2], 0x12);
    }

    #[test]
    fn test_encode_bitfield_overwrite_existing() {
        let mut data = [0xFF; 4]; // Start with all bits set

        // Clear specific bits
        encode_bitfield(&mut data, 0, 4, 0x0).unwrap();
        assert_eq!(data[0], 0xF0);

        // Set different bits
        encode_bitfield(&mut data, 4, 4, 0x5).unwrap();
        assert_eq!(data[0], 0x50);
    }

    #[test]
    fn test_decode_bitfield_single_bit() {
        let data = [0b10000001, 0x00, 0x00, 0x00];

        assert_eq!(decode_bitfield(&data, 0, 1).unwrap(), 1); // Bit 0
        assert_eq!(decode_bitfield(&data, 1, 1).unwrap(), 0); // Bit 1
        assert_eq!(decode_bitfield(&data, 7, 1).unwrap(), 1); // Bit 7
    }

    #[test]
    fn test_decode_bitfield_out_of_bounds() {
        let data = [0xFF];

        // Partially and completely out of bounds are errors
        assert_eq!(decode_bitfield(&data, 4, 8), Err(Error::BufferTooSmall));
        assert_eq!(decode_bitfield(&data, 8, 8), Err(Error::BufferTooSmall));
    }

    #[test]
    fn test_decode_bitfield_zero_length() {
        let data = [0xFF, 0xFF, 0xFF, 0xFF];

        assert_eq!(decode_bitfield(&data, 0, 0), Err(Error::InvalidLength));
        assert_eq!(decode_bitfield(&data, 10, 0), Err(Error::InvalidLength));
    }

    #[test]
    fn test_decode_bitfield_16_bits() {
        let data = [0x34, 0x12, 0x00, 0x00];

        assert_eq!(decode_bitfield(&data, 0, 16).unwrap(), 0x1234);
    }

    #[test]
    fn test_decode_bitfield_sparse_bits() {
        let data = [0b10101010, 0x00, 0x00, 0x00];

        // Extract every other bit
        assert_eq!(decode_bitfield(&data, 0, 2).unwrap(), 0b10); // Bits 0-1: 10
        assert_eq!(decode_bitfield(&data, 2, 2).unwrap(), 0b10); // Bits 2-3: 10
        assert_eq!(decode_bitfield(&data, 4, 2).unwrap(), 0b10); // Bits 4-5: 10
        assert_eq!(decode_bitfield(&data, 6, 2).unwrap(), 0b10); // Bits 6-7: 10
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
        assert_eq!(sign_extend(0x3, 3), 3); // 011 = +3
        assert_eq!(sign_extend(0x4, 3), -4); // 100 = -4
        assert_eq!(sign_extend(0x7, 3), -1); // 111 = -1
    }

    #[test]
    fn test_sign_extend_8_bit() {
        // Full 8-bit range
        assert_eq!(sign_extend(0x00, 8), 0);
        assert_eq!(sign_extend(0x7F, 8), 127); // Maximum positive
        assert_eq!(sign_extend(0x80, 8), -128); // Minimum negative
        assert_eq!(sign_extend(0xFF, 8), -1); // -1 in 8-bit

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
        assert_eq!(sign_extend(0x1234, 0), 0x1234);

        // Widths above 16 bits now extend correctly
        assert_eq!(sign_extend(0x1234, 17), 0x1234);
        assert_eq!(sign_extend(0x1_8000, 17), -32768);
        assert_eq!(sign_extend(0x1234, 32), 0x1234);
    }

    #[test]
    fn test_sign_extend_j1939_common_sizes() {
        // Common J1939 signal sizes

        // 12-bit signed (common for temperatures, angles)
        assert_eq!(sign_extend(0x000, 12), 0); // 0
        assert_eq!(sign_extend(0x7FF, 12), 2047); // Maximum positive
        assert_eq!(sign_extend(0x800, 12), -2048); // Minimum negative
        assert_eq!(sign_extend(0xFFF, 12), -1); // -1

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
            let unsigned_value = value as u16 as u64;
            encode_bitfield(&mut data, 0, 8, unsigned_value).unwrap();
            let decoded = decode_bitfield(&data, 0, 8).unwrap();
            let sign_extended = sign_extend(decoded, 8);
            assert_eq!(
                sign_extended, value as i64,
                "Failed roundtrip for value {}",
                value
            );
        }
    }

    #[test]
    fn test_encode_decode_roundtrip_12_bit_signed() {
        let mut data = [0u8; 8];

        // Test 12-bit signed values
        let test_values = [-2048i16, -1, 0, 1, 2047];

        for &value in &test_values {
            data.fill(0);
            let unsigned_value = value as u16 as u64;
            encode_bitfield(&mut data, 4, 12, unsigned_value).unwrap(); // Test with offset
            let decoded = decode_bitfield(&data, 4, 12).unwrap();
            let sign_extended = sign_extend(decoded, 12);
            assert_eq!(
                sign_extended, value as i64,
                "Failed 12-bit roundtrip for value {}",
                value
            );
        }
    }
}
