use j1939_core::tp::timing::{BAM_INTERVAL_MS, T1_MS};
use j1939_core::tp::{CanFrame, TpCm, TpRx, TpTx, TxState};
use j1939_core::{Error, J1939Id, J1939Message};

/// Priority 7 because the TP wire format cannot carry the original message priority.
fn message(len: u8) -> J1939Message {
    let data: Vec<u8> = (1..=len).collect();
    let id = J1939Id::new(7, 0xFECA, None, 0x80).unwrap();
    J1939Message::from_id_and_data(id, &data).unwrap()
}

/// Drive a transmitter with a fake clock and collect (time, frame) pairs.
fn drain(tx: &mut TpTx) -> Vec<(u32, CanFrame)> {
    let mut out = Vec::new();
    let mut now = 1000u32;
    while tx.state() == TxState::Active {
        if let Some(f) = tx.poll(now) {
            out.push((now, f));
        }
        now += 10;
        assert!(now < 100_000, "transmitter never finished");
    }
    out
}

#[test]
fn tx_sequence_and_timing() {
    let msg = message(20);
    let mut tx = TpTx::bam(&msg).unwrap();
    assert_eq!(tx.state(), TxState::Active);
    let frames = drain(&mut tx);
    assert_eq!(tx.state(), TxState::Done);
    assert_eq!(frames.len(), 4);

    let announce = &frames[0].1;
    assert_eq!(announce.id.pgn(), 0xEC00);
    assert_eq!(announce.id.destination(), Some(0xFF));
    assert_eq!(announce.id.source_address(), 0x80);
    assert_eq!(announce.id.priority(), 7);
    assert_eq!(
        announce.data(),
        &[0x20, 0x14, 0x00, 0x03, 0xFF, 0xCA, 0xFE, 0x00]
    );

    let expected: [&[u8]; 3] = [
        &[1, 2, 3, 4, 5, 6, 7],
        &[8, 9, 10, 11, 12, 13, 14],
        &[15, 16, 17, 18, 19, 20],
    ];
    for (i, chunk) in expected.iter().enumerate() {
        let f = &frames[i + 1].1;
        assert_eq!(f.id.pgn(), 0xEB00);
        assert_eq!(f.id.destination(), Some(0xFF));
        assert_eq!(f.id.priority(), 7);
        assert_eq!(f.len, 8);
        assert_eq!(f.data[0], i as u8 + 1);
        let mut want = [0xFFu8; 7];
        want[..chunk.len()].copy_from_slice(chunk);
        assert_eq!(&f.data[1..], &want);
        assert!(frames[i + 1].0 - frames[i].0 >= BAM_INTERVAL_MS);
    }
    assert_eq!(tx.poll(1_000_000), None);
}

#[test]
fn tx_waits_for_interval() {
    let mut tx = TpTx::bam(&message(20)).unwrap();
    assert!(tx.poll(100).is_some());
    assert_eq!(tx.poll(100), None);
    assert_eq!(tx.poll(100 + BAM_INTERVAL_MS - 1), None);
    assert!(tx.poll(100 + BAM_INTERVAL_MS).is_some());
}

#[test]
fn tx_clock_wraps() {
    let mut tx = TpTx::bam(&message(9)).unwrap();
    assert!(tx.poll(u32::MAX - 10).is_some());
    assert_eq!(tx.poll(u32::MAX), None);
    assert!(tx.poll(BAM_INTERVAL_MS - 11).is_some());
}

#[test]
fn tx_length_limits() {
    assert_eq!(TpTx::bam(&message(8)).err(), Some(Error::InvalidLength));
    assert!(TpTx::bam(&message(9)).is_ok());
    assert!(TpTx::bam(&message(223)).is_ok());
    let mut big = message(223);
    big.length = 224;
    assert_eq!(TpTx::bam(&big).err(), Some(Error::InvalidLength));
}

#[test]
fn tx_ignores_incoming_frames() {
    let mut tx = TpTx::bam(&message(20)).unwrap();
    let id = J1939Id::new(7, 0xEC00, Some(0x80), 0x00).unwrap();
    let abort = CanFrame::new(id, &[0xFF, 3, 0xFF, 0xFF, 0xFF, 0xCA, 0xFE, 0]).unwrap();
    assert_eq!(tx.on_frame(&abort, 0), Ok(()));
    assert_eq!(tx.state(), TxState::Active);
}

#[test]
fn round_trip() {
    let msg = message(20);
    let mut tx = TpTx::bam(&msg).unwrap();
    let frames = drain(&mut tx);
    let mut rx = TpRx::<1>::new(0x00);
    let mut got = None;
    for (i, (t, f)) in frames.iter().enumerate() {
        let r = rx.on_frame(f, *t);
        assert_eq!(r.reply, None);
        if i + 1 < frames.len() {
            assert_eq!(r.message, None);
        } else {
            got = r.message;
        }
    }
    assert_eq!(got, Some(msg));
}

#[test]
fn round_trip_max_size() {
    let msg = message(223);
    let mut tx = TpTx::bam(&msg).unwrap();
    let frames = drain(&mut tx);
    let mut rx = TpRx::<1>::new(0x00);
    let mut got = None;
    for (t, f) in &frames {
        got = rx.on_frame(f, *t).message.or(got);
    }
    assert_eq!(got, Some(msg));
}

#[test]
fn dt_without_session_is_ignored() {
    let mut rx = TpRx::<1>::new(0x00);
    let id = J1939Id::new(7, 0xEB00, Some(0xFF), 0x80).unwrap();
    let f = CanFrame::new(id, &[1, 1, 2, 3, 4, 5, 6, 7]).unwrap();
    let r = rx.on_frame(&f, 0);
    assert_eq!((r.reply, r.message), (None, None));
}

#[test]
fn poll_timeout_frees_the_slot() {
    let a = message(20);
    let mut b = message(20);
    b.source_address = 0x90;
    let fa = drain(&mut TpTx::bam(&a).unwrap());
    let fb = drain(&mut TpTx::bam(&b).unwrap());
    let mut rx = TpRx::<1>::new(0x00);
    // Source A starts a session at t=0 and then stalls.
    assert_eq!(rx.on_frame(&fa[0].1, 0).message, None);
    // Just before the timeout the slot is still occupied: B's BAM is ignored.
    assert_eq!(rx.poll_timeout(T1_MS - 1), None);
    rx.on_frame(&fb[0].1, T1_MS - 1);
    for (_, f) in &fb[1..] {
        assert_eq!(rx.on_frame(f, T1_MS - 1).message, None);
    }
    // After the timeout poll_timeout drops the session silently (no abort for BAM).
    assert_eq!(rx.poll_timeout(T1_MS), None);
    // B can now use the slot. Its own activity is fresh, so only the freed slot matters.
    let t = T1_MS + 10;
    let mut got = None;
    for (_, f) in &fb {
        got = rx.on_frame(f, t).message.or(got);
    }
    assert_eq!(got, Some(b));
    // The dropped session from A is gone: its DTs yield nothing.
    for (_, f) in &fa[1..] {
        assert_eq!(rx.on_frame(f, t).message, None);
    }
}

#[test]
fn stale_session_does_not_complete_without_poll() {
    let msg = message(20);
    let mut tx = TpTx::bam(&msg).unwrap();
    let frames = drain(&mut tx);
    let mut rx = TpRx::<1>::new(0x00);
    rx.on_frame(&frames[0].1, 0);
    rx.on_frame(&frames[1].1, 50);
    rx.on_frame(&frames[2].1, 100);
    assert_eq!(rx.on_frame(&frames[3].1, 100 + T1_MS + 1).message, None);
}

#[test]
fn bad_sequence_drops_session() {
    let msg = message(20);
    let mut tx = TpTx::bam(&msg).unwrap();
    let frames = drain(&mut tx);
    let mut rx = TpRx::<1>::new(0x00);
    rx.on_frame(&frames[0].1, 0);
    // Skip DT 1.
    assert_eq!(rx.on_frame(&frames[2].1, 50).message, None);
    assert_eq!(rx.on_frame(&frames[1].1, 100).message, None);
    assert_eq!(rx.on_frame(&frames[3].1, 150).message, None);
}

#[test]
fn invalid_announcements_are_ignored() {
    let mut rx = TpRx::<1>::new(0x00);
    let id = J1939Id::new(7, 0xEC00, Some(0xFF), 0x80).unwrap();
    let bam = |size: u16, packets: u8| {
        let cm = TpCm::Bam {
            size,
            packets,
            pgn: 0xFECA,
        };
        CanFrame::new(id, &cm.encode()).unwrap()
    };
    let did = J1939Id::new(7, 0xEB00, Some(0xFF), 0x80).unwrap();
    let dt = |seq: u8| CanFrame::new(did, &[seq, 1, 2, 3, 4, 5, 6, 7]).unwrap();
    // Too large, wrong packet count, too small.
    for (size, packets) in [(224u16, 32u8), (1785, 255), (20, 4), (20, 2), (8, 2)] {
        rx.on_frame(&bam(size, packets), 0);
        for s in 1..=packets.min(40) {
            assert_eq!(rx.on_frame(&dt(s), 10).message, None);
        }
    }
}

#[test]
fn second_bam_from_same_source_replaces_first() {
    let msg = message(20);
    let mut tx = TpTx::bam(&msg).unwrap();
    let frames = drain(&mut tx);
    let mut rx = TpRx::<1>::new(0x00);
    rx.on_frame(&frames[0].1, 0);
    rx.on_frame(&frames[1].1, 50);
    // Restart.
    let mut got = None;
    for (i, (_, f)) in frames.iter().enumerate() {
        got = rx.on_frame(f, 100 + 50 * i as u32).message.or(got);
    }
    assert_eq!(got, Some(msg));
}

#[test]
fn concurrent_sources_use_separate_sessions() {
    let a = message(20);
    let mut b = message(30);
    b.source_address = 0x90;
    let fa = drain(&mut TpTx::bam(&a).unwrap());
    let fb = drain(&mut TpTx::bam(&b).unwrap());
    let mut rx = TpRx::<2>::new(0x00);
    let mut got = Vec::new();
    for i in 0..fa.len().max(fb.len()) {
        for frames in [&fa, &fb] {
            let Some((_, f)) = frames.get(i) else {
                continue;
            };
            if let Some(m) = rx.on_frame(f, 10 * i as u32).message {
                got.push(m);
            }
        }
    }
    assert_eq!(got.len(), 2);
    assert!(got.contains(&a) && got.contains(&b));
}

#[test]
fn full_table_ignores_new_source() {
    let a = message(20);
    let mut b = message(20);
    b.source_address = 0x90;
    let fa = drain(&mut TpTx::bam(&a).unwrap());
    let fb = drain(&mut TpTx::bam(&b).unwrap());
    let mut rx = TpRx::<1>::new(0x00);
    rx.on_frame(&fa[0].1, 0);
    for (_, f) in &fb {
        assert_eq!(rx.on_frame(f, 10).message, None);
    }
    let mut got = None;
    for (_, f) in &fa[1..] {
        got = rx.on_frame(f, 20).message.or(got);
    }
    assert_eq!(got, Some(a));
}

fn pdu1_message(pgn: u32) -> J1939Message {
    let mut msg = J1939Message {
        pgn,
        length: 20,
        source_address: 0x80,
        destination_address: 0xFF,
        ..Default::default()
    };
    msg.data[..20].copy_from_slice(&[7; 20]);
    msg
}

#[test]
fn bam_rejects_pdu1_pgn_with_nonzero_low_byte() {
    assert_eq!(
        TpTx::bam(&pdu1_message(0x5001)).err(),
        Some(Error::InvalidPgn)
    );
    assert!(TpTx::bam(&pdu1_message(0x5000)).is_ok());
}

#[test]
fn bam_of_pdu2_pgn_round_trips_through_receiver() {
    let msg = message(20);
    assert_eq!(msg.pgn, 0xFECA);
    let mut tx = TpTx::bam(&msg).unwrap();
    let mut rx = TpRx::<1>::new(0x20);
    let mut got = None;
    for (now, f) in drain(&mut tx) {
        got = got.or(rx.on_frame(&f, now).message);
    }
    let got = got.expect("delivered");
    assert_eq!(got.pgn, 0xFECA);
    assert_eq!(got.data[..20], msg.data[..20]);
}
