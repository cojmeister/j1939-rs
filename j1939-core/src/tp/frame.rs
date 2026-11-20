use crate::{Error, J1939Id, Result};

pub const PGN_TP_CM: u32 = 0xEC00;
pub const PGN_TP_DT: u32 = 0xEB00;
/// Largest message the transport protocol can carry (255 packets of 7 bytes).
pub const MAX_TP_BYTES: usize = 1785;

/// A raw CAN data frame with a decoded J1939 identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanFrame {
    pub id: J1939Id,
    pub len: u8,
    pub data: [u8; 8],
}

impl CanFrame {
    /// Build a frame from up to 8 data bytes.
    pub fn new(id: J1939Id, data: &[u8]) -> Result<Self> {
        if data.len() > 8 {
            return Err(Error::InvalidLength);
        }
        let mut buf = [0u8; 8];
        buf[..data.len()].copy_from_slice(data);
        Ok(Self {
            id,
            len: data.len() as u8,
            data: buf,
        })
    }

    pub fn data(&self) -> &[u8] {
        &self.data[..self.len as usize]
    }
}

/// Connection abort reason codes (J1939-21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortReason {
    AlreadyInSession,
    Resources,
    Timeout,
    CtsWhileTransferring,
    MaxRetransmit,
    UnexpectedDt,
    BadSequence,
    DuplicateSequence,
    SizeExceeded,
    Other(u8),
}

impl AbortReason {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::AlreadyInSession,
            2 => Self::Resources,
            3 => Self::Timeout,
            4 => Self::CtsWhileTransferring,
            5 => Self::MaxRetransmit,
            6 => Self::UnexpectedDt,
            7 => Self::BadSequence,
            8 => Self::DuplicateSequence,
            9 => Self::SizeExceeded,
            o => Self::Other(o),
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            Self::AlreadyInSession => 1,
            Self::Resources => 2,
            Self::Timeout => 3,
            Self::CtsWhileTransferring => 4,
            Self::MaxRetransmit => 5,
            Self::UnexpectedDt => 6,
            Self::BadSequence => 7,
            Self::DuplicateSequence => 8,
            Self::SizeExceeded => 9,
            Self::Other(o) => o,
        }
    }
}

/// Transport protocol connection management message (PGN 0xEC00 payload).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TpCm {
    Rts {
        size: u16,
        packets: u8,
        max_per_cts: u8,
        pgn: u32,
    },
    Cts {
        packets: u8,
        next: u8,
        pgn: u32,
    },
    EndOfMsgAck {
        size: u16,
        packets: u8,
        pgn: u32,
    },
    Bam {
        size: u16,
        packets: u8,
        pgn: u32,
    },
    Abort {
        reason: AbortReason,
        pgn: u32,
    },
}

const CB_RTS: u8 = 0x10;
const CB_CTS: u8 = 0x11;
const CB_EOMA: u8 = 0x13;
const CB_BAM: u8 = 0x20;
const CB_ABORT: u8 = 0xFF;

impl TpCm {
    pub fn encode(&self) -> [u8; 8] {
        let (cb, b1, b2, b3, b4, pgn) = match *self {
            Self::Rts {
                size,
                packets,
                max_per_cts,
                pgn,
            } => (
                CB_RTS,
                size as u8,
                (size >> 8) as u8,
                packets,
                max_per_cts,
                pgn,
            ),
            Self::Cts { packets, next, pgn } => (CB_CTS, packets, next, 0xFF, 0xFF, pgn),
            Self::EndOfMsgAck { size, packets, pgn } => {
                (CB_EOMA, size as u8, (size >> 8) as u8, packets, 0xFF, pgn)
            }
            Self::Bam { size, packets, pgn } => {
                (CB_BAM, size as u8, (size >> 8) as u8, packets, 0xFF, pgn)
            }
            Self::Abort { reason, pgn } => (CB_ABORT, reason.to_u8(), 0xFF, 0xFF, 0xFF, pgn),
        };
        [
            cb,
            b1,
            b2,
            b3,
            b4,
            pgn as u8,
            (pgn >> 8) as u8,
            (pgn >> 16) as u8,
        ]
    }

    /// Decode an 8-byte CM frame; anything else is `Error::InvalidTp`.
    pub fn decode(data: &[u8]) -> Result<Self> {
        if data.len() != 8 {
            return Err(Error::InvalidTp);
        }
        let pgn = u32::from(data[5]) | u32::from(data[6]) << 8 | u32::from(data[7]) << 16;
        let size = u16::from(data[1]) | u16::from(data[2]) << 8;
        match data[0] {
            CB_RTS => Ok(Self::Rts {
                size,
                packets: data[3],
                max_per_cts: data[4],
                pgn,
            }),
            CB_CTS => Ok(Self::Cts {
                packets: data[1],
                next: data[2],
                pgn,
            }),
            CB_EOMA => Ok(Self::EndOfMsgAck {
                size,
                packets: data[3],
                pgn,
            }),
            CB_BAM => Ok(Self::Bam {
                size,
                packets: data[3],
                pgn,
            }),
            CB_ABORT => Ok(Self::Abort {
                reason: AbortReason::from_u8(data[1]),
                pgn,
            }),
            _ => Err(Error::InvalidTp),
        }
    }
}

/// Build a TP.DT data frame: sequence number then up to 7 bytes, padded with 0xFF.
pub fn dt_packet(seq: u8, payload: &[u8]) -> [u8; 8] {
    debug_assert!(payload.len() <= 7);
    let mut out = [0xFF; 8];
    out[0] = seq;
    let n = payload.len().min(7);
    out[1..1 + n].copy_from_slice(&payload[..n]);
    out
}

/// Number of DT packets needed for `size` bytes.
pub fn packets_for(size: usize) -> u8 {
    size.div_ceil(7) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    const PGN: u32 = 0x00FECA;

    fn check(cm: TpCm, bytes: [u8; 8]) {
        assert_eq!(cm.encode(), bytes);
        assert_eq!(TpCm::decode(&bytes), Ok(cm));
    }

    #[test]
    fn rts() {
        check(
            TpCm::Rts {
                size: 20,
                packets: 3,
                max_per_cts: 0xFF,
                pgn: PGN,
            },
            [0x10, 0x14, 0x00, 0x03, 0xFF, 0xCA, 0xFE, 0x00],
        );
    }

    #[test]
    fn bam() {
        check(
            TpCm::Bam {
                size: 20,
                packets: 3,
                pgn: PGN,
            },
            [0x20, 0x14, 0x00, 0x03, 0xFF, 0xCA, 0xFE, 0x00],
        );
    }

    #[test]
    fn cts() {
        check(
            TpCm::Cts {
                packets: 3,
                next: 1,
                pgn: PGN,
            },
            [0x11, 0x03, 0x01, 0xFF, 0xFF, 0xCA, 0xFE, 0x00],
        );
    }

    #[test]
    fn end_of_msg_ack() {
        check(
            TpCm::EndOfMsgAck {
                size: 20,
                packets: 3,
                pgn: PGN,
            },
            [0x13, 0x14, 0x00, 0x03, 0xFF, 0xCA, 0xFE, 0x00],
        );
    }

    #[test]
    fn abort() {
        check(
            TpCm::Abort {
                reason: AbortReason::Timeout,
                pgn: PGN,
            },
            [0xFF, 0x03, 0xFF, 0xFF, 0xFF, 0xCA, 0xFE, 0x00],
        );
    }

    #[test]
    fn abort_codes() {
        for c in 1..=9u8 {
            assert_eq!(AbortReason::from_u8(c).to_u8(), c);
            assert!(!matches!(AbortReason::from_u8(c), AbortReason::Other(_)));
        }
        assert_eq!(AbortReason::from_u8(0x0A), AbortReason::Other(0x0A));
        assert_eq!(AbortReason::Other(0x0A).to_u8(), 0x0A);
        let b = [0xFF, 0x0A, 0xFF, 0xFF, 0xFF, 0xCA, 0xFE, 0x00];
        assert_eq!(
            TpCm::decode(&b),
            Ok(TpCm::Abort {
                reason: AbortReason::Other(0x0A),
                pgn: PGN
            })
        );
    }

    #[test]
    fn bad_decode() {
        assert_eq!(
            TpCm::decode(&[0x55, 0, 0, 0, 0, 0, 0, 0]),
            Err(Error::InvalidTp)
        );
        assert_eq!(
            TpCm::decode(&[0x10, 0x14, 0, 3, 0xFF, 0xCA, 0xFE]),
            Err(Error::InvalidTp)
        );
    }

    #[test]
    fn dt_and_packets() {
        assert_eq!(
            dt_packet(3, &[1, 2, 3]),
            [3, 1, 2, 3, 0xFF, 0xFF, 0xFF, 0xFF]
        );
        assert_eq!(packets_for(20), 3);
        assert_eq!(packets_for(1785), 255);
    }

    #[test]
    fn can_frame() {
        let id = crate::J1939Id::new(6, 0xEC00, Some(0x80), 0x25).unwrap();
        let f = CanFrame::new(id, &[1, 2, 3]).unwrap();
        assert_eq!(f.data(), &[1, 2, 3]);
        assert_eq!(f.len, 3);
        assert_eq!(CanFrame::new(id, &[0; 9]), Err(Error::InvalidLength));
    }
}
