use j1939_rs::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[j1939_enum]
/// Represents different gear positions
pub enum GearPosition {
    /// Park gear
    Park = 0,
    /// Reverse gear
    Reverse = 1,
    /// Neutral position
    Neutral = 2,
    Drive = 3,
    /// Low gear for climbing
    Low = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[j1939_enum]
/// Engine status indicators
pub enum EngineStatus {
    Off = 0,
    /// Engine is starting up
    Starting = 1,
    /// Engine running normally
    Running = 2,
    /// Engine has a fault condition
    Fault = 7,
}

/// Test message with multiple enum fields
#[j1939_message(pgn = 12345, priority = 3, length = 24)]
pub struct TestMessage {
    #[j1939(bits = 0..8)]
    pub message_id: u8,

    #[j1939(bits = 8..11)]
    pub gear: GearPosition,

    #[j1939(bits = 11..14)]
    pub engine_status: EngineStatus,

    #[j1939(bits = 14..24, scale = 0.1, unit = "mph")]
    pub speed: f32,
}

fn main() {
    let msg = TestMessage {
        message_id: 42,
        gear: GearPosition::Drive,
        engine_status: EngineStatus::Running,
        speed: 65.5,
    };

    let mut j1939_msg = J1939Message::default();
    msg.marshall(&mut j1939_msg).unwrap();

    println!("PGN: {}", j1939_msg.pgn);
    println!("Length: {}", j1939_msg.length);
    println!("Data: {:02X?}", &j1939_msg.data[..j1939_msg.length as usize]);

    let decoded = TestMessage::unmarshall(&j1939_msg).unwrap();
    println!("\nDecoded:");
    println!("  Message ID: {}", decoded.message_id);
    println!("  Gear: {:?}", decoded.gear);
    println!("  Engine Status: {:?}", decoded.engine_status);
    println!("  Speed: {:.1} mph", decoded.speed);
}