use crate::tui::state::AppState;
use std::collections::HashMap;

pub fn row_to_visible_index(
    line_counts: &[usize],
    first_content_row: usize,
    click_row: usize,
) -> Option<usize> {
    if click_row < first_content_row {
        return None;
    }
    let mut current_row = first_content_row;

    for (idx, line_count) in line_counts.iter().copied().enumerate() {
        let message_end_row = current_row + line_count;
        if click_row < message_end_row {
            return Some(idx);
        }
        current_row = message_end_row;
    }

    None
}

pub fn broadcast_receipt_prefix(
    msg_id: Option<&str>,
    sender_id: Option<&str>,
    broadcast_receipts: &HashMap<String, HashMap<String, f64>>,
) -> &'static str {
    match (msg_id, sender_id) {
        (Some(msg_id), None) => {
            let confirmed = broadcast_receipts.get(msg_id).map(|m| m.len()).unwrap_or(0);
            if confirmed == 0 { "  " } else { "v " }
        }
        _ => "  ",
    }
}

pub fn dm_receipt_prefix(
    msg_id: Option<&str>,
    dm_receipts: &HashMap<String, (String, f64)>,
) -> &'static str {
    match msg_id {
        Some(msg_id) if dm_receipts.contains_key(msg_id) => "v ",
        _ => "  ",
    }
}

pub fn nickname_counts(state: &AppState) -> HashMap<String, usize> {
    let mut counts = HashMap::new();

    for (peer_id, _first_seen, _last_seen) in &state.peers {
        let nick = state
            .local_nicknames
            .get(peer_id)
            .or_else(|| state.received_nicknames.get(peer_id))
            .map(|s| s.trim())
            .filter(|s| !s.is_empty());
        if let Some(nick) = nick {
            *counts.entry(nick.to_string()).or_insert(0) += 1;
        }
    }

    counts
}

pub fn format_peer_line(
    peer_id: &str,
    last_seen: &str,
    nickname: Option<&str>,
    nickname_counts: &HashMap<String, usize>,
) -> String {
    if let Some(nickname) = nickname
        && nickname_counts.get(nickname).copied().unwrap_or(0) == 1
    {
        format!("{} {} {}", nickname, peer_id, last_seen)
    } else {
        format!("{} {}", peer_id, last_seen)
    }
}

pub fn format_broadcast_title(nickname: Option<&str>, short_id: &str) -> String {
    format!(
        "Broadcast from {}{}",
        nickname.map(|n| format!("{} ", n)).unwrap_or_default(),
        short_id
    )
}

pub fn format_dm_title(
    nickname: Option<&str>,
    short_id: &str,
    last_seen: Option<&str>,
    first_seen: Option<&str>,
) -> String {
    format!(
        "DM: {}{} | seen: {}{}",
        nickname.map(|n| format!("{} ", n)).unwrap_or_default(),
        short_id,
        last_seen.unwrap_or("?"),
        first_seen
            .map(|first_seen| format!(" (first: {})", first_seen))
            .unwrap_or_default()
    )
}

#[cfg(test)]
mod tests {
    use super::{
        broadcast_receipt_prefix, dm_receipt_prefix, format_broadcast_title, format_dm_title,
        format_peer_line, row_to_visible_index,
    };
    use std::collections::HashMap;

    #[test]
    fn row_to_visible_index_tracks_multiline_boundaries() {
        let line_counts = vec![2, 1, 3];

        assert_eq!(row_to_visible_index(&line_counts, 3, 3), Some(0));
        assert_eq!(row_to_visible_index(&line_counts, 3, 4), Some(0));
        assert_eq!(row_to_visible_index(&line_counts, 3, 5), Some(1));
        assert_eq!(row_to_visible_index(&line_counts, 3, 8), Some(2));
        assert_eq!(row_to_visible_index(&line_counts, 3, 9), None);
    }

    #[test]
    fn broadcast_receipt_prefix_only_marks_outgoing_confirmed_messages() {
        let mut receipts = HashMap::new();
        receipts.insert(
            "msg-1".to_string(),
            HashMap::from([("peer-1".to_string(), 1.0)]),
        );

        assert_eq!(
            broadcast_receipt_prefix(Some("msg-1"), None, &receipts),
            "v "
        );
        assert_eq!(
            broadcast_receipt_prefix(Some("msg-2"), None, &receipts),
            "  "
        );
        assert_eq!(
            broadcast_receipt_prefix(Some("msg-1"), Some("peer-1"), &receipts),
            "  "
        );
    }

    #[test]
    fn dm_receipt_prefix_marks_confirmed_messages() {
        let receipts = HashMap::from([("msg-1".to_string(), ("peer-1".to_string(), 2.0))]);

        assert_eq!(dm_receipt_prefix(Some("msg-1"), &receipts), "v ");
        assert_eq!(dm_receipt_prefix(Some("msg-2"), &receipts), "  ");
        assert_eq!(dm_receipt_prefix(None, &receipts), "  ");
    }

    #[test]
    fn format_peer_line_shows_unique_nickname_only() {
        let unique_counts = HashMap::from([("alice".to_string(), 1usize)]);
        let duplicate_counts = HashMap::from([("alice".to_string(), 2usize)]);

        assert_eq!(
            format_peer_line("peer-1", "just now", Some("alice"), &unique_counts),
            "alice peer-1 just now"
        );
        assert_eq!(
            format_peer_line("peer-1", "just now", Some("alice"), &duplicate_counts),
            "peer-1 just now"
        );
    }

    #[test]
    fn titles_include_optional_nickname_context() {
        assert_eq!(
            format_broadcast_title(Some("alice"), "12345678"),
            "Broadcast from alice 12345678"
        );
        assert_eq!(
            format_dm_title(Some("alice"), "12345678", Some("now"), Some("earlier")),
            "DM: alice 12345678 | seen: now (first: earlier)"
        );
    }

    #[test]
    fn titles_work_without_nickname() {
        assert_eq!(
            format_broadcast_title(None, "12345678"),
            "Broadcast from 12345678"
        );
        assert_eq!(
            format_dm_title(None, "12345678", Some("now"), None),
            "DM: 12345678 | seen: now"
        );
        assert_eq!(
            format_dm_title(None, "12345678", None, None),
            "DM: 12345678 | seen: ?"
        );
    }

    #[test]
    fn format_peer_line_no_nickname() {
        let counts = std::collections::HashMap::new();
        assert_eq!(
            format_peer_line("peer-1", "just now", None, &counts),
            "peer-1 just now"
        );
    }

    #[test]
    fn row_to_visible_index_empty_line_counts() {
        assert_eq!(row_to_visible_index(&[], 3, 3), None);
    }

    #[test]
    fn row_to_visible_index_before_content_start() {
        let line_counts = vec![2, 1];
        // row 0 and 1 are before first_content_row=3 — content only starts at row 3
        assert_eq!(row_to_visible_index(&line_counts, 3, 0), None);
    }
}
