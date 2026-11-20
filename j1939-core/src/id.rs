//! The 29-bit J1939 CAN identifier.
//!
//! ```text
//!  28..26   25   24   23..16  15..8  7..0
//! priority  EDP  DP     PF      PS     SA
//! ```
//!
//! The 18-bit PGN is `EDP << 17 | DP << 16 | PF << 8 | PS`, **with `PS` zeroed for PDU1** (`PF <
//! 240`, destination-specific), where `PS` carries the destination address instead. The EDP and DP
//! bits are part of the PGN: dropping them silently aliases e.g. PGN `0x1F004` onto `0xF004`.
use crate::{Error, Result};

/// A decoded J1939 29-bit identifier.
///
/// # Examples
///
/// ```
/// use j1939_core::J1939Id;
///
/// // EEC1: priority 3, PGN 0xF004 (PDU2), source address 0
/// let id = J1939Id::new(3, 0xF004, None, 0).unwrap();
/// assert_eq!(id.to_raw(), 0x0CF0_0400);
///
/// // A PDU1 request to address 0x80 from 0xF9; the DA is not part of the PGN
/// let id = J1939Id::from_raw(0x18EA_80F9).unwrap();
/// assert_eq!(id.pgn(), 0xEA00);
/// assert_eq!(id.destination(), Some(0x80));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct J1939Id {
    priority: u8,
    pgn: u32,
    destination: Option<u8>,
    source_address: u8,
}

impl J1939Id {
    /// Largest valid 29-bit identifier.
    pub const MAX_RAW: u32 = 0x1FFF_FFFF;
    /// Largest valid 18-bit PGN.
    pub const MAX_PGN: u32 = 0x3_FFFF;

    /// Builds an identifier.
    ///
    /// * `pgn` is the 18-bit PGN including EDP/DP. For PDU1 PGNs its low byte must be 0.
    /// * `destination` is required for PDU1 PGNs and ignored for PDU2.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidId`] for priority > 7 or a PDU1 PGN without a destination;
    /// [`Error::InvalidPgn`] for a PGN above 18 bits or a PDU1 PGN with a non-zero low byte.
    pub fn new(
        priority: u8,
        pgn: u32,
        destination: Option<u8>,
        source_address: u8,
    ) -> Result<Self> {
        if priority > 7 {
            return Err(Error::InvalidId);
        }
        if pgn > Self::MAX_PGN {
            return Err(Error::InvalidPgn);
        }
        let destination = if Self::pgn_is_pdu1(pgn) {
            if pgn & 0xFF != 0 {
                return Err(Error::InvalidPgn);
            }
            Some(destination.ok_or(Error::InvalidId)?)
        } else {
            None
        };
        Ok(Self {
            priority,
            pgn,
            destination,
            source_address,
        })
    }

    /// Decodes a raw 29-bit identifier. Every 29-bit value is a valid J1939 identifier.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidId`] if `raw` has bits above bit 28 set.
    pub fn from_raw(raw: u32) -> Result<Self> {
        if raw > Self::MAX_RAW {
            return Err(Error::InvalidId);
        }
        let priority = (raw >> 26) as u8;
        let page = (raw >> 16) & 0x3FF; // EDP, DP and PF (10 bits)
        let ps = ((raw >> 8) & 0xFF) as u8;
        let (pgn, destination) = if Self::pgn_is_pdu1(page << 8) {
            (page << 8, Some(ps))
        } else {
            ((page << 8) | ps as u32, None)
        };
        Ok(Self {
            priority,
            pgn,
            destination,
            source_address: raw as u8,
        })
    }

    /// Encodes to the raw 29-bit identifier.
    pub fn to_raw(self) -> u32 {
        let ps = match self.destination {
            // PDU1: PS is the destination address (the PGN's low byte is zero)
            Some(da) => da as u32,
            None => self.pgn & 0xFF,
        };
        ((self.priority as u32) << 26)
            | ((self.pgn >> 8) << 16)
            | (ps << 8)
            | self.source_address as u32
    }

    /// Priority, 0 (highest) to 7.
    pub fn priority(self) -> u8 {
        self.priority
    }

    /// The 18-bit PGN including EDP and DP (PS is zero for PDU1).
    pub fn pgn(self) -> u32 {
        self.pgn
    }

    /// Destination address for PDU1 (destination-specific) PGNs, `None` for PDU2 (broadcast).
    pub fn destination(self) -> Option<u8> {
        self.destination
    }

    /// Source address.
    pub fn source_address(self) -> u8 {
        self.source_address
    }

    /// `true` for PDU1 (PF < 240, destination-specific).
    pub fn is_pdu1(self) -> bool {
        self.destination.is_some()
    }

    fn pgn_is_pdu1(pgn: u32) -> bool {
        ((pgn >> 8) & 0xFF) < 240
    }
}

#[cfg(feature = "embedded-can")]
impl From<J1939Id> for embedded_can::ExtendedId {
    fn from(id: J1939Id) -> Self {
        // to_raw() is at most 29 bits by construction
        embedded_can::ExtendedId::new(id.to_raw()).unwrap_or(embedded_can::ExtendedId::MAX)
    }
}

#[cfg(feature = "embedded-can")]
impl From<J1939Id> for embedded_can::Id {
    fn from(id: J1939Id) -> Self {
        embedded_can::Id::Extended(id.into())
    }
}

#[cfg(feature = "embedded-can")]
impl From<embedded_can::ExtendedId> for J1939Id {
    fn from(id: embedded_can::ExtendedId) -> Self {
        // every 29-bit value is a valid J1939 identifier
        Self::from_raw(id.as_raw()).unwrap_or(Self {
            priority: 7,
            pgn: 0x3_FFFF,
            destination: None,
            source_address: 0xFF,
        })
    }
}

/// J1939 uses 29-bit identifiers only; an 11-bit standard id is rejected with [`Error::InvalidId`].
#[cfg(feature = "embedded-can")]
impl TryFrom<embedded_can::Id> for J1939Id {
    type Error = Error;

    fn try_from(id: embedded_can::Id) -> Result<Self> {
        match id {
            embedded_can::Id::Extended(e) => Ok(e.into()),
            embedded_can::Id::Standard(_) => Err(Error::InvalidId),
        }
    }
}
