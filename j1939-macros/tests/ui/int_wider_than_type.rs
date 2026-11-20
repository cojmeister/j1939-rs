use j1939_macros::j1939_message;

#[j1939_message(pgn = 1, length = 4)]
struct TooWide {
    #[j1939(bits = 0..20)]
    v: u16,
    #[j1939(bits = 20..32, reserved)]
    _r: (),
}

fn main() {}
