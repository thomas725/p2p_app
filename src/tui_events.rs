//! TUI event and command types with channel creation

use libp2p::PeerId;
use tokio::sync::mpsc;

/// Events sent from network/swarm to TUI
#[derive(Debug, Clone)]
pub enum TuiEvent {
    /// A system or status message with an optional peer ID
    Message(String, Option<String>),
    /// A broadcast message received: (content, peer_id, optional msg_id)
    Broadcast(String, String, Option<String>),
    /// A direct message received: (peer_id, content)
    Direct(String, String),
    /// A new peer has connected
    PeerConnected(PeerId),
    /// An existing peer has disconnected
    PeerDisconnected(PeerId),
    /// A local listen address has been established
    ListenAddr(String),
    /// Full peer list refresh: list of (peer_id, display_name, last_seen)
    UpdatePeers(Vec<(String, String, String)>),
    /// Open or focus a DM tab for the given peer ID
    AddDmTab(String),
    /// Close the DM tab for the given peer ID
    RemoveDmTab(String),
    /// Switch the active tab to the given index
    SetActiveTab(usize),
    /// Raw input string forwarded to the TUI
    Input(String),
    /// Signal the TUI to shut down
    Exit,
}

/// Commands issued from TUI to network
#[derive(Debug, Clone)]
pub enum InputCommand {
    /// Broadcast a message to all peers
    SendBroadcast(String),
    /// Send a direct message to a specific peer
    SendDm(String, PeerId),
    /// Update the local user's nickname
    SetNickname(String),
    /// Open a DM conversation with the given peer ID
    OpenDm(String),
    /// Close the DM conversation with the given peer ID
    CloseDm(String),
    /// Scroll the active view up
    ScrollUp,
    /// Scroll the active view down
    ScrollDown,
}

/// Sender half of the TUI event channel
pub type EventTx = mpsc::Sender<TuiEvent>;
/// Receiver half of the TUI event channel
pub type EventRx = mpsc::Receiver<TuiEvent>;
/// Sender half of the input command channel
pub type InputTx = mpsc::Sender<InputCommand>;
/// Receiver half of the input command channel
pub type InputRx = mpsc::Receiver<InputCommand>;

/// Create communication channels for TUI
pub fn create_channels() -> (EventTx, EventRx, InputTx, InputRx) {
    let (event_tx, event_rx) = mpsc::channel(100);
    let (input_tx, input_rx) = mpsc::channel(100);
    (event_tx, event_rx, input_tx, input_rx)
}

/// Handle for TUI spawned tasks
pub struct TuiThreads {
    /// Join handle for the event processing task
    pub event_handle: tokio::task::JoinHandle<()>,
    /// Join handle for the input processing task
    pub input_handle: tokio::task::JoinHandle<()>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_channels_returns_four_channels() {
        let (event_tx, event_rx, input_tx, input_rx) = create_channels();
        assert!(!event_tx.is_closed());
        assert!(!event_rx.is_closed());
        assert!(!input_tx.is_closed());
        assert!(!input_rx.is_closed());
    }

    #[test]
    fn test_tui_event_variants() {
        let msg = TuiEvent::Message("hello".to_string(), None);
        let broadcast = TuiEvent::Broadcast(
            "msg".to_string(),
            "peer1".to_string(),
            Some("id1".to_string()),
        );
        let direct = TuiEvent::Direct("peer2".to_string(), "dm".to_string());
        let exit = TuiEvent::Exit;

        match msg {
            TuiEvent::Message(_, _) => {}
            _ => panic!("expected Message"),
        }
        match broadcast {
            TuiEvent::Broadcast(_, _, _) => {}
            _ => panic!("expected Broadcast"),
        }
        match direct {
            TuiEvent::Direct(_, _) => {}
            _ => panic!("expected Direct"),
        }
        match exit {
            TuiEvent::Exit => {}
            _ => panic!("expected Exit"),
        }
    }

    #[test]
    fn test_input_command_variants() {
        let send_broadcast = InputCommand::SendBroadcast("hello".to_string());
        let send_dm = InputCommand::SendDm("hi".to_string(), libp2p::PeerId::random());
        let set_nickname = InputCommand::SetNickname("Alice".to_string());
        let open_dm = InputCommand::OpenDm("peer1".to_string());
        let scroll_up = InputCommand::ScrollUp;
        let scroll_down = InputCommand::ScrollDown;

        match send_broadcast {
            InputCommand::SendBroadcast(_) => {}
            _ => panic!(),
        }
        match send_dm {
            InputCommand::SendDm(_, _) => {}
            _ => panic!(),
        }
        match set_nickname {
            InputCommand::SetNickname(_) => {}
            _ => panic!(),
        }
        match open_dm {
            InputCommand::OpenDm(_) => {}
            _ => panic!(),
        }
        match scroll_up {
            InputCommand::ScrollUp => {}
            _ => panic!(),
        }
        match scroll_down {
            InputCommand::ScrollDown => {}
            _ => panic!(),
        }
    }
}
