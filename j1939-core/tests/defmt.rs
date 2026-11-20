#![cfg(feature = "defmt")]

use j1939_core::tp::{AbortReason, CanFrame, RxResult, TpCm, TpRx, TpTx, TxState};
use j1939_core::{
    Dm1, Dm2, Dtc, Error, J1939Id, J1939Message, LampStatus, Q, RoundingMode, Signal, SignalClass,
    UQ,
};

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
    assert_format::<Dtc>();
    assert_format::<LampStatus>();
    assert_format::<Dm1<4>>();
    assert_format::<Dm2<4>>();
}

#[test]
fn tp_types_implement_format() {
    assert_format::<CanFrame>();
    assert_format::<TpCm>();
    assert_format::<AbortReason>();
    assert_format::<TxState>();
    assert_format::<RxResult>();
    assert_format::<TpTx>();
    assert_format::<TpRx<2>>();
}
