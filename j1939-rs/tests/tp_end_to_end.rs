//! Macro-generated messages sent through the transport protocol and reassembled.

use j1939_rs::prelude::*;
use j1939_rs::tp::{TpCm, TpRx, TpTx, TxState};

#[j1939_message(pgn = 0x5000, length = 47)]
struct Long47 {
    #[j1939(bits = 0..8)]
    first: u8,
    #[j1939(bits = 8..368, reserved)]
    _mid: (),
    #[j1939(bits = 368..376)]
    last: u8,
}

const SENDER: u8 = 0x80;
const RECEIVER: u8 = 0x20;

fn sample() -> Long47 {
    Long47 {
        first: 0x11,
        _mid: (),
        last: 0xA5,
    }
}

fn marshalled(destination: u8) -> J1939Message {
    let mut msg = J1939Message::default();
    sample().marshall(&mut msg).unwrap();
    // Distinct middle bytes so swapped or duplicated packets are detected.
    for i in 1..46 {
        msg.data[i] = i as u8;
    }
    msg.source_address = SENDER;
    msg.destination_address = destination;
    msg
}

fn assert_reassembled(sent: &J1939Message, got: &J1939Message) {
    assert_eq!(got.pgn, sent.pgn);
    assert_eq!(got.length, 47);
    assert_eq!(got.source_address, SENDER);
    assert_eq!(got.data[..47], sent.data[..47], "whole payload");
    let back = Long47::unmarshall(got).unwrap();
    assert_eq!(back.first, 0x11);
    assert_eq!(back.last, 0xA5);
}

#[test]
fn bam_delivers_marshalled_message_to_receiver() {
    let sent = marshalled(0xFF);
    let mut tx = TpTx::bam(&sent).unwrap();
    let mut rx = TpRx::<1>::new(RECEIVER);
    let mut now = 0;
    let mut delivered = None;
    while tx.state() == TxState::Active {
        if let Some(frame) = tx.poll(now) {
            let r = rx.on_frame(&frame, now);
            assert!(r.reply.is_none());
            if r.message.is_some() {
                delivered = r.message;
            }
        }
        now += 10;
        assert!(now < 10_000, "BAM did not finish");
    }
    assert_eq!(tx.state(), TxState::Done);
    assert_reassembled(&sent, &delivered.expect("message reassembled"));
}

#[test]
fn rts_cts_delivers_marshalled_message_and_acks() {
    let sent = marshalled(RECEIVER);
    let mut tx = TpTx::rts(&sent).unwrap().with_max_per_cts(3);
    let mut rx = TpRx::<1>::new(RECEIVER);
    let mut now = 0;
    let mut delivered = None;
    let mut acked = false;
    while tx.state() == TxState::Active {
        if let Some(frame) = tx.poll(now) {
            let r = rx.on_frame(&frame, now);
            if r.message.is_some() {
                delivered = r.message;
            }
            if let Some(reply) = r.reply {
                if matches!(TpCm::decode(reply.data()), Ok(TpCm::EndOfMsgAck { .. })) {
                    acked = true;
                }
                tx.on_frame(&reply, now).unwrap();
            }
        }
        now += 10;
        assert!(now < 10_000, "RTS did not finish");
    }
    assert_eq!(tx.state(), TxState::Done);
    assert!(acked, "receiver sent End of Message ack");
    assert_reassembled(&sent, &delivered.expect("message reassembled"));
}

#[j1939_message(pgn = 0xFECA, length = 20)]
struct Pdu2Msg {
    #[j1939(bits = 0..8)]
    a: u8,
    #[j1939(bits = 8..152, reserved)]
    _mid: (),
    #[j1939(bits = 152..160)]
    z: u8,
}

#[test]
fn bam_delivers_pdu2_pgn_message() {
    let mut sent = J1939Message::default();
    Pdu2Msg {
        a: 0x42,
        _mid: (),
        z: 0x99,
    }
    .marshall(&mut sent)
    .unwrap();
    sent.source_address = SENDER;
    sent.destination_address = 0xFF;
    let mut tx = TpTx::bam(&sent).unwrap();
    let mut rx = TpRx::<1>::new(RECEIVER);
    let mut now = 0;
    let mut delivered = None;
    while tx.state() == TxState::Active {
        if let Some(frame) = tx.poll(now) {
            delivered = delivered.or(rx.on_frame(&frame, now).message);
        }
        now += 10;
        assert!(now < 10_000, "BAM did not finish");
    }
    let got = delivered.expect("message reassembled");
    assert_eq!(got.pgn, 0xFECA);
    assert_eq!(got.data[..20], sent.data[..20]);
    let back = Pdu2Msg::unmarshall(&got).unwrap();
    assert_eq!((back.a, back.z), (0x42, 0x99));
}
