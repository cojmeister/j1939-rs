//! Diagnostic messages DM1 (active DTCs) and DM2 (previously active DTCs).

use crate::{Error, J1939Message, Marshall, Result, Unmarshall};

/// A diagnostic trouble code (SPN, FMI, conversion method, occurrence count).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dtc {
    /// Suspect parameter number, at most `0x7FFFF`.
    pub spn: u32,
    /// Failure mode identifier, at most 31.
    pub fmi: u8,
    pub conversion_method: bool,
    /// Occurrence count, at most 126 (127 means not available).
    pub occurrence_count: u8,
}

impl Dtc {
    /// Encodes the DTC into its 4-byte wire form.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidLength`] if `spn > 0x7FFFF`, `fmi > 31` or `occurrence_count > 126`.
    pub fn encode(&self) -> Result<[u8; 4]> {
        if self.spn > 0x7FFFF || self.fmi > 31 || self.occurrence_count > 126 {
            return Err(Error::InvalidLength);
        }
        Ok([
            self.spn as u8,
            (self.spn >> 8) as u8,
            (((self.spn >> 16) as u8) << 5) | self.fmi,
            (u8::from(self.conversion_method) << 7) | self.occurrence_count,
        ])
    }

    /// Decodes a 4-byte DTC.
    pub fn decode(b: [u8; 4]) -> Dtc {
        Dtc {
            spn: u32::from(b[0]) | (u32::from(b[1]) << 8) | (u32::from(b[2] >> 5) << 16),
            fmi: b[2] & 0x1F,
            conversion_method: b[3] & 0x80 != 0,
            occurrence_count: b[3] & 0x7F,
        }
    }
}

/// Lamp status bits (each field 0..=3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct LampStatus {
    pub malfunction: u8,
    pub red_stop: u8,
    pub amber_warning: u8,
    pub protect: u8,
}

impl LampStatus {
    /// Packs the four 2-bit fields (malfunction in bits 7-6, protect in bits 1-0).
    pub fn encode(&self) -> u8 {
        ((self.malfunction & 3) << 6)
            | ((self.red_stop & 3) << 4)
            | ((self.amber_warning & 3) << 2)
            | (self.protect & 3)
    }

    pub fn decode(b: u8) -> LampStatus {
        LampStatus {
            malfunction: b >> 6,
            red_stop: (b >> 4) & 3,
            amber_warning: (b >> 2) & 3,
            protect: b & 3,
        }
    }
}

const DM_PRIORITY: u8 = 6;
const MIN_LENGTH: usize = 8;

/// DM1: lamp status plus up to `N` active DTCs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dm1<const N: usize> {
    pub lamps: LampStatus,
    pub flash: LampStatus,
    dtcs: [Dtc; N],
    count: usize,
}

impl<const N: usize> Dm1<N> {
    pub const PGN: u32 = 0xFECA;

    pub fn new(lamps: LampStatus, flash: LampStatus) -> Self {
        Self {
            lamps,
            flash,
            dtcs: [Dtc::default(); N],
            count: 0,
        }
    }

    /// Appends a DTC.
    ///
    /// # Errors
    ///
    /// [`Error::BufferTooSmall`] when `N` DTCs are already stored.
    pub fn push(&mut self, dtc: Dtc) -> Result<()> {
        if self.count == N {
            return Err(Error::BufferTooSmall);
        }
        self.dtcs[self.count] = dtc;
        self.count += 1;
        Ok(())
    }

    pub fn dtcs(&self) -> &[Dtc] {
        &self.dtcs[..self.count]
    }

    fn marshall_with_pgn(&self, pgn: u32, msg: &mut J1939Message) -> Result<()> {
        let length = core::cmp::max(MIN_LENGTH, 2 + 4 * self.count);
        if length > J1939Message::MAX_LENGTH {
            return Err(Error::InvalidLength);
        }
        let mut data = [0u8; J1939Message::MAX_LENGTH];
        data[0] = self.lamps.encode();
        data[1] = self.flash.encode();
        for (i, dtc) in self.dtcs().iter().enumerate() {
            data[2 + 4 * i..6 + 4 * i].copy_from_slice(&dtc.encode()?);
        }
        // Zero DTCs is signalled by one all-zero DTC, which `data` already holds.
        let used = if self.count == 0 {
            6
        } else {
            2 + 4 * self.count
        };
        data[used..length].fill(0xFF);
        msg.priority = DM_PRIORITY;
        msg.pgn = pgn;
        msg.length = length as u16;
        msg.data = data;
        Ok(())
    }

    fn unmarshall_from(msg: &J1939Message) -> Result<Self> {
        let length = usize::from(msg.length);
        if !(MIN_LENGTH..=J1939Message::MAX_LENGTH).contains(&length) {
            return Err(Error::InvalidLength);
        }
        let mut dm = Self::new(
            LampStatus::decode(msg.data[0]),
            LampStatus::decode(msg.data[1]),
        );
        let count = (length - 2) / 4;
        for i in 0..count {
            let o = 2 + 4 * i;
            let raw = [
                msg.data[o],
                msg.data[o + 1],
                msg.data[o + 2],
                msg.data[o + 3],
            ];
            if count == 1 && raw == [0; 4] {
                break;
            }
            dm.push(Dtc::decode(raw))?;
        }
        Ok(dm)
    }
}

impl<const N: usize> Marshall for Dm1<N> {
    fn marshall(&self, msg: &mut J1939Message) -> Result<()> {
        self.marshall_with_pgn(Self::PGN, msg)
    }
}

impl<const N: usize> Unmarshall for Dm1<N> {
    fn unmarshall(msg: &J1939Message) -> Result<Self> {
        Self::unmarshall_from(msg)
    }
}

/// DM2: same layout as [`Dm1`] for previously active DTCs (PGN `0xFECB`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Dm2<const N: usize>(pub Dm1<N>);

impl<const N: usize> Dm2<N> {
    pub const PGN: u32 = 0xFECB;
}

impl<const N: usize> Marshall for Dm2<N> {
    fn marshall(&self, msg: &mut J1939Message) -> Result<()> {
        self.0.marshall_with_pgn(Self::PGN, msg)
    }
}

impl<const N: usize> Unmarshall for Dm2<N> {
    fn unmarshall(msg: &J1939Message) -> Result<Self> {
        Dm1::unmarshall_from(msg).map(Dm2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dtc(spn: u32, fmi: u8, n: u8) -> Dtc {
        Dtc {
            spn,
            fmi,
            conversion_method: false,
            occurrence_count: n,
        }
    }

    #[test]
    fn dtc_encodings() {
        let max = Dtc {
            spn: 0x7FFFF,
            fmi: 31,
            conversion_method: true,
            occurrence_count: 126,
        };
        assert_eq!(max.encode().unwrap(), [0xFF, 0xFF, 0xFF, 0xFE]);
        assert_eq!(Dtc::decode([0xFF, 0xFF, 0xFF, 0xFE]), max);
        assert_eq!(dtc(110, 3, 1).encode().unwrap(), [0x6E, 0x00, 0x03, 0x01]);
    }

    #[test]
    fn dtc_out_of_range() {
        assert_eq!(dtc(1, 32, 0).encode(), Err(Error::InvalidLength));
        assert_eq!(dtc(0x80000, 0, 0).encode(), Err(Error::InvalidLength));
        assert_eq!(dtc(1, 0, 127).encode(), Err(Error::InvalidLength));
    }

    #[test]
    fn lamp_encoding() {
        let l = LampStatus {
            malfunction: 1,
            red_stop: 0,
            amber_warning: 3,
            protect: 2,
        };
        assert_eq!(l.encode(), 0b01_00_11_10);
        assert_eq!(LampStatus::decode(0b01_00_11_10), l);
    }

    #[test]
    fn empty_dm1() {
        let dm = Dm1::<4>::new(LampStatus::default(), LampStatus::default());
        let mut msg = J1939Message::default();
        dm.marshall(&mut msg).unwrap();
        assert_eq!(msg.length, 8);
        assert_eq!(msg.priority, 6);
        assert_eq!(msg.pgn, 0xFECA);
        assert_eq!(&msg.data[..8], &[0, 0, 0, 0, 0, 0, 0xFF, 0xFF]);
        let back = Dm1::<4>::unmarshall(&msg).unwrap();
        assert!(back.dtcs().is_empty());
        assert_eq!(back, dm);
    }

    #[test]
    fn one_dtc() {
        let mut dm = Dm1::<4>::new(LampStatus::decode(0b0100_0000), LampStatus::default());
        dm.push(dtc(110, 3, 1)).unwrap();
        let mut msg = J1939Message::default();
        dm.marshall(&mut msg).unwrap();
        assert_eq!(msg.length, 8);
        assert_eq!(
            &msg.data[..8],
            &[0x40, 0, 0x6E, 0x00, 0x03, 0x01, 0xFF, 0xFF]
        );
        let back = Dm1::<4>::unmarshall(&msg).unwrap();
        assert_eq!(back.dtcs(), &[dtc(110, 3, 1)]);
        assert_eq!(back, dm);
    }

    #[test]
    fn five_dtcs_round_trip() {
        let mut dm = Dm1::<8>::new(LampStatus::default(), LampStatus::default());
        for i in 0..5u8 {
            dm.push(dtc(100 + u32::from(i), i, i + 1)).unwrap();
        }
        let mut msg = J1939Message::default();
        dm.marshall(&mut msg).unwrap();
        assert_eq!(msg.length, 22);
        assert_eq!(Dm1::<8>::unmarshall(&msg).unwrap(), dm);
    }

    #[test]
    fn push_full() {
        let mut dm = Dm1::<2>::new(LampStatus::default(), LampStatus::default());
        dm.push(dtc(1, 1, 1)).unwrap();
        dm.push(dtc(2, 1, 1)).unwrap();
        assert_eq!(dm.push(dtc(3, 1, 1)), Err(Error::BufferTooSmall));
    }

    #[test]
    fn decode_errors() {
        let mut dm = Dm1::<8>::new(LampStatus::default(), LampStatus::default());
        for i in 0..5u8 {
            dm.push(dtc(100, 1, i + 1)).unwrap();
        }
        let mut msg = J1939Message::default();
        dm.marshall(&mut msg).unwrap();
        assert_eq!(Dm1::<2>::unmarshall(&msg), Err(Error::BufferTooSmall));
        msg.length = 7;
        assert_eq!(Dm1::<8>::unmarshall(&msg), Err(Error::InvalidLength));
        msg.length = 224;
        assert_eq!(Dm1::<8>::unmarshall(&msg), Err(Error::InvalidLength));
    }

    #[test]
    fn dm2_uses_its_pgn() {
        let mut inner = Dm1::<4>::new(LampStatus::default(), LampStatus::default());
        inner.push(dtc(5, 2, 3)).unwrap();
        let dm = Dm2(inner);
        let mut msg = J1939Message::default();
        dm.marshall(&mut msg).unwrap();
        assert_eq!(msg.pgn, 0xFECB);
        assert_eq!(Dm2::<4>::unmarshall(&msg).unwrap(), dm);
    }
}
