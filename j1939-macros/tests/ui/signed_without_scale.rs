use j1939_macros::j1939_message;

#[j1939_message(pgn = 1, length = 1)]
struct SignedFlag {
    #[j1939(bits = 0..8, signed)]
    v: u8,
}

fn main() {}
