//! JSON method dispatch for the web frontend (`docs/web_frontend.md`).
//!
//! One arm per FRB-exposed function; method names mirror the Dart-side FRB
//! names so `HttpNodeApi` is a mechanical mapping. Every arm runs on a
//! `spawn_blocking` worker (see `super::answer_frame`), so the sync DB calls
//! behave exactly like they do under FRB's thread pool.
//!
//! [`dispatch`] routes by method name to a domain sub-dispatcher; a method
//! reaching a domain that doesn't own it is a routing bug and yields an
//! explicit error rather than a panic (panics are denied crate-wide).

use crate::api;
use crate::mobile_api;
use crate::mobile_node;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Run one method. `params` is the frame's `params` payload (`Null` when
/// absent).
///
/// # Errors
/// Returns the method's error string (mirrors the `Result<_, String>` shape
/// of the FRB surface), a parameter-parse error, or "unknown method".
pub fn dispatch(method: &str, params: Value) -> Result<Value, String> {
    match method {
        m @ ("getMobilePeerStatus"
        | "startNode"
        | "startNodeAuto"
        | "stopNode"
        | "getLogs"
        | "clearLogs"
        | "getKnownPeers"
        | "getPeersWithStats"
        | "sortPeers"
        | "dialPeer") => node_state(m, params),
        m @ ("loadBroadcastMessages"
        | "loadDmMessages"
        | "saveOutgoingBroadcast"
        | "saveOutgoingDm"
        | "saveIncomingMessage") => messages(m, params),
        m @ ("createGroup" | "listGroups" | "loadGroupMessages" | "saveOutgoingGroup"
        | "acceptGroupInvite" | "deleteGroup") => groups(m, params),
        m @ ("setSelfNickname"
        | "setPeerLocalNickname"
        | "validateNickname"
        | "networkSizeLabel"
        | "getMyUserId"
        | "provisionMyUser"
        | "sendDeviceBindingIntro"
        | "acceptDeviceBinding"
        | "verifyDeviceBindingClaim"
        | "addDevice"
        | "getUserDevices") => identity(m, params),
        m @ ("getContacts"
        | "addContact"
        | "removeContact"
        | "getContactsWithStats"
        | "sortContacts"
        | "startCall"
        | "sendCallSignal") => contacts_calls(m, params),
        m @ ("formatTimeHhmm" | "isAtBottom" | "parseLastSeenMs") => ui_helpers(m, params),
        other => Err(format!("unknown method: {other}")),
    }
}

// --- Domain sub-dispatchers ---

/// Node lifecycle, logs, and the peers table.
fn node_state(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "getMobilePeerStatus" => json(api::get_mobile_peer_status()),
        "startNode" => json(api::start_node(parse(params)?)),
        "startNodeAuto" => json(api::start_node_auto()),
        "stopNode" => json(api::stop_node()),
        "getLogs" => ok(api::get_logs()),
        "clearLogs" => {
            api::clear_logs();
            ok(())
        }
        "getKnownPeers" => json(api::get_known_peers()),
        "getPeersWithStats" => json(mobile_api::get_peers_with_stats()),
        "sortPeers" => {
            let p: SortPeersParams = parse(params)?;
            ok(mobile_api::sort_peers(p.rows, p.sort_column, p.ascending))
        }
        "dialPeer" => {
            let p: DialParams = parse(params)?;
            ok(api::dial_peer(p.address))
        }
        other => Err(format!("method not routed to node_state: {other}")),
    }
}

/// Broadcast + DM message history and sends.
fn messages(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "loadBroadcastMessages" => {
            let p: LimitParams = parse(params)?;
            json(api::load_broadcast_messages(p.limit))
        }
        "loadDmMessages" => {
            let p: PeerLimitParams = parse(params)?;
            json(api::load_dm_messages(p.peer_id, p.limit))
        }
        "saveOutgoingBroadcast" => {
            let p: SendParams = parse(params)?;
            json(match (p.attachment_path, p.attachment) {
                (_, Some(att)) => {
                    mobile_node::save_outgoing_broadcast_attached(p.content, Some(att))
                }
                (path, None) => api::save_outgoing_broadcast(p.content, path),
            })
        }
        "saveOutgoingDm" => {
            let p: SendDmParams = parse(params)?;
            json(match (p.attachment_path, p.attachment) {
                (_, Some(att)) => {
                    mobile_node::save_outgoing_dm_attached(p.peer_id, p.content, Some(att))
                }
                (path, None) => api::save_outgoing_dm(p.peer_id, p.content, path),
            })
        }
        "saveIncomingMessage" => {
            let p: SaveIncomingParams = parse(params)?;
            json(api::save_incoming_message(
                p.content,
                p.peer_id,
                p.is_direct,
                p.nickname,
                p.latency,
            ))
        }
        other => Err(format!("method not routed to messages: {other}")),
    }
}

/// Group lifecycle and group chat.
fn groups(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "createGroup" => {
            let p: NameParams = parse(params)?;
            json(api::create_group(&p.name))
        }
        "listGroups" => json(api::list_groups()),
        "loadGroupMessages" => {
            let p: GroupLimitParams = parse(params)?;
            json(api::load_group_messages(p.group_id, p.limit))
        }
        "saveOutgoingGroup" => {
            let p: SendGroupParams = parse(params)?;
            json(match (p.attachment_path, p.attachment) {
                (_, Some(att)) => {
                    mobile_node::save_outgoing_group_attached(p.group_id, p.content, Some(att))
                }
                (path, None) => api::save_outgoing_group(p.group_id, p.content, path),
            })
        }
        "acceptGroupInvite" => {
            let p: GroupIdParams = parse(params)?;
            json(api::accept_group_invite(p.group_id))
        }
        "deleteGroup" => {
            let p: GroupIdParams = parse(params)?;
            json(api::delete_group(p.group_id))
        }
        other => Err(format!("method not routed to groups: {other}")),
    }
}

/// Nicknames, user identity, and the device-binding exchange (lane §11).
fn identity(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "setSelfNickname" => {
            let p: NameParams = parse(params)?;
            json(api::set_self_nickname(&p.name))
        }
        "setPeerLocalNickname" => {
            let p: PeerNickParams = parse(params)?;
            json(api::set_peer_local_nickname(&p.peer_id, &p.nickname))
        }
        "validateNickname" => {
            let p: NameParams = parse(params)?;
            ok(api::validate_nickname(&p.name))
        }
        "networkSizeLabel" => {
            let p: PeerCountParams = parse(params)?;
            ok(api::network_size_label(p.peer_count))
        }
        "getMyUserId" => json(api::get_my_user_id()),
        "provisionMyUser" => {
            let p: ProvisionParams = parse(params)?;
            json(api::provision_my_user(p.self_username, p.self_nickname))
        }
        "sendDeviceBindingIntro" => {
            let p: PeerIdParams = parse(params)?;
            json(api::send_device_binding_intro(p.peer_id))
        }
        "acceptDeviceBinding" => {
            let p: DeviceBindingParams = parse(params)?;
            json(api::accept_device_binding(p.peer_id, p.device_pubkey))
        }
        "verifyDeviceBindingClaim" => {
            let p: VerifyClaimParams = parse(params)?;
            json(api::verify_device_binding_claim(
                p.user_id,
                p.device_pubkey,
                p.claim_id,
                p.claim,
            ))
        }
        "addDevice" => {
            let p: AddDeviceParams = parse(params)?;
            json(api::add_device(
                p.user_id,
                p.peer_id,
                p.device_pubkey,
                p.claim_id,
                p.claim,
            ))
        }
        "getUserDevices" => {
            let p: UserIdParams = parse(params)?;
            json(api::get_user_devices(p.user_id))
        }
        other => Err(format!("method not routed to identity: {other}")),
    }
}

/// Contacts roster, contacts sorting, and call signaling.
fn contacts_calls(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "getContacts" => json(api::get_contacts()),
        "addContact" => {
            let p: AddContactParams = parse(params)?;
            json(api::add_contact(p.contact_user_id, p.nickname_override))
        }
        "removeContact" => {
            let p: ContactIdParams = parse(params)?;
            json(api::remove_contact(p.contact_user_id))
        }
        "getContactsWithStats" => json(api::get_contacts_with_stats()),
        "sortContacts" => {
            let p: SortContactsParams = parse(params)?;
            ok(mobile_api::sort_contacts(
                p.contacts,
                p.sort_column,
                p.ascending,
            ))
        }
        "startCall" => {
            let p: PeerIdParams = parse(params)?;
            json(api::start_call(p.peer_id))
        }
        "sendCallSignal" => {
            let p: CallSignalParams = parse(params)?;
            json(api::send_call_signal(p.peer_id, p.call_id, p.kind))
        }
        other => Err(format!("method not routed to contacts_calls: {other}")),
    }
}

/// Pure UI helpers — Rust is the single source of these rules (parity with
/// the TUI and the in-process FRB callers).
fn ui_helpers(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "formatTimeHhmm" => {
            let p: FormatTimeParams = parse(params)?;
            ok(mobile_api::format_time_hhmm(&p.dt))
        }
        "isAtBottom" => {
            let p: IsAtBottomParams = parse(params)?;
            ok(mobile_api::is_at_bottom(
                p.scroll_offset,
                p.total,
                p.visible,
            ))
        }
        "parseLastSeenMs" => {
            let p: ParseLastSeenParams = parse(params)?;
            ok(mobile_api::parse_last_seen_ms(&p.last_seen))
        }
        other => Err(format!("method not routed to ui_helpers: {other}")),
    }
}

// --- Serialization + param helpers ---

/// Wrap a fallible FRB-surface result.
fn json<T: Serialize>(result: Result<T, String>) -> Result<Value, String> {
    serde_json::to_value(result?).map_err(|e| e.to_string())
}

/// Wrap an infallible value (pure helpers, void results).
fn ok<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|e| e.to_string())
}

fn parse<T: DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value(params).map_err(|e| format!("invalid params: {e}"))
}

// --- Parameter shapes (camelCase on the wire) ---

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LimitParams {
    limit: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PeerLimitParams {
    peer_id: String,
    limit: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupLimitParams {
    group_id: String,
    limit: i64,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PeerIdParams {
    peer_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PeerNickParams {
    peer_id: String,
    nickname: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GroupIdParams {
    group_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct UserIdParams {
    user_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContactIdParams {
    contact_user_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NameParams {
    name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PeerCountParams {
    peer_count: i64,
}

/// Outgoing message with an optional attachment: either a server-local path
/// (mobile-style) or a descriptor already stored via `/api/upload`.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendParams {
    content: String,
    #[serde(default)]
    attachment_path: Option<String>,
    #[serde(default)]
    attachment: Option<crate::files::MessageAttachment>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendDmParams {
    peer_id: String,
    content: String,
    #[serde(default)]
    attachment_path: Option<String>,
    #[serde(default)]
    attachment: Option<crate::files::MessageAttachment>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendGroupParams {
    group_id: String,
    content: String,
    #[serde(default)]
    attachment_path: Option<String>,
    #[serde(default)]
    attachment: Option<crate::files::MessageAttachment>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveIncomingParams {
    content: String,
    peer_id: String,
    is_direct: bool,
    nickname: Option<String>,
    /// Live round-trip latency of the event (decorates the reply only).
    #[serde(default)]
    latency: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DialParams {
    address: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProvisionParams {
    self_username: Option<String>,
    self_nickname: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceBindingParams {
    peer_id: String,
    device_pubkey: Vec<u8>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct VerifyClaimParams {
    user_id: String,
    device_pubkey: Vec<u8>,
    claim_id: String,
    claim: Vec<u8>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddDeviceParams {
    user_id: String,
    peer_id: String,
    device_pubkey: Vec<u8>,
    claim_id: String,
    claim: Vec<u8>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AddContactParams {
    contact_user_id: String,
    nickname_override: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CallSignalParams {
    peer_id: String,
    call_id: String,
    kind: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormatTimeParams {
    dt: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IsAtBottomParams {
    scroll_offset: usize,
    total: usize,
    visible: usize,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ParseLastSeenParams {
    last_seen: String,
}

/// Sort request for the peers table.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SortPeersParams {
    rows: Vec<mobile_api::PeerSortInput>,
    sort_column: u32,
    ascending: bool,
}

/// Sort request for the contacts table.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SortContactsParams {
    contacts: Vec<mobile_api::ContactSortInput>,
    sort_column: u32,
    ascending: bool,
}
