//! `Signal<T>` fields: not-available / error / reserved raw values.

use j1939_rs::prelude::*;

fn frame(pgn: u32, length: u16, bytes: &[u8]) -> J1939Message {
    let mut raw = J1939Message {
        pgn,
        length,
        ..Default::default()
    };
    raw.data[..bytes.len()].copy_from_slice(bytes);
    raw
}

fn encode<T: Marshall>(m: &T) -> J1939Message {
    let mut raw = J1939Message::default();
    m.marshall(&mut raw).unwrap();
    raw
}

#[j1939_message(pgn = 0xF201, length = 3)]
struct Sig {
    #[j1939(bits = 0..8)]
    a: Signal<u8>,
    #[j1939(bits = 8..24, scale = 0.125)]
    rpm: Signal<f32>,
}

#[test]
fn not_available_encodes_all_ones() {
    let raw = encode(&Sig {
        a: Signal::NotAvailable,
        rpm: Signal::NotAvailable,
    });
    assert_eq!(&raw.data[..3], &[0xFF, 0xFF, 0xFF]);
}

#[test]
fn error_encodes_error_raw() {
    let raw = encode(&Sig {
        a: Signal::Error,
        rpm: Signal::Error,
    });
    assert_eq!(&raw.data[..3], &[0xFE, 0x00, 0xFE]);
}

#[test]
fn decodes_not_available() {
    let s = Sig::unmarshall(&frame(0xF201, 3, &[0xFF, 0xFF, 0xFF])).unwrap();
    assert_eq!(s.a, Signal::NotAvailable);
    assert_eq!(s.rpm, Signal::NotAvailable);
}

#[test]
fn value_roundtrips() {
    let raw = encode(&Sig {
        a: Signal::Value(7),
        rpm: Signal::Value(1850.0),
    });
    let back = Sig::unmarshall(&raw).unwrap();
    assert_eq!(back.a, Signal::Value(7));
    assert_eq!(back.rpm, Signal::Value(1850.0));
}

#[test]
fn reserved_raw_is_preserved() {
    let s = Sig::unmarshall(&frame(0xF201, 3, &[0xFC, 0x00, 0x00])).unwrap();
    assert_eq!(s.a, Signal::Reserved(0xFC));
    assert_eq!(encode(&s).data[0], 0xFC);
}

#[test]
fn out_of_range_values_clamp_below_special_values() {
    let raw = encode(&Sig {
        a: Signal::Value(255),
        rpm: Signal::Value(1.0e9),
    });
    assert_eq!(raw.data[0], 0xFA);
    assert_eq!(&raw.data[1..3], &[0xFF, 0xFA]);
    let back = Sig::unmarshall(&raw).unwrap();
    assert_eq!(back.rpm, Signal::Value(8031.875));
}
