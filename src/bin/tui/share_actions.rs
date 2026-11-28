//! Shares-tab keyboard actions for the TUI.
//!
//! Every handler here keeps its policy calls in Rust (`p2p_app::shares` and the
//! canonical swarm commands) and owns a narrow slice of the keyboard. `AppState`
//! is the single view/model; these functions only mutate it and queue commands.
//! Dispatch precedence lives in [`handle_shares_tab_keys`], which is called
//! before global navigation so modal editors keep ownership of every key.

use super::state::{ShareGrantAction, SharedState};
use p2p_app::{SwarmCommand, p2plog_debug};
use tokio::sync::mpsc;

/// `Tab` toggles the canonical mode while the add-grant prompt is open.
///
/// This runs before global navigation: a modal editor normally makes Tab a
/// no-op so a prompt cannot be stranded on another tab, but here Tab is the
/// prompt's own mode selector. Revoke prompts have no mode and keep the
/// standard modal Tab behavior.
fn handle_share_grant_prompt_key(
    key_event: &crossterm::event::KeyEvent,
    is_ctrl: bool,
    s: &mut super::state::AppState,
) -> bool {
    use crossterm::event::KeyCode;

    let is_add = s
        .share_grant_prompt
        .as_ref()
        .is_some_and(|prompt| matches!(&prompt.action, ShareGrantAction::Add { .. }));
    if is_ctrl || !s.is_editing_share_grant() || !is_add {
        return false;
    }
    if !matches!(key_event.code, KeyCode::Tab | KeyCode::BackTab) {
        return false;
    }
    s.toggle_share_grant_mode();
    true
}

/// `Ctrl+Up`/`Ctrl+Down` moves the grant cursor on the Shares tab. Bare arrows
/// keep owning the share list, matching the message-cursor pattern on chat tabs.
fn handle_share_grant_selection_key(
    key_event: &crossterm::event::KeyEvent,
    is_ctrl: bool,
    tab: &p2p_app::tui_tabs::TabContent,
    s: &mut super::state::AppState,
) -> bool {
    use crossterm::event::KeyCode;
    use p2p_app::tui_tabs::TabContent;

    if !is_ctrl || s.is_modal_editor() || !matches!(tab, TabContent::Shares) {
        return false;
    }
    let delta = match key_event.code {
        KeyCode::Up => -1,
        KeyCode::Down => 1,
        _ => return false,
    };
    s.move_share_grant_selection(delta);
    true
}

/// `Ctrl+Left`/`Ctrl+Right` moves the manifest-file cursor. Bare Left/Right keep
/// owning the network-peer cursor, matching the share/grant cursor split.
fn handle_share_manifest_cursor_key(
    key_event: &crossterm::event::KeyEvent,
    is_ctrl: bool,
    tab: &p2p_app::tui_tabs::TabContent,
    s: &mut super::state::AppState,
) -> bool {
    use crossterm::event::KeyCode;
    use p2p_app::tui_tabs::TabContent;

    if !is_ctrl || s.is_modal_editor() || !matches!(tab, TabContent::Shares) {
        return false;
    }
    let delta = match key_event.code {
        KeyCode::Left => -1,
        KeyCode::Right => 1,
        _ => return false,
    };
    s.move_share_manifest_selection(delta);
    true
}

/// `d` marks the highlighted manifest file for deletion; `Ctrl+D` publishes the
/// marked set as a Rust-built revision. Both are inert while a modal editor owns
/// the keyboard.
fn handle_share_manifest_action_key(
    key_event: &crossterm::event::KeyEvent,
    is_ctrl: bool,
    tab: &p2p_app::tui_tabs::TabContent,
    s: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) -> bool {
    use crossterm::event::KeyCode;
    use p2p_app::tui_tabs::TabContent;

    if s.is_modal_editor() || !matches!(tab, TabContent::Shares) {
        return false;
    }
    match (key_event.code, is_ctrl) {
        (KeyCode::Char('d'), false) => {
            s.toggle_share_manifest_deletion();
            true
        }
        (KeyCode::Char('d'), true) => {
            publish_share_deletions(s, swarm_cmd_tx);
            true
        }
        _ => false,
    }
}

/// Left/Right move the Shares network-peer cursor. Modal editors own every key,
/// so grant/nickname prompts never move the target underneath the user.
fn handle_share_peer_selection_key(
    key_event: &crossterm::event::KeyEvent,
    s: &mut super::state::AppState,
    tab: &p2p_app::tui_tabs::TabContent,
) -> bool {
    use crossterm::event::KeyCode;
    use p2p_app::tui_tabs::TabContent;

    if s.is_modal_editor() || !matches!(tab, TabContent::Shares) {
        return false;
    }
    if !matches!(key_event.code, KeyCode::Left | KeyCode::Right) {
        return false;
    }
    s.move_share_peer_selection(key_event.code);
    true
}

/// `j`/`k` move the discovered-offer cursor. They are the only unclaimed
/// navigation keys on the Shares tab: bare arrows already own the share list
/// and the peer target, while the Ctrl variants own grants and manifest files.
fn handle_share_discovery_cursor_key(
    key_event: &crossterm::event::KeyEvent,
    is_ctrl: bool,
    tab: &p2p_app::tui_tabs::TabContent,
    s: &mut super::state::AppState,
) -> bool {
    use crossterm::event::KeyCode;
    use p2p_app::tui_tabs::TabContent;

    if is_ctrl || s.is_modal_editor() || !matches!(tab, TabContent::Shares) {
        return false;
    }
    let delta = match key_event.code {
        KeyCode::Char('k') => -1,
        KeyCode::Char('j') => 1,
        _ => return false,
    };
    s.move_share_discovery_selection(delta);
    true
}

/// `f` asks the selected peer to list shares it is willing to serve. `Enter`
/// accepts the highlighted offer through the canonical Rust store, creating a
/// local stub; the advertised owner stays unverified until a manifest arrives.
fn handle_share_discovery_action_key(
    key_event: &crossterm::event::KeyEvent,
    is_ctrl: bool,
    tab: &p2p_app::tui_tabs::TabContent,
    s: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) -> bool {
    use crossterm::event::{KeyCode, KeyModifiers};
    use p2p_app::tui_tabs::TabContent;

    if is_ctrl || s.is_modal_editor() || !matches!(tab, TabContent::Shares) {
        return false;
    }
    match key_event.code {
        KeyCode::Char('f') => {
            discover_peer_shares(s, swarm_cmd_tx);
            true
        }
        KeyCode::Enter if !key_event.modifiers.contains(KeyModifiers::SHIFT) => {
            accept_discovered_share(s);
            true
        }
        _ => false,
    }
}

/// Queue `ListShares` for the canonical connected-peer target.
fn discover_peer_shares(s: &mut super::state::AppState, swarm_cmd_tx: &mpsc::Sender<SwarmCommand>) {
    let Some(peer_id) = s.selected_share_peer_id() else {
        s.popup = Some("Connect a peer before discovering shares".to_string());
        return;
    };
    match swarm_cmd_tx.try_send(SwarmCommand::ListShares {
        peer_id: peer_id.clone(),
    }) {
        Ok(()) => {
            s.share_event_notice = Some(format!("Share discovery requested from {peer_id}"));
        }
        Err(error) => {
            s.popup = Some(format!("Could not request share list: {error}"));
        }
    }
}

/// Materialise the highlighted remote offer. Rust validates the name and id and
/// refuses to let an untrusted owner claim rewrite a share we already own or
/// have bound to a folder.
fn accept_discovered_share(s: &mut super::state::AppState) {
    let Some(discovered) = s.selected_discovered_share().cloned() else {
        s.popup = Some("No discovered share selected".to_string());
        return;
    };
    match p2p_app::shares::accept_offer(&discovered.offer, None) {
        Ok(share) => {
            s.remove_discovered_share(&discovered.peer_id, &discovered.offer.share_id);
            s.reload_shares();
            let position = s
                .shares
                .iter()
                .position(|row| row.share_id == share.share_id)
                .unwrap_or(s.share_selection);
            s.select_share(position);
            s.share_event_notice =
                Some(format!("Added {} from {}", share.name, discovered.peer_id));
            p2plog_debug(format!("Accepted share offer: {}", share.share_id));
        }
        Err(error) => {
            s.popup = Some(format!("Could not add discovered share: {error}"));
        }
    }
}

/// Share id plus canonical connected peer selected for a Shares network action.
fn share_action_target(s: &super::state::AppState) -> Option<(String, String)> {
    let share_id = s
        .shares
        .get(s.share_selection)
        .map(|share| share.share_id.clone())?;
    let peer_id = s.selected_share_peer_id()?;
    Some((share_id, peer_id))
}

/// Ask the selected peer for the highlighted share's current manifest.
fn request_share_manifest(
    s: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) {
    let Some((share_id, peer_id)) = share_action_target(s) else {
        s.popup = Some("Select a share and a connected peer".to_string());
        return;
    };
    match swarm_cmd_tx.try_send(SwarmCommand::RequestShareManifest {
        peer_id: peer_id.clone(),
        share_id,
    }) {
        Ok(()) => {
            s.share_event_notice = Some(format!("Manifest requested from {peer_id}"));
        }
        Err(error) => {
            s.popup = Some(format!("Could not request share manifest: {error}"));
        }
    }
}

/// Re-advertise the highlighted share to the selected peer. The offer is built
/// in Rust so no frontend can hand-build or spoof an ownership claim.
fn readvertise_share(s: &mut super::state::AppState, swarm_cmd_tx: &mpsc::Sender<SwarmCommand>) {
    let Some((share_id, peer_id)) = share_action_target(s) else {
        s.popup = Some("Select a share and a connected peer".to_string());
        return;
    };
    match p2p_app::shares::offer_for_share(&share_id) {
        Ok(Some(offer)) => {
            match swarm_cmd_tx.try_send(SwarmCommand::SendShareOffer {
                peer_ids: vec![peer_id.clone()],
                offer: Box::new(offer),
            }) {
                Ok(()) => {
                    s.share_event_notice = Some(format!("Share re-advertised to {peer_id}"));
                }
                Err(error) => {
                    s.popup = Some(format!("Could not re-advertise share: {error}"));
                }
            }
        }
        Ok(None) => {
            s.popup = Some("Share no longer exists".to_string());
            s.reload_shares();
        }
        Err(error) => {
            s.popup = Some(format!("Could not build share offer: {error}"));
        }
    }
}

/// Publish the marked manifest deletions as the next revision. The new snapshot
/// is built from the canonical store, while the base revision comes from the
/// cached manifest validated by [`super::state::AppState::sync_share_manifest_draft`].
fn publish_share_deletions(
    s: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) {
    s.sync_share_manifest_draft();
    if s.share_deleted_paths.is_empty() {
        s.popup = Some("Mark at least one file with d first".to_string());
        return;
    }
    let Some(base_revision) = s
        .selected_share_manifest()
        .map(|manifest| manifest.revision)
    else {
        s.popup = Some("Manifest is stale: reload it with r before publishing".to_string());
        return;
    };
    let Some((share_id, peer_id)) = share_action_target(s) else {
        s.popup = Some("Select a share and a connected peer".to_string());
        return;
    };
    let paths: Vec<String> = s.share_deleted_paths.iter().cloned().collect();
    let manifest = match p2p_app::shares::manifest_with_deletions(&share_id, &paths) {
        Ok(manifest) => manifest,
        Err(error) => {
            s.popup = Some(format!("Could not build deletion revision: {error}"));
            return;
        }
    };
    // The candidate is rebuilt from the canonical store. If the store moved
    // after this view was loaded, its next revision cannot follow the base we
    // would send: drop the command instead of guaranteeing a rejection.
    if base_revision.checked_add(1) != Some(manifest.revision) {
        s.popup = Some("Manifest changed in the store: reload it with r".to_string());
        return;
    }
    match swarm_cmd_tx.try_send(SwarmCommand::PublishShareSnapshot {
        peer_id: peer_id.clone(),
        base_revision,
        manifest: Box::new(manifest),
    }) {
        Ok(()) => {
            let count = paths.len();
            let noun = if count == 1 { "file" } else { "files" };
            s.reset_share_manifest_draft();
            s.sync_share_manifest_draft();
            s.share_event_notice =
                Some(format!("Deletion of {count} {noun} submitted to {peer_id}"));
        }
        Err(error) => {
            s.popup = Some(format!("Could not publish share deletion: {error}"));
        }
    }
}

/// Arm the add-grant editor for the highlighted private share.
fn arm_add_share_grant(s: &mut super::state::AppState) {
    let Some((share_id, kind)) = s
        .shares
        .get(s.share_selection)
        .map(|share| (share.share_id.clone(), share.kind))
    else {
        s.popup = Some("No share selected".to_string());
        return;
    };
    if kind != p2p_app::shares::ShareKind::Private {
        s.popup = Some("Grants only apply to private shares".to_string());
        return;
    }
    s.begin_add_share_grant(share_id);
}

/// Arm the revoke-grant editor for the highlighted grant on a private share.
fn arm_revoke_share_grant(s: &mut super::state::AppState) {
    let Some((share_id, kind)) = s
        .shares
        .get(s.share_selection)
        .map(|share| (share.share_id.clone(), share.kind))
    else {
        s.popup = Some("No share selected".to_string());
        return;
    };
    if kind != p2p_app::shares::ShareKind::Private {
        s.popup = Some("Grants only apply to private shares".to_string());
        return;
    }
    let Some(user_id) = s
        .selected_share_grants()
        .get(s.share_grant_selection)
        .map(|grant| grant.user_id.clone())
    else {
        s.popup = Some("No grant selected to revoke".to_string());
        return;
    };
    s.begin_revoke_share_grant(share_id, user_id);
}

/// Apply a visibility change to the highlighted locally owned share.
///
/// Ownership is checked in Rust before the canonical store write, so the UI
/// cannot reclassify an accepted peer share. The store repeats the check.
fn apply_share_visibility(
    s: &mut super::state::AppState,
    kind: p2p_app::shares::ShareKind,
    public_write: bool,
) {
    let Some(share) = s.shares.get(s.share_selection).cloned() else {
        s.popup = Some("No share selected".to_string());
        return;
    };
    let Some(owner) = s.own_user_id.clone() else {
        s.popup = Some("Local user is not provisioned".to_string());
        return;
    };
    if share.owner_user_id != owner {
        s.popup = Some("Only the share owner can change visibility".to_string());
        return;
    }
    match p2p_app::shares::set_visibility(&share.share_id, &owner, kind, public_write) {
        Ok(updated) => {
            let notice = format!(
                "Share is {} (writes: {})",
                p2p_app::shares::kind_label(updated.kind),
                if updated.public_write {
                    "allowed"
                } else {
                    "no"
                }
            );
            s.share_event_notice = Some(notice.clone());
            s.reload_shares();
            p2plog_debug(format!("Share visibility: {notice}"));
        }
        Err(error) => s.popup = Some(format!("Could not change share visibility: {error}")),
    }
}

/// `p` flips private/public; `w` toggles anonymous writes only on public shares.
fn toggle_share_visibility(s: &mut super::state::AppState) {
    let Some(share) = s.shares.get(s.share_selection).cloned() else {
        s.popup = Some("No share selected".to_string());
        return;
    };
    let next = if share.kind == p2p_app::shares::ShareKind::Private {
        p2p_app::shares::ShareKind::Public
    } else {
        p2p_app::shares::ShareKind::Private
    };
    let public_write = next == p2p_app::shares::ShareKind::Public && share.public_write;
    apply_share_visibility(s, next, public_write);
}

fn toggle_share_public_write(s: &mut super::state::AppState) {
    let Some(share) = s.shares.get(s.share_selection).cloned() else {
        s.popup = Some("No share selected".to_string());
        return;
    };
    if share.kind != p2p_app::shares::ShareKind::Public {
        s.popup = Some("Only public shares can allow anonymous writes".to_string());
        return;
    }
    apply_share_visibility(s, p2p_app::shares::ShareKind::Public, !share.public_write);
}

/// Create a private, read-only share from the shared name prompt.
///
/// Name validation and the DB write live in the canonical store, so a rejection
/// keeps the editor open with its text for the next correction key.
pub(super) fn confirm_create_share(s: &mut super::state::AppState) {
    let name = s.chat_input.lines().join("\n");
    let Some(owner) = s.own_user_id.clone() else {
        s.popup = Some("Local user is not provisioned".to_string());
        return;
    };
    match p2p_app::shares::create_share(
        &name,
        &owner,
        p2p_app::shares::ShareKind::Private,
        false,
        None,
    ) {
        Ok(share) => {
            s.cancel_create_share();
            s.reload_shares();
            let fallback = s.share_selection;
            let position = s
                .shares
                .iter()
                .position(|row| row.share_id == share.share_id)
                .unwrap_or(fallback);
            s.select_share(position);
            s.share_event_notice = Some(format!("Share created: {}", share.name));
            p2plog_debug(format!("Created share: {}", share.share_id));
        }
        Err(error) => {
            s.popup = Some(format!("Share not created: {error}"));
        }
    }
}

/// `D` arms the local-delete confirmation for the highlighted share. Deleting
/// an accepted peer share locally is valid — it removes this device's copy — so
/// ownership is deliberately not checked here. The id and name are captured
/// now; while the prompt is open no key can move the share selection.
fn arm_share_delete(s: &mut super::state::AppState) {
    let Some(share) = s.shares.get(s.share_selection).cloned() else {
        s.popup = Some("No share selected".to_string());
        return;
    };
    s.begin_share_delete(share.share_id.clone(), share.name);
    p2plog_debug(format!("Share deletion armed for {}", share.share_id));
}

/// Apply a confirmed local share deletion through the canonical facade. Rust
/// removes the share row, entries and grants in one transaction and leaves the
/// bound folder plus content-addressed blobs untouched; no swarm command is
/// queued, so peers that already accepted the share keep their copy.
pub(super) async fn confirm_share_delete(state: &SharedState) {
    let prompt = {
        let mut s = state.lock().await;
        s.share_delete_prompt.take()
    };
    let Some(prompt) = prompt else {
        return;
    };
    match p2p_app::api::delete_share(&prompt.share_id) {
        Ok(()) => {
            let mut s = state.lock().await;
            s.reload_shares();
            s.share_event_notice = Some(format!("Share deleted: {}", prompt.name));
            drop(s);
            p2plog_debug(format!(
                "Deleted share locally: {} ({})",
                prompt.name, prompt.share_id
            ));
        }
        Err(error) => {
            let mut s = state.lock().await;
            s.popup = Some(format!("Could not delete share: {error}"));
            drop(s);
            p2plog_debug(format!(
                "Failed to delete share {}: {error}",
                prompt.share_id
            ));
        }
    }
}

/// Arm the folder editor for the highlighted share. Binding a peer-owned share
/// as a local mirror is valid, so ownership is deliberately not checked here;
/// Rust validates and canonicalises the folder when the prompt is committed.
fn arm_bind_share(s: &mut super::state::AppState) {
    let Some(share) = s.shares.get(s.share_selection).cloned() else {
        s.popup = Some("No share selected".to_string());
        return;
    };
    s.begin_bind_share(share.share_id, share.local_root.as_deref());
}

/// Commit the active folder editor to the share captured when it opened. An
/// empty line clears the binding; a rejected path keeps the editor open with its
/// text so the operator can correct it without retargeting another share.
pub(super) fn confirm_share_binding(s: &mut super::state::AppState) {
    let Some(prompt) = s.share_binding_prompt.clone() else {
        return;
    };
    let root = s.chat_input.lines().join("\n").trim().to_string();
    let binding = (!root.is_empty()).then_some(root.as_str());
    let notice = if binding.is_some() {
        format!("Folder bound for {}", prompt.share_id)
    } else {
        format!("Local folder cleared for {}", prompt.share_id)
    };
    match p2p_app::shares::set_local_root(&prompt.share_id, binding) {
        Ok(()) => {
            s.cancel_share_binding();
            s.share_event_notice = Some(notice.clone());
            s.reload_shares();
            select_share_by_id(s, &prompt.share_id);
            p2plog_debug(format!("Share binding: {notice}"));
        }
        Err(error) => {
            s.popup = Some(format!("Could not bind share folder: {error}"));
        }
    }
}

/// Snapshot the highlighted share's bound folder into the next canonical
/// manifest revision. A missing binding stays a UI hint instead of guessing a
/// path; all folder walking and blob storage live in Rust.
fn snapshot_share_folder(s: &mut super::state::AppState) {
    let Some(share) = s.shares.get(s.share_selection).cloned() else {
        s.popup = Some("No share selected".to_string());
        return;
    };
    let Some(root) = share.local_root.clone() else {
        s.popup = Some("Bind a local folder with b first".to_string());
        return;
    };
    match p2p_app::shares::snapshot_share(&share.share_id, std::path::Path::new(&root)) {
        Ok(manifest) => {
            s.share_event_notice = Some(format!(
                "Snapshot saved for {} (revision {})",
                share.name, manifest.revision
            ));
            s.reload_shares();
            select_share_by_id(s, &share.share_id);
            p2plog_debug(format!("Snapshotted share: {}", share.share_id));
        }
        Err(error) => {
            s.popup = Some(format!("Could not snapshot share folder: {error}"));
        }
    }
}

/// Publish the cached snapshot of the highlighted share to the selected peer.
/// The local revision is accepted by a peer whose copy is one behind, so the
/// base sent on the wire is exactly `manifest.revision - 1`; ACLs and the stale
/// check remain in the receiving Rust peer.
fn publish_share_snapshot(
    s: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) {
    let Some(share) = s.shares.get(s.share_selection).cloned() else {
        s.popup = Some("No share selected".to_string());
        return;
    };
    let Some(manifest) = share.manifest.clone() else {
        s.popup = Some("Snapshot the shared folder with s first".to_string());
        return;
    };
    if !p2p_app::shares::manifest_matches_share(&share) || manifest.revision != share.revision {
        s.popup = Some("Snapshot is stale: refresh it with r or s".to_string());
        return;
    }
    let Some(base_revision) = manifest.revision.checked_sub(1) else {
        s.popup = Some("The initial revision has no previous version to publish".to_string());
        return;
    };
    let Some(peer_id) = s.selected_share_peer_id() else {
        s.popup = Some("Select a connected peer before publishing".to_string());
        return;
    };
    let revision = manifest.revision;
    match swarm_cmd_tx.try_send(SwarmCommand::PublishShareSnapshot {
        peer_id: peer_id.clone(),
        base_revision,
        manifest: Box::new(manifest),
    }) {
        Ok(()) => {
            s.share_event_notice = Some(format!("Revision {revision} submitted to {peer_id}"));
        }
        Err(error) => {
            s.popup = Some(format!("Could not publish share snapshot: {error}"));
        }
    }
}

/// Re-highlight `share_id` after an action reloads the store. The fallback keeps
/// the previous cursor when the share vanished (for example, a peer removed it).
fn select_share_by_id(s: &mut super::state::AppState, share_id: &str) {
    let fallback = s.share_selection;
    let position = s
        .shares
        .iter()
        .position(|row| row.share_id == share_id)
        .unwrap_or(fallback);
    s.select_share(position);
}

/// Commit the active grant editor through the canonical store, then re-read the
/// Shares pane. Any validation or DB failure keeps the editor open with a hint.
pub(super) fn confirm_share_grant(s: &mut super::state::AppState) {
    let Some(prompt) = s.share_grant_prompt.clone() else {
        return;
    };
    let (outcome, notice) = match prompt.action {
        ShareGrantAction::Add { mode } => {
            let user_id = s.chat_input.lines().join("\n").trim().to_string();
            if user_id.is_empty() {
                s.popup = Some("Enter a verified user ID".to_string());
                return;
            }
            (
                p2p_app::shares::set_grant(&prompt.share_id, &user_id, mode),
                format!("Grant set for {user_id}"),
            )
        }
        // The highlighted grant is the authority for a revoke; the prefilled
        // text is only a confirmation and cannot retarget the write.
        ShareGrantAction::Revoke { user_id } => (
            p2p_app::shares::remove_grant(&prompt.share_id, &user_id),
            format!("Grant revoked for {user_id}"),
        ),
    };
    match outcome {
        Ok(()) => {
            s.cancel_share_grant();
            s.share_event_notice = Some(notice.clone());
            s.reload_shares();
            p2plog_debug(format!("Share grant editor: {notice}"));
        }
        Err(error) => {
            s.popup = Some(format!("Could not update share grant: {error}"));
        }
    }
}

/// Route the Shares-tab shortcut helpers in precedence order.
///
/// Keeping this pre-dispatch in one helper leaves `handle_key_dispatch` focused
/// on tab-wide routing and preserves modal-editor ownership.
pub(super) fn handle_shares_tab_keys(
    key_event: &crossterm::event::KeyEvent,
    is_ctrl: bool,
    tab: &p2p_app::tui_tabs::TabContent,
    s: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) -> bool {
    if handle_share_grant_prompt_key(key_event, is_ctrl, s) {
        return true;
    }
    if handle_share_grant_selection_key(key_event, is_ctrl, tab, s) {
        return true;
    }
    if handle_share_manifest_cursor_key(key_event, is_ctrl, tab, s) {
        return true;
    }
    if handle_share_manifest_action_key(key_event, is_ctrl, tab, s, swarm_cmd_tx) {
        return true;
    }
    if handle_share_discovery_cursor_key(key_event, is_ctrl, tab, s) {
        return true;
    }
    if handle_share_discovery_action_key(key_event, is_ctrl, tab, s, swarm_cmd_tx) {
        return true;
    }
    handle_share_peer_selection_key(key_event, s, tab)
}

/// Plain (non-modified) shortcuts owned by the Shares tab. Splitting them out
/// keeps the tab dispatcher small without giving any handler a lint allowance.
pub(super) fn handle_shares_tab_shortcut(
    code: crossterm::event::KeyCode,
    s: &mut super::state::AppState,
    swarm_cmd_tx: &mpsc::Sender<SwarmCommand>,
) -> bool {
    match code {
        crossterm::event::KeyCode::Char('c') => {
            s.begin_create_share();
            true
        }
        crossterm::event::KeyCode::Char('b') => {
            arm_bind_share(s);
            true
        }
        crossterm::event::KeyCode::Char('s') => {
            snapshot_share_folder(s);
            true
        }
        crossterm::event::KeyCode::Char('u') => {
            publish_share_snapshot(s, swarm_cmd_tx);
            true
        }
        crossterm::event::KeyCode::Char('D') => {
            arm_share_delete(s);
            true
        }
        crossterm::event::KeyCode::Char('a') => {
            arm_add_share_grant(s);
            true
        }
        crossterm::event::KeyCode::Char('x') => {
            arm_revoke_share_grant(s);
            true
        }
        crossterm::event::KeyCode::Char('p') => {
            toggle_share_visibility(s);
            true
        }
        crossterm::event::KeyCode::Char('w') => {
            toggle_share_public_write(s);
            true
        }
        crossterm::event::KeyCode::Char('m') => {
            request_share_manifest(s, swarm_cmd_tx);
            true
        }
        crossterm::event::KeyCode::Char('o') => {
            readvertise_share(s, swarm_cmd_tx);
            true
        }
        // Live share events also reload the store, but a manual refresh keeps
        // the view authoritative after an asynchronously delivered change.
        crossterm::event::KeyCode::Char('r') => {
            s.reload_shares();
            true
        }
        _ => false,
    }
}
