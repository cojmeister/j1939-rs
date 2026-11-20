use super::timing::{BAM_INTERVAL_MS, T3_MS, T4_MS};
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
    Rts(RtsPhase),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RtsPhase {
    /// The RTS has not been sent yet.
    SendRts,
    /// Waiting for a CTS or the end-of-message ack (T3).
    WaitCts,
    /// Sending the block requested by the last CTS.
    Sending,
    /// The receiver asked us to wait with `packets == 0` (T4).
    Hold,
}

/// A multi-packet transmit session. Drive it with [`TpTx::poll`].
#[derive(Debug, Clone, Copy)]
pub struct TpTx {
    mode: TxMode,
    msg: J1939Message,
    packets: u8,
    /// Sequence number of the next DT to send; 0 means the announce is still pending.
    next_seq: u8,
    /// Last DT sequence number of the block currently being sent (RTS mode).
    block_end: u8,
    max_per_cts: u8,
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
            block_end: 0,
            max_per_cts: 0xFF,
            last_activity_ms: 0,
            state: TxState::Active,
        })
    }

    /// Start a connection-mode (RTS/CTS) transfer of `msg` to `msg.destination_address`.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidId`] if the destination is the global address 0xFF;
    /// [`Error::InvalidLength`] unless `9 <= msg.length <= 223`.
    pub fn rts(msg: &J1939Message) -> Result<Self> {
        if msg.destination_address == 0xFF {
            return Err(Error::InvalidId);
        }
        let mut tx = Self::bam(msg)?;
        tx.mode = TxMode::Rts(RtsPhase::SendRts);
        Ok(tx)
    }

    /// Limit how many packets the receiver may request per CTS (announced in the RTS).
    /// Defaults to 0xFF (no limit); 0 is treated as no limit.
    #[must_use]
    pub fn with_max_per_cts(mut self, max: u8) -> Self {
        self.max_per_cts = if max == 0 { 0xFF } else { max };
        self
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
            TxMode::Rts(phase) => self.poll_rts(phase, now_ms),
        }
    }

    fn poll_rts(&mut self, phase: RtsPhase, now_ms: u32) -> Option<CanFrame> {
        match phase {
            RtsPhase::SendRts => {
                let cm = TpCm::Rts {
                    size: self.msg.length,
                    packets: self.packets,
                    max_per_cts: self.max_per_cts,
                    pgn: self.msg.pgn,
                };
                let frame = self.cm_frame(&cm)?;
                self.last_activity_ms = now_ms;
                self.mode = TxMode::Rts(RtsPhase::WaitCts);
                Some(frame)
            }
            RtsPhase::WaitCts | RtsPhase::Hold => {
                let limit = if phase == RtsPhase::Hold {
                    T4_MS
                } else {
                    T3_MS
                };
                if now_ms.wrapping_sub(self.last_activity_ms) < limit {
                    return None;
                }
                self.state = TxState::Aborted(AbortReason::Timeout);
                self.cm_frame(&TpCm::Abort {
                    reason: AbortReason::Timeout,
                    pgn: self.msg.pgn,
                })
            }
            RtsPhase::Sending => {
                let seq = self.next_seq;
                let start = (usize::from(seq) - 1) * 7;
                let end = (start + 7).min(usize::from(self.msg.length));
                let dt = dt_packet(seq, &self.msg.data[start..end]);
                let frame = self.frame(PGN_TP_DT, &dt, self.msg.destination_address)?;
                self.last_activity_ms = now_ms;
                if seq == self.block_end {
                    self.mode = TxMode::Rts(RtsPhase::WaitCts);
                } else {
                    self.next_seq += 1;
                }
                Some(frame)
            }
        }
    }

    fn cm_frame(&self, cm: &TpCm) -> Option<CanFrame> {
        self.frame(PGN_TP_CM, &cm.encode(), self.msg.destination_address)
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
            self.frame(PGN_TP_CM, &cm.encode(), 0xFF)
        } else {
            let start = (usize::from(self.next_seq) - 1) * 7;
            let end = (start + 7).min(usize::from(self.msg.length));
            let dt = dt_packet(self.next_seq, &self.msg.data[start..end]);
            self.frame(PGN_TP_DT, &dt, 0xFF)
        }?;
        self.last_activity_ms = now_ms;
        if self.next_seq == self.packets {
            self.state = TxState::Done;
        } else {
            self.next_seq += 1;
        }
        Some(frame)
    }

    fn frame(&self, pgn: u32, data: &[u8], destination: u8) -> Option<CanFrame> {
        let id = J1939Id::new(7, pgn, Some(destination), self.msg.source_address).ok()?;
        CanFrame::new(id, data).ok()
    }

    /// Feed a received frame to the session.
    ///
    /// BAM sessions have no peer, so every frame is ignored. In RTS mode, CTS, end-of-message
    /// ack and abort frames from the destination are acted on; others are ignored.
    ///
    /// # Errors
    ///
    /// [`Error::InvalidTp`] for a CTS whose `next`/`packets` do not fit the message.
    pub fn on_frame(&mut self, frame: &CanFrame, now_ms: u32) -> Result<()> {
        let TxMode::Rts(phase) = self.mode else {
            return Ok(());
        };
        if self.state != TxState::Active
            || frame.id.pgn() != PGN_TP_CM
            || frame.id.source_address() != self.msg.destination_address
            || frame.id.destination() != Some(self.msg.source_address)
        {
            return Ok(());
        }
        match TpCm::decode(frame.data()) {
            Ok(TpCm::Cts { packets, next, pgn }) if pgn == self.msg.pgn => {
                // A CTS in the middle of our own block is not meaningful; keep sending.
                if phase == RtsPhase::Sending || phase == RtsPhase::SendRts {
                    return Ok(());
                }
                if packets == 0 {
                    self.last_activity_ms = now_ms;
                    self.mode = TxMode::Rts(RtsPhase::Hold);
                    return Ok(());
                }
                let last = u16::from(next) + u16::from(packets) - 1;
                if next == 0 || last > u16::from(self.packets) {
                    return Err(Error::InvalidTp);
                }
                self.next_seq = next;
                self.block_end = last as u8;
                self.last_activity_ms = now_ms;
                self.mode = TxMode::Rts(RtsPhase::Sending);
            }
            Ok(TpCm::EndOfMsgAck { pgn, .. }) if pgn == self.msg.pgn => {
                self.state = TxState::Done;
            }
            Ok(TpCm::Abort { reason, pgn }) if pgn == self.msg.pgn => {
                self.state = TxState::Aborted(reason);
            }
            _ => {}
        }
        Ok(())
    }
}
