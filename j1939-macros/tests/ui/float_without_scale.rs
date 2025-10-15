use j1939_macros::j1939_message;

#[j1939_message(pgn = 12345)]
struct FloatWithoutScale {
    // Float fields require either scale or encoding attribute
    #[j1939(bits = 0..16)]
    speed: f32,
}

fn main() {}
