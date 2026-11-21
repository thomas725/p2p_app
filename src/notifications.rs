//! Notification policy and payloads (US-027 sounds, US-028 desktop alerts).
//!
//! Rust decides *what* is worth notifying about and builds the display text;
//! the frontends only decide whether the conversation is on screen and hand
//! the payload to the platform. That keeps the "new message → notification"
//! rule identical in the TUI, Android and the browser.
//!
//! A [`Notification`] is attached to the incoming-message event envelope, so
//! every transport receives exactly the same title/body without re-deriving
//! display names or truncating text in Dart.

use serde::Serialize;

/// One notification for an incoming message.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Notification {
    /// Sender (or group + sender) display label.
    pub title: String,
    /// Short, single-line message preview.
    pub body: String,
    /// Stable key of the conversation, for focus suppression:
    /// `broadcast`, `dm:<peer-id>` or `group:<group-id>`.
    pub conversation_key: String,
    /// Whether a sound should accompany this notification.
    pub sound: bool,
    /// Whether a desktop/system notification should be shown.
    pub desktop: bool,
}

/// Persisted delivery channels (US-027 / US-028).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationSettings {
    /// Play a sound on a new message. On by default; no permission needed.
    pub sound: bool,
    /// Show a desktop/system notification. Off by default because the browser
    /// and Android 13+ both require an explicit permission grant first.
    pub desktop: bool,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            sound: true,
            desktop: false,
        }
    }
}

/// Read the current delivery channels from persisted settings.
#[must_use]
pub fn settings() -> NotificationSettings {
    NotificationSettings {
        sound: crate::settings::notification_sound(),
        desktop: crate::settings::notification_desktop(),
    }
}

/// Persist both channels in one call. Both writes succeed or the first error
/// is returned, so a partial settings update surfaces to the caller.
///
/// # Errors
/// Returns a human-readable error if either database write fails.
pub fn set_settings(sound: bool, desktop: bool) -> Result<(), String> {
    crate::settings::set_notification_sound(sound)?;
    crate::settings::set_notification_desktop(desktop)
}

/// Build the notification for an incoming message event.
///
/// Returns `None` when both channels are disabled (nothing to deliver) or the
/// event kind is not a user message (call/control events are handled by their
/// own UI). Display-name resolution stays in Rust so TUI and Flutter agree.
#[must_use]
pub fn for_message(
    event_type: &str,
    peer_id: &str,
    nickname: Option<&str>,
    content: &str,
    group_id: Option<&str>,
) -> Option<Notification> {
    let channels = settings();
    if !channels.sound && !channels.desktop {
        return None;
    }
    // Reject control/call events before the DB-backed display-name lookup below,
    // so a receipt never pays for a resolution it cannot use.
    let conversation_key = conversation_key(event_type, peer_id, group_id)?;

    let sender = nickname.filter(|name| !name.is_empty()).map_or_else(
        || crate::nickname::get_peer_display_name_or_short(peer_id),
        ToString::to_string,
    );
    let title = match group_id {
        Some(id) if !id.is_empty() => {
            format!(
                "Group {} · {sender}",
                id.chars().take(8).collect::<String>()
            )
        }
        _ => sender,
    };

    Some(Notification {
        title,
        body: preview(content),
        conversation_key,
        sound: channels.sound,
        desktop: channels.desktop,
    })
}

/// Stable focus-suppression key for a message event. `None` for any event kind
/// that is not a user message, so control/call events never notify.
fn conversation_key(event_type: &str, peer_id: &str, group_id: Option<&str>) -> Option<String> {
    match event_type {
        "broadcast" => Some("broadcast".to_string()),
        "dm" => Some(format!("dm:{peer_id}")),
        "group_message" => group_id
            .filter(|id| !id.is_empty())
            .map(|id| format!("group:{id}")),
        _ => None,
    }
}

/// Terminal control sequence that delivers one notification in the TUI.
///
/// An optional bell (US-027) is followed by an OSC 777 `notify` sequence
/// (US-028, supported by kitty, iTerm2, GNOME Terminal and `WezTerm`). Titles
/// and bodies are sanitized so control characters and the `;` field separator
/// can never inject a second OSC command or corrupt the frame.
#[must_use]
pub fn terminal_sequence(n: &Notification) -> String {
    let mut out = String::new();
    if n.sound {
        out.push('\u{7}');
    }
    if n.desktop {
        out.push_str("\u{1b}]777;notify;");
        out.push_str(&osc_field(&n.title));
        out.push(';');
        out.push_str(&osc_field(&n.body));
        out.push('\u{7}');
    }
    out
}

/// Flatten a value for use inside an OSC 777 field: strip control characters
/// and replace `;` (the field separator) with a space.
fn osc_field(value: &str) -> String {
    value
        .chars()
        .map(|c| if c.is_control() || c == ';' { ' ' } else { c })
        .collect()
}

/// Single-line preview of a message body, bounded so a notification never
/// dominates the screen (or the OS notification shade).
fn preview(content: &str) -> String {
    const LIMIT: usize = 140;
    let flat: String = content
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if flat.chars().count() <= LIMIT {
        return if flat.is_empty() {
            "[new message]".to_string()
        } else {
            flat
        };
    }
    let head: String = flat.chars().take(LIMIT.saturating_sub(1)).collect();
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_flattens_controls_and_bounds_length() {
        assert_eq!(preview(""), "[new message]");
        assert_eq!(preview("line one\nline two"), "line one line two");
        let long = "x".repeat(300);
        let result = preview(&long);
        assert_eq!(result.chars().count(), 140);
        assert!(result.ends_with('…'));
    }

    #[test]
    fn for_message_is_none_for_control_events() {
        // No DB access is attempted for non-message kinds, so this is safe
        // without a test database.
        assert!(for_message("receipt", "peer", None, "hi", None).is_none());
    }

    #[test]
    fn conversation_key_maps_message_kinds() {
        assert_eq!(
            conversation_key("broadcast", "peer", None).as_deref(),
            Some("broadcast")
        );
        assert_eq!(
            conversation_key("dm", "peer42", None).as_deref(),
            Some("dm:peer42")
        );
        assert_eq!(
            conversation_key("group_message", "peer", Some("grp1")).as_deref(),
            Some("group:grp1")
        );
        // A group event without an id cannot be focused — never notify.
        assert!(conversation_key("group_message", "peer", None).is_none());
        assert!(conversation_key("call_invite", "peer", None).is_none());
    }

    #[test]
    fn terminal_sequence_encodes_sound_and_desktop_channels() {
        let sound = Notification {
            title: "Alice".to_string(),
            body: "hello".to_string(),
            conversation_key: "dm:peer".to_string(),
            sound: true,
            desktop: false,
        };
        assert_eq!(terminal_sequence(&sound), "\u{7}");

        let both = Notification {
            title: "Group ab;cd".to_string(),
            body: "line\ntext".to_string(),
            conversation_key: "group:grp".to_string(),
            sound: true,
            desktop: true,
        };
        assert_eq!(
            terminal_sequence(&both),
            "\u{7}\u{1b}]777;notify;Group ab cd;line text\u{7}"
        );
    }
}
