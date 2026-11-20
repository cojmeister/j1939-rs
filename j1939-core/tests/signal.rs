use j1939_core::{
    Signal, SignalClass, classify_raw, error_raw, not_available_raw, raw_to_signal, signal_to_raw,
    valid_max_raw,
};

#[test]
fn special_value_constants() {
    let cases: [(u32, u64, u64, u64); 5] = [
        (8, 0xFF, 0xFE, 0xFA),
        (16, 0xFFFF, 0xFE00, 0xFAFF),
        (32, 0xFFFF_FFFF, 0xFE00_0000, 0xFAFF_FFFF),
        (12, 0xFFF, 0xFE0, 0xFAF),
        (4, 0xF, 0xE, 0xD),
    ];
    for (bits, na, err, max) in cases {
        assert_eq!(not_available_raw(bits), na, "na {bits}");
        assert_eq!(error_raw(bits), err, "err {bits}");
        assert_eq!(valid_max_raw(bits), max, "max {bits}");
    }
    assert_eq!(not_available_raw(2), 0x3);
    assert_eq!(error_raw(2), 0x2);
    assert_eq!(valid_max_raw(2), 0x1);
    assert_eq!(not_available_raw(64), u64::MAX);
    assert_eq!(error_raw(64), 0xFE00_0000_0000_0000);
}

#[test]
fn classification() {
    assert_eq!(classify_raw(0xFC, 8), SignalClass::Reserved);
    assert_eq!(classify_raw(0xFE05, 16), SignalClass::Error);
    assert_eq!(classify_raw(0xFA00, 16), SignalClass::Valid);
    assert_eq!(classify_raw(0xFF12, 16), SignalClass::NotAvailable);
    assert_eq!(classify_raw(0xFB00, 16), SignalClass::Reserved);
    assert_eq!(classify_raw(0x3, 2), SignalClass::NotAvailable);
    assert_eq!(classify_raw(0x2, 2), SignalClass::Error);
    assert_eq!(classify_raw(0x1, 2), SignalClass::Valid);
    assert_eq!(classify_raw(0xE, 4), SignalClass::Error);
    assert_eq!(classify_raw(0xD, 4), SignalClass::Valid);
}

#[test]
fn raw_to_signal_variants() {
    assert_eq!(raw_to_signal(10, 8, |r| r * 2), Signal::Value(20));
    assert_eq!(raw_to_signal(0xFF, 8, |r| r), Signal::NotAvailable);
    assert_eq!(raw_to_signal(0xFE, 8, |r| r), Signal::Error);
    assert_eq!(raw_to_signal(0xFC, 8, |r| r), Signal::Reserved(0xFC));
    assert_eq!(Signal::Value(3u8).value(), Some(3));
    assert_eq!(Signal::<u8>::Error.value(), None);
}

#[test]
fn signal_to_raw_clamps_and_roundtrips() {
    assert_eq!(
        signal_to_raw(&Signal::Value(65000u16), 16, |v| *v as u64),
        0xFAFF
    );
    assert_eq!(signal_to_raw(&Signal::Value(5u16), 16, |v| *v as u64), 5);
    assert_eq!(
        signal_to_raw(&Signal::<u8>::NotAvailable, 8, |v| *v as u64),
        0xFF
    );
    assert_eq!(
        signal_to_raw(&Signal::<u8>::Error, 16, |v| *v as u64),
        0xFE00
    );
    assert_eq!(
        signal_to_raw(&Signal::<u8>::Reserved(0xFC), 8, |v| *v as u64),
        0xFC
    );
}
