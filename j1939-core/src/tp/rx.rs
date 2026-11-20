use super::timing::{T1_MS, T2_MS};
use super::{AbortReason, CanFrame, PGN_TP_CM, PGN_TP_DT, TpCm, packets_for};
use crate::{J1939Id, J1939Message};

/// Outcome of feeding one frame to [`TpRx`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct RxResult {
    /// A frame (CTS, ACK or abort) that must be sent in response.
    pub reply: Option<CanFrame>,
    /// A completely reassembled message.
    pub message: Option<J1939Message>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum RxMode {
    Bam,
    Rts,
}

#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
struct Session {
    mode: RxMode,
    source: u8,
    size: u16,
    packets: u8,
    next_seq: u8,
    /// Last sequence number requested by the outstanding CTS (RTS mode).
    block_end: u8,
    max_per_cts: u8,
    last_activity_ms: u32,
    /// How long after `last_activity_ms` the session expires.
    timeout_ms: u32,
    msg: J1939Message,
}

/// Reassembles multi-packet messages.
///
/// As J1939-21 allows, each source address may run one BAM session and one RTS/CTS
/// session at the same time; each open session takes one of the `SESSIONS` slots.
/// TP.DT frames are matched to a session by source and destination (global for BAM,
/// our address for RTS/CTS).
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct TpRx<const SESSIONS: usize = 1> {
    local_address: u8,
    sessions: [Option<Session>; SESSIONS],
}

impl<const SESSIONS: usize> TpRx<SESSIONS> {
    pub fn new(local_address: u8) -> Self {
        Self {
            local_address,
            sessions: [None; SESSIONS],
        }
    }

    /// Process a received frame; frames that are not TP.CM or TP.DT are ignored.
    pub fn on_frame(&mut self, frame: &CanFrame, now_ms: u32) -> RxResult {
        let mut result = RxResult::default();
        if frame.id.source_address() == self.local_address {
            return result;
        }
        match frame.id.pgn() {
            PGN_TP_CM => match TpCm::decode(frame.data()) {
                Ok(TpCm::Bam { size, packets, pgn }) => {
                    self.start_bam(frame.id, size, packets, pgn, now_ms);
                }
                Ok(TpCm::Rts {
                    size,
                    packets,
                    max_per_cts,
                    pgn,
                }) => {
                    result.reply =
                        self.start_rts(frame.id, size, packets, max_per_cts, pgn, now_ms);
                }
                Ok(TpCm::Abort { pgn, .. }) => self.on_sender_abort(frame.id, pgn),
                _ => {}
            },
            PGN_TP_DT => self.on_dt(frame, now_ms, &mut result),
            _ => {}
        }
        result
    }

    /// Drop sessions that have been silent for too long (`T1_MS` between DTs, `T2_MS` after
    /// a CTS). Returns an abort frame for RTS/CTS sessions; BAM sessions are dropped
    /// silently. At most one session is expired per call, so call it until it returns `None`
    /// if several may be due.
    pub fn poll_timeout(&mut self, now_ms: u32) -> Option<CanFrame> {
        for slot in &mut self.sessions {
            let Some(s) = *slot else { continue };
            if now_ms.wrapping_sub(s.last_activity_ms) < s.timeout_ms {
                continue;
            }
            *slot = None;
            if s.mode == RxMode::Rts {
                return self.abort_frame(s.source, s.msg.pgn, AbortReason::Timeout);
            }
        }
        None
    }

    /// The sender cancelled an RTS/CTS transfer: free the session without replying.
    /// Only an abort addressed to us, from the session's source and naming its PGN counts.
    /// BAM sessions have no abort in J1939-21, so they are left alone.
    fn on_sender_abort(&mut self, id: J1939Id, pgn: u32) {
        if id.destination() != Some(self.local_address) {
            return;
        }
        let source = id.source_address();
        for slot in &mut self.sessions {
            if slot.is_some_and(|s| s.mode == RxMode::Rts && s.source == source && s.msg.pgn == pgn)
            {
                *slot = None;
            }
        }
    }

    fn abort_frame(&self, to: u8, pgn: u32, reason: AbortReason) -> Option<CanFrame> {
        self.cm_frame(to, &TpCm::Abort { reason, pgn })
    }

    fn cm_frame(&self, to: u8, cm: &TpCm) -> Option<CanFrame> {
        let id = J1939Id::new(7, PGN_TP_CM, Some(to), self.local_address).ok()?;
        CanFrame::new(id, &cm.encode()).ok()
    }

    /// Handle an RTS. Returns the CTS or abort to send, or `None` if the RTS is ignored
    /// (not for us, or malformed: J1939-21 sizes below 9 belong in a single frame and a
    /// packet count that does not match the size cannot be answered meaningfully).
    fn start_rts(
        &mut self,
        id: J1939Id,
        size: u16,
        packets: u8,
        max_per_cts: u8,
        pgn: u32,
        now_ms: u32,
    ) -> Option<CanFrame> {
        if id.destination() != Some(self.local_address) {
            return None;
        }
        let source = id.source_address();
        let len = usize::from(size);
        if len < 9 {
            return None;
        }
        if self.find(source, RxMode::Rts).is_some() {
            return self.abort_frame(source, pgn, AbortReason::AlreadyInSession);
        }
        if len > J1939Message::MAX_LENGTH {
            return self.abort_frame(source, pgn, AbortReason::Resources);
        }
        if packets != packets_for(len) {
            return None;
        }
        let msg_id = J1939Id::new(id.priority(), pgn, Some(self.local_address), source).ok()?;
        let mut msg = J1939Message::from_id_and_data(msg_id, &[]).ok()?;
        msg.length = size;
        // PDU2 PGNs carry no destination in the id; the TP session knows it.
        msg.destination_address = self.local_address;
        let Some(slot) = self.sessions.iter_mut().find(|s| s.is_none()) else {
            return self.abort_frame(source, pgn, AbortReason::Resources);
        };
        let max_per_cts = if max_per_cts == 0 { 0xFF } else { max_per_cts };
        let block = packets.min(max_per_cts);
        *slot = Some(Session {
            mode: RxMode::Rts,
            source,
            size,
            packets,
            next_seq: 1,
            block_end: block,
            max_per_cts,
            last_activity_ms: now_ms,
            timeout_ms: T2_MS,
            msg,
        });
        self.cm_frame(
            source,
            &TpCm::Cts {
                packets: block,
                next: 1,
                pgn,
            },
        )
    }

    fn start_bam(&mut self, id: J1939Id, size: u16, packets: u8, pgn: u32, now_ms: u32) {
        let source = id.source_address();
        // BAM is broadcast only. Without an abort to send, bad announcements are ignored;
        // a valid announcement ends the source's previous BAM session (never its RTS/CTS one).
        let len = usize::from(size);
        if id.destination() != Some(0xFF)
            || !(9..=J1939Message::MAX_LENGTH).contains(&len)
            || packets != packets_for(len)
        {
            return;
        }
        let Ok(msg_id) = J1939Id::new(id.priority(), pgn, Some(0xFF), source) else {
            return;
        };
        let Ok(mut msg) = J1939Message::from_id_and_data(msg_id, &[]) else {
            return;
        };
        msg.length = size;
        if let Some(idx) = self.find(source, RxMode::Bam) {
            self.sessions[idx] = None;
        }
        if let Some(slot) = self.sessions.iter_mut().find(|s| s.is_none()) {
            *slot = Some(Session {
                mode: RxMode::Bam,
                source,
                size,
                packets,
                next_seq: 1,
                block_end: 0,
                max_per_cts: 0,
                last_activity_ms: now_ms,
                timeout_ms: T1_MS,
                msg,
            });
        }
    }

    fn on_dt(&mut self, frame: &CanFrame, now_ms: u32, result: &mut RxResult) {
        let source = frame.id.source_address();
        // BAM data is broadcast; connection-mode data is addressed to us.
        let mode = match frame.id.destination() {
            Some(0xFF) => RxMode::Bam,
            Some(da) if da == self.local_address => RxMode::Rts,
            _ => return,
        };
        let Some(idx) = self.find(source, mode) else {
            return;
        };
        let Some(mut s) = self.sessions[idx] else {
            return;
        };
        let seq = frame.data[0];
        let timed_out = now_ms.wrapping_sub(s.last_activity_ms) >= s.timeout_ms;
        if frame.len != 8 && s.mode == RxMode::Rts {
            return;
        }
        let failure = if timed_out {
            Some(AbortReason::Timeout)
        } else if frame.len != 8 {
            Some(AbortReason::BadSequence)
        } else if seq == s.next_seq {
            None
        } else if seq != 0 && seq < s.next_seq {
            Some(AbortReason::DuplicateSequence)
        } else {
            Some(AbortReason::BadSequence)
        };
        if let Some(reason) = failure {
            self.sessions[idx] = None;
            if s.mode == RxMode::Rts {
                result.reply = self.abort_frame(source, s.msg.pgn, reason);
            }
            return;
        }
        let start = (usize::from(seq) - 1) * 7;
        let end = (start + 7).min(usize::from(s.size));
        s.msg.data[start..end].copy_from_slice(&frame.data[1..1 + end - start]);
        s.last_activity_ms = now_ms;
        if seq == s.packets {
            self.sessions[idx] = None;
            result.message = Some(s.msg);
            if s.mode == RxMode::Rts {
                result.reply = self.cm_frame(
                    source,
                    &TpCm::EndOfMsgAck {
                        size: s.size,
                        packets: s.packets,
                        pgn: s.msg.pgn,
                    },
                );
            }
            return;
        }
        s.next_seq += 1;
        s.timeout_ms = T1_MS;
        if s.mode == RxMode::Rts && seq == s.block_end {
            let block = (s.packets - seq).min(s.max_per_cts);
            s.block_end = seq + block;
            s.timeout_ms = T2_MS;
            result.reply = self.cm_frame(
                source,
                &TpCm::Cts {
                    packets: block,
                    next: seq + 1,
                    pgn: s.msg.pgn,
                },
            );
        }
        self.sessions[idx] = Some(s);
    }

    /// Index of the session `source` is running in `mode`, if any.
    fn find(&self, source: u8, mode: RxMode) -> Option<usize> {
        self.sessions
            .iter()
            .position(|s| s.is_some_and(|s| s.source == source && s.mode == mode))
    }
}
