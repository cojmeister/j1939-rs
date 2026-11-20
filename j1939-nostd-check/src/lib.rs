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

#[j1939_message(pgn = 61445, length = 8, single_frame)]
pub struct Short {
    #[j1939(bits = 0..8)]
    pub value: u8,
    #[j1939(bits = 8..64, reserved)]
    pub _reserved: (),
}

#[j1939_message(pgn = 0xFF10, length = 8)]
pub struct MuxA {
    #[j1939(bits = 0..8, reserved)]
    pub _sel: (),
    #[j1939(bits = 8..24)]
    pub value: u16,
    #[j1939(bits = 24..64, reserved)]
    pub _reserved: (),
}

#[j1939_message(pgn = 0xFF10, length = 8)]
pub struct MuxB {
    #[j1939(bits = 0..8, reserved)]
    pub _sel: (),
    #[j1939(bits = 8..16)]
    pub count: u8,
    #[j1939(bits = 16..64, reserved)]
    pub _reserved: (),
}

#[j1939_macros::j1939_mux(selector = 0..8)]
pub enum Mux {
    #[j1939(selector = 1)]
    A(MuxA),
    #[j1939(selector = 2)]
    B(MuxB),
}

/// Keeps the generated mux code reachable in a `no_std` build.
pub fn mux_roundtrip(m: &Mux) -> j1939_core::Result<Mux> {
    let mut msg = j1939_core::J1939Message::default();
    m.marshall(&mut msg)?;
    Mux::unmarshall(&msg)
}

/// Drives a BAM transfer of a 20-byte message into a receiver with a simulated clock and
/// returns the reassembled message.
pub fn tp_bam_roundtrip() -> Option<j1939_core::J1939Message> {
    use j1939_core::tp::{TpRx, TpTx, TxState};

    let mut msg = j1939_core::J1939Message {
        pgn: 0xFECA,
        length: 20,
        source_address: 0x80,
        destination_address: 0xFF,
        ..Default::default()
    };
    for (i, b) in msg.data[..20].iter_mut().enumerate() {
        *b = i as u8;
    }
    let mut tx = TpTx::bam(&msg).ok()?;
    let mut rx = TpRx::<1>::new(0x20);
    let mut now_ms = 0u32;
    let mut delivered = None;
    while tx.state() == TxState::Active && now_ms < 10_000 {
        if let Some(frame) = tx.poll(now_ms) {
            let result = rx.on_frame(&frame, now_ms);
            if result.message.is_some() {
                delivered = result.message;
            }
        }
        now_ms += 10;
    }
    delivered
}

/// Marshals and unmarshals a DM1 with one DTC.
pub fn dm1_roundtrip() -> j1939_core::Result<j1939_core::Dm1<4>> {
    use j1939_core::{Dm1, Dtc, J1939Message, LampStatus};

    let mut dm = Dm1::<4>::new(LampStatus::default(), LampStatus::default());
    dm.push(Dtc {
        spn: 110,
        fmi: 3,
        conversion_method: false,
        occurrence_count: 1,
    })?;
    let mut msg = J1939Message::default();
    dm.marshall(&mut msg)?;
    Dm1::<4>::unmarshall(&msg)
}
