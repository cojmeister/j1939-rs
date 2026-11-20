use j1939_macros::j1939_message;

#[j1939_message(pgn = 0xF001, length = 9, single_frame)]
struct TooLong {
    #[j1939(bits = 0..8)]
    v: u8,
    #[j1939(bits = 8..72, reserved)]
    _r: (),
}

fn main() {}
