use super::timing::T1_MS;
use super::{CanFrame, PGN_TP_CM, PGN_TP_DT, TpCm, packets_for};
use crate::{J1939Id, J1939Message};

/// Outcome of feeding one frame to [`TpRx`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RxResult {
    /// A frame (CTS, ACK or abort) that must be sent in response.
    pub reply: Option<CanFrame>,
    /// A completely reassembled message.
    pub message: Option<J1939Message>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RxMode {
    Bam,
}

#[derive(Debug, Clone, Copy)]
struct Session {
    mode: RxMode,
    source: u8,
    size: u16,
    packets: u8,
    next_seq: u8,
    last_activity_ms: u32,
    msg: J1939Message,
}

/// Reassembles multi-packet messages, one session per source address.
#[derive(Debug, Clone, Copy)]
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
            PGN_TP_CM => {
                if let Ok(TpCm::Bam { size, packets, pgn }) = TpCm::decode(frame.data()) {
                    self.start_bam(frame.id, size, packets, pgn, now_ms);
                }
            }
            PGN_TP_DT => result.message = self.on_dt(frame, now_ms),
            _ => {}
        }
        result
    }

    /// Drop sessions with no activity for `T1_MS`. Returns an abort frame for modes that
    /// have a peer to tell; BAM sessions are dropped silently.
    pub fn poll_timeout(&mut self, now_ms: u32) -> Option<CanFrame> {
        for slot in &mut self.sessions {
            let Some(s) = *slot else { continue };
            if now_ms.wrapping_sub(s.last_activity_ms) < T1_MS {
                continue;
            }
            match s.mode {
                // No peer to notify: drop silently. RTS/CTS sessions will return an abort.
                RxMode::Bam => *slot = None,
            }
        }
        None
    }

    fn start_bam(&mut self, id: J1939Id, size: u16, packets: u8, pgn: u32, now_ms: u32) {
        let source = id.source_address();
        // BAM is broadcast only. Without an abort to send, bad announcements are ignored,
        // and a new announcement from a source always ends its previous session.
        self.remove(source);
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
        if let Some(slot) = self.sessions.iter_mut().find(|s| s.is_none()) {
            *slot = Some(Session {
                mode: RxMode::Bam,
                source,
                size,
                packets,
                next_seq: 1,
                last_activity_ms: now_ms,
                msg,
            });
        }
    }

    fn on_dt(&mut self, frame: &CanFrame, now_ms: u32) -> Option<J1939Message> {
        let source = frame.id.source_address();
        let idx = self
            .sessions
            .iter()
            .position(|s| s.is_some_and(|s| s.source == source))?;
        let s = self.sessions[idx].as_mut()?;
        if now_ms.wrapping_sub(s.last_activity_ms) >= T1_MS
            || frame.len != 8
            || frame.data[0] != s.next_seq
        {
            self.sessions[idx] = None;
            return None;
        }
        let start = (usize::from(s.next_seq) - 1) * 7;
        let end = (start + 7).min(usize::from(s.size));
        s.msg.data[start..end].copy_from_slice(&frame.data[1..1 + end - start]);
        s.last_activity_ms = now_ms;
        if s.next_seq == s.packets {
            let msg = s.msg;
            self.sessions[idx] = None;
            return Some(msg);
        }
        s.next_seq += 1;
        None
    }

    fn remove(&mut self, source: u8) {
        for slot in &mut self.sessions {
            if slot.is_some_and(|s| s.source == source) {
                *slot = None;
            }
        }
    }
}
