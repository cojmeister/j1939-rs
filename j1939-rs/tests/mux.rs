//! `#[j1939_mux]` multiplexed messages.

use j1939_rs::prelude::*;

#[j1939_message(pgn = 0xFF10, length = 8)]
struct MsgA {
    #[j1939(bits = 0..8, reserved)]
    _sel: (),
    #[j1939(bits = 8..24)]
    value: u16,
    #[j1939(bits = 24..64, reserved)]
    _r: (),
}

#[j1939_message(pgn = 0xFF10, length = 8)]
struct MsgB {
    #[j1939(bits = 0..8, reserved)]
    _sel: (),
    #[j1939(bits = 8..16)]
    count: u8,
    #[j1939(bits = 16..32, scale = 0.5)]
    level: f32,
    #[j1939(bits = 32..64, reserved)]
    _r: (),
}

/// Does not reserve the selector bits: the field owns byte 0.
#[j1939_message(pgn = 0xFF10, length = 8)]
struct MsgC {
    #[j1939(bits = 0..8)]
    clash: u8,
    #[j1939(bits = 8..64, reserved)]
    _r: (),
}

#[j1939_mux(selector = 0..8)]
/// Docs are kept.
enum Foo {
    #[j1939(selector = 1)]
    A(MsgA),
    #[j1939(selector = 2)]
    B(MsgB),
    #[j1939(selector = 3)]
    C(MsgC),
}

fn a() -> MsgA {
    MsgA {
        _sel: (),
        value: 0xBEEF,
        _r: (),
    }
}

fn b() -> MsgB {
    MsgB {
        _sel: (),
        count: 7,
        level: 100.5,
        _r: (),
    }
}

#[test]
fn marshall_writes_selector_and_roundtrips() {
    let mut msg = J1939Message::default();
    Foo::A(a()).marshall(&mut msg).unwrap();
    assert_eq!(msg.data[0], 1);
    assert_eq!(msg.pgn, 0xFF10);
    match Foo::unmarshall(&msg).unwrap() {
        Foo::A(m) => assert_eq!(m.value, 0xBEEF),
        other => panic!("wrong variant {other:?}"),
    }

    Foo::B(b()).marshall(&mut msg).unwrap();
    assert_eq!(msg.data[0], 2);
    match Foo::unmarshall(&msg).unwrap() {
        Foo::B(m) => {
            assert_eq!(m.count, 7);
            assert_eq!(m.level, 100.5);
        }
        other => panic!("wrong variant {other:?}"),
    }
}

#[test]
fn unknown_selector_is_reported() {
    let mut msg = J1939Message::default();
    Foo::A(a()).marshall(&mut msg).unwrap();
    msg.data[0] = 9;
    assert!(matches!(
        Foo::unmarshall(&msg),
        Err(Error::UnknownSelector { raw: 9 })
    ));
}

#[test]
fn pgn_mismatch_is_invalid_pgn() {
    let mut msg = J1939Message::default();
    Foo::A(a()).marshall(&mut msg).unwrap();
    msg.pgn = 0x1234;
    assert!(matches!(Foo::unmarshall(&msg), Err(Error::InvalidPgn)));
}

#[test]
fn selector_overwrites_unreserved_variant_bits() {
    let mut msg = J1939Message::default();
    Foo::C(MsgC {
        clash: 0xAA,
        _r: (),
    })
    .marshall(&mut msg)
    .unwrap();
    assert_eq!(msg.data[0], 3);
}
