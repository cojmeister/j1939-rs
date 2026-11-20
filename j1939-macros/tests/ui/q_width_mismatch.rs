use j1939_macros::j1939_message;

#[j1939_message(pgn = 1, length = 2)]
struct QWidth {
    #[j1939(bits = 0..8, q = 9)]
    v: f32,
    #[j1939(bits = 8..16, reserved)]
    _r: (),
}

fn main() {}
