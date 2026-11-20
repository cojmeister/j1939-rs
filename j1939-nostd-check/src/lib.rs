//! Build with: `cargo build -p j1939-nostd-check --target thumbv6m-none-eabi`
//! (Cortex-M0+, no FPU, no `std`). Generated code must not use anything outside `core`.
#![no_std]
#![forbid(unsafe_code)]

use j1939_core::{Marshall, Unmarshall};
use j1939_macros::{j1939_enum, j1939_message};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
#[j1939_enum]
pub enum Mode {
    Off = 0,
    On = 1,
}

#[j1939_message(pgn = 61444, priority = 3)]
pub struct Engine {
    #[j1939(bits = 0..16, scale = 0.125, unit = "rpm")]
    pub speed: f32,
    #[j1939(bits = 16..24, scale = 1.0, offset = -40.0, unit = "C")]
    pub temp: f32,
    #[j1939(bits = 24..34, encoding = "q9")]
    pub gain: f32,
    #[j1939(bits = 34..36)]
    pub mode: Mode,
    #[j1939(bits = 36..52, scale = 0.5)]
    pub load: j1939_core::Signal<f32>,
    #[j1939(bits = 52..64, reserved)]
    pub _reserved: (),
    #[j1939(bits = 64..80, byte_order = "big")]
    pub counter: u16,
}

/// Keeps the generated code reachable so it is actually compiled and linked in a `no_std` build.
pub fn roundtrip(e: &Engine) -> j1939_core::Result<Engine> {
    let mut msg = j1939_core::J1939Message::default();
    e.marshall(&mut msg)?;
    Engine::unmarshall(&msg)
}
