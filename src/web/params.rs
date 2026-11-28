//! JSON parameter shapes for the web frontend (`docs/web_frontend.md`).
//!
//! One struct per FRB-exposed function that takes arguments, mirroring Dart's
//! named arguments in camelCase on the wire. Kept apart from the dispatch arms
//! in [`super::methods`] so the router stays about routing; that module reports
//! `invalid params:` when a frame does not match its method's shape.

use crate::mobile_api;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Parse a frame's `params` payload into a specific method's argument shape.
pub fn parse<T: DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|e| format!("invalid params: {e}"))
}

// --- Parameter shapes (camelCase on the wire) ---

/// `startNode` takes a named argument (`dbPath`) on both FRB and Dart, so the
/// web arm must accept an object rather than a bare string.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartNodeParams {
    pub db_path: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LimitParams {
    pub limit: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerLimitParams {
    pub peer_id: String,
    pub limit: i64,
}

/// Local-only delete of one sent message (`isGroup` selects the table).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteMessageParams {
    pub message_id: i32,
    pub is_group: bool,
}

/// Local-only edit of one sent message (`isGroup` selects the table).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EditMessageParams {
    pub message_id: i32,
    pub is_group: bool,
    pub content: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupLimitParams {
    pub group_id: String,
    pub limit: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerIdParams {
    pub peer_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TypingParams {
    pub peer_id: String,
    pub is_typing: bool,
}

/// Toggle a reaction on a message (US-008).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReactionParams {
    pub msg_id: String,
    pub emoji: String,
}

/// Fetch aggregated reactions for one message (US-008).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MsgIdParams {
    pub msg_id: String,
}

/// Validate a reaction token from the Flutter "Custom…" dialog.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmojiParams {
    pub emoji: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerNickParams {
    pub peer_id: String,
    pub nickname: String,
}

/// Owner-side contact nickname set: the caller supplies only the contact and
/// the optional override; the owner is derived in Rust.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContactNicknameParams {
    pub contact_user_id: String,
    pub nickname: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupIdParams {
    pub group_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserIdParams {
    pub user_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NameParams {
    pub name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerCountParams {
    pub peer_count: i64,
}

/// Outgoing message with an optional attachment descriptor already stored via
/// `/api/upload`. Client-supplied server-local paths are deliberately not
/// accepted: the web surface never reads files off the server's filesystem.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendParams {
    pub content: String,
    #[serde(default)]
    pub attachment: Option<crate::files::MessageAttachment>,
    #[serde(default)]
    pub reply_to: Option<crate::ReplyRef>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendDmParams {
    pub peer_id: String,
    pub content: String,
    #[serde(default)]
    pub attachment: Option<crate::files::MessageAttachment>,
    #[serde(default)]
    pub reply_to: Option<crate::ReplyRef>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendGroupParams {
    pub group_id: String,
    pub content: String,
    #[serde(default)]
    pub attachment: Option<crate::files::MessageAttachment>,
    #[serde(default)]
    pub reply_to: Option<crate::ReplyRef>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveIncomingParams {
    pub content: String,
    pub peer_id: String,
    pub is_direct: bool,
    pub nickname: Option<String>,
    /// Live round-trip latency of the event (decorates the reply only).
    #[serde(default)]
    pub latency: Option<String>,
    /// Stable wire id of the incoming row, persisted for future replies.
    #[serde(default)]
    pub msg_id: Option<String>,
    /// Immutable reply snapshot carried by the live event (US-006).
    #[serde(default)]
    pub reply_to: Option<crate::ReplyRef>,
    /// Rust-built file descriptor from the live event (files lane §12). The
    /// browser only echoes the `MobileAttachment` it received; Rust converts it
    /// to the canonical store descriptor before persisting.
    #[serde(default)]
    pub attachment: Option<crate::mobile_node::MobileAttachment>,
    /// Whether the live DM arrived sealed end to end (US-042).
    #[serde(default)]
    pub encrypted: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DialParams {
    pub address: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPeerParams {
    pub kind: String,
    pub multiaddr: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdParams {
    pub id: i32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LanOnlyParams {
    pub enabled: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThemeParams {
    pub dark: bool,
    pub accent_name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSettingsParams {
    pub sound: bool,
    pub desktop: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationMapUrlParams {
    pub latitude: f64,
    pub longitude: f64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IsLocationMessageParams {
    pub content: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationSessionParams {
    pub started_at: f64,
    pub mode: String,
    pub duration_secs: Option<u32>,
    pub now: f64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationSendDueParams {
    pub last_sent_at: Option<f64>,
    pub now: f64,
    pub interval_secs: u32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationQueryDueParams {
    pub last_query_at: Option<f64>,
    pub now: f64,
    pub interval_secs: u32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationTimeFilterParams {
    pub text: String,
    pub now: f64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadLocationTrackParams {
    pub peer_id: String,
    #[serde(default)]
    pub from_text: Option<String>,
    #[serde(default)]
    pub to_text: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationGpxParams {
    pub points: Vec<crate::location::LocationTrackPoint>,
    pub device_label: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LocationTrackUrlParams {
    pub points: Vec<crate::location::LocationTrackPoint>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendLocationParams {
    pub selected_peer_ids: Vec<String>,
    pub all_devices: bool,
    pub body: String,
}

/// Web JSON mirror of the flat FRB location-share DTO (camelCase on the wire).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildLocationMessageParams {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
    pub mode: String,
    pub duration_secs: Option<u32>,
    pub sensor_interval_secs: u32,
    pub send_interval_secs: u32,
    pub motion_gate: bool,
    pub session_id: String,
    pub sequence: u32,
    pub is_final: bool,
}

impl From<BuildLocationMessageParams> for crate::location::LocationShareInput {
    fn from(p: BuildLocationMessageParams) -> Self {
        Self {
            latitude: p.latitude,
            longitude: p.longitude,
            accuracy: p.accuracy,
            mode: p.mode,
            duration_secs: p.duration_secs,
            sensor_interval_secs: p.sensor_interval_secs,
            send_interval_secs: p.send_interval_secs,
            motion_gate: p.motion_gate,
            session_id: p.session_id,
            sequence: p.sequence,
            is_final: p.is_final,
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceBindingParams {
    pub peer_id: String,
    pub device_pubkey: Vec<u8>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallSignalParams {
    pub peer_id: String,
    pub call_id: String,
    pub kind: String,
}

/// Open the Rust-owned voice session for an accepted call (calls lane, US-044).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartCallMediaParams {
    pub call_id: String,
    pub peer_id: String,
}

/// One 16 kHz mono PCM block from the local audio adapter.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendCallMediaParams {
    pub call_id: String,
    pub pcm: Vec<i16>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallMediaMuteParams {
    pub muted: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatTimeParams {
    pub dt: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IsAtBottomParams {
    pub scroll_offset: usize,
    pub total: usize,
    pub visible: usize,
}

/// The Flutter chat-row fields the canonical reply-draft rule reads.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplyDraftParams {
    pub msg_id: Option<String>,
    pub sender_nickname: Option<String>,
    pub peer_id: Option<String>,
    pub content: String,
    #[serde(default)]
    pub attachment: Option<crate::mobile_node::MobileAttachment>,
}

/// Sort request for the contacts table.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SortContactsParams {
    pub contacts: Vec<mobile_api::ContactSortInput>,
    pub sort_column: u32,
    pub ascending: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareIdParams {
    pub share_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebCreateShareParams {
    pub name: String,
    pub kind: String,
    pub public_write: bool,
    #[serde(default)]
    pub local_root: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareVisibilityParams {
    pub share_id: String,
    pub kind: String,
    pub public_write: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareGrantParams {
    pub share_id: String,
    pub user_id: String,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebReviseShareManifestForDeletionParams {
    pub share_id: String,
    pub removed_paths: Vec<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveShareManifestParams {
    pub share_id: String,
    pub manifest: crate::shares_json::ShareManifestJson,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptShareOfferParams {
    pub offer: crate::shares_json::ShareOfferJson,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotShareParams {
    pub root: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestShareManifestParams {
    pub peer_id: String,
    pub share_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishShareSnapshotParams {
    pub peer_id: String,
    pub base_revision: i64,
    pub manifest: crate::shares_json::ShareManifestJson,
}
