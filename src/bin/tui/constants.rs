//! TUI-specific constants

/// Channel capacity for inter-task communication (events per task)
pub const CHANNEL_CAPACITY: usize = 100;

/// Maximum messages to keep in memory (older messages are dropped)
pub const MAX_MESSAGE_HISTORY: usize = 1000;

/// Maximum direct messages to keep per peer conversation
pub const MAX_DM_HISTORY: usize = 1000;

/// Maximum peers to track concurrently
/// Beyond this, oldest peers are likely pruned by network layer anyway
pub const MAX_PEERS: usize = 10_000;

/// Frame time in milliseconds for 60 FPS rendering
pub const FRAME_TIME_MS: u64 = 16;

/// Maximum logs to keep in memory before dropping oldest
pub const MAX_LOGS: usize = 2000;

/// Gossipsub maximum transmit size in bytes
pub const GOSSIPSUB_MAX_TRANSMIT: u32 = 262144;
