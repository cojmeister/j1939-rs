#![no_std]

pub mod bitfield;
pub mod fixed_point;
pub mod message;

pub use bitfield::*;
pub use fixed_point::*;
pub use message::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    InvalidPgn,
    InvalidLength,
    BufferTooSmall,
}

pub type Result<T> = core::result::Result<T, Error>;
