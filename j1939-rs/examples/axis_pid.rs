use j1939_rs::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Axis {
    Roll = 0,
    Pitch = 1,
    Yaw = 2,
    Undefined = 3,
}

/// PGN 65417 (0xFF89): FDR/Telemetry: Axis PID (Roll, Pitch, Yaw)
/// This message contains PID control data for aircraft stabilization
#[j1939_message(pgn = 65417, priority = 0, length = 112)]
pub struct AxisPidRpy {
    /// SID - The sequence identifier field is used to tie related PGNs together
    #[j1939(bits = 0..8)]
    pub sid: u8,

    /// Command angle (deg, unused for Yaw)
    #[j1939(bits = 8..18, scale = 0.1, unit = "deg")]
    pub angle_command: f32,

    /// Angle P (unused for Yaw)
    #[j1939(bits = 18..28, scale = 0.1, unit = "deg/s")]
    pub angle_p: f32,

    /// Angle I (unused for Yaw)
    #[j1939(bits = 28..38, scale = 0.1, unit = "deg/s")]
    pub angle_i: f32,

    /// Angle D (unused for Yaw)
    #[j1939(bits = 38..48, scale = 0.1, unit = "deg/s")]
    pub angle_d: f32,

    /// Rate Command (output of Angle PID for Roll/Pitch, direct Rate input for Yaw)
    #[j1939(bits = 48..58, scale = 0.1, unit = "deg/s")]
    pub rate_command: f32,

    /// Rate P - Proportional gain for rate control
    #[j1939(bits = 58..68, encoding = "q9")]
    pub rate_p: f32,

    /// Rate I - Integral gain for rate control
    #[j1939(bits = 68..78, encoding = "q9")]
    pub rate_i: f32,

    /// Rate D - Derivative gain for rate control
    #[j1939(bits = 78..88, encoding = "q9")]
    pub rate_d: f32,

    /// PID Output - Final control output value
    #[j1939(bits = 88..98, encoding = "q9")]
    pub output: f32,

    /// Axis selection (Roll, Pitch, Yaw, or Undefined)
    #[j1939(bits = 98..100)]
    pub axis: Axis,

    /// Saturation Mask - Indicates which PID terms are saturated
    /// This is a multiline doc
    #[j1939(bits = 104..112)]
    pub saturation_mask: u8,
}

fn main() {
    let msg = AxisPidRpy {
        sid: 42,
        axis: Axis::Roll,
        saturation_mask: 0b10101010,
        angle_command: 12.5,
        angle_p: -5.3,
        angle_i: 0.0,
        angle_d: 10.1,
        rate_command: -25.0,
        rate_p: 0.5,
        rate_i: -0.25,
        rate_d: 0.1,
        output: -0.75,
    };

    let mut j1939_msg = J1939Message::default();
    msg.marshall(&mut j1939_msg).unwrap();

    println!("PGN: {}", j1939_msg.pgn);
    println!("Length: {}", j1939_msg.length);
    println!("Data: {:02X?}", &j1939_msg.data[..j1939_msg.length as usize]);

    let decoded = AxisPidRpy::unmarshall(&j1939_msg).unwrap();
    println!("\nDecoded:");
    println!("  SID: {}", decoded.sid);
    println!("  Angle Command: {:.1}", decoded.angle_command);
    println!("  Rate P: {:.3}", decoded.rate_p);
}