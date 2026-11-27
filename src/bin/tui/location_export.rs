//! TUI receiver side of device-centric location sharing.
//!
//! A Direct/DM tab can export the track sent by *that exact device* (`Ctrl+L`).
//! The one-line prompt collects only a `from;to` filter: every time grammar and
//! every DM query stays in Rust (`messages::load_location_track`), and the GPX
//! and map URL are built by `location`. This module only owns the prompt, the
//! deterministic output file name and the notice shown after an export.

use super::state::AppState;
use p2p_app::location::LocationTrackPoint;
use p2p_app::tui_tabs::TabContent;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Open the export prompt for the device behind an active Direct tab.
///
/// The peer id is captured now, so an export can never follow a later tab
/// change or mix two devices of the same account. Other tabs are a no-op.
pub fn arm_export_prompt(state: &mut AppState, tab: &TabContent) {
    let TabContent::Direct(peer_id) = tab else {
        return;
    };
    state.begin_export_location_track(peer_id.clone());
    p2p_app::p2plog_debug(format!("Exporting location track for {peer_id}"));
}

/// Split the prompt's one filter line without interpreting any timestamp.
fn split_filter(input: &str) -> Result<(Option<&str>, Option<&str>), String> {
    let text = input.trim();
    if text.is_empty() || text.eq_ignore_ascii_case("all") {
        return Ok((None, None));
    }
    let (from, to) = text.split_once(';').unwrap_or((text, ""));
    if to.contains(';') {
        return Err("use at most one ';' in 'from;to'".to_string());
    }
    Ok((Some(from.trim()), Some(to.trim())))
}

/// Whole Unix seconds for the output file name.
fn unix_seconds() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|error| format!("System clock is before the Unix epoch: {error}"))
}

/// Sanitised, deterministic file name: no user text and no path separators.
#[must_use]
pub fn export_file_name(peer_id: &str, epoch_secs: u64) -> String {
    let tag: String = p2p_app::fmt::short_peer_id(peer_id)
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        .collect();
    let safe_tag = if tag.is_empty() { "peer" } else { &tag };
    format!("location_{safe_tag}_{epoch_secs}.gpx")
}

/// Write one device's GPX under `root`, returning the written path.
///
/// # Errors
/// Returns a human-readable error when the clock or the file write fails.
pub fn write_gpx(
    root: &Path,
    peer_id: &str,
    epoch_secs: u64,
    points: &[LocationTrackPoint],
    label: &str,
) -> Result<PathBuf, String> {
    let path = root.join(export_file_name(peer_id, epoch_secs));
    let gpx = p2p_app::location::location_gpx(points, label);
    std::fs::write(&path, gpx)
        .map_err(|error| format!("Could not write {}: {error}", path.display()))?;
    Ok(path)
}

/// Current working directory, falling back to a relative path if unavailable.
#[must_use]
pub fn default_export_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Commit the export prompt through the canonical Rust track loader.
pub fn submit_location_export(state: &mut AppState, root: &Path) {
    submit_location_export_with(state, root, p2p_app::messages::load_location_track);
}

/// The loader seam is private: production always uses the canonical Rust
/// loader, while tests can prove the prompt's filter and device routing without
/// depending on any frontend-side time parsing.
fn submit_location_export_with<F>(state: &mut AppState, root: &Path, loader: F)
where
    F: Fn(String, Option<String>, Option<String>) -> Result<Vec<LocationTrackPoint>, String>,
{
    let Some(peer_id) = state.location_export_peer_id.clone() else {
        state.cancel_export_location_track();
        state.popup = Some("No device selected for a location export".to_string());
        return;
    };
    let filter = state.chat_input.lines().join("\n");
    let (from, to) = match split_filter(&filter) {
        Ok(bounds) => bounds,
        Err(error) => {
            state.popup = Some(format!("Location export filter: {error}"));
            return;
        }
    };
    let points = match loader(
        peer_id.clone(),
        from.map(str::to_owned),
        to.map(str::to_owned),
    ) {
        Ok(points) => points,
        Err(error) => {
            state.popup = Some(format!(
                "Could not load location track for {peer_id}: {error}"
            ));
            return;
        }
    };
    if points.is_empty() {
        state.cancel_export_location_track();
        state.popup = Some(format!(
            "No location points found for {peer_id} in this range; no file was written"
        ));
        return;
    }
    let epoch_secs = match unix_seconds() {
        Ok(epoch) => epoch,
        Err(error) => {
            state.popup = Some(format!("Could not export location track: {error}"));
            return;
        }
    };
    let label = p2p_app::fmt::peer_display_name(
        &peer_id,
        &state.local_nicknames,
        &state.received_nicknames,
    );
    match write_gpx(root, &peer_id, epoch_secs, &points, &label) {
        Ok(path) => {
            let count = points.len();
            let unit = if count == 1 { "point" } else { "points" };
            let map_note = p2p_app::location::location_track_url(&points).map_or_else(
                || "Map URL unavailable".to_string(),
                |url| format!("Map: {url}"),
            );
            state.cancel_export_location_track();
            state.popup = Some(format!(
                "Exported {count} location {unit} from {peer_id} to {}\n{map_note}",
                path.display()
            ));
        }
        Err(error) => state.popup = Some(error),
    }
}

#[cfg(test)]
mod tests {
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::indexing_slicing,
        clippy::panic,
        clippy::panic_in_result_fn,
        clippy::unreachable,
        clippy::todo,
        clippy::unimplemented,
        clippy::significant_drop_tightening
    )]

    use super::*;
    use crate::tui::test_helpers::{KeySession, test_app_state};
    use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
    use p2p_app::InputMode;
    use ratatui_textarea::TextArea;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;
    use tempfile::TempDir;

    type SeenBounds = Vec<(String, Option<String>, Option<String>)>;

    fn point(latitude: f64, timestamp_secs: f64) -> LocationTrackPoint {
        LocationTrackPoint {
            latitude,
            longitude: 0.0,
            accuracy: 8.0,
            timestamp_secs,
            sequence: 1,
            is_final: false,
        }
    }

    fn file_for(dir: &Path, peer_id: &str) -> PathBuf {
        let prefix = export_file_name(peer_id, 0)
            .strip_suffix("_0.gpx")
            .unwrap_or("location_")
            .to_string();
        std::fs::read_dir(dir)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .find(|path| {
                path.file_name()
                    .and_then(std::ffi::OsStr::to_str)
                    .is_some_and(|name| name.starts_with(&prefix))
            })
            .unwrap_or_else(|| panic!("no export file for {peer_id} in {}", dir.display()))
    }

    #[test]
    fn test_split_filter_leaves_time_grammar_to_rust() {
        assert_eq!(split_filter("all"), Ok((None, None)));
        assert_eq!(split_filter(""), Ok((None, None)));
        assert_eq!(
            split_filter("24 hours ago;now"),
            Ok((Some("24 hours ago"), Some("now")))
        );
        assert_eq!(
            split_filter(";2026-11-27"),
            Ok((Some(""), Some("2026-11-27")))
        );
        assert_eq!(split_filter("tomorrow"), Ok((Some("tomorrow"), Some(""))));
        assert!(
            split_filter("a;b;c").is_err(),
            "extra separators must be rejected"
        );
    }

    #[test]
    fn test_export_file_name_sanitizes_peer_id() {
        let name = export_file_name("../escape/../peer", 1_234);
        assert!(name.starts_with("location_"));
        assert!(name.ends_with("_1234.gpx"), "name: {name}");
        assert!(!name.contains('/'));
        assert!(!name.contains('\\'));
        assert!(!name.contains(".."), "name: {name}");
    }

    #[test]
    fn test_write_gpx_contains_only_the_supplied_points() {
        let dir = TempDir::new().unwrap();
        let points = vec![point(51.5, 1_700_000_000.0), point(51.6, 1_700_000_060.0)];
        let path = write_gpx(dir.path(), "device-laptop", 42, &points, "Laptop").unwrap();
        assert_eq!(
            path.file_name().and_then(std::ffi::OsStr::to_str),
            Some(export_file_name("device-laptop", 42).as_str())
        );
        let gpx = std::fs::read_to_string(&path).unwrap();
        assert!(gpx.contains("<name>Laptop</name>"), "gpx: {gpx}");
        assert!(gpx.contains("lat=\"51.500000\""), "gpx: {gpx}");
        assert!(gpx.contains("lat=\"51.600000\""), "gpx: {gpx}");
        assert!(!gpx.contains("lat=\"41.500000\""), "gpx: {gpx}");
        assert_eq!(gpx.matches("<trkpt").count(), 2);
    }

    #[test]
    fn test_track_url_keeps_only_the_newest_max_points() {
        let points: Vec<LocationTrackPoint> =
            (0..60).map(|i| point(f64::from(i), f64::from(i))).collect();
        let url = p2p_app::location::location_track_url(&points).expect("route URL");
        let route = url.split_once("route=").map_or("", |(_, route)| route);
        assert_eq!(
            route.split(';').count(),
            p2p_app::location::MAX_TRACK_URL_POINTS
        );
        let waypoints: Vec<&str> = route.split(';').collect();
        assert_eq!(waypoints.first().copied(), Some("10.000000,0.000000"));
        assert_eq!(waypoints.last().copied(), Some("59.000000,0.000000"));
        assert!(!waypoints.contains(&"9.000000,0.000000"), "route: {route}");
    }

    #[test]
    fn test_submit_writes_two_devices_to_separate_files() {
        let dir = TempDir::new().unwrap();
        let seen: Rc<RefCell<SeenBounds>> = Rc::new(RefCell::new(Vec::new()));
        let bounds_seen = Rc::clone(&seen);
        let points_by_peer = HashMap::from([
            ("device-laptop".to_string(), vec![point(51.5, 100.0)]),
            ("device-phone".to_string(), vec![point(41.5, 200.0)]),
        ]);
        let loader = move |peer: String, from: Option<String>, to: Option<String>| {
            bounds_seen.borrow_mut().push((peer.clone(), from, to));
            points_by_peer
                .get(&peer)
                .cloned()
                .ok_or_else(|| format!("no points for {peer}"))
        };
        let mut state = test_app_state();
        for (peer, own_lat, other_lat) in [
            ("device-laptop", "51.500000", "41.500000"),
            ("device-phone", "41.500000", "51.500000"),
        ] {
            state.begin_export_location_track(peer.to_string());
            state.chat_input = TextArea::new(vec!["24 hours ago;now".to_string()]);
            submit_location_export_with(&mut state, dir.path(), &loader);
            assert!(!state.is_exporting_location_track());
            assert!(
                state
                    .popup
                    .as_deref()
                    .is_some_and(|popup| popup.contains("Exported 1 location point")),
                "popup: {:?}",
                state.popup
            );
            let gpx = std::fs::read_to_string(file_for(dir.path(), peer)).unwrap();
            assert!(gpx.contains(&format!("lat=\"{own_lat}\"")), "gpx: {gpx}");
            assert!(!gpx.contains(&format!("lat=\"{other_lat}\"")), "gpx: {gpx}");
        }
        let seen = seen.borrow();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen[0].0, "device-laptop");
        assert_eq!(seen[1].0, "device-phone");
        assert!(
            seen.iter()
                .all(|(_, from, to)| from.as_deref() == Some("24 hours ago")
                    && to.as_deref() == Some("now"))
        );
    }

    #[test]
    fn test_submit_with_no_points_writes_no_file() {
        let dir = TempDir::new().unwrap();
        let mut state = test_app_state();
        state.begin_export_location_track("device-empty".to_string());
        state.chat_input = TextArea::new(vec!["all".to_string()]);
        submit_location_export_with(
            &mut state,
            dir.path(),
            |_peer: String, from: Option<String>, to: Option<String>| {
                assert_eq!(from, None);
                assert_eq!(to, None);
                Ok(Vec::new())
            },
        );
        assert!(!state.is_exporting_location_track());
        assert!(
            state
                .popup
                .as_deref()
                .is_some_and(|popup| popup.contains("No location points found"))
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[tokio::test]
    async fn test_ctrl_l_prompt_owns_keys_and_mouse_on_direct_tab() {
        let mut state = test_app_state();
        let tab_index = state.dynamic_tabs.add_dm_tab("device-laptop".to_string());
        state.switch_to_tab(tab_index);
        let mut session = KeySession::new(state);

        assert!(!session.press_ctrl_char('l').await);
        {
            let s = session.state.lock().await;
            assert_eq!(s.input_mode, InputMode::ExportingLocationTrack);
            assert!(s.is_modal_editor());
            assert_eq!(s.location_export_peer_id.as_deref(), Some("device-laptop"));
            assert_eq!(s.chat_input.lines().join("\n"), "");
        }

        // A modal prompt keeps Tab, scrolling and clicks away from the tabs.
        let active_tab = session.state.lock().await.active_tab;
        assert!(!session.press_code(KeyCode::Tab).await);
        assert!(
            !session
                .press_mouse(MouseEvent {
                    kind: MouseEventKind::ScrollDown,
                    column: 10,
                    row: 5,
                    modifiers: KeyModifiers::NONE,
                })
                .await
        );
        assert!(
            !session
                .press_mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: 10,
                    row: 5,
                    modifiers: KeyModifiers::NONE,
                })
                .await
        );
        assert_eq!(session.state.lock().await.active_tab, active_tab);

        assert!(!session.type_text("24 hours ago;now").await);
        assert_eq!(
            session.state.lock().await.chat_input.lines().join("\n"),
            "24 hours ago;now"
        );

        assert!(!session.press_code(KeyCode::Esc).await);
        let s = session.state.lock().await;
        assert_eq!(s.input_mode, InputMode::Chat);
        assert_eq!(s.location_export_peer_id, None);
    }

    #[tokio::test]
    async fn test_ctrl_l_is_inert_outside_a_direct_tab() {
        let mut session = KeySession::new(test_app_state());
        assert!(!session.press_ctrl_char('l').await);
        let s = session.state.lock().await;
        assert_eq!(s.input_mode, InputMode::Chat);
        assert_eq!(s.location_export_peer_id, None);
    }

    #[tokio::test]
    async fn test_invalid_filter_reaches_the_canonical_loader_and_keeps_the_prompt() {
        let mut state = test_app_state();
        let tab_index = state.dynamic_tabs.add_dm_tab("device-filter".to_string());
        state.switch_to_tab(tab_index);
        let mut session = KeySession::new(state);

        assert!(!session.press_ctrl_char('l').await);
        assert!(!session.type_text("tomorrow;now").await);
        assert!(!session.press_code(KeyCode::Enter).await);
        let s = session.state.lock().await;
        assert!(
            s.popup
                .as_deref()
                .is_some_and(|popup| popup.contains("unrecognised time")),
            "popup: {:?}",
            s.popup
        );
        assert_eq!(s.input_mode, InputMode::ExportingLocationTrack);
        assert_eq!(s.location_export_peer_id.as_deref(), Some("device-filter"));
    }
}
