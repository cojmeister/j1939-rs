#![cfg(feature = "defmt")]

use j1939_core::{Error, J1939Id, J1939Message, Q, RoundingMode, Signal, SignalClass, UQ};

fn assert_format<T: defmt::Format>() {}

#[test]
fn core_types_implement_format() {
    assert_format::<Error>();
    assert_format::<J1939Id>();
    assert_format::<J1939Message>();
    assert_format::<RoundingMode>();
    assert_format::<Q<9>>();
    assert_format::<UQ<9>>();
    assert_format::<Signal<u8>>();
    assert_format::<Signal<f32>>();
    assert_format::<SignalClass>();
}
