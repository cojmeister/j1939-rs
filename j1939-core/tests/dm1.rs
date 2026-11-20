use j1939_core::tp::{TpRx, TpTx, TxState};
use j1939_core::{Dm1, Dtc, J1939Message, LampStatus, Marshall, Unmarshall};

#[test]
fn dm1_over_bam() {
    let mut dm = Dm1::<8>::new(
        LampStatus {
            malfunction: 1,
            red_stop: 0,
            amber_warning: 1,
            protect: 0,
        },
        LampStatus::default(),
    );
    for i in 0..5u8 {
        dm.push(Dtc {
            spn: 1000 + u32::from(i),
            fmi: i,
            conversion_method: false,
            occurrence_count: i + 1,
        })
        .unwrap();
    }
    let mut msg = J1939Message::default();
    dm.marshall(&mut msg).unwrap();
    msg.source_address = 0x80;
    assert_eq!(msg.length, 22);

    let mut tx = TpTx::bam(&msg).unwrap();
    let mut rx = TpRx::<1>::new(0x00);
    let mut now = 1000u32;
    let mut got = None;
    while tx.state() == TxState::Active {
        let reassembled = tx.poll(now).and_then(|f| rx.on_frame(&f, now).message);
        got = got.or(reassembled);
        now += 10;
        assert!(now < 100_000, "transmitter never finished");
    }
    let got = got.expect("message reassembled");
    assert_eq!(got.length, msg.length);
    assert_eq!(got.data[..22], msg.data[..22]);
    assert_eq!(Dm1::<8>::unmarshall(&got).unwrap(), dm);
}
