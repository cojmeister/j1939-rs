use j1939_macros::j1939_message;

#[j1939_message(pgn = 0xF001, length = 1)]
struct SignalOneBit {
    #[j1939(bits = 0..1)]
    v: j1939_core::Signal<u8>,
    #[j1939(bits = 1..8, reserved)]
    _r: (),
}

fn main() {}
