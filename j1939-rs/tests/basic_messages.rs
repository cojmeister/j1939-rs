//! Basic message marshalling and unmarshalling

use j1939_rs::prelude::*;

#[test]
fn test_simple_uint_message() {
    #[j1939_message(pgn = 12345)]
    struct SimpleMessage {
        #[j1939(bits = 0..8)]
        value1: u8,
        #[j1939(bits = 8..16)]
        value2: u8,
        #[j1939(bits = 16..64, reserved)]
        _reserved: (),
    }

    let msg = SimpleMessage {
        value1: 42,
        value2: 100,
        _reserved: (),
    };

    let mut j1939_msg = J1939Message::default();
    msg.marshall(&mut j1939_msg).unwrap();

    assert_eq!(j1939_msg.pgn, 12345);
    assert_eq!(j1939_msg.data[0], 42);
    assert_eq!(j1939_msg.data[1], 100);

    let decoded = SimpleMessage::unmarshall(&j1939_msg).unwrap();
    assert_eq!(decoded.value1, 42);
    assert_eq!(decoded.value2, 100);
}

#[test]
fn test_scaled_fields_without_offset() {
    #[j1939_message(pgn = 61444)]
    struct EngineSpeed {
        #[j1939(bits = 0..16, scale = 0.125, unit = "rpm")]
        engine_speed: f32,
        #[j1939(bits = 16..64, reserved)]
        _reserved: (),
    }

    let msg = EngineSpeed {
        engine_speed: 1850.0,
        _reserved: (),
    };

    let mut j1939_msg = J1939Message::default();
    msg.marshall(&mut j1939_msg).unwrap();

    let decoded = EngineSpeed::unmarshall(&j1939_msg).unwrap();
    assert!((decoded.engine_speed - 1850.0).abs() < 0.2); // Allow for rounding
}

#[test]
fn test_scaled_fields_with_offset() {
    #[j1939_message(pgn = 65262)]
    struct TemperatureMessage {
        #[j1939(bits = 0..8, scale = 1.0, offset = -40.0, unit = "°C")]
        coolant_temp: f32,
        #[j1939(bits = 8..16, scale = 1.0, offset = -125.0, unit = "%")]
        torque: f32,
        #[j1939(bits = 16..64, reserved)]
        _reserved: (),
    }

    let msg = TemperatureMessage {
        coolant_temp: 85.0,
        torque: 45.0,
        _reserved: (),
    };

    let mut j1939_msg = J1939Message::default();
    msg.marshall(&mut j1939_msg).unwrap();

    let decoded = TemperatureMessage::unmarshall(&j1939_msg).unwrap();
    assert!((decoded.coolant_temp - 85.0).abs() < 0.1);
    assert!((decoded.torque - 45.0).abs() < 0.1);
}

#[test]
fn test_message_constants() {
    #[j1939_message(pgn = 61444, priority = 3, length = 64)]
    struct TestMessage {
        #[j1939(bits = 0..64, reserved)]
        _reserved: (),
    }

    assert_eq!(TestMessage::PGN, 61444);
    assert_eq!(TestMessage::PRIORITY, 3);
    assert_eq!(TestMessage::LENGTH, 8); // 64 bits = 8 bytes
}

#[test]
fn test_roundtrip_precision() {
    #[j1939_message(pgn = 65265)]
    struct PrecisionTest {
        #[j1939(bits = 0..16, scale = 0.1)]
        value1: f32,
        #[j1939(bits = 16..32, scale = 0.01)]
        value2: f32,
        #[j1939(bits = 32..64, reserved)]
        _reserved: (),
    }

    // Note: Without offset, sign extension is applied, so we use values
    // within the positive signed 16-bit range (0 to 32767)
    let test_values = [
        (0.0, 0.0),
        (123.4, 123.45),
        (3276.7, 327.67), // Max safe values for signed 16-bit with these scales
    ];

    for (v1, v2) in test_values {
        let msg = PrecisionTest {
            value1: v1,
            value2: v2,
            _reserved: (),
        };

        let mut j1939_msg = J1939Message::default();
        msg.marshall(&mut j1939_msg).unwrap();
        let decoded = PrecisionTest::unmarshall(&j1939_msg).unwrap();

        assert!(
            (decoded.value1 - v1).abs() < 0.001,
            "value1 mismatch: expected {}, got {}",
            v1,
            decoded.value1
        );
        assert!(
            (decoded.value2 - v2).abs() < 0.001,
            "value2 mismatch: expected {}, got {}",
            v2,
            decoded.value2
        );
    }
}
