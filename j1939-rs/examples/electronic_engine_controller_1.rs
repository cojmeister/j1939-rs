use j1939_rs::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[j1939_enum]
/// Engine torque mode
pub enum TorqueMode {
    /// Low idle governor, vehicle accelerator pedal released
    LowIdle = 0,
    /// Accelerator pedal position or manual selection controls engine
    AcceleratorControl = 1,
    /// Cruise control active
    CruiseControl = 2,
    /// PTO (power take-off) governor or engine speed control active
    PtoGovernor = 3,
    /// Road speed governor or vehicle speed limit control active
    RoadSpeedGovernor = 4,
    /// ASR (Anti-Slip Regulation) control active
    AsrControl = 5,
    /// Transmission control active
    TransmissionControl = 6,
    /// ABS (Anti-lock Braking System) control active
    AbsControl = 7,
    /// Torque limiting active
    TorqueLimiting = 8,
    /// High speed governor active
    HighSpeedGovernor = 9,
    /// Braking active
    Braking = 10,
    /// Remote accelerator override active
    RemoteAccelerator = 11,
    /// Not available
    NotAvailable = 15,
}

/// PGN 61444 (0xF004): Electronic Engine Controller 1 (EEC1)
/// This message provides engine speed, torque, and operating mode information
#[j1939_message(pgn = 61444, priority = 3, length = 64)]
pub struct ElectronicEngineController1 {
    /// Engine Torque Mode - Current engine operating mode
    #[j1939(bits = 0..4)]
    pub torque_mode: TorqueMode,

    /// Reserved bits for future use
    #[j1939(bits = 4..8, reserved)]
    pub reserved1: (),

    /// Driver's Demand Engine - Percent Torque
    /// The requested torque output of the engine by the driver
    #[j1939(bits = 8..16, offset = -125.0, scale = 1.0, unit = "%")]
    pub drivers_demand_torque: f32,

    /// Actual Engine - Percent Torque
    /// The calculated output torque of the engine
    #[j1939(bits = 16..24, offset = -125.0, scale = 1.0, unit = "%")]
    pub actual_engine_torque: f32,

    /// Engine Speed - The actual engine speed which is calculated over a minimum crankshaft angle
    #[j1939(bits = 24..40, scale = 0.125, unit = "rpm")]
    pub engine_speed: f32,

    /// Source Address of device controlling engine
    #[j1939(bits = 40..48)]
    pub source_address: u8,

    /// Engine Starter Mode
    /// 0 = start not requested, 1 = starter active, 2 = error, 3 = not available
    #[j1939(bits = 48..50)]
    pub starter_mode: u8,

    /// Reserved bits
    #[j1939(bits = 50..56, reserved)]
    pub reserved2: (),

    /// Engine Demand - Percent Torque
    /// The requested torque output from all external sources (cruise, PTO, etc.)
    #[j1939(bits = 56..64, offset = -125.0, scale = 1.0, unit = "%")]
    pub engine_demand_torque: f32,
}

fn main() {
    // Create an EEC1 message representing an engine at cruising speed
    let msg = ElectronicEngineController1 {
        torque_mode: TorqueMode::CruiseControl,
        reserved1: (),
        drivers_demand_torque: 45.0, // Driver requesting 45% torque
        actual_engine_torque: 43.0,  // Engine producing 43% torque
        engine_speed: 1850.0,        // Engine at 1850 RPM
        source_address: 0,           // Engine ECU at address 0
        starter_mode: 0,             // Starter not requested
        reserved2: (),
        engine_demand_torque: 45.0, // External demand matches driver
    };

    // Marshall the message into J1939 format
    let mut j1939_msg = J1939Message::default();
    msg.marshall(&mut j1939_msg).unwrap();

    println!("PGN: {} (0x{:04X})", j1939_msg.pgn, j1939_msg.pgn);
    println!("Priority: 3");
    println!("Length: {} bytes", j1939_msg.length);
    println!(
        "Data: {:02X?}",
        &j1939_msg.data[..j1939_msg.length as usize]
    );

    // Unmarshall and display the decoded values
    let decoded = ElectronicEngineController1::unmarshall(&j1939_msg).unwrap();
    println!("\nDecoded EEC1 Message:");
    println!("  Torque Mode: {:?}", decoded.torque_mode);
    println!(
        "  Driver's Demand Torque: {:.1}%",
        decoded.drivers_demand_torque
    );
    println!(
        "  Actual Engine Torque: {:.1}%",
        decoded.actual_engine_torque
    );
    println!("  Engine Speed: {:.1} RPM", decoded.engine_speed);
    println!("  Source Address: 0x{:02X}", decoded.source_address);
    println!(
        "  Engine Demand Torque: {:.1}%",
        decoded.engine_demand_torque
    );
}
