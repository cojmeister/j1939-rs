//! One originator may run a BAM and an RTS/CTS session at the same time (J1939-21).

use j1939_core::tp::{AbortReason, CanFrame, RxResult, TpCm, TpRx, TpTx, TxState, dt_packet};
use j1939_core::{J1939Id, J1939Message};

const TX_SA: u8 = 0x80;
const RX_SA: u8 = 0x00;
const BAM_PGN: u32 = 0xFECA;
const RTS_PGN: u32 = 0xFEEE;

fn bam_message(len: u8) -> J1939Message {
    let data: Vec<u8> = (1..=len).collect();
    let id = J1939Id::new(7, BAM_PGN, None, TX_SA).unwrap();
    J1939Message::from_id_and_data(id, &data).unwrap()
}

fn rts_message(len: u8) -> J1939Message {
    let data: Vec<u8> = (1..=len).map(|b| b.wrapping_mul(3)).collect();
    let id = J1939Id::new(7, RTS_PGN, None, TX_SA).unwrap();
    let mut msg = J1939Message::from_id_and_data(id, &data).unwrap();
    msg.destination_address = RX_SA;
    msg
}

fn cm(dst: u8, cm: TpCm) -> CanFrame {
    let id = J1939Id::new(7, 0xEC00, Some(dst), TX_SA).unwrap();
    CanFrame::new(id, &cm.encode()).unwrap()
}

fn bam(size: u16) -> CanFrame {
    cm(
        0xFF,
        TpCm::Bam {
            size,
            packets: size.div_ceil(7) as u8,
            pgn: BAM_PGN,
        },
    )
}

fn rts(size: u16) -> CanFrame {
    cm(
        RX_SA,
        TpCm::Rts {
            size,
            packets: size.div_ceil(7) as u8,
            max_per_cts: 0xFF,
            pgn: RTS_PGN,
        },
    )
}

fn dt(dst: u8, seq: u8) -> CanFrame {
    let id = J1939Id::new(7, 0xEB00, Some(dst), TX_SA).unwrap();
    CanFrame::new(id, &dt_packet(seq, &[seq; 7])).unwrap()
}

fn reply(r: &RxResult) -> Option<TpCm> {
    r.reply.map(|f| TpCm::decode(f.data()).unwrap())
}

fn is_cts(r: &RxResult) -> bool {
    matches!(reply(r), Some(TpCm::Cts { .. }))
}

#[test]
fn bam_and_rts_from_one_source_interleaved_both_complete() {
    let a = bam_message(30);
    let b = rts_message(40);
    let mut bam_tx = TpTx::bam(&a).unwrap();
    let mut rts_tx = TpTx::rts(&b).unwrap().with_max_per_cts(2);
    let mut rx = TpRx::<2>::new(RX_SA);
    let mut got = Vec::new();
    let mut now = 1000u32;
    while bam_tx.state() == TxState::Active || rts_tx.state() == TxState::Active {
        // One frame from each transmitter per tick, so the sessions interleave frame by frame.
        if let Some(f) = bam_tx.poll(now) {
            got.extend(rx.on_frame(&f, now).message);
        }
        if let Some(f) = rts_tx.poll(now) {
            let r = rx.on_frame(&f, now);
            got.extend(r.message);
            if let Some(reply) = r.reply {
                rts_tx.on_frame(&reply, now).unwrap();
            }
        }
        now += 10;
        assert!(now < 100_000, "transfers never finished");
    }
    assert_eq!(bam_tx.state(), TxState::Done);
    assert_eq!(rts_tx.state(), TxState::Done);
    assert_eq!(got.len(), 2);
    assert!(got.contains(&a), "BAM payload missing: {got:?}");
    assert!(got.contains(&b), "RTS payload missing: {got:?}");
}

#[test]
fn bam_starting_mid_rts_does_not_disturb_it() {
    let mut rx = TpRx::<2>::new(RX_SA);
    assert!(is_cts(&rx.on_frame(&rts(20), 0)));
    assert_eq!(rx.on_frame(&dt(RX_SA, 1), 1).reply, None);
    // The same node now announces a broadcast.
    assert_eq!(rx.on_frame(&bam(9), 2), RxResult::default());
    assert_eq!(rx.on_frame(&dt(RX_SA, 2), 3).reply, None);
    let r = rx.on_frame(&dt(RX_SA, 3), 4);
    let msg = r.message.expect("RTS session completes");
    assert_eq!(msg.pgn, RTS_PGN);
    assert_eq!(msg.length, 20);
    assert_eq!(&msg.data[..20], &[[1u8; 7], [2; 7], [3; 7]].concat()[..20]);
    assert!(matches!(reply(&r), Some(TpCm::EndOfMsgAck { .. })));
    // The BAM session runs alongside.
    assert_eq!(rx.on_frame(&dt(0xFF, 1), 5).message, None);
    let msg = rx.on_frame(&dt(0xFF, 2), 6).message.expect("BAM completes");
    assert_eq!(msg.pgn, BAM_PGN);
    assert_eq!(msg.length, 9);
}

#[test]
fn rts_starting_mid_bam_gets_cts() {
    let mut rx = TpRx::<2>::new(RX_SA);
    rx.on_frame(&bam(20), 0);
    assert_eq!(rx.on_frame(&dt(0xFF, 1), 1).message, None);
    let r = rx.on_frame(&rts(9), 2);
    assert!(is_cts(&r), "expected CTS, got {:?}", reply(&r));
    // Both sessions complete, each picked by the DT destination.
    assert_eq!(rx.on_frame(&dt(RX_SA, 1), 3).message, None);
    assert_eq!(rx.on_frame(&dt(0xFF, 2), 4).message, None);
    let rts_msg = rx.on_frame(&dt(RX_SA, 2), 5).message.unwrap();
    assert_eq!((rts_msg.pgn, rts_msg.length), (RTS_PGN, 9));
    let bam_msg = rx.on_frame(&dt(0xFF, 3), 6).message.unwrap();
    assert_eq!((bam_msg.pgn, bam_msg.length), (BAM_PGN, 20));
    assert_eq!(
        &bam_msg.data[..20],
        &[[1u8; 7], [2; 7], [3; 7]].concat()[..20]
    );
}

#[test]
fn same_mode_rules_still_apply_with_both_sessions_open() {
    let mut rx = TpRx::<2>::new(RX_SA);
    rx.on_frame(&rts(20), 0);
    rx.on_frame(&bam(20), 0);
    // A second RTS from the source is still AlreadyInSession.
    let r = rx.on_frame(&rts(30), 1);
    assert!(matches!(
        reply(&r),
        Some(TpCm::Abort {
            reason: AbortReason::AlreadyInSession,
            ..
        })
    ));
    // A second BAM replaces the first BAM only.
    rx.on_frame(&dt(0xFF, 1), 2);
    rx.on_frame(&bam(9), 3);
    assert_eq!(rx.on_frame(&dt(0xFF, 1), 4).message, None);
    assert_eq!(rx.on_frame(&dt(0xFF, 2), 5).message.unwrap().length, 9);
    // A sender abort frees only the RTS session.
    rx.on_frame(&bam(9), 6);
    rx.on_frame(
        &cm(
            RX_SA,
            TpCm::Abort {
                reason: AbortReason::Timeout,
                pgn: RTS_PGN,
            },
        ),
        7,
    );
    assert_eq!(rx.on_frame(&dt(RX_SA, 1), 8), RxResult::default());
    assert_eq!(rx.on_frame(&dt(0xFF, 1), 8).message, None);
    assert_eq!(rx.on_frame(&dt(0xFF, 2), 9).message.unwrap().length, 9);
}

#[test]
fn invalid_bam_does_not_end_sessions() {
    let mut rx = TpRx::<2>::new(RX_SA);
    rx.on_frame(&rts(9), 0);
    rx.on_frame(&bam(9), 0);
    // Wrong packet count: ignored, and neither session is touched.
    rx.on_frame(
        &cm(
            0xFF,
            TpCm::Bam {
                size: 20,
                packets: 2,
                pgn: BAM_PGN,
            },
        ),
        1,
    );
    rx.on_frame(&dt(RX_SA, 1), 2);
    assert_eq!(rx.on_frame(&dt(RX_SA, 2), 3).message.unwrap().length, 9);
    rx.on_frame(&dt(0xFF, 1), 4);
    assert_eq!(rx.on_frame(&dt(0xFF, 2), 5).message.unwrap().length, 9);
}

#[test]
fn single_slot_rts_during_bam_aborts_with_resources() {
    let mut rx = TpRx::<1>::new(RX_SA);
    rx.on_frame(&bam(9), 0);
    let r = rx.on_frame(&rts(9), 1);
    assert!(matches!(
        reply(&r),
        Some(TpCm::Abort {
            reason: AbortReason::Resources,
            ..
        })
    ));
    rx.on_frame(&dt(0xFF, 1), 2);
    assert_eq!(rx.on_frame(&dt(0xFF, 2), 3).message.unwrap().length, 9);
}

#[test]
fn single_slot_bam_during_rts_is_ignored() {
    let mut rx = TpRx::<1>::new(RX_SA);
    assert!(is_cts(&rx.on_frame(&rts(9), 0)));
    assert_eq!(rx.on_frame(&bam(9), 1), RxResult::default());
    assert_eq!(rx.on_frame(&dt(0xFF, 1), 2), RxResult::default());
    assert_eq!(rx.on_frame(&dt(0xFF, 2), 3), RxResult::default());
    rx.on_frame(&dt(RX_SA, 1), 4);
    assert_eq!(rx.on_frame(&dt(RX_SA, 2), 5).message.unwrap().length, 9);
}
