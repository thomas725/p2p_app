//! Shared tracking of currently connected peers.
//!
//! Both the TUI (`AppState`) and the Flutter backend (`mobile_node`) drive the
//! same [`ConnectedTracker`] from the `PeerConnected`/`PeerDisconnected` swarm
//! events, so every frontend reports the same set of peers that are *currently*
//! connected (plus the most recent disconnection) instead of inventing its own
//! bookkeeping.

use std::collections::HashSet;

/// Tracks which peers are currently connected and, on disconnect, remembers the
/// most recent disconnection so frontends can show "last connected to <peer>".
///
/// A single instance must only be mutated from the task that processes swarm
/// events (the TUI's command processor, or `mobile_node`'s event poller).
#[derive(Debug, Default, Clone)]
pub struct ConnectedTracker {
    connected: HashSet<String>,
    last_peer: Option<String>,
    last_time: Option<f64>,
}

impl ConnectedTracker {
    /// Create an empty tracker.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that `peer_id` just connected.
    pub fn on_peer_connected(&mut self, peer_id: String) {
        self.connected.insert(peer_id);
    }

    /// Record that `peer_id` just disconnected at `at` (epoch seconds).
    pub fn on_peer_disconnected(&mut self, peer_id: &str, at: f64) {
        self.connected.remove(peer_id);
        self.last_peer = Some(peer_id.to_string());
        self.last_time = Some(at);
    }

    /// Number of peers currently connected.
    #[must_use]
    pub fn len(&self) -> usize {
        self.connected.len()
    }

    /// Whether no peer is currently connected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.connected.is_empty()
    }

    /// Whether `peer_id` is currently connected.
    #[must_use]
    pub fn contains(&self, peer_id: &str) -> bool {
        self.connected.contains(peer_id)
    }

    /// Iterate over the IDs of the currently connected peers (`HashSet` order).
    pub fn connected_peer_ids(&self) -> impl Iterator<Item = &str> {
        self.connected.iter().map(String::as_str)
    }

    /// The most recent disconnection as `(peer_id, epoch_seconds)`, if any.
    #[must_use]
    pub const fn last_disconnection(&self) -> Option<(&str, f64)> {
        match (&self.last_peer, self.last_time) {
            (Some(peer), Some(at)) => Some((peer.as_str(), at)),
            _ => None,
        }
    }

    /// Forget every connected peer and the last-disconnection record.
    pub fn clear(&mut self) {
        self.connected.clear();
        self.last_peer = None;
        self.last_time = None;
    }
}

#[cfg(test)]
#[path = "../tests/unit/unit_connected.rs"]
mod tests;