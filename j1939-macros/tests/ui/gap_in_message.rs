use j1939_macros::j1939_message;

#[j1939_message(pgn = 0xF039, length = 8)]
struct MessageWithGap {
    #[j1939(bits = 0..8)]
    value1: u8,

    // Gap from bits 8..16
    #[j1939(bits = 16..24)]
    value2: u8,
    // Gap from bits 24..64
}

fn main() {}
