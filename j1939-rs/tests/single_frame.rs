//! `SINGLE_FRAME` constant and the `single_frame` message attribute.

use j1939_rs::prelude::*;

#[j1939_message(pgn = 0xF201, length = 8, single_frame)]
struct Short {
    #[j1939(bits = 0..8)]
    a: u8,
    #[j1939(bits = 8..64, reserved)]
    _r: (),
}

#[j1939_message(pgn = 0xF202, length = 9)]
struct Long {
    #[j1939(bits = 0..8)]
    a: u8,
    #[j1939(bits = 8..72, reserved)]
    _r: (),
}

#[test]
fn single_frame_const_true_for_eight_bytes() {
    let flag = std::hint::black_box(Short::SINGLE_FRAME);
    assert!(flag);
}

#[test]
fn single_frame_const_false_for_nine_bytes_without_flag() {
    let flag = std::hint::black_box(Long::SINGLE_FRAME);
    assert!(!flag);
}
