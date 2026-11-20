//! `byte_order = "big"` fields.

use j1939_rs::prelude::*;

fn encode<T: Marshall>(m: &T) -> J1939Message {
    let mut raw = J1939Message::default();
    m.marshall(&mut raw).unwrap();
    raw
}

#[j1939_message(pgn = 0xF301, length = 8)]
struct Be {
    #[j1939(bits = 0..8)]
    le_a: u8,
    #[j1939(bits = 8..24, byte_order = "big")]
    v: u16,
    #[j1939(bits = 24..56, byte_order = "big")]
    s: i32,
    #[j1939(bits = 56..64, byte_order = "little")]
    le_b: u8,
}

#[test]
fn u16_bytes_are_big_endian() {
    let raw = encode(&Be {
        le_a: 0xAA,
        v: 0x1234,
        s: 0,
        le_b: 0xBB,
    });
    assert_eq!(raw.data[0], 0xAA);
    assert_eq!(raw.data[1], 0x12);
    assert_eq!(raw.data[2], 0x34);
    assert_eq!(raw.data[7], 0xBB);
}

#[test]
fn signed_32_roundtrips_and_neighbours_untouched() {
    let m = Be {
        le_a: 0xAA,
        v: 0x1234,
        s: -2,
        le_b: 0xBB,
    };
    let raw = encode(&m);
    assert_eq!(&raw.data[3..7], &[0xFF, 0xFF, 0xFF, 0xFE]);
    let back = Be::unmarshall(&raw).unwrap();
    assert_eq!(back.le_a, 0xAA);
    assert_eq!(back.v, 0x1234);
    assert_eq!(back.s, -2);
    assert_eq!(back.le_b, 0xBB);
}

#[test]
fn signed_decode_swaps_before_sign_extension() {
    let mut raw = J1939Message {
        pgn: 0xF301,
        length: 8,
        ..Default::default()
    };
    raw.data[3..7].copy_from_slice(&[0x80, 0x00, 0x00, 0x00]);
    assert_eq!(Be::unmarshall(&raw).unwrap().s, i32::MIN);
}

#[j1939_message(pgn = 0xF302, length = 8)]
struct BeScaled {
    #[j1939(bits = 0..16, scale = 0.5, byte_order = "big")]
    f: f32,
    #[j1939(bits = 16..32, scale = 0.5, signed, byte_order = "big")]
    g: f32,
    #[j1939(bits = 32..48, q = 15, byte_order = "big")]
    q: f32,
    #[j1939(bits = 48..64, uq = 16, byte_order = "big")]
    uq: f32,
}

#[test]
fn scaled_q_uq_roundtrip() {
    let m = BeScaled {
        f: 1000.5,
        g: -20.5,
        q: -0.25,
        uq: 0.5,
    };
    let raw = encode(&m);
    // 1000.5 / 0.5 = 2001 = 0x07D1
    assert_eq!(&raw.data[0..2], &[0x07, 0xD1]);
    let back = BeScaled::unmarshall(&raw).unwrap();
    assert_eq!(back.f, 1000.5);
    assert_eq!(back.g, -20.5);
    assert_eq!(back.q, -0.25);
    assert!((back.uq - 0.5).abs() < 1e-4);
}

#[j1939_message(pgn = 0xF303, length = 4)]
struct BeSignal {
    #[j1939(bits = 0..16, byte_order = "big")]
    a: Signal<u16>,
    #[j1939(bits = 16..32, scale = 0.5, byte_order = "big")]
    b: Signal<f32>,
}

#[test]
fn signal_special_values_are_big_endian() {
    let raw = encode(&BeSignal {
        a: Signal::Error,
        b: Signal::Value(2.0),
    });
    // error raw for 16 bits is 0xFE00
    assert_eq!(&raw.data[0..2], &[0xFE, 0x00]);
    assert_eq!(&raw.data[2..4], &[0x00, 0x04]);
    let back = BeSignal::unmarshall(&raw).unwrap();
    assert_eq!(back.a, Signal::Error);
    assert_eq!(back.b, Signal::Value(2.0));

    let na = BeSignal::unmarshall(&J1939Message {
        pgn: 0xF303,
        length: 4,
        data: {
            let mut d = [0u8; J1939Message::MAX_LENGTH];
            d[..4].copy_from_slice(&[0xFF, 0xFF, 0xFF, 0xFF]);
            d
        },
        ..Default::default()
    })
    .unwrap();
    assert_eq!(na.a, Signal::NotAvailable);
}
