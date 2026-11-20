#![cfg(feature = "defmt")]

use j1939_rs::prelude::*;

#[j1939_message(pgn = 1, defmt)]
struct Logged {
    #[j1939(bits = 0..8)]
    a: u8,
    #[j1939(bits = 8..16, scale = 0.5)]
    b: f32,
    #[j1939(bits = 16..64, reserved)]
    _r: (),
}

fn assert_format<T: defmt::Format>() {}

#[test]
fn defmt_flag_derives_format() {
    assert_format::<Logged>();
}
