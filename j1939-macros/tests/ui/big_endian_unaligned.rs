use j1939_macros::j1939_message;

#[j1939_message(pgn = 1, length = 3)]
struct BigEndianUnaligned {
    #[j1939(bits = 0..4, reserved)]
    _r: (),
    #[j1939(bits = 4..20, byte_order = "big")]
    v: u16,
    #[j1939(bits = 20..24, reserved)]
    _r2: (),
}

fn main() {}
