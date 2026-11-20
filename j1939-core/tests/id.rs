//! 29-bit identifier handling. The DP and EDP bits are part of the PGN (18 bits) and must survive.
use j1939_core::{Error, J1939Id, J1939Message};

#[test]
fn eec1_pdu2_known_identifier() {
    // EEC1: priority 3, PGN 0xF004, SA 0 -> 0x0CF00400 (the classic "0CF00400")
    let id = J1939Id::new(3, 0xF004, None, 0x00).unwrap();
    assert_eq!(id.to_raw(), 0x0CF0_0400);
    assert_eq!(J1939Id::from_raw(0x0CF0_0400).unwrap(), id);
}

#[test]
fn pdu1_destination_lives_in_ps_field_not_in_pgn() {
    // Request (PGN 0xEA00), priority 6, DA 0x80, SA 0xF9 -> 0x18EA80F9
    let id = J1939Id::new(6, 0xEA00, Some(0x80), 0xF9).unwrap();
    assert_eq!(id.to_raw(), 0x18EA_80F9);
    let back = J1939Id::from_raw(0x18EA_80F9).unwrap();
    assert_eq!(back.pgn(), 0xEA00);
    assert_eq!(back.destination(), Some(0x80));
    assert_eq!(back.source_address(), 0xF9);
    assert!(back.is_pdu1());
}

#[test]
fn data_page_bit_is_preserved() {
    let id = J1939Id::new(3, 0x1F004, None, 0x10).unwrap();
    assert_eq!(id.to_raw(), 0x0DF0_0410);
    let back = J1939Id::from_raw(0x0DF0_0410).unwrap();
    assert_eq!(back.pgn(), 0x1F004);
    assert_ne!(back.pgn(), 0xF004);
}

#[test]
fn extended_data_page_bit_is_preserved() {
    let id = J1939Id::new(3, 0x2F004, None, 0x10).unwrap();
    assert_eq!(id.to_raw(), 0x0EF0_0410);
    assert_eq!(J1939Id::from_raw(0x0EF0_0410).unwrap().pgn(), 0x2F004);
}

#[test]
fn every_raw_id_roundtrips_for_sampled_values() {
    for raw in (0u32..=0x1FFF_FFFF).step_by(0x0001_0F3B) {
        let id = J1939Id::from_raw(raw).unwrap();
        assert_eq!(id.to_raw(), raw, "raw {raw:#x}");
    }
}

#[test]
fn invalid_inputs_are_rejected() {
    assert_eq!(J1939Id::from_raw(0x2000_0000), Err(Error::InvalidId));
    assert_eq!(J1939Id::new(8, 0xF004, None, 0), Err(Error::InvalidId));
    assert_eq!(J1939Id::new(3, 0x4_0000, None, 0), Err(Error::InvalidPgn));
    // PDU1 PGN must have a zero PS byte and needs a destination
    assert_eq!(
        J1939Id::new(6, 0xEA80, Some(0x80), 0),
        Err(Error::InvalidPgn)
    );
    assert_eq!(J1939Id::new(6, 0xEA00, None, 0), Err(Error::InvalidId));
}

#[test]
fn message_id_roundtrip_via_fields() {
    let msg = J1939Message {
        priority: 3,
        pgn: 0x1F004,
        source_address: 0x10,
        ..Default::default()
    };
    assert_eq!(msg.id().unwrap().to_raw(), 0x0DF0_0410);

    let id = J1939Id::from_raw(0x18EA_80F9).unwrap();
    let msg = J1939Message::from_id_and_data(id, &[1, 2, 3]).unwrap();
    assert_eq!(msg.pgn, 0xEA00);
    assert_eq!(msg.destination_address, 0x80);
    assert_eq!(msg.source_address, 0xF9);
    assert_eq!(msg.priority, 6);
    assert_eq!(msg.length, 3);
    assert_eq!(&msg.data[..3], &[1, 2, 3]);
    assert_eq!(msg.id().unwrap(), id);
}

#[test]
fn from_id_and_data_rejects_oversize_payload() {
    let id = J1939Id::from_raw(0x18EA_80F9).unwrap();
    let big = [0u8; J1939Message::MAX_LENGTH + 1];
    assert_eq!(
        J1939Message::from_id_and_data(id, &big),
        Err(Error::InvalidLength)
    );
}
