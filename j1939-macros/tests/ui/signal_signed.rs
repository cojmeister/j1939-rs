use j1939_macros::j1939_message;

#[j1939_message(pgn = 1, length = 1)]
struct SignalSigned {
    #[j1939(bits = 0..8, scale = 1.0, signed)]
    v: j1939_core::Signal<f32>,
}

fn main() {}
