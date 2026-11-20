use j1939_core::tp::timing::{T1_MS, T2_MS, T3_MS, T4_MS};
use j1939_core::tp::{AbortReason, CanFrame, TpCm, TpRx, TpTx, TxState, dt_packet};
use j1939_core::{Error, J1939Id, J1939Message};

const TX_SA: u8 = 0x80;
const RX_SA: u8 = 0x00;
const PGN: u32 = 0xFECA;

fn message(len: u8, dest: u8) -> J1939Message {
    let data: Vec<u8> = (1..=len).collect();
    let id = J1939Id::new(7, PGN, Some(dest), TX_SA).unwrap();
    // PDU2 PGNs have no destination; carry it in the struct as the stack does.
    let mut msg = J1939Message::from_id_and_data(id, &data).unwrap();
    msg.destination_address = dest;
    msg
}

fn cm(src: u8, dst: u8, cm: TpCm) -> CanFrame {
    let id = J1939Id::new(7, 0xEC00, Some(dst), src).unwrap();
    CanFrame::new(id, &cm.encode()).unwrap()
}

fn dt(src: u8, dst: u8, seq: u8) -> CanFrame {
    let id = J1939Id::new(7, 0xEB00, Some(dst), src).unwrap();
    CanFrame::new(id, &dt_packet(seq, &[seq; 7])).unwrap()
}

fn rts(src: u8, dst: u8, size: u16, max: u8) -> CanFrame {
    let packets = size.div_ceil(7) as u8;
    cm(
        src,
        dst,
        TpCm::Rts {
            size,
            packets,
            max_per_cts: max,
            pgn: PGN,
        },
    )
}

fn decode(f: &CanFrame) -> TpCm {
    TpCm::decode(f.data()).unwrap()
}

fn abort_reason(f: &CanFrame) -> AbortReason {
    match decode(f) {
        TpCm::Abort { reason, pgn } => {
            assert_eq!(pgn, PGN);
            reason
        }
        other => panic!("expected abort, got {other:?}"),
    }
}

/// Run a full transfer between `tx` and `rx`; returns (delivered message, CTS count).
fn transfer(tx: &mut TpTx, rx: &mut TpRx<1>) -> (Option<J1939Message>, usize) {
    let mut now = 1000u32;
    let mut cts = 0;
    let mut got = None;
    while tx.state() == TxState::Active {
        while let Some(f) = tx.poll(now) {
            let r = rx.on_frame(&f, now);
            got = r.message.or(got);
            if let Some(reply) = r.reply {
                if matches!(decode(&reply), TpCm::Cts { .. }) {
                    cts += 1;
                }
                tx.on_frame(&reply, now).unwrap();
            }
        }
        now += 10;
        assert!(now < 100_000, "transfer never finished");
    }
    (got, cts)
}

#[test]
fn transfer_20_bytes() {
    let msg = message(20, RX_SA);
    let mut tx = TpTx::rts(&msg).unwrap();
    let mut rx = TpRx::<1>::new(RX_SA);
    let (got, cts) = transfer(&mut tx, &mut rx);
    assert_eq!(tx.state(), TxState::Done);
    assert_eq!(cts, 1);
    assert_eq!(got, Some(msg));
}

#[test]
fn transfer_with_one_packet_per_cts() {
    let msg = message(20, RX_SA);
    let mut tx = TpTx::rts(&msg).unwrap().with_max_per_cts(1);
    let mut rx = TpRx::<1>::new(RX_SA);
    let (got, cts) = transfer(&mut tx, &mut rx);
    assert_eq!(tx.state(), TxState::Done);
    assert_eq!(cts, 3);
    assert_eq!(got, Some(msg));
}

#[test]
fn transfer_223_bytes() {
    let msg = message(223, RX_SA);
    let mut tx = TpTx::rts(&msg).unwrap();
    let mut rx = TpRx::<1>::new(RX_SA);
    let (got, _) = transfer(&mut tx, &mut rx);
    assert_eq!(tx.state(), TxState::Done);
    assert_eq!(got, Some(msg));
}

#[test]
fn tx_frames_on_the_wire() {
    let mut tx = TpTx::rts(&message(20, RX_SA)).unwrap().with_max_per_cts(2);
    let f = tx.poll(0).unwrap();
    assert_eq!(f.id.pgn(), 0xEC00);
    assert_eq!(f.id.destination(), Some(RX_SA));
    assert_eq!(f.id.source_address(), TX_SA);
    assert_eq!(f.data(), &[0x10, 20, 0, 3, 2, 0xCA, 0xFE, 0]);
    // No DT before a CTS.
    assert_eq!(tx.poll(10), None);
    let c = cm(
        RX_SA,
        TX_SA,
        TpCm::Cts {
            packets: 2,
            next: 1,
            pgn: PGN,
        },
    );
    tx.on_frame(&c, 20).unwrap();
    let d1 = tx.poll(20).unwrap();
    assert_eq!(d1.id.pgn(), 0xEB00);
    assert_eq!(d1.id.destination(), Some(RX_SA));
    assert_eq!(d1.data(), &[1, 1, 2, 3, 4, 5, 6, 7]);
    let d2 = tx.poll(20).unwrap();
    assert_eq!(d2.data(), &[2, 8, 9, 10, 11, 12, 13, 14]);
    // Only the requested block is sent.
    assert_eq!(tx.poll(30), None);
}

#[test]
fn tx_rejects_bad_messages() {
    assert_eq!(TpTx::rts(&message(20, 0xFF)).unwrap_err(), Error::InvalidId);
    assert_eq!(
        TpTx::rts(&message(8, RX_SA)).unwrap_err(),
        Error::InvalidLength
    );
}

#[test]
fn tx_end_of_msg_ack_completes() {
    let mut tx = TpTx::rts(&message(9, RX_SA)).unwrap();
    tx.poll(0).unwrap();
    let c = cm(
        RX_SA,
        TX_SA,
        TpCm::Cts {
            packets: 2,
            next: 1,
            pgn: PGN,
        },
    );
    tx.on_frame(&c, 0).unwrap();
    tx.poll(0).unwrap();
    tx.poll(0).unwrap();
    assert_eq!(tx.state(), TxState::Active);
    let ack = cm(
        RX_SA,
        TX_SA,
        TpCm::EndOfMsgAck {
            size: 9,
            packets: 2,
            pgn: PGN,
        },
    );
    tx.on_frame(&ack, 5).unwrap();
    assert_eq!(tx.state(), TxState::Done);
}

#[test]
fn tx_peer_abort() {
    let mut tx = TpTx::rts(&message(20, RX_SA)).unwrap();
    tx.poll(0).unwrap();
    let a = cm(
        RX_SA,
        TX_SA,
        TpCm::Abort {
            reason: AbortReason::Resources,
            pgn: PGN,
        },
    );
    tx.on_frame(&a, 1).unwrap();
    assert_eq!(tx.state(), TxState::Aborted(AbortReason::Resources));
    assert_eq!(tx.poll(2), None);
}

#[test]
fn tx_ignores_frames_from_other_nodes() {
    let mut tx = TpTx::rts(&message(20, RX_SA)).unwrap();
    tx.poll(0).unwrap();
    let a = cm(
        0x33,
        TX_SA,
        TpCm::Abort {
            reason: AbortReason::Resources,
            pgn: PGN,
        },
    );
    tx.on_frame(&a, 1).unwrap();
    assert_eq!(tx.state(), TxState::Active);
}

#[test]
fn tx_resends_from_cts_next() {
    let mut tx = TpTx::rts(&message(20, RX_SA)).unwrap();
    tx.poll(0).unwrap();
    let cts = |packets, next| {
        cm(
            RX_SA,
            TX_SA,
            TpCm::Cts {
                packets,
                next,
                pgn: PGN,
            },
        )
    };
    tx.on_frame(&cts(3, 1), 1).unwrap();
    assert_eq!(tx.poll(1).unwrap().data()[0], 1);
    assert_eq!(tx.poll(1).unwrap().data()[0], 2);
    assert_eq!(tx.poll(1).unwrap().data()[0], 3);
    assert_eq!(tx.poll(2), None);
    // Receiver lost packets 2 and 3.
    tx.on_frame(&cts(2, 2), 3).unwrap();
    assert_eq!(tx.poll(3).unwrap().data()[0], 2);
    assert_eq!(tx.poll(3).unwrap().data()[0], 3);
    assert_eq!(tx.poll(3), None);
    assert_eq!(tx.state(), TxState::Active);
}

#[test]
fn tx_cts_hold_and_t4() {
    let mut tx = TpTx::rts(&message(20, RX_SA)).unwrap();
    tx.poll(0).unwrap();
    let cts = |packets, next| {
        cm(
            RX_SA,
            TX_SA,
            TpCm::Cts {
                packets,
                next,
                pgn: PGN,
            },
        )
    };
    // Hold: no data while packets == 0.
    tx.on_frame(&cts(0, 0xFF), 100).unwrap();
    assert_eq!(tx.poll(100), None);
    assert_eq!(tx.poll(100 + T4_MS - 1), None);
    assert_eq!(tx.state(), TxState::Active);
    // A fresh CTS releases the hold and restarts the timer.
    tx.on_frame(&cts(1, 1), 100 + T4_MS - 1).unwrap();
    assert_eq!(tx.poll(100 + T4_MS - 1).unwrap().data()[0], 1);

    // Hold that is never released times out after T4.
    let mut tx = TpTx::rts(&message(20, RX_SA)).unwrap();
    tx.poll(0).unwrap();
    tx.on_frame(&cts(0, 0xFF), 100).unwrap();
    let f = tx.poll(100 + T4_MS).unwrap();
    assert_eq!(abort_reason(&f), AbortReason::Timeout);
    assert_eq!(tx.state(), TxState::Aborted(AbortReason::Timeout));
}

#[test]
fn tx_t3_without_cts() {
    let mut tx = TpTx::rts(&message(20, RX_SA)).unwrap();
    tx.poll(5000).unwrap();
    assert_eq!(tx.poll(5000 + T3_MS - 1), None);
    assert_eq!(tx.state(), TxState::Active);
    let f = tx.poll(5000 + T3_MS).unwrap();
    assert_eq!(f.id.pgn(), 0xEC00);
    assert_eq!(f.id.destination(), Some(RX_SA));
    assert_eq!(f.id.source_address(), TX_SA);
    assert_eq!(abort_reason(&f), AbortReason::Timeout);
    assert_eq!(tx.state(), TxState::Aborted(AbortReason::Timeout));
    assert_eq!(tx.poll(100_000), None);
}

#[test]
fn tx_t3_without_end_of_msg_ack() {
    let mut tx = TpTx::rts(&message(9, RX_SA)).unwrap();
    tx.poll(0).unwrap();
    let c = cm(
        RX_SA,
        TX_SA,
        TpCm::Cts {
            packets: 2,
            next: 1,
            pgn: PGN,
        },
    );
    tx.on_frame(&c, 0).unwrap();
    tx.poll(100).unwrap();
    tx.poll(100).unwrap();
    assert_eq!(tx.poll(100 + T3_MS - 1), None);
    let f = tx.poll(100 + T3_MS).unwrap();
    assert_eq!(abort_reason(&f), AbortReason::Timeout);
    assert_eq!(tx.state(), TxState::Aborted(AbortReason::Timeout));
}

#[test]
fn rx_oversize_rts_aborts_with_resources() {
    let mut rx = TpRx::<1>::new(RX_SA);
    let r = rx.on_frame(&rts(TX_SA, RX_SA, 224, 0xFF), 0);
    let reply = r.reply.unwrap();
    assert_eq!(abort_reason(&reply), AbortReason::Resources);
    assert_eq!(reply.id.destination(), Some(TX_SA));
    assert_eq!(reply.id.source_address(), RX_SA);
    assert_eq!(r.message, None);
    // No session was created: the DT is ignored without a reply.
    let r = rx.on_frame(&dt(TX_SA, RX_SA, 1), 1);
    assert_eq!(r, Default::default());
    // And the only slot is still free.
    let r = rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 2);
    assert!(matches!(decode(&r.reply.unwrap()), TpCm::Cts { .. }));
}

#[test]
fn rx_undersize_rts_is_ignored() {
    let mut rx = TpRx::<1>::new(RX_SA);
    assert_eq!(
        rx.on_frame(&rts(TX_SA, RX_SA, 8, 0xFF), 0),
        Default::default()
    );
}

#[test]
fn rx_rts_for_other_destination_is_ignored() {
    let mut rx = TpRx::<1>::new(RX_SA);
    assert_eq!(
        rx.on_frame(&rts(TX_SA, 0x55, 20, 0xFF), 0),
        Default::default()
    );
    assert_eq!(
        rx.on_frame(&rts(TX_SA, 0xFF, 20, 0xFF), 0),
        Default::default()
    );
    // No session exists for either.
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 1), 1), Default::default());
}

#[test]
fn rx_cts_content_and_blocks() {
    let mut rx = TpRx::<1>::new(RX_SA);
    let r = rx.on_frame(&rts(TX_SA, RX_SA, 30, 2), 0);
    let cts = r.reply.unwrap();
    assert_eq!(cts.id.destination(), Some(TX_SA));
    assert_eq!(cts.id.source_address(), RX_SA);
    assert_eq!(
        decode(&cts),
        TpCm::Cts {
            packets: 2,
            next: 1,
            pgn: PGN
        }
    );
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 1), 1).reply, None);
    let r = rx.on_frame(&dt(TX_SA, RX_SA, 2), 2);
    assert_eq!(
        decode(&r.reply.unwrap()),
        TpCm::Cts {
            packets: 2,
            next: 3,
            pgn: PGN
        }
    );
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 3), 3).reply, None);
    // Last block has one packet left.
    let r = rx.on_frame(&dt(TX_SA, RX_SA, 4), 4);
    assert_eq!(
        decode(&r.reply.unwrap()),
        TpCm::Cts {
            packets: 1,
            next: 5,
            pgn: PGN
        }
    );
    let r = rx.on_frame(&dt(TX_SA, RX_SA, 5), 5);
    assert_eq!(
        decode(&r.reply.unwrap()),
        TpCm::EndOfMsgAck {
            size: 30,
            packets: 5,
            pgn: PGN
        }
    );
    let msg = r.message.unwrap();
    assert_eq!(msg.length, 30);
    assert_eq!(msg.destination_address, RX_SA);
    assert_eq!(msg.source_address, TX_SA);
    assert_eq!(msg.pgn, PGN);
}

#[test]
fn rx_wrong_sequence_aborts_and_frees_session() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 0);
    // Skips packet 1.
    let r = rx.on_frame(&dt(TX_SA, RX_SA, 2), 1);
    assert_eq!(abort_reason(&r.reply.unwrap()), AbortReason::BadSequence);
    assert_eq!(r.message, None);
    // The slot is free again: another source is accepted instead of Resources.
    let r = rx.on_frame(&rts(0x90, RX_SA, 20, 0xFF), 2);
    assert!(matches!(decode(&r.reply.unwrap()), TpCm::Cts { .. }));
}

#[test]
fn rx_sequence_zero_is_bad_sequence() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 0);
    let r = rx.on_frame(&dt(TX_SA, RX_SA, 0), 1);
    assert_eq!(abort_reason(&r.reply.unwrap()), AbortReason::BadSequence);
}

#[test]
fn rx_duplicate_dt_aborts_and_frees_session() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 0);
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 1), 1).reply, None);
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 2), 2).reply, None);
    let r = rx.on_frame(&dt(TX_SA, RX_SA, 2), 3);
    assert_eq!(
        abort_reason(&r.reply.unwrap()),
        AbortReason::DuplicateSequence
    );
    // Freed: a DT that would have been in order is now unexpected and ignored.
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 3), 4), Default::default());
    let r = rx.on_frame(&rts(0x90, RX_SA, 20, 0xFF), 5);
    assert!(matches!(decode(&r.reply.unwrap()), TpCm::Cts { .. }));
}

#[test]
fn rx_t2_timeout_after_cts() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 1000);
    // Not yet due, even past T1 which only applies between DTs.
    assert_eq!(rx.poll_timeout(1000 + T2_MS - 1), None);
    let abort = rx.poll_timeout(1000 + T2_MS).unwrap();
    assert_eq!(abort_reason(&abort), AbortReason::Timeout);
    assert_eq!(abort.id.destination(), Some(TX_SA));
    assert_eq!(abort.id.source_address(), RX_SA);
    // Reported once only.
    assert_eq!(rx.poll_timeout(1000 + T2_MS + 1), None);
    // A late DT yields no message and no reply.
    assert_eq!(
        rx.on_frame(&dt(TX_SA, RX_SA, 1), 1000 + T2_MS + 2),
        Default::default()
    );
    // The slot was freed by the timeout.
    let r = rx.on_frame(&rts(0x90, RX_SA, 20, 0xFF), 1000 + T2_MS + 3);
    assert!(matches!(decode(&r.reply.unwrap()), TpCm::Cts { .. }));
}

#[test]
fn rx_t1_timeout_between_dts() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 0);
    rx.on_frame(&dt(TX_SA, RX_SA, 1), 100);
    assert_eq!(rx.poll_timeout(100 + T1_MS - 1), None);
    let abort = rx.poll_timeout(100 + T1_MS).unwrap();
    assert_eq!(abort_reason(&abort), AbortReason::Timeout);
}

#[test]
fn rx_late_dt_is_rejected_with_timeout_abort() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 0);
    // poll_timeout was never called; the DT itself is checked against the timer.
    let r = rx.on_frame(&dt(TX_SA, RX_SA, 1), T2_MS);
    assert_eq!(abort_reason(&r.reply.unwrap()), AbortReason::Timeout);
    assert_eq!(r.message, None);
}

#[test]
fn rx_timer_restarts_after_each_cts() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 1), 0);
    // DT 1 completes the block and triggers the next CTS at t = 1000.
    assert!(rx.on_frame(&dt(TX_SA, RX_SA, 1), 1000).reply.is_some());
    assert_eq!(rx.poll_timeout(1000 + T2_MS - 1), None);
    assert!(rx.poll_timeout(1000 + T2_MS).is_some());
}

#[test]
fn rx_second_rts_gets_already_in_session() {
    let mut rx = TpRx::<2>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 0);
    let r = rx.on_frame(&rts(TX_SA, RX_SA, 30, 0xFF), 1);
    assert_eq!(
        abort_reason(&r.reply.unwrap()),
        AbortReason::AlreadyInSession
    );
    // The original session is unaffected and still completes (20 bytes, 3 packets).
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 1), 2).reply, None);
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 2), 3).reply, None);
    let r = rx.on_frame(&dt(TX_SA, RX_SA, 3), 4);
    assert_eq!(r.message.unwrap().length, 20);
}

#[test]
fn rx_no_free_slot_aborts_with_resources() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 0);
    let r = rx.on_frame(&rts(0x90, RX_SA, 20, 0xFF), 1);
    assert_eq!(abort_reason(&r.reply.unwrap()), AbortReason::Resources);
}

#[test]
fn rx_dt_with_wrong_destination_is_ignored() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 9, 0xFF), 0);
    // Broadcast and other-node DTs do not belong to an RTS session.
    assert_eq!(rx.on_frame(&dt(TX_SA, 0xFF, 1), 1), Default::default());
    assert_eq!(rx.on_frame(&dt(TX_SA, 0x55, 1), 1), Default::default());
    // The session is untouched: DT 1 still expected.
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 1), 2).reply, None);
    let r = rx.on_frame(&dt(TX_SA, RX_SA, 2), 3);
    assert_eq!(r.message.unwrap().length, 9);
}

#[test]
fn rx_bam_dt_must_be_broadcast() {
    let mut rx = TpRx::<1>::new(RX_SA);
    let bam = cm(
        TX_SA,
        0xFF,
        TpCm::Bam {
            size: 9,
            packets: 2,
            pgn: PGN,
        },
    );
    rx.on_frame(&bam, 0);
    // A DT addressed to a specific node is not part of the broadcast session.
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 1), 1), Default::default());
    assert_eq!(rx.on_frame(&dt(TX_SA, 0xFF, 1), 2), Default::default());
    let r = rx.on_frame(&dt(TX_SA, 0xFF, 2), 3);
    assert_eq!(r.message.unwrap().length, 9);
}

fn sender_abort(src: u8, dst: u8, pgn: u32) -> CanFrame {
    cm(
        src,
        dst,
        TpCm::Abort {
            reason: AbortReason::Timeout,
            pgn,
        },
    )
}

fn is_cts(r: &j1939_core::tp::RxResult) -> bool {
    r.reply
        .is_some_and(|f| matches!(decode(&f), TpCm::Cts { .. }))
}

#[test]
fn rx_sender_abort_frees_session_silently() {
    let mut rx = TpRx::<2>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 0);
    let r = rx.on_frame(&sender_abort(TX_SA, RX_SA, PGN), 1);
    assert_eq!(r, Default::default());
    // A retried RTS is accepted, not AlreadyInSession.
    let r = rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 2);
    assert!(is_cts(&r));
    // No stale session remains to time out silently or with an abort: only the new one.
    assert_eq!(rx.poll_timeout(1 + T2_MS), None);
    assert!(rx.poll_timeout(2 + T2_MS).is_some());
}

#[test]
fn rx_sender_abort_frees_the_only_slot() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 0);
    rx.on_frame(&sender_abort(TX_SA, RX_SA, PGN), 1);
    let r = rx.on_frame(&rts(0x90, RX_SA, 20, 0xFF), 2);
    assert!(is_cts(&r));
    // The aborted session does not emit a Timeout abort later.
    assert_eq!(rx.poll_timeout(1 + T2_MS), None);
}

#[test]
fn rx_unrelated_aborts_keep_the_session() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 0);
    // Different PGN, different source, and not addressed to us.
    rx.on_frame(&sender_abort(TX_SA, RX_SA, 0xFEEE), 1);
    rx.on_frame(&sender_abort(0x90, RX_SA, PGN), 1);
    rx.on_frame(&sender_abort(TX_SA, 0x55, PGN), 1);
    rx.on_frame(&sender_abort(TX_SA, 0xFF, PGN), 1);
    let r = rx.on_frame(&rts(TX_SA, RX_SA, 20, 0xFF), 2);
    assert_eq!(
        abort_reason(&r.reply.unwrap()),
        AbortReason::AlreadyInSession
    );
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 1), 3).reply, None);
    assert_eq!(rx.on_frame(&dt(TX_SA, RX_SA, 2), 4).reply, None);
    assert!(rx.on_frame(&dt(TX_SA, RX_SA, 3), 5).message.is_some());
}

#[test]
fn rx_abort_does_not_touch_bam_session() {
    // BAMs have no abort in J1939-21, so an Abort naming a BAM's PGN is ignored.
    let mut rx = TpRx::<1>::new(RX_SA);
    let bam = cm(
        TX_SA,
        0xFF,
        TpCm::Bam {
            size: 9,
            packets: 2,
            pgn: PGN,
        },
    );
    rx.on_frame(&bam, 0);
    rx.on_frame(&sender_abort(TX_SA, RX_SA, PGN), 1);
    rx.on_frame(&dt(TX_SA, 0xFF, 1), 2);
    assert!(rx.on_frame(&dt(TX_SA, 0xFF, 2), 3).message.is_some());
}
