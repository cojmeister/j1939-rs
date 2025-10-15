use j1939_macros::j1939_message;

#[j1939_message(pgn = 12345)]
struct MissingBits {
    // Missing #[j1939(bits = ...)] attribute
    value: u8,
}

fn main() {}
