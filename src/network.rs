use crate::get_average_peer_count;

/// Adaptive network size classification based on average peer count.
///
/// Used to configure gossipsub mesh parameters appropriately for network conditions.
/// Smaller networks use aggressive flooding; larger networks use lazy gossip.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkSize {
    /// 0-3 peers: Use flood_publish, aggressive heartbeat
    Small,
    /// 4-15 peers: Balanced mesh topology
    Medium,
    /// 16+ peers: Larger mesh, lazy gossip
    Large,
}

impl NetworkSize {
    /// Classify network size based on average peer count.
    ///
    /// # Arguments
    /// * `avg` - Average number of concurrent peers from historical data
    ///
    /// # Returns
    /// NetworkSize classification for configuring gossipsub behavior
    pub fn from_peer_count(avg: f64) -> Self {
        match avg as i32 {
            0..=3 => Self::Small,
            4..=15 => Self::Medium,
            _ => Self::Large,
        }
    }
}

pub fn get_network_size() -> color_eyre::Result<NetworkSize> {
    let avg = get_average_peer_count()?;
    Ok(NetworkSize::from_peer_count(avg))
}
