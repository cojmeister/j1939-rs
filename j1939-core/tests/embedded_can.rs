#![cfg(feature = "embedded-can")]
//! Conversions to/from `embedded_can` frames, using a minimal mock frame type.
use embedded_can::{ExtendedId, Frame, Id, StandardId};
use j1939_core::{Error, J1939Id, J1939Message};

#[derive(Debug, Clone, PartialEq)]
struct MockFrame {
    id: Id,
    remote: bool,
    dlc: usize,
    data: [u8; 8],
}

impl Frame for MockFrame {
    fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
        if data.len() > 8 {
            return None;
        }
        let mut d = [0; 8];
        d[..data.len()].copy_from_slice(data);
        Some(Self {
            id: id.into(),
            remote: false,
            dlc: data.len(),
            data: d,
        })
    }
    fn new_remote(id: impl Into<Id>, dlc: usize) -> Option<Self> {
        Some(Self {
            id: id.into(),
            remote: true,
            dlc,
            data: [0; 8],
        })
    }
    fn is_extended(&self) -> bool {
        matches!(self.id, Id::Extended(_))
    }
    fn is_remote_frame(&self) -> bool {
        self.remote
    }
    fn id(&self) -> Id {
        self.id
    }
    fn dlc(&self) -> usize {
        self.dlc
    }
    fn data(&self) -> &[u8] {
        &self.data[..self.dlc]
    }
}

#[test]
fn j1939_id_converts_to_and_from_extended_id() {
    let id = J1939Id::new(3, 0x1F004, None, 0x10).unwrap();
    let ext: ExtendedId = id.into();
    assert_eq!(ext.as_raw(), 0x0DF0_0410);
    assert_eq!(J1939Id::try_from(Id::Extended(ext)).unwrap(), id);
}

#[test]
fn standard_ids_are_rejected() {
    let std_id = Id::Standard(StandardId::new(0x123).unwrap());
    assert_eq!(J1939Id::try_from(std_id), Err(Error::InvalidId));
}

#[test]
fn message_roundtrips_through_a_frame_keeping_dp_and_edp() {
    for pgn in [0xF004u32, 0x1F004, 0x2F004, 0x3F004] {
        let mut msg = J1939Message {
            priority: 3,
            pgn,
            source_address: 0x10,
            length: 8,
            ..Default::default()
        };
        msg.data[..8].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let frame: MockFrame = msg.to_frame().unwrap();
        assert!(frame.is_extended());
        assert_eq!(frame.data(), &[1, 2, 3, 4, 5, 6, 7, 8]);
        let back = J1939Message::from_frame(&frame).unwrap();
        assert_eq!(back.pgn, pgn);
        assert_eq!(back.priority, 3);
        assert_eq!(back.source_address, 0x10);
        assert_eq!(back.length, 8);
        assert_eq!(&back.data[..8], &msg.data[..8]);
    }
}

#[test]
fn pdu1_destination_survives_frame_roundtrip() {
    let msg = J1939Message {
        priority: 6,
        pgn: 0xEA00,
        destination_address: 0x80,
        source_address: 0xF9,
        length: 3,
        ..Default::default()
    };
    let frame: MockFrame = msg.to_frame().unwrap();
    assert_eq!(
        frame.id(),
        Id::Extended(ExtendedId::new(0x18EA_80F9).unwrap())
    );
    let back = J1939Message::from_frame(&frame).unwrap();
    assert_eq!(back.destination_address, 0x80);
    assert_eq!(back.pgn, 0xEA00);
}

#[test]
fn from_frame_rejects_standard_and_remote_frames() {
    let std_frame = MockFrame::new(StandardId::new(1).unwrap(), &[1]).unwrap();
    assert_eq!(J1939Message::from_frame(&std_frame), Err(Error::InvalidId));
    let remote = MockFrame::new_remote(ExtendedId::new(0x18EA_80F9).unwrap(), 0).unwrap();
    assert_eq!(J1939Message::from_frame(&remote), Err(Error::InvalidId));
}

#[test]
fn to_frame_rejects_multipacket_payloads() {
    let msg = J1939Message {
        pgn: 0xFECA,
        length: 9,
        ..Default::default()
    };
    assert_eq!(msg.to_frame::<MockFrame>(), Err(Error::InvalidLength));
}
