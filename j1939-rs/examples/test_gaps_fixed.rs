use j1939_rs::prelude::*;

/// Test message with properly marked reserved bits
#[j1939_message(pgn = 12345, priority = 3, length = 32)]
pub struct TestMessageFixed {
    #[j1939(bits = 0..8)]
    pub field1: u8,

    /// Reserved bits - explicitly marked
    #[j1939(bits = 8..16, reserved)]
    pub reserved1: (),

    #[j1939(bits = 16..24)]
    pub field2: u8,

    /// More reserved bits
    #[j1939(bits = 24..32, reserved)]
    pub reserved2: (),
}

fn main() {
    let msg = TestMessageFixed {
        field1: 42,
        reserved1: (),
        field2: 123,
        reserved2: (),
    };

    let mut j1939_msg = J1939Message::default();
    msg.marshall(&mut j1939_msg).unwrap();

    println!("Message with properly marked reserved bits:");
    println!("PGN: {}", j1939_msg.pgn);
    println!(
        "Data: {:02X?}",
        &j1939_msg.data[..j1939_msg.length as usize]
    );

    let decoded = TestMessageFixed::unmarshall(&j1939_msg).unwrap();
    println!("Field1: {}, Field2: {}", decoded.field1, decoded.field2);
}
