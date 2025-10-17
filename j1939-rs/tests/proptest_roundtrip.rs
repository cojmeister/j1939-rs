//! Property-based tests using proptest

use j1939_rs::prelude::*;
use proptest::prelude::*;

#[j1939_message(pgn = 65000)]
struct ScaledTestMessage {
    #[j1939(bits = 0..8, scale = 0.5)]
    value1: f32,
    #[j1939(bits = 8..16, scale = 1.0)]
    value2: f32,
    #[j1939(bits = 16..64, reserved)]
    _reserved: (),
}

#[j1939_message(pgn = 65001)]
struct OffsetTestMessage {
    #[j1939(bits = 0..8, scale = 1.0, offset = -40.0)]
    temp: f32,
    #[j1939(bits = 8..16, scale = 1.0, offset = -125.0)]
    torque: f32,
    #[j1939(bits = 16..64, reserved)]
    _reserved: (),
}

proptest! {
    #[test]
    fn test_scaled_roundtrip(
        v1 in 0.0f32..=63.5,   // Max safe value for 8-bit signed with scale 0.5
        v2 in 0.0f32..=127.0,  // Max safe value for 8-bit signed with scale 1.0
    ) {
        let msg = ScaledTestMessage {
            value1: v1,
            value2: v2,
            _reserved: (),
        };

        let mut j1939_msg = J1939Message::default();
        msg.marshall(&mut j1939_msg).unwrap();
        let decoded = ScaledTestMessage::unmarshall(&j1939_msg).unwrap();

        // Allow for rounding errors due to float precision
        prop_assert!((decoded.value1 - v1).abs() < 1.0);
        prop_assert!((decoded.value2 - v2).abs() < 1.0);
    }

    #[test]
    fn test_offset_roundtrip(
        temp in -40.0f32..=215.0,   // Standard J1939 temperature range
        torque in -125.0f32..=125.0, // Standard J1939 torque range
    ) {
        let msg = OffsetTestMessage {
            temp,
            torque,
            _reserved: (),
        };

        let mut j1939_msg = J1939Message::default();
        msg.marshall(&mut j1939_msg).unwrap();
        let decoded = OffsetTestMessage::unmarshall(&j1939_msg).unwrap();

        prop_assert!((decoded.temp - temp).abs() < 1.0);
        prop_assert!((decoded.torque - torque).abs() < 1.0);
    }

    #[test]
    fn test_uint_field_roundtrip(
        val in 0u8..=255,
    ) {
        #[j1939_message(pgn = 65002)]
        struct UintTestMessage {
            #[j1939(bits = 0..8)]
            value: u8,
            #[j1939(bits = 8..64, reserved)]
            _reserved: (),
        }

        let msg = UintTestMessage {
            value: val,
            _reserved: (),
        };

        let mut j1939_msg = J1939Message::default();
        msg.marshall(&mut j1939_msg).unwrap();
        let decoded = UintTestMessage::unmarshall(& j1939_msg).unwrap();

        prop_assert_eq!(decoded.value, val);
    }

    #[test]
    fn test_multi_bit_uint_roundtrip(
        val in 0u16..=4095,  // 12-bit value
    ) {
        #[j1939_message(pgn = 65003)]
        struct MultiBitMessage {
            #[j1939(bits = 0..12)]
            value: u16,
            #[j1939(bits = 12..64, reserved)]
            _reserved: (),
        }

        let msg = MultiBitMessage {
            value: val,
            _reserved: (),
        };

        let mut j1939_msg = J1939Message::default();
        msg.marshall(&mut j1939_msg).unwrap();
        let decoded = MultiBitMessage::unmarshall(& j1939_msg).unwrap();

        prop_assert_eq!(decoded.value, val);
    }
}

#[test]
fn test_boundary_values_for_offset() {
    // Test exact boundary values that proptest might miss
    #[j1939_message(pgn = 65004)]
    struct BoundaryTest {
        #[j1939(bits = 0..8, scale = 1.0, offset = -40.0)]
        value: f32,
        #[j1939(bits = 8..64, reserved)]
        _reserved: (),
    }

    let test_values = [
        -40.0, // Minimum
        0.0,   // Zero crossing
        215.0, // Maximum
        85.0,  // Common operating temperature
    ];

    for &val in &test_values {
        let msg = BoundaryTest {
            value: val,
            _reserved: (),
        };

        let mut j1939_msg = J1939Message::default();
        msg.marshall(&mut j1939_msg).unwrap();
        let decoded = BoundaryTest::unmarshall(&j1939_msg).unwrap();

        assert!(
            (decoded.value - val).abs() < 0.1,
            "Failed for value {}: got {}",
            val,
            decoded.value
        );
    }
}
