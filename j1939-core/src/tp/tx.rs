use super::timing::BAM_INTERVAL_MS;
use super::{AbortReason, CanFrame, PGN_TP_CM, PGN_TP_DT, TpCm, dt_packet, packets_for};
use crate::{Error, J1939Id, J1939Message, Result};

/// Progress of a transmit session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxState {
    Active,
    Done,
    Aborted(AbortReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TxMode {
    Bam,
}

/// A multi-packet transmit session. Drive it with [`TpTx::poll`].
#[derive(Debug, Clone, Copy)]
pub struct TpTx {
    mode: TxMode,
    msg: J1939Message,
    packets: u8,
    /// Sequence number of the next DT to send; 0 means the announce is still pending.
    next_seq: u8,
    last_activity_ms: u32,
    state: TxState,
}

impl TpTx {
    /// Start a broadcast announce message (BAM) transfer of `msg`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidLength`] unless `9 <= msg.length <= 223`.
    pub fn bam(msg: &J1939Message) -> Result<Self> {
        let len = usize::from(msg.length);
        if !(9..=J1939Message::MAX_LENGTH).contains(&len) {
            return Err(Error::InvalidLength);
        }
        Ok(Self {
            mode: TxMode::Bam,
            msg: *msg,
            packets: packets_for(len),
            next_seq: 0,
            last_activity_ms: 0,
            state: TxState::Active,
        })
    }

    pub fn state(&self) -> TxState {
        self.state
    }

    /// The next frame to put on the bus, if one is due at `now_ms`.
    pub fn poll(&mut self, now_ms: u32) -> Option<CanFrame> {
        if self.state != TxState::Active {
            return None;
        }
        match self.mode {
            TxMode::Bam => self.poll_bam(now_ms),
        }
    }

    fn poll_bam(&mut self, now_ms: u32) -> Option<CanFrame> {
        if self.next_seq != 0 && now_ms.wrapping_sub(self.last_activity_ms) < BAM_INTERVAL_MS {
            return None;
        }
        let frame = if self.next_seq == 0 {
            let cm = TpCm::Bam {
                size: self.msg.length,
                packets: self.packets,
                pgn: self.msg.pgn,
            };
            self.frame(PGN_TP_CM, &cm.encode())
        } else {
            let start = (usize::from(self.next_seq) - 1) * 7;
            let end = (start + 7).min(usize::from(self.msg.length));
            let dt = dt_packet(self.next_seq, &self.msg.data[start..end]);
            self.frame(PGN_TP_DT, &dt)
        }?;
        self.last_activity_ms = now_ms;
        if self.next_seq == self.packets {
            self.state = TxState::Done;
        } else {
            self.next_seq += 1;
        }
        Some(frame)
    }

    fn frame(&self, pgn: u32, data: &[u8]) -> Option<CanFrame> {
        let id = J1939Id::new(7, pgn, Some(0xFF), self.msg.source_address).ok()?;
        CanFrame::new(id, data).ok()
    }

    /// Feed a received frame to the session. BAM sessions have no peer, so it is ignored.
    pub fn on_frame(&mut self, _frame: &CanFrame, _now_ms: u32) -> Result<()> {
        Ok(())
    }
}
