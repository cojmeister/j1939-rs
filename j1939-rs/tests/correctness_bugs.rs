//! Regression tests for the correctness bugs fixed in 0.2.0 (numbering follows the bug list).

use j1939_rs::prelude::*;

fn roundtrip<T: Marshall + Unmarshall>(m: &T) -> (J1939Message, T) {
    let mut raw = J1939Message::default();
    m.marshall(&mut raw).unwrap();
    let back = T::unmarshall(&raw).unwrap();
    (raw, back)
}

fn frame(pgn: u32, length: u16, bytes: &[u8]) -> J1939Message {
    let mut raw = J1939Message {
        pgn,
        length,
        ..Default::default()
    };
    raw.data[..bytes.len()].copy_from_slice(bytes);
    raw
}

// ---- bug 1: wide fields ----
#[j1939_message(pgn = 0x1001, length = 8)]
struct Wide32 {
    #[j1939(bits = 0..32)]
    a: u32,
    #[j1939(bits = 32..64, reserved)]
    _r: (),
}

#[test]
fn bug1_u32_field_keeps_all_32_bits() {
    let (raw, back) = roundtrip(&Wide32 {
        a: 0xDEAD_BEEF,
        _r: (),
    });
    assert_eq!(&raw.data[..4], &[0xEF, 0xBE, 0xAD, 0xDE]);
    assert_eq!(back.a, 0xDEAD_BEEF);
}

#[j1939_message(pgn = 0x1002, length = 9)]
struct Wide64 {
    #[j1939(bits = 0..3, reserved)]
    _r0: (),
    #[j1939(bits = 3..67)]
    a: u64,
    #[j1939(bits = 67..72, reserved)]
    _r1: (),
}

#[test]
fn bug1_unaligned_u64_field_at_bit_3() {
    let v = 0x0123_4567_89AB_CDEFu64;
    let (_, back) = roundtrip(&Wide64 {
        _r0: (),
        a: v,
        _r1: (),
    });
    assert_eq!(back.a, v);
}

#[j1939_message(pgn = 0x1003, length = 4)]
struct Signed24 {
    #[j1939(bits = 0..24)]
    a: i32,
    #[j1939(bits = 24..32, reserved)]
    _r: (),
}

#[test]
fn bug1_signed_24_bit_minus_one() {
    let (raw, back) = roundtrip(&Signed24 { a: -1, _r: () });
    assert_eq!(&raw.data[..4], &[0xFF, 0xFF, 0xFF, 0x00]);
    assert_eq!(back.a, -1);
}

#[j1939_message(pgn = 0x1004, length = 8)]
struct SignedI64 {
    #[j1939(bits = 0..64)]
    a: i64,
}

#[test]
fn bug1_i64_field() {
    let (_, back) = roundtrip(&SignedI64 { a: i64::MIN + 7 });
    assert_eq!(back.a, i64::MIN + 7);
}

// ---- bug 10: i32 in 20 bits ----
#[j1939_message(pgn = 0x1005, length = 3)]
struct I32In20 {
    #[j1939(bits = 0..20)]
    a: i32,
    #[j1939(bits = 20..24, reserved)]
    _r: (),
}

#[test]
fn bug10_i32_minus_100000_in_20_bits() {
    let (_, back) = roundtrip(&I32In20 {
        a: -100_000,
        _r: (),
    });
    assert_eq!(back.a, -100_000);
}

// ---- bug 2: scaled encode saturation ----
#[j1939_message(pgn = 0x2001, length = 2)]
struct Rpm {
    #[j1939(bits = 0..16, scale = 0.125, unit = "rpm")]
    rpm: f32,
}

#[test]
fn bug2_6000_rpm_is_raw_48000() {
    let (raw, back) = roundtrip(&Rpm { rpm: 6000.0 });
    assert_eq!(u16::from_le_bytes([raw.data[0], raw.data[1]]), 48000);
    assert_eq!(back.rpm, 6000.0);
}

#[test]
fn bug2_overrange_saturates_to_field_max_not_i16_max() {
    let (raw, _) = roundtrip(&Rpm { rpm: 1.0e9 });
    assert_eq!(u16::from_le_bytes([raw.data[0], raw.data[1]]), 65535);
    let (raw, _) = roundtrip(&Rpm { rpm: -5.0 });
    assert_eq!(u16::from_le_bytes([raw.data[0], raw.data[1]]), 0);
}

#[j1939_message(pgn = 0x2002, length = 3)]
struct Wide24Scaled {
    #[j1939(bits = 0..24, scale = 1.0, signed)]
    v: f32,
}

#[test]
fn bug2_signed_raw_below_i16_min_does_not_saturate() {
    let (_, back) = roundtrip(&Wide24Scaled { v: -40000.0 });
    assert_eq!(back.v, -40000.0);
}

// ---- bug 4: signedness comes from the field, not the offset ----
#[j1939_message(pgn = 0x4001, length = 1)]
struct Unsigned8 {
    #[j1939(bits = 0..8, scale = 1.0)]
    v: f32,
}

#[j1939_message(pgn = 0x4002, length = 1)]
struct Signed8 {
    #[j1939(bits = 0..8, scale = 1.0, signed)]
    v: f32,
}

#[test]
fn bug4_unsigned_zero_offset_raw_200_is_200() {
    let raw = frame(0x4001, 1, &[200]);
    assert_eq!(Unsigned8::unmarshall(&raw).unwrap().v, 200.0);
}

#[test]
fn bug4_signed_zero_offset_raw_0xc8_is_minus_56() {
    let raw = frame(0x4002, 1, &[0xC8]);
    assert_eq!(Signed8::unmarshall(&raw).unwrap().v, -56.0);
    let (r, back) = roundtrip(&Signed8 { v: -56.0 });
    assert_eq!(r.data[0], 0xC8);
    assert_eq!(back.v, -56.0);
}

// ---- bug 5: message length is bytes ----
#[j1939_message(pgn = 0x5001, length = 47)]
struct Long47 {
    #[j1939(bits = 0..8)]
    first: u8,
    #[j1939(bits = 8..368, reserved)]
    _mid: (),
    #[j1939(bits = 368..376)]
    last: u8,
}

#[test]
fn bug5_47_byte_message_roundtrips_with_field_in_last_byte() {
    assert_eq!(Long47::LENGTH, 47);
    let (raw, back) = roundtrip(&Long47 {
        first: 1,
        _mid: (),
        last: 0xA5,
    });
    assert_eq!(raw.length, 47);
    assert_eq!(raw.data[46], 0xA5);
    assert_eq!(back.last, 0xA5);
    assert_eq!(back.first, 1);
}

#[test]
fn bug5_max_length_is_223_bytes() {
    assert_eq!(J1939Message::MAX_LENGTH, 223);
}

// ---- bug 6 / 7: Q formats through the macro ----
#[j1939_message(pgn = 0x6001, length = 2)]
struct QMsg {
    #[j1939(bits = 0..10, encoding = "q9")]
    q9: f32,
    #[j1939(bits = 10..16, reserved)]
    _r: (),
}

#[test]
fn bug6_q9_plus_one_does_not_wrap_to_minus_one() {
    let (_, back) = roundtrip(&QMsg { q9: 1.0, _r: () });
    assert!(back.q9 > 0.99, "got {}", back.q9);
    let (_, back) = roundtrip(&QMsg { q9: -1.0, _r: () });
    assert_eq!(back.q9, -1.0);
}

#[j1939_message(pgn = 0x6002, length = 2)]
struct QRanged {
    #[j1939(bits = 0..10, q = 9, range = 90.0, unit = "deg")]
    angle: f32,
    #[j1939(bits = 10..16, reserved)]
    _r: (),
}

#[test]
fn bug7_ranged_q9_over_90_degrees() {
    let (raw, back) = roundtrip(&QRanged {
        angle: 45.0,
        _r: (),
    });
    assert_eq!(raw.data[0], 0x00);
    assert_eq!(raw.data[1] & 0x03, 0x01); // raw 256 = 0b01_0000_0000
    assert_eq!(back.angle, 45.0);
    let (_, back) = roundtrip(&QRanged {
        angle: 500.0,
        _r: (),
    });
    assert!(back.angle > 89.0 && back.angle < 90.0, "got {}", back.angle);
    let (_, back) = roundtrip(&QRanged {
        angle: -90.0,
        _r: (),
    });
    assert_eq!(back.angle, -90.0);
}

#[j1939_message(pgn = 0x6003, length = 4)]
struct Qs {
    #[j1939(bits = 0..8, q = 7)]
    q7: f32,
    #[j1939(bits = 8..24, q = 15)]
    q15: f32,
    #[j1939(bits = 24..32, uq = 8)]
    uq8: f32,
}

#[test]
fn bug7_q7_q15_uq8_through_macro() {
    let (raw, back) = roundtrip(&Qs {
        q7: -1.0,
        q15: 0.5,
        uq8: 1.0,
    });
    assert_eq!(raw.data[0], 0x80);
    assert_eq!(back.q7, -1.0);
    assert_eq!(back.q15, 0.5);
    assert_eq!(raw.data[3], 0xFF);
    assert_eq!(back.uq8, 1.0);
}

#[j1939_message(pgn = 0x6004, length = 2)]
struct Trunc {
    #[j1939(bits = 0..10, q = 9, rounding = "truncate")]
    t: f32,
    #[j1939(bits = 10..16, reserved)]
    _r: (),
}

#[j1939_message(pgn = 0x6005, length = 2)]
struct Rounded {
    #[j1939(bits = 0..10, q = 9)]
    t: f32,
    #[j1939(bits = 10..16, reserved)]
    _r: (),
}

#[test]
fn bug6_rounding_attribute_selects_truncate_or_round() {
    let (a, _) = roundtrip(&Trunc { t: 0.0039, _r: () });
    let (b, _) = roundtrip(&Rounded { t: 0.0039, _r: () });
    assert_eq!(a.data[0], 1);
    assert_eq!(b.data[0], 2);
}

// ---- bug 8: enums ----
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[j1939_enum]
enum Mode {
    Off = 0,
    On = 1,
    Fault = 3,
}

#[j1939_message(pgn = 0x8001, length = 1)]
struct ModeMsg {
    #[j1939(bits = 0..2)]
    mode: Mode,
    #[j1939(bits = 2..8, reserved)]
    _r: (),
}

#[test]
fn bug8_undefined_enum_value_is_an_error_not_ub() {
    let mut raw = frame(0x8001, 1, &[2]); // no variant has discriminant 2
    assert_eq!(
        ModeMsg::unmarshall(&raw).unwrap_err(),
        Error::InvalidEnum {
            field: "mode",
            raw: 2
        }
    );
    raw.data[0] = 3;
    assert_eq!(ModeMsg::unmarshall(&raw).unwrap().mode, Mode::Fault);
}

#[test]
fn bug8_enum_try_from_u8() {
    assert_eq!(Mode::try_from(1u8), Ok(Mode::On));
    assert_eq!(Mode::try_from(2u8), Err(2u8));
}

// ---- bug 9: padded frames ----
#[j1939_message(pgn = 0x9001, length = 6)]
struct Six {
    #[j1939(bits = 0..48)]
    v: u64,
}

#[j1939_message(pgn = 0x9002, length = 6, strict)]
struct SixStrict {
    #[j1939(bits = 0..48)]
    v: u64,
}

#[test]
fn bug9_six_byte_message_decodes_from_padded_8_byte_frame() {
    let raw = frame(0x9001, 8, &[1, 2, 3, 4, 5, 6, 0xFF, 0xFF]);
    assert_eq!(Six::unmarshall(&raw).unwrap().v, 0x0000_0605_0403_0201);
}

#[test]
fn bug9_short_frames_still_rejected_and_strict_requires_exact() {
    let mut raw = frame(0x9001, 5, &[]);
    assert_eq!(Six::unmarshall(&raw).unwrap_err(), Error::InvalidLength);
    raw.pgn = 0x9002;
    raw.length = 8;
    assert_eq!(
        SixStrict::unmarshall(&raw).unwrap_err(),
        Error::InvalidLength
    );
    raw.length = 6;
    assert!(SixStrict::unmarshall(&raw).is_ok());
}

// ---- reserved bits ----
#[j1939_message(pgn = 0xA001, length = 2)]
struct Rsv {
    #[j1939(bits = 0..4)]
    a: u8,
    #[j1939(bits = 4..16, reserved)]
    _r: (),
}

#[test]
fn reserved_bits_are_zero_on_encode_even_with_dirty_buffer_and_ignored_on_decode() {
    let mut raw = J1939Message {
        data: [0xFF; J1939Message::MAX_LENGTH],
        ..Default::default()
    };
    Rsv { a: 0x5, _r: () }.marshall(&mut raw).unwrap();
    assert_eq!(&raw.data[..2], &[0x05, 0x00]);
    raw.data[1] = 0xFF;
    assert_eq!(Rsv::unmarshall(&raw).unwrap().a, 0x5);
}
