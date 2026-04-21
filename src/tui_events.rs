//! TUI event and command types with channel creation

use libp2p::PeerId;
use tokio::sync::mpsc;

/// Events sent from network/swarm to TUI
#[derive(Debug, Clone)]
pub enum TuiEvent {
    Message(String, Option<String>),
    Broadcast(String, String, Option<String>),
    Direct(String, String),
    PeerConnected(PeerId),
    PeerDisconnected(PeerId),
    ListenAddr(String),
    UpdatePeers(Vec<(String, String, String)>),
    AddDmTab(String),
    RemoveDmTab(String),
    SetActiveTab(usize),
    Input(String),
    Exit,
}

/// Commands issued from TUI to network
#[derive(Debug, Clone)]
pub enum InputCommand {
    SendBroadcast(String),
    SendDm(String, PeerId),
    SetNickname(String),
    OpenDm(String),
    CloseDm(String),
    ScrollUp,
    ScrollDown,
}

pub type EventTx = mpsc::Sender<TuiEvent>;
pub type EventRx = mpsc::Receiver<TuiEvent>;
pub type InputTx = mpsc::Sender<InputCommand>;
pub type InputRx = mpsc::Receiver<InputCommand>;

/// Create communication channels for TUI
pub fn create_channels() -> (EventTx, EventRx, InputTx, InputRx) {
    let (event_tx, event_rx) = mpsc::channel(100);
    let (input_tx, input_rx) = mpsc::channel(100);
    (event_tx, event_rx, input_tx, input_rx)
}

/// Handle for TUI spawned tasks
pub struct TuiThreads {
    pub event_handle: tokio::task::JoinHandle<()>,
    pub input_handle: tokio::task::JoinHandle<()>,
}
