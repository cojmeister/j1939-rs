use j1939_macros::j1939_message;

#[j1939_message(pgn = 1, length = 224)]
struct TooLong {
    #[j1939(bits = 0..8)]
    v: u8,
}

fn main() {}
