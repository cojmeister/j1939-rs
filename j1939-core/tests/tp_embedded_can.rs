#![cfg(feature = "embedded-can")]
//! `CanFrame` <-> `embedded_can` frame conversions, using a minimal mock frame type.
use embedded_can::{Frame, Id, StandardId};
use j1939_core::tp::{CanFrame, TpCm};
use j1939_core::{Error, J1939Id};

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

fn rts_frame() -> CanFrame {
    let id = J1939Id::new(7, 0xEC00, Some(0x80), 0x20).unwrap();
    let cm = TpCm::Rts {
        size: 47,
        packets: 7,
        max_per_cts: 0xFF,
        pgn: 0x5001,
    };
    CanFrame::new(id, &cm.encode()).unwrap()
}

#[test]
fn rts_can_frame_round_trips_through_mock_frame() {
    let f = rts_frame();
    let mock: MockFrame = f.to_frame().unwrap();
    assert!(mock.is_extended());
    assert_eq!(mock.dlc(), 8);
    assert_eq!(CanFrame::from_frame(&mock).unwrap(), f);
}

#[test]
fn short_can_frame_keeps_its_length() {
    let id = J1939Id::new(6, 0xFECA, None, 0x10).unwrap();
    let f = CanFrame::new(id, &[1, 2, 3]).unwrap();
    let mock: MockFrame = f.to_frame().unwrap();
    assert_eq!(mock.data(), &[1, 2, 3]);
    assert_eq!(CanFrame::from_frame(&mock).unwrap(), f);
}

#[test]
fn standard_id_frame_is_rejected() {
    let mock = MockFrame::new(StandardId::new(0x123).unwrap(), &[0; 8]).unwrap();
    assert_eq!(CanFrame::from_frame(&mock), Err(Error::InvalidId));
}

#[test]
fn remote_frame_is_rejected() {
    let id = J1939Id::new(6, 0xFECA, None, 0x10).unwrap();
    let ext: embedded_can::ExtendedId = id.into();
    let mock = MockFrame::new_remote(Id::Extended(ext), 8).unwrap();
    assert_eq!(CanFrame::from_frame(&mock), Err(Error::InvalidId));
}

#[test]
fn constructor_failure_maps_to_invalid_length() {
    #[derive(Debug)]
    struct Refuses;
    impl Frame for Refuses {
        fn new(_: impl Into<Id>, _: &[u8]) -> Option<Self> {
            None
        }
        fn new_remote(_: impl Into<Id>, _: usize) -> Option<Self> {
            None
        }
        fn is_extended(&self) -> bool {
            true
        }
        fn is_remote_frame(&self) -> bool {
            false
        }
        fn id(&self) -> Id {
            Id::Standard(StandardId::ZERO)
        }
        fn dlc(&self) -> usize {
            0
        }
        fn data(&self) -> &[u8] {
            &[]
        }
    }
    assert_eq!(
        rts_frame().to_frame::<Refuses>().err(),
        Some(Error::InvalidLength)
    );
}
