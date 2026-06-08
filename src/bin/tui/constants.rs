//! TUI-specific constants

/// Channel capacity for inter-task communication (events per task)
pub const CHANNEL_CAPACITY: usize = 100;

/// Maximum messages to keep in memory (older messages are dropped)
pub const MAX_MESSAGE_HISTORY: usize = 1000;

/// Maximum direct messages to keep per peer conversation
pub const MAX_DM_HISTORY: usize = 1000;

/// Input poll interval in milliseconds
pub const FRAME_TIME_MS: u64 = 16;

/// Trim a VecDeque to a maximum length, removing oldest (front) items.
pub fn trim_history<T>(queue: &mut std::collections::VecDeque<T>, limit: usize) {
    while queue.len() > limit {
        queue.pop_front();
    }
}
