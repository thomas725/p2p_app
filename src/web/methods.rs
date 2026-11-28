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
use serde_json::Value;

use super::params::{
    AcceptShareOfferParams, BuildLocationMessageParams, CallMediaMuteParams, CallSignalParams,
    ContactNicknameParams, DeleteMessageParams, DeviceBindingParams, DialParams, EditMessageParams,
    EmojiParams, FormatTimeParams, GroupIdParams, GroupLimitParams, IdParams, IsAtBottomParams,
    IsLocationMessageParams, LanOnlyParams, LimitParams, LoadLocationTrackParams,
    LocationGpxParams, LocationMapUrlParams, LocationQueryDueParams, LocationSendDueParams,
    LocationSessionParams, LocationTimeFilterParams, LocationTrackUrlParams, MsgIdParams,
    NameParams, NetworkPeerParams, NotificationSettingsParams, ParseLastSeenParams,
    PeerCountParams, PeerIdParams, PeerLimitParams, PeerNickParams, PublishShareSnapshotParams,
    ReactionParams, ReplyDraftParams, RequestShareManifestParams, SaveIncomingParams,
    SaveShareManifestParams, SendCallMediaParams, SendDmParams, SendGroupParams,
    SendLocationParams, SendParams, ShareGrantParams, ShareIdParams, ShareVisibilityParams,
    SnapshotShareParams, SortContactsParams, SortPeersParams, StartCallMediaParams,
    StartNodeParams, ThemeParams, TypingParams, UserIdParams, WebCreateShareParams,
    WebReviseShareManifestForDeletionParams, parse,
};

/// Run one method. `params` is the frame's `params` payload (`Null` when
/// absent).
///
/// # Errors
/// Returns the method's error string (mirrors the `Result<_, String>` shape
/// of the FRB surface), a parameter-parse error, or "unknown method".
pub fn dispatch(method: &str, params: Value) -> Result<Value, String> {
    if let Some(result) = no_param_helpers(method) {
        return result;
    }
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
        | "setContactNickname"
        | "validateNickname"
        | "networkSizeLabel"
        | "getMyUserId"
        | "sendDeviceBindingIntro"
        | "acceptDeviceBinding"
        | "getUserDevices") => identity(m, params),
        m @ ("getContactsWithStats"
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
        | "reviseShareManifestForDeletion"
        | "saveShareManifest"
        | "acceptShareOffer"
        | "snapshotShare"
        | "requestPeerShares"
        | "requestShareManifest"
        | "readvertiseShare"
        | "publishShareSnapshot") => shares(m, params),
        m @ ("locationMapUrl"
        | "buildLocationMessage"
        | "isLocationMessage"
        | "locationMessageUrl"
        | "locationSessionActive"
        | "locationSendDue"
        | "locationQueryDue"
        | "parseLocationTimeFilter"
        | "loadLocationTrack"
        | "locationGpx"
        | "locationTrackUrl"
        | "sendLocationToDevices"
        | "setLocationDefaults") => location(m, params),
        m @ ("formatTimeHhmm"
        | "isAtBottom"
        | "parseLastSeenMs"
        | "replyDraft"
        | "setTheme"
        | "setNotificationSettings") => ui_helpers(m, params),
        other => Err(format!("unknown method: {other}")),
    }
}

/// Small no-parameter helpers, split out to keep [`dispatch`] readable.
fn no_param_helpers(method: &str) -> Option<Result<Value, String>> {
    let result = match method {
        "getLanOnly" => ok(api::get_lan_only()),
        "quickReactions" => ok(api::quick_reactions()),
        "getBroadcastUnreadCount" => ok(api::get_broadcast_unread_count()),
        "markBroadcastViewed" => json(api::mark_broadcast_viewed()),
        "getTheme" => ok(api::get_theme()),
        "getNotificationSettings" => ok(api::get_notification_settings()),
        "getLocationDefaults" => ok(api::get_location_defaults()),
        "listLocationRecipients" => json(api::list_location_recipients()),
        _ => return None,
    };
    Some(result)
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
        m @ ("getShare" | "deleteShare" | "listShareGrants" | "loadShareManifest") => {
            share_id_methods(m, params)
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
        "setShareVisibility" => {
            let p: ShareVisibilityParams = parse(params)?;
            json(api::set_share_visibility(
                &p.share_id,
                &p.kind,
                p.public_write,
            ))
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
        "reviseShareManifestForDeletion" => {
            let p: WebReviseShareManifestForDeletionParams = parse(params)?;
            let manifest = api::revise_share_manifest_for_deletion(
                api::ReviseShareManifestForDeletionParams {
                    share_id: p.share_id,
                    removed_paths: p.removed_paths,
                },
            )?;
            json(Ok(crate::shares_json::ShareManifestJson::from(&manifest)))
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

/// Share methods whose only input is a share id.
fn share_id_methods(method: &str, params: Value) -> Result<Value, String> {
    let p: ShareIdParams = parse(params)?;
    match method {
        "getShare" => json(api::get_share(&p.share_id)),
        "deleteShare" => json(api::delete_share(&p.share_id)),
        "listShareGrants" => json(api::list_share_grants(&p.share_id)),
        "loadShareManifest" => {
            let manifest = api::load_share_manifest(&p.share_id)?
                .as_ref()
                .map(crate::shares_json::ShareManifestJson::from);
            ok(manifest)
        }
        other => Err(format!("method not routed to share-id methods: {other}")),
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
        "setContactNickname" => {
            let p: ContactNicknameParams = parse(params)?;
            json(api::set_contact_nickname(p.contact_user_id, p.nickname))
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
        "sendDeviceBindingIntro" => {
            let p: PeerIdParams = parse(params)?;
            json(api::send_device_binding_intro(p.peer_id))
        }
        "acceptDeviceBinding" => {
            let p: DeviceBindingParams = parse(params)?;
            json(api::accept_device_binding(p.peer_id, p.device_pubkey))
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

/// Location-sharing surface. Every policy decision (canonical bodies, time
/// filters, GPX, cadence, recipients) is made by `crate::location`; the web
/// arm only decodes camelCase JSON and re-encodes the result.
fn location(method: &str, params: Value) -> Result<Value, String> {
    match method {
        "locationMapUrl" => {
            let p: LocationMapUrlParams = parse(params)?;
            ok(api::location_map_url(p.latitude, p.longitude))
        }
        "buildLocationMessage" => {
            let p: BuildLocationMessageParams = parse(params)?;
            json(api::build_location_message(p.into()))
        }
        "isLocationMessage" => {
            let p: IsLocationMessageParams = parse(params)?;
            ok(api::is_location_message(&p.content))
        }
        "locationMessageUrl" => {
            let p: IsLocationMessageParams = parse(params)?;
            ok(api::location_message_url(&p.content))
        }
        "locationSessionActive" => {
            let p: LocationSessionParams = parse(params)?;
            json(api::location_session_active(
                p.started_at,
                &p.mode,
                p.duration_secs,
                p.now,
            ))
        }
        "locationSendDue" => {
            let p: LocationSendDueParams = parse(params)?;
            ok(api::location_send_due(
                p.last_sent_at,
                p.now,
                p.interval_secs,
            ))
        }
        "locationQueryDue" => {
            let p: LocationQueryDueParams = parse(params)?;
            ok(api::location_query_due(
                p.last_query_at,
                p.now,
                p.interval_secs,
            ))
        }
        "parseLocationTimeFilter" => {
            let p: LocationTimeFilterParams = parse(params)?;
            json(api::parse_location_time_filter(&p.text, p.now))
        }
        "loadLocationTrack" => {
            let p: LoadLocationTrackParams = parse(params)?;
            json(api::load_location_track(p.peer_id, p.from_text, p.to_text))
        }
        "locationGpx" => {
            let p: LocationGpxParams = parse(params)?;
            ok(api::location_gpx(p.points, p.device_label))
        }
        "locationTrackUrl" => {
            let p: LocationTrackUrlParams = parse(params)?;
            ok(api::location_track_url(p.points))
        }
        "sendLocationToDevices" => {
            let p: SendLocationParams = parse(params)?;
            json(api::send_location_to_devices(
                p.selected_peer_ids,
                p.all_devices,
                &p.body,
            ))
        }
        "setLocationDefaults" => {
            let p: crate::settings::LocationDefaults = parse(params)?;
            json(api::set_location_defaults(p))
        }
        other => Err(format!("method not routed to location: {other}")),
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
    fn location_methods_are_routed_and_validate_params() {
        // Every method here takes parameters, so Null is a safe probe that each
        // is reachable through the shared dispatcher (the no-arg methods are
        // covered by the drift list in `web::mod`).
        for method in [
            "locationMapUrl",
            "buildLocationMessage",
            "isLocationMessage",
            "locationMessageUrl",
            "locationSessionActive",
            "locationSendDue",
            "locationQueryDue",
            "parseLocationTimeFilter",
            "loadLocationTrack",
            "locationGpx",
            "locationTrackUrl",
            "sendLocationToDevices",
            "setLocationDefaults",
        ] {
            let err = dispatch(method, Value::Null).expect_err("missing params must fail");
            assert!(
                err.starts_with("invalid params:"),
                "{method}: unexpected error: {err}"
            );
        }
        let url = dispatch(
            "locationMapUrl",
            json!({"latitude": 52.52, "longitude": 13.405}),
        )
        .expect("url");
        assert!(
            url.as_str()
                .is_some_and(|u| u.contains("openstreetmap.org")),
            "{url}"
        );
        let body = dispatch(
            "buildLocationMessage",
            json!({
                "latitude": 52.52, "longitude": 13.405, "accuracy": 12.5,
                "mode": "once", "durationSecs": null, "sensorIntervalSecs": 10,
                "sendIntervalSecs": 60, "motionGate": true,
                "sessionId": "s-1", "sequence": 1, "isFinal": false
            }),
        )
        .expect("body");
        assert!(
            body.as_str().is_some_and(|b| b.starts_with("[Location]")),
            "{body}"
        );
        let detected = dispatch(
            "isLocationMessage",
            json!({"content": body.as_str().unwrap_or("")}),
        )
        .expect("detected");
        assert_eq!(detected, json!(true));
    }

    #[test]
    fn location_methods_are_not_duplicated_in_ui_helpers() {
        // `dispatch` routes every location name to `location()`. A copy in
        // `ui_helpers` would give one method two owners and let a future edit
        // silently change which policy path a web client reaches.
        for method in [
            "locationMapUrl",
            "buildLocationMessage",
            "isLocationMessage",
            "locationMessageUrl",
            "locationSessionActive",
            "locationSendDue",
            "locationQueryDue",
            "parseLocationTimeFilter",
            "loadLocationTrack",
            "locationGpx",
            "locationTrackUrl",
            "sendLocationToDevices",
            "setLocationDefaults",
        ] {
            let err = ui_helpers(method, Value::Null)
                .expect_err("location methods must stay out of ui_helpers");
            assert!(
                err.contains("not routed to ui_helpers"),
                "{method}: unexpected error: {err}"
            );
        }
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
            "reviseShareManifestForDeletion",
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
    fn deletion_revision_params_use_the_camel_case_web_shape() {
        let parsed: WebReviseShareManifestForDeletionParams = parse(json!({
            "shareId": "s-42",
            "removedPaths": ["docs/a.md", "docs/b.md"],
        }))
        .expect("camelCase params");
        assert_eq!(parsed.share_id, "s-42");
        assert_eq!(parsed.removed_paths, ["docs/a.md", "docs/b.md"]);
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
