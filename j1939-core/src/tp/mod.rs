//! SAE J1939-21 transport protocol (BAM and RTS/CTS) building blocks.

mod frame;
mod rx;
mod tx;

pub use frame::*;
pub use rx::*;
pub use tx::*;

/// Transport protocol timeouts and intervals, in milliseconds.
pub mod timing {
    pub const T1_MS: u32 = 750;
    pub const T2_MS: u32 = 1250;
    pub const T3_MS: u32 = 1250;
    pub const T4_MS: u32 = 1050;
    pub const BAM_INTERVAL_MS: u32 = 50;
}
