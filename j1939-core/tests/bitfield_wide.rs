//! Bug 1 / 10 / 11: the bitfield API must handle 1..=64 bit fields and report bad bounds.
use j1939_core::{Error, decode_bitfield, encode_bitfield, sign_extend};

#[test]
fn u32_field_roundtrips_full_width() {
    let mut data = [0u8; 8];
    encode_bitfield(&mut data, 0, 32, 0xDEAD_BEEF).unwrap();
    assert_eq!(&data[..4], &[0xEF, 0xBE, 0xAD, 0xDE]);
    assert_eq!(decode_bitfield(&data, 0, 32).unwrap(), 0xDEAD_BEEF);
}

#[test]
fn unaligned_64_bit_field_at_offset_3() {
    let mut data = [0u8; 9];
    let v = 0x0123_4567_89AB_CDEFu64;
    encode_bitfield(&mut data, 3, 64, v).unwrap();
    assert_eq!(decode_bitfield(&data, 3, 64).unwrap(), v);
    // bits outside the field are untouched
    assert_eq!(data[0] & 0b0000_0111, 0);
    assert_eq!(data[8] & 0b1111_1000, 0);
}

#[test]
fn encode_does_not_clobber_neighbouring_bits() {
    let mut data = [0xFFu8; 9];
    encode_bitfield(&mut data, 3, 64, 0).unwrap();
    assert_eq!(data[0], 0b0000_0111);
    assert_eq!(data[8], 0b1111_1000);
}

#[test]
fn signed_24_bit_minus_one() {
    let mut data = [0u8; 4];
    encode_bitfield(&mut data, 0, 24, (-1i64) as u64).unwrap();
    assert_eq!(&data[..3], &[0xFF, 0xFF, 0xFF]);
    assert_eq!(data[3], 0);
    let raw = decode_bitfield(&data, 0, 24).unwrap();
    assert_eq!(sign_extend(raw, 24), -1i64);
}

#[test]
fn sign_extend_wide() {
    assert_eq!(sign_extend(0x80_0000, 24), -8_388_608i64);
    assert_eq!(sign_extend(0x7F_FFFF, 24), 8_388_607i64);
    assert_eq!(sign_extend(u64::MAX, 64), -1i64);
    assert_eq!(sign_extend(0x8000_0000, 32), i32::MIN as i64);
}

#[test]
fn i32_minus_100000_in_20_bits() {
    let mut data = [0u8; 4];
    let v: i32 = -100_000;
    encode_bitfield(&mut data, 2, 20, v as i64 as u64).unwrap();
    let raw = decode_bitfield(&data, 2, 20).unwrap();
    assert_eq!(sign_extend(raw, 20) as i32, v);
}

#[test]
fn out_of_bounds_is_an_error_not_a_panic() {
    let mut data = [0u8; 2];
    assert_eq!(
        encode_bitfield(&mut data, 8, 16, 1),
        Err(Error::BufferTooSmall)
    );
    assert_eq!(decode_bitfield(&data, 8, 16), Err(Error::BufferTooSmall));
}

#[test]
fn invalid_bit_length_is_an_error() {
    let mut data = [0u8; 16];
    assert_eq!(
        encode_bitfield(&mut data, 0, 0, 1),
        Err(Error::InvalidLength)
    );
    assert_eq!(
        encode_bitfield(&mut data, 0, 65, 1),
        Err(Error::InvalidLength)
    );
    assert_eq!(decode_bitfield(&data, 0, 0), Err(Error::InvalidLength));
    assert_eq!(decode_bitfield(&data, 0, 65), Err(Error::InvalidLength));
}
