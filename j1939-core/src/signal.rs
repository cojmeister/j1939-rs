//! J1939 signal special-value handling (valid / reserved / error / not available).
//!
//! For signals of 8 or more bits the top byte (relative to the signal width)
//! selects the class: `0x00..=0xFA` valid, `0xFB..=0xFD` reserved, `0xFE` error,
//! `0xFF` not available. Signals of 2..=7 bits have no reserved class: all-ones
//! is not available, all-ones minus one is error.

/// A decoded signal value, or one of the J1939 special states.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Signal<T> {
    Value(T),
    NotAvailable,
    Error,
    /// A reserved raw value, preserved so it can be re-encoded unchanged.
    Reserved(u64),
}

impl<T> Signal<T> {
    /// The contained value, if the signal is valid.
    pub fn value(self) -> Option<T> {
        match self {
            Signal::Value(v) => Some(v),
            _ => None,
        }
    }
}

/// Classification of a raw signal value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalClass {
    Valid,
    Reserved,
    Error,
    NotAvailable,
}

fn all_ones(bits: u32) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}

/// Largest raw value that is a valid measurement. Requires `2 <= bits <= 64`.
pub fn valid_max_raw(bits: u32) -> u64 {
    debug_assert!((2..=64).contains(&bits));
    if bits >= 8 {
        (0xFBu64 << (bits - 8)) - 1
    } else {
        all_ones(bits) - 2
    }
}

/// Lowest raw value of the error range. Requires `2 <= bits <= 64`.
pub fn error_raw(bits: u32) -> u64 {
    debug_assert!((2..=64).contains(&bits));
    if bits >= 8 {
        0xFEu64 << (bits - 8)
    } else {
        all_ones(bits) - 1
    }
}

/// The canonical not-available raw value (all ones). Requires `2 <= bits <= 64`.
pub fn not_available_raw(bits: u32) -> u64 {
    debug_assert!((2..=64).contains(&bits));
    all_ones(bits)
}

/// Lowest raw value of the not-available range (`0xFF` in the top byte).
fn not_available_base(bits: u32) -> u64 {
    if bits >= 8 {
        0xFFu64 << (bits - 8)
    } else {
        all_ones(bits)
    }
}

/// Classify a raw value of a `bits`-wide signal. Requires `2 <= bits <= 64`.
pub fn classify_raw(raw: u64, bits: u32) -> SignalClass {
    if raw >= not_available_base(bits) {
        SignalClass::NotAvailable
    } else if raw >= error_raw(bits) {
        SignalClass::Error
    } else if raw > valid_max_raw(bits) {
        SignalClass::Reserved
    } else {
        SignalClass::Valid
    }
}

/// Convert a raw value to a [`Signal`], applying `valid` only to valid raws.
pub fn raw_to_signal<T>(raw: u64, bits: u32, valid: impl FnOnce(u64) -> T) -> Signal<T> {
    match classify_raw(raw, bits) {
        SignalClass::Valid => Signal::Value(valid(raw)),
        SignalClass::Reserved => Signal::Reserved(raw),
        SignalClass::Error => Signal::Error,
        SignalClass::NotAvailable => Signal::NotAvailable,
    }
}

/// Convert a [`Signal`] to its raw value; valid results are clamped to
/// [`valid_max_raw`] so they never collide with special values.
pub fn signal_to_raw<T>(s: &Signal<T>, bits: u32, valid: impl FnOnce(&T) -> u64) -> u64 {
    match s {
        Signal::Value(v) => valid(v).min(valid_max_raw(bits)),
        Signal::NotAvailable => not_available_raw(bits),
        Signal::Error => error_raw(bits),
        Signal::Reserved(r) => *r,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn narrow_signals_have_no_reserved_class() {
        for raw in 0..=0x1u64 {
            assert_eq!(classify_raw(raw, 2), SignalClass::Valid);
        }
        assert_eq!(classify_raw(0x7E, 7), SignalClass::Error);
        assert_eq!(classify_raw(0x7D, 7), SignalClass::Valid);
    }

    #[test]
    fn sixty_four_bit_bounds() {
        assert_eq!(valid_max_raw(64), 0xFAFF_FFFF_FFFF_FFFF);
        assert_eq!(classify_raw(u64::MAX, 64), SignalClass::NotAvailable);
    }
}
