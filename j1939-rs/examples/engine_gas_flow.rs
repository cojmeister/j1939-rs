use j1939_rs::prelude::*;

#[j1939_message(pgn = 61450, priority = 2)]
pub struct EngineGasFlowRate {
    #[j1939(bits = 0..16, scale = 0.05, unit = "kg/h")]
    pub egr1_mass_flow_rate: f32,

    #[j1939(bits = 16..18)]
    pub egr1_error_status: u8,

    #[j1939(bits = 18..26)]
    pub intake_manifold_1_temp: u8,

    #[j1939(bits = 26..34)]
    pub egr1_temperature: u8,

    #[j1939(bits = 34..42, scale = 0.05, unit = "kPa")]
    pub egr1_diff_pressure: f32,

    #[j1939(bits = 42..50, scale = 0.1, unit = "L/min")]
    pub crankcase_vent_flow_rate: f32,

    #[j1939(bits = 50..58, scale = 0.4, unit = "%")]
    pub egr1_control: f32,

    #[j1939(bits = 58..64)]
    pub reserved: u8,
}

fn main() {
    // Create an example engine gas flow rate message
    let gas_flow = EngineGasFlowRate {
        egr1_mass_flow_rate: 150.5,    // 150.5 kg/h
        egr1_error_status: 0,          // No error
        intake_manifold_1_temp: 85,    // 45°C (85 + (-40) = 45)
        egr1_temperature: 120,         // 80°C (120 + (-40) = 80)
        egr1_diff_pressure: 2.5,       // 2.5 kPa
        crankcase_vent_flow_rate: 5.2, // 5.2 L/min
        egr1_control: 15.6,            // 15.6%
        reserved: 0,
    };

    // Marshall the data into a J1939 message
    let mut j1939_msg = J1939Message::default();
    gas_flow.marshall(&mut j1939_msg).unwrap();

    println!("=== Engine Gas Flow Rate Message ===");
    println!("PGN: {} (0x{:04X})", j1939_msg.pgn, j1939_msg.pgn);
    println!("Priority: {}", j1939_msg.priority);
    println!("Source Address: {}", j1939_msg.source_address);
    println!("Length: {} bytes", j1939_msg.length);
    println!(
        "Raw Data: {:02X?}",
        &j1939_msg.data[..j1939_msg.length as usize]
    );

    // Decode the message back
    let decoded = EngineGasFlowRate::unmarshall(&j1939_msg).unwrap();

    println!("\n=== Decoded Values ===");
    println!(
        "EGR1 Mass Flow Rate: {:.2} kg/h",
        decoded.egr1_mass_flow_rate
    );
    println!("EGR1 Error Status: {}", decoded.egr1_error_status);
    println!(
        "Intake Manifold Temp: {}°C",
        (decoded.intake_manifold_1_temp as i16) - 40
    );
    println!(
        "EGR1 Temperature: {}°C",
        (decoded.egr1_temperature as i16) - 40
    );
    println!("EGR1 Diff Pressure: {:.2} kPa", decoded.egr1_diff_pressure);
    println!(
        "Crankcase Vent Flow: {:.1} L/min",
        decoded.crankcase_vent_flow_rate
    );
    println!("EGR1 Control: {:.1}%", decoded.egr1_control);

    // Verify roundtrip accuracy
    println!("\n=== Roundtrip Verification ===");
    let tolerance = 0.1;
    let egr_flow_ok =
        (decoded.egr1_mass_flow_rate - gas_flow.egr1_mass_flow_rate).abs() < tolerance;
    let diff_pressure_ok =
        (decoded.egr1_diff_pressure - gas_flow.egr1_diff_pressure).abs() < tolerance;
    let vent_flow_ok =
        (decoded.crankcase_vent_flow_rate - gas_flow.crankcase_vent_flow_rate).abs() < tolerance;
    let control_ok = (decoded.egr1_control - gas_flow.egr1_control).abs() < tolerance;

    println!(
        "EGR1 Flow Rate: {} (diff: {:.3})",
        if egr_flow_ok { "✓" } else { "✗" },
        (decoded.egr1_mass_flow_rate - gas_flow.egr1_mass_flow_rate).abs()
    );
    println!(
        "EGR1 Error Status: {}",
        if decoded.egr1_error_status == gas_flow.egr1_error_status {
            "✓"
        } else {
            "✗"
        }
    );
    println!(
        "Intake Temp: {}",
        if decoded.intake_manifold_1_temp == gas_flow.intake_manifold_1_temp {
            "✓"
        } else {
            "✗"
        }
    );
    println!(
        "EGR1 Temperature: {}",
        if decoded.egr1_temperature == gas_flow.egr1_temperature {
            "✓"
        } else {
            "✗"
        }
    );
    println!(
        "EGR1 Diff Pressure: {} (diff: {:.3})",
        if diff_pressure_ok { "✓" } else { "✗" },
        (decoded.egr1_diff_pressure - gas_flow.egr1_diff_pressure).abs()
    );
    println!(
        "Crankcase Vent Flow: {} (diff: {:.3})",
        if vent_flow_ok { "✓" } else { "✗" },
        (decoded.crankcase_vent_flow_rate - gas_flow.crankcase_vent_flow_rate).abs()
    );
    println!(
        "EGR1 Control: {} (diff: {:.3})",
        if control_ok { "✓" } else { "✗" },
        (decoded.egr1_control - gas_flow.egr1_control).abs()
    );

    if egr_flow_ok
        && diff_pressure_ok
        && vent_flow_ok
        && control_ok
        && decoded.egr1_error_status == gas_flow.egr1_error_status
        && decoded.intake_manifold_1_temp == gas_flow.intake_manifold_1_temp
        && decoded.egr1_temperature == gas_flow.egr1_temperature
    {
        println!("\n🎉 All values match within tolerance!");
    } else {
        println!("\n⚠️  Some values don't match - check scaling factors");
    }
}
