#![no_std]
#![forbid(unsafe_code)]

pub mod bitfield;
pub mod byteorder;
pub mod fixed_point;
pub mod id;
pub mod message;
pub mod signal;

pub use bitfield::*;
pub use byteorder::*;
pub use fixed_point::*;
pub use id::*;
pub use message::*;
pub use signal::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error {
    InvalidPgn,
    /// Not a valid J1939 identifier (bad priority, missing destination, 11-bit id, remote frame).
    InvalidId,
    InvalidLength,
    BufferTooSmall,
    /// A wire value does not correspond to any variant of the enum used by `field`.
    InvalidEnum {
        field: &'static str,
        raw: u8,
    },
    /// The selector bits of a multiplexed message hold a value no variant is registered for.
    UnknownSelector {
        raw: u64,
    },
}

pub type Result<T> = core::result::Result<T, Error>;
