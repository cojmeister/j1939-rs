use j1939_macros::j1939_message;

#[j1939_message(pgn = 1, length = 3)]
struct BigEndianOddWidth {
    #[j1939(bits = 0..8)]
    a: u8,
    #[j1939(bits = 8..20, byte_order = "big")]
    v: u16,
    #[j1939(bits = 20..24, reserved)]
    _r: (),
}

fn main() {}
