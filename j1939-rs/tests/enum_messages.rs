//! Tests for messages with enum fields

use j1939_rs::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[j1939_enum]
enum GearPosition {
    Park = 0,
    Reverse = 1,
    Neutral = 2,
    Drive = 3,
    Low = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[j1939_enum]
enum EngineStatus {
    Off = 0,
    Starting = 1,
    Running = 2,
    Fault = 7,
}

#[test]
fn test_single_enum_field() {
    #[j1939_message(pgn = 65265)]
    struct TransmissionStatus {
        #[j1939(bits = 0..3)]
        gear: GearPosition,
        #[j1939(bits = 3..64, reserved)]
        _reserved: (),
    }

    let msg = TransmissionStatus {
        gear: GearPosition::Drive,
        _reserved: (),
    };

    let mut j1939_msg = J1939Message::default();
    msg.marshall(&mut j1939_msg).unwrap();

    let decoded = TransmissionStatus::unmarshall(&j1939_msg).unwrap();
    assert_eq!(decoded.gear, GearPosition::Drive);
}

#[test]
fn test_multiple_enum_fields() {
    #[j1939_message(pgn = 12345)]
    struct VehicleStatus {
        #[j1939(bits = 0..3)]
        gear: GearPosition,
        #[j1939(bits = 3..6)]
        engine_status: EngineStatus,
        #[j1939(bits = 6..64, reserved)]
        _reserved: (),
    }

    let test_cases = [
        (GearPosition::Park, EngineStatus::Off),
        (GearPosition::Drive, EngineStatus::Running),
        (GearPosition::Neutral, EngineStatus::Starting),
        (GearPosition::Low, EngineStatus::Fault),
    ];

    for (gear, status) in test_cases {
        let msg = VehicleStatus {
            gear,
            engine_status: status,
            _reserved: (),
        };

        let mut j1939_msg = J1939Message::default();
        msg.marshall(&mut j1939_msg).unwrap();
        let decoded = VehicleStatus::unmarshall(&j1939_msg).unwrap();

        assert_eq!(decoded.gear, gear);
        assert_eq!(decoded.engine_status, status);
    }
}

#[test]
fn test_enum_with_other_fields() {
    #[j1939_message(pgn = 61444)]
    struct MixedMessage {
        #[j1939(bits = 0..3)]
        status: EngineStatus,
        #[j1939(bits = 3..19, scale = 0.125, unit = "rpm")]
        engine_speed: f32,
        #[j1939(bits = 19..27, scale = 1.0, offset = -40.0, unit = "°C")]
        temperature: f32,
        #[j1939(bits = 27..64, reserved)]
        _reserved: (),
    }

    let msg = MixedMessage {
        status: EngineStatus::Running,
        engine_speed: 1850.0,
        temperature: 85.0,
        _reserved: (),
    };

    let mut j1939_msg = J1939Message::default();
    msg.marshall(&mut j1939_msg).unwrap();
    let decoded = MixedMessage::unmarshall(&j1939_msg).unwrap();

    assert_eq!(decoded.status, EngineStatus::Running);
    assert!((decoded.engine_speed - 1850.0).abs() < 0.2);
    assert!((decoded.temperature - 85.0).abs() < 0.1);
}

#[test]
fn test_all_enum_variants() {
    #[j1939_message(pgn = 65266)]
    struct AllGears {
        #[j1939(bits = 0..3)]
        gear: GearPosition,
        #[j1939(bits = 3..64, reserved)]
        _reserved: (),
    }

    let all_gears = [
        GearPosition::Park,
        GearPosition::Reverse,
        GearPosition::Neutral,
        GearPosition::Drive,
        GearPosition::Low,
    ];

    for gear in all_gears {
        let msg = AllGears {
            gear,
            _reserved: (),
        };

        let mut j1939_msg = J1939Message::default();
        msg.marshall(&mut j1939_msg).unwrap();
        let decoded = AllGears::unmarshall(&j1939_msg).unwrap();

        assert_eq!(decoded.gear, gear, "Failed for gear: {:?}", gear);
    }
}
