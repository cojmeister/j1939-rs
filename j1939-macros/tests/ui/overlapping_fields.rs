use j1939_macros::j1939_message;

#[j1939_message(pgn = 12345)]
struct OverlappingFields {
    #[j1939(bits = 0..8)]
    value1: u8,

    // This overlaps with value1
    #[j1939(bits = 4..12)]
    value2: u8,
}

fn main() {}
