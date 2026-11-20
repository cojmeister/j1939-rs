//! Byte-order helpers for big-endian (Motorola-style) multi-byte fields.

/// Reverses the byte order of the low `bit_length / 8` bytes of `value`.
///
/// `bit_length` must be a multiple of 8 and at most 64; otherwise `value` is returned unchanged
/// (and a debug assertion fires). Bytes above the field are left untouched. The operation is its
/// own inverse, so it serves both encoding and decoding.
pub fn swap_field_bytes(value: u64, bit_length: usize) -> u64 {
    debug_assert!(
        bit_length & 7 == 0 && bit_length <= 64,
        "byte swap needs a whole number of bytes (<= 64 bits)"
    );
    if bit_length & 7 != 0 || bit_length == 0 || bit_length > 64 {
        return value;
    }
    let low = if bit_length == 64 {
        u64::MAX
    } else {
        (1u64 << bit_length) - 1
    };
    let swapped = (value & low).swap_bytes() >> (64 - bit_length);
    (value & !low) | swapped
}
