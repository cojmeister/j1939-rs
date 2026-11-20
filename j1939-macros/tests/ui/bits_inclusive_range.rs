use j1939_macros::j1939_message;

#[j1939_message(pgn = 0xF004)]
struct Bad {
    #[j1939(bits = 0..=7)]
    v: u8,
    #[j1939(bits = 8..64, reserved)]
    rest: u64,
}

fn main() {}
