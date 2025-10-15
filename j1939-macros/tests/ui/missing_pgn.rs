use j1939_macros::j1939_message;

// Missing required 'pgn' attribute
#[j1939_message(priority = 3)]
struct MissingPgn {
    #[j1939(bits = 0..8)]
    value: u8,
}

fn main() {}
