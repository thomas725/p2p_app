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
        | "getNetworkDetails"
        | "getPeerQuality"
        | "sortPeers"
        | "dialPeer") => node_state(m, params),
        m @ ("loadBroadcastMessages"
        | "loadDmMessages"
        | "deleteMessage"
        | "editMessage"
        | "sendTyping"
        | "toggleReaction"
        | "getReactions"
        | "validateEmoji"
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
        | "sendCallSignal"
        | "startCallMedia"
        | "stopCallMedia"
        | "sendCallMedia"
        | "setCallMediaMute") => contacts_calls(m, params),
        m @ ("listNetworkPeers" | "addNetworkPeer" | "removeNetworkPeer" | "setLanOnly") => {
            network_config(m, params)
        }
        "getLanOnly" => ok(api::get_lan_only()),
        "quickReactions" => ok(api::quick_reactions()),
        "getBroadcastUnreadCount" => ok(api::get_broadcast_unread_count()),
        "markBroadcastViewed" => json(api::mark_broadcast_viewed()),
        m @ ("isPeerBlocked" | "blockPeer" | "unblockPeer" | "listBlockedPeers") => {
            blocklist(m, params)
        }
        m @ ("createShare"
        | "listShares"
        | "getShare"
        | "bindShare"
        | "deleteShare"
        | "setShareVisibility"
        | "listShareGrants"
        | "setShareGrant"
        | "removeShareGrant"
        | "loadShareManifest"
        | "saveShareManifest"
        | "acceptShareOffer"
        | "snapshotShare"
        | "requestPeerShares"
        | "requestShareManifest"
        | "readvertiseShare"
        | "publishShareSnapshot") => shares(m, params),
        "getTheme" => ok(api::get_theme()),
        "getNotificationSettings" => ok(api::get_notification_settings()),
        m @ ("formatTimeHhmm"
        | "isAtBottom"
        | "parseLastSeenMs"
        | "replyDraft"
        | "setTheme"
        | "setNotificationSettings") => ui_helpers(m, params),
        other => Err(format!("unknown method: {other}")),
    }
}

// --- Domain sub-dispatchers ---

/// Node lifecycle, logs, and the peers table.
fn node_state(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "getMobilePeerStatus" => json(api::get_mobile_peer_status()),
        "startNode" => {
            // The shared Dart `NodeApi.startNode({required String dbPath})` —
            // and FRB's named argument — send an object, not a bare string.
            let p: StartNodeParams = parse(params)?;
            json(api::start_node(p.db_path))
        }
        "startNodeAuto" => json(api::start_node_auto()),
        "stopNode" => json(api::stop_node()),
        "getLogs" => ok(api::get_logs()),
        "clearLogs" => {
            api::clear_logs();
            ok(())
        }
        "getKnownPeers" => json(api::get_known_peers()),
        "getPeersWithStats" => json(mobile_api::get_peers_with_stats()),
        "getNetworkDetails" => ok(api::get_network_details()),
        "getPeerQuality" => {
            let p: PeerIdParams = parse(params)?;
            ok(api::get_peer_quality(&p.peer_id))
        }
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

/// Persisted network configuration (bootstrap + relay multiaddrs).
fn network_config(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "listNetworkPeers" => json(api::list_network_peers()),
        "addNetworkPeer" => {
            let p: NetworkPeerParams = parse(params)?;
            json(api::add_network_peer(&p.kind, &p.multiaddr))
        }
        "removeNetworkPeer" => {
            let p: IdParams = parse(params)?;
            json(api::remove_network_peer(p.id))
        }
        "setLanOnly" => {
            let p: LanOnlyParams = parse(params)?;
            json(api::set_lan_only(p.enabled))
        }
        other => Err(format!("method not routed to network_config: {other}")),
    }
}

/// Peer blocklist / mute (US-054): the swarm boundary drops blocked peers'
/// traffic; these methods only read and edit the persisted list.
fn blocklist(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "isPeerBlocked" => {
            let p: PeerIdParams = parse(params)?;
            ok(api::is_peer_blocked(&p.peer_id))
        }
        "blockPeer" => {
            let p: PeerIdParams = parse(params)?;
            json(api::block_peer(&p.peer_id))
        }
        "unblockPeer" => {
            let p: PeerIdParams = parse(params)?;
            json(api::unblock_peer(&p.peer_id))
        }
        "listBlockedPeers" => json(api::list_blocked_peers()),
        other => Err(format!("method not routed to blocklist: {other}")),
    }
}

/// Versioned file shares / folder sync. Local folder paths are a native-only
/// capability: the browser transport can list, grant and negotiate shares but
/// must never let a client pick a directory on the server.
fn shares(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "createShare" => {
            let p: WebCreateShareParams = parse(params)?;
            reject_web_share_root(p.local_root.as_deref())?;
            json(api::create_share(api::CreateShareParams {
                name: p.name,
                kind: p.kind,
                public_write: p.public_write,
                local_root: None,
            }))
        }
        "listShares" => json(api::list_shares()),
        "getShare" => {
            let p: ShareIdParams = parse(params)?;
            json(api::get_share(&p.share_id))
        }
        "bindShare" => {
            // A browser has no server-local folder to bind: a non-null root is
            // a path escape attempt and a null root is a meaningless no-op that
            // would silently clear a native binding. Validate the share id
            // first (so malformed params still report `invalid params:`) then
            // reject the operation outright.
            let _: ShareIdParams = parse(params)?;
            Err("server-local share roots are not supported over the web transport".into())
        }
        "deleteShare" => {
            let p: ShareIdParams = parse(params)?;
            json(api::delete_share(&p.share_id))
        }
        "setShareVisibility" => {
            let p: ShareVisibilityParams = parse(params)?;
            json(api::set_share_visibility(
                &p.share_id,
                &p.kind,
                p.public_write,
            ))
        }
        "listShareGrants" => {
            let p: ShareIdParams = parse(params)?;
            json(api::list_share_grants(&p.share_id))
        }
        "setShareGrant" => {
            let p: ShareGrantParams = parse(params)?;
            let mode = p
                .mode
                .ok_or_else(|| "invalid params: missing mode".to_string())?;
            json(api::set_share_grant(&p.share_id, &p.user_id, &mode))
        }
        "removeShareGrant" => {
            let p: ShareGrantParams = parse(params)?;
            json(api::remove_share_grant(&p.share_id, &p.user_id))
        }
        "loadShareManifest" => {
            let p: ShareIdParams = parse(params)?;
            let manifest = api::load_share_manifest(&p.share_id)?
                .as_ref()
                .map(crate::shares_json::ShareManifestJson::from);
            ok(manifest)
        }
        "saveShareManifest" => {
            // The raw store write bypasses ACLs and base_revision checks. Web
            // clients must use publishShareSnapshot, which goes through the
            // owner's swarm handler and rejects stale submissions. Parse first
            // so a malformed payload still reports `invalid params:`.
            let SaveShareManifestParams { share_id, manifest } = parse(params)?;
            let _ = (share_id, crate::shares::ShareManifest::from(manifest));
            Err("direct share manifest writes are not supported over the web transport; use publishShareSnapshot".into())
        }
        "acceptShareOffer" => {
            let p: AcceptShareOfferParams = parse(params)?;
            json(api::accept_share_offer(crate::shares::ShareOffer::from(
                p.offer,
            )))
        }
        "snapshotShare" => {
            let _root = parse::<SnapshotShareParams>(params)?.root;
            Err("server-local share roots are not supported over the web transport".into())
        }
        "requestPeerShares" => {
            let p: PeerIdParams = parse(params)?;
            json(api::request_peer_shares(&p.peer_id))
        }
        "requestShareManifest" => {
            let p: RequestShareManifestParams = parse(params)?;
            json(api::request_share_manifest(&p.peer_id, &p.share_id))
        }
        "readvertiseShare" => {
            let p: RequestShareManifestParams = parse(params)?;
            json(api::readvertise_share(&p.share_id, &p.peer_id))
        }
        "publishShareSnapshot" => {
            let p: PublishShareSnapshotParams = parse(params)?;
            json(api::publish_share_snapshot(
                &p.peer_id,
                p.base_revision,
                crate::shares::ShareManifest::from(p.manifest),
            ))
        }
        other => Err(format!("method not routed to shares: {other}")),
    }
}

fn reject_web_share_root(root: Option<&str>) -> Result<(), String> {
    if root.is_some() {
        return Err("server-local share roots are not supported over the web transport".into());
    }
    Ok(())
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
        "deleteMessage" => {
            let p: DeleteMessageParams = parse(params)?;
            json(api::delete_message(p.message_id, p.is_group))
        }
        "editMessage" => {
            let p: EditMessageParams = parse(params)?;
            json(api::edit_message(p.message_id, p.is_group, &p.content))
        }
        "sendTyping" => {
            let p: TypingParams = parse(params)?;
            json(api::send_typing(p.peer_id, p.is_typing))
        }
        "toggleReaction" => {
            let p: ReactionParams = parse(params)?;
            json(api::toggle_reaction(p.msg_id, p.emoji))
        }
        "getReactions" => {
            let p: MsgIdParams = parse(params)?;
            ok(api::get_reactions(&p.msg_id))
        }
        "validateEmoji" => {
            let p: EmojiParams = parse(params)?;
            ok(api::validate_emoji(&p.emoji))
        }
        "saveOutgoingBroadcast" => {
            let p: SendParams = parse(params)?;
            json(mobile_node::save_outgoing_broadcast_attached(
                p.content,
                p.attachment,
                p.reply_to,
            ))
        }
        "saveOutgoingDm" => {
            let p: SendDmParams = parse(params)?;
            json(mobile_node::save_outgoing_dm_attached(
                p.peer_id,
                p.content,
                p.attachment,
                p.reply_to,
            ))
        }
        "saveIncomingMessage" => {
            let p: SaveIncomingParams = parse(params)?;
            json(api::save_incoming_message(api::SaveIncomingMessageParams {
                content: p.content,
                peer_id: p.peer_id,
                is_direct: p.is_direct,
                nickname: p.nickname,
                latency: p.latency,
                msg_id: p.msg_id,
                reply_to: p.reply_to,
                attachment: p.attachment,
                encrypted: p.encrypted,
            }))
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
            json(mobile_node::save_outgoing_group_attached(
                p.group_id,
                p.content,
                p.attachment,
                p.reply_to,
            ))
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
        "startCallMedia" => {
            let p: StartCallMediaParams = parse(params)?;
            json(api::start_call_media(p.call_id, p.peer_id))
        }
        "stopCallMedia" => json(api::stop_call_media()),
        "sendCallMedia" => {
            let p: SendCallMediaParams = parse(params)?;
            json(api::send_call_media(p.call_id, p.pcm))
        }
        "setCallMediaMute" => {
            let p: CallMediaMuteParams = parse(params)?;
            json(api::set_call_media_mute(p.muted))
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
        "replyDraft" => {
            let p: ReplyDraftParams = parse(params)?;
            ok(mobile_api::reply_draft(
                p.msg_id,
                p.sender_nickname,
                p.peer_id,
                p.content,
                p.attachment,
            ))
        }
        "setTheme" => {
            let p: ThemeParams = parse(params)?;
            json(api::set_theme(p.dark, &p.accent_name))
        }
        "setNotificationSettings" => {
            let p: NotificationSettingsParams = parse(params)?;
            json(api::set_notification_settings(p.sound, p.desktop))
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

/// `startNode` takes a named argument (`dbPath`) on both FRB and Dart, so the
/// web arm must accept an object rather than a bare string.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartNodeParams {
    db_path: String,
}

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

/// Local-only delete of one sent message (`isGroup` selects the table).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeleteMessageParams {
    message_id: i32,
    is_group: bool,
}

/// Local-only edit of one sent message (`isGroup` selects the table).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditMessageParams {
    message_id: i32,
    is_group: bool,
    content: String,
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
struct TypingParams {
    peer_id: String,
    is_typing: bool,
}

/// Toggle a reaction on a message (US-008).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReactionParams {
    msg_id: String,
    emoji: String,
}

/// Fetch aggregated reactions for one message (US-008).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct MsgIdParams {
    msg_id: String,
}

/// Validate a reaction token from the Flutter "Custom…" dialog.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmojiParams {
    emoji: String,
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

/// Outgoing message with an optional attachment descriptor already stored via
/// `/api/upload`. Client-supplied server-local paths are deliberately not
/// accepted: the web surface never reads files off the server's filesystem.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendParams {
    content: String,
    #[serde(default)]
    attachment: Option<crate::files::MessageAttachment>,
    #[serde(default)]
    reply_to: Option<crate::ReplyRef>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendDmParams {
    peer_id: String,
    content: String,
    #[serde(default)]
    attachment: Option<crate::files::MessageAttachment>,
    #[serde(default)]
    reply_to: Option<crate::ReplyRef>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendGroupParams {
    group_id: String,
    content: String,
    #[serde(default)]
    attachment: Option<crate::files::MessageAttachment>,
    #[serde(default)]
    reply_to: Option<crate::ReplyRef>,
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
    /// Stable wire id of the incoming row, persisted for future replies.
    #[serde(default)]
    msg_id: Option<String>,
    /// Immutable reply snapshot carried by the live event (US-006).
    #[serde(default)]
    reply_to: Option<crate::ReplyRef>,
    /// Rust-built file descriptor from the live event (files lane §12). The
    /// browser only echoes the `MobileAttachment` it received; Rust converts it
    /// to the canonical store descriptor before persisting.
    #[serde(default)]
    attachment: Option<crate::mobile_node::MobileAttachment>,
    /// Whether the live DM arrived sealed end to end (US-042).
    #[serde(default)]
    encrypted: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DialParams {
    address: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NetworkPeerParams {
    kind: String,
    multiaddr: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdParams {
    id: i32,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct LanOnlyParams {
    enabled: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ThemeParams {
    dark: bool,
    accent_name: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct NotificationSettingsParams {
    sound: bool,
    desktop: bool,
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

/// Open the Rust-owned voice session for an accepted call (calls lane, US-044).
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartCallMediaParams {
    call_id: String,
    peer_id: String,
}

/// One 16 kHz mono PCM block from the local audio adapter.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SendCallMediaParams {
    call_id: String,
    pcm: Vec<i16>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CallMediaMuteParams {
    muted: bool,
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

/// The Flutter chat-row fields the canonical reply-draft rule reads.
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReplyDraftParams {
    msg_id: Option<String>,
    sender_nickname: Option<String>,
    peer_id: Option<String>,
    content: String,
    #[serde(default)]
    attachment: Option<crate::mobile_node::MobileAttachment>,
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

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShareIdParams {
    share_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebCreateShareParams {
    name: String,
    kind: String,
    public_write: bool,
    #[serde(default)]
    local_root: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShareVisibilityParams {
    share_id: String,
    kind: String,
    public_write: bool,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct ShareGrantParams {
    share_id: String,
    user_id: String,
    #[serde(default)]
    mode: Option<String>,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveShareManifestParams {
    share_id: String,
    manifest: crate::shares_json::ShareManifestJson,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct AcceptShareOfferParams {
    offer: crate::shares_json::ShareOfferJson,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotShareParams {
    root: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct RequestShareManifestParams {
    peer_id: String,
    share_id: String,
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublishShareSnapshotParams {
    peer_id: String,
    base_revision: i64,
    manifest: crate::shares_json::ShareManifestJson,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn start_node_params_accept_dart_object_shape() {
        // HttpNodeApi sends `{'dbPath': dbPath}`, matching FRB's named arg.
        let p: StartNodeParams = parse(json!({"dbPath": "/tmp/p2p-test.db"})).expect("object");
        assert_eq!(p.db_path, "/tmp/p2p-test.db");
    }

    #[test]
    fn start_node_params_reject_old_bare_string_shape() {
        // The previous web arm parsed a bare String, which is not what any
        // frontend sends — keep the mismatch documented so it cannot regress.
        assert!(parse::<StartNodeParams>(json!("/tmp/p2p-test.db")).is_err());
    }

    #[test]
    fn share_methods_are_routed_and_validate_params() {
        // Parsing happens before any DB/node work, so `Null` is a safe probe
        // that every with-params share method is actually reachable.
        for method in [
            "createShare",
            "getShare",
            "bindShare",
            "deleteShare",
            "setShareVisibility",
            "listShareGrants",
            "setShareGrant",
            "removeShareGrant",
            "loadShareManifest",
            "saveShareManifest",
            "acceptShareOffer",
            "snapshotShare",
            "requestPeerShares",
            "requestShareManifest",
            "readvertiseShare",
            "publishShareSnapshot",
        ] {
            let err = dispatch(method, Value::Null).expect_err("missing params must fail");
            assert!(
                err.starts_with("invalid params:"),
                "{method}: unexpected error: {err}"
            );
        }
    }

    #[test]
    fn web_share_roots_are_rejected_before_any_db_access() {
        let err = dispatch(
            "createShare",
            json!({"name": "Docs", "kind": "public", "publicWrite": false, "localRoot": "/srv/private"}),
        )
        .expect_err("a client must not pick a server directory");
        assert!(err.contains("server-local share roots"), "{err}");

        let err = dispatch(
            "bindShare",
            json!({"shareId": "s", "localRoot": "/srv/private"}),
        )
        .expect_err("binding a server directory must fail");
        assert!(err.contains("server-local share roots"), "{err}");

        // A null root is equally meaningless over the web: there is no local
        // folder to bind, and accepting it would clear a native binding.
        let err = dispatch("bindShare", json!({"shareId": "s", "localRoot": null}))
            .expect_err("a null share root must not clear a native binding");
        assert!(err.contains("server-local share roots"), "{err}");

        let err = dispatch(
            "snapshotShare",
            json!({"shareId": "s", "root": "/srv/private"}),
        )
        .expect_err("snapshotting a server directory must fail");
        assert!(err.contains("server-local share roots"), "{err}");
    }

    #[test]
    fn web_manifest_writes_must_go_through_the_revision_checked_lane() {
        // The browser sends camelCase (see `shares_json.dart`). The DTO must
        // accept that shape with real values, then the arm rejects the raw
        // write regardless — parsing proves the boundary contract still works.
        let manifest = json!({
            "shareId": "s-42",
            "name": "Docs",
            "revision": 3,
            "entries": [{
                "path": "docs/readme.md",
                "fileId": "abc123",
                "size": 12,
                "mime": "text/markdown"
            }]
        });
        let parsed: crate::shares_json::ShareManifestJson =
            parse(json!({"shareId": "s-42", "manifest": manifest})["manifest"].clone())
                .expect("camelCase manifest parses at the web boundary");
        let domain = crate::shares::ShareManifest::from(parsed);
        assert_eq!(domain.share_id, "s-42");
        assert_eq!(domain.revision, 3);
        assert_eq!(domain.entries[0].file_id, "abc123");
        assert_eq!(domain.entries[0].size, 12);

        let err = dispatch(
            "saveShareManifest",
            json!({"shareId": "s-42", "manifest": manifest}),
        )
        .expect_err("the raw store write bypasses ACLs and revisions");
        assert!(err.contains("publishShareSnapshot"), "{err}");
    }

    #[test]
    fn validate_emoji_uses_the_canonical_reaction_rule() {
        // Same rule as the TUI reaction editor and `toggle_reaction` on write.
        assert_eq!(
            dispatch("validateEmoji", json!({"emoji": "👍"})),
            Ok(json!(true))
        );
        assert_eq!(
            dispatch("validateEmoji", json!({"emoji": "  🎉  "})),
            Ok(json!(true))
        );
        assert_eq!(
            dispatch("validateEmoji", json!({"emoji": ""})),
            Ok(json!(false))
        );
        assert_eq!(
            dispatch("validateEmoji", json!({"emoji": "x".repeat(33)})),
            Ok(json!(false))
        );
    }
}
