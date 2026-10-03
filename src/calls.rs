//! Call session model (calls lane; signaling slice).
//!
//! The wire protocol ([`CallSignal`]/[`CallSignalAck`], plus the transport
//! lane) lives in [`crate::behavior`]; this module holds the canonical
//! per-session state machine both frontends evolve. Signals only *carry* the
//! wire intent — a local session is updated here when a matching signal lands
//! (an inbound request, or the peer's answer to one of our own signals).
//!
//! Media transport (audio/video) is explicitly out of scope for this module
//! and lands on a later `calls`-gated slice; [`CallSignal::descriptor`] is the
//! reserved seam for its offer/answer payload.

use crate::behavior::CallSignal;

/// The stage a call session is in, as seen by the local side.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CallState {
    /// We sent an invite and are waiting for the callee to answer.
    Calling,
    /// The other side invited us; we have not answered yet.
    Ringing,
    /// Both sides agreed; the call is live (media transport lands later).
    InCall,
    /// Nothing live: the call ended or was declined, or it never started.
    #[default]
    Ended,
}

/// A single call session between this install and one peer.
///
/// One session at a time (mirrors the single-slot device-binding prompt). A
/// default value is `Ended` with empty ids — the "no active call" sentinel.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CallSession {
    /// End-to-end session id (minted by the caller; echoed by every signal).
    pub call_id: String,
    /// The peer this call is (or was) with.
    pub peer_id: String,
    /// The local side's view of the session.
    pub state: CallState,
}

impl CallSession {
    /// Start an outgoing call to `peer_id`, minting a fresh session id.
    #[must_use]
    pub fn outgoing(peer_id: &str) -> Self {
        Self {
            call_id: crate::gen_msg_id(),
            peer_id: peer_id.to_string(),
            state: CallState::Calling,
        }
    }

    /// Adopt an incoming invite from `peer_id` into a ringing session.
    #[must_use]
    pub fn incoming(call_id: &str, peer_id: &str) -> Self {
        Self {
            call_id: call_id.to_string(),
            peer_id: peer_id.to_string(),
            state: CallState::Ringing,
        }
    }

    /// Whether this session is live (ringing or talking).
    #[must_use]
    pub const fn is_active(&self) -> bool {
        !matches!(self.state, CallState::Ended)
    }

    /// Apply an inbound signal to this session.
    ///
    /// Only a signal addressed to *this* session's `call_id` mutates state; a
    /// stale or foreign `call_id` returns `false` and leaves state untouched.
    /// Incoming invites are adopted via [`Self::incoming`], never here.
    pub fn on_signal(&mut self, signal: &CallSignal) -> bool {
        if signal.call_id != self.call_id {
            return false;
        }
        match signal.kind.as_str() {
            "accept" => self.state = CallState::InCall,
            "reject" | "end" => self.state = CallState::Ended,
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
#[path = "../tests/unit/unit_calls.rs"]
mod tests;
