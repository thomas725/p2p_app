//! Immutable reply/quote snapshots shared by the wire, database and frontends.
//!
//! A reply carries the id of the message it answers plus a sender label and a
//! short preview captured at the moment of sending. The snapshot travels in the
//! message payload and is stored verbatim: a quoted header therefore keeps
//! rendering even when the source row was deleted locally or belongs to a peer
//! we no longer have. No frontend resolves the id or formats attachment bytes.

use serde::{Deserialize, Serialize};

/// Maximum number of characters kept in a quoted preview.
const PREVIEW_CHARS: usize = 80;

/// An immutable snapshot of the message being answered (US-006).
///
/// `msg_id` is the stable wire id of the source message. `sender` and `preview`
/// are display strings frozen when the reply is composed; they are never looked
/// up again, so a local tombstone cannot blank the quote.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ReplyRef {
    /// Stable wire id of the message being replied to.
    pub msg_id: String,
    /// Sender label of the quoted message at reply time.
    pub sender: String,
    /// Short single-line preview of the quoted message body.
    pub preview: String,
}

impl ReplyRef {
    /// Build a snapshot, collapsing the preview to one bounded line.
    #[must_use]
    pub fn new(msg_id: String, sender: String, preview: &str) -> Self {
        Self {
            msg_id,
            sender,
            preview: preview_line(preview),
        }
    }
}

/// Collapse whitespace and bound a preview to 80 characters.
#[must_use]
pub fn preview_line(value: &str) -> String {
    let flat = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= PREVIEW_CHARS {
        return flat;
    }
    let mut short = flat.chars().take(PREVIEW_CHARS).collect::<String>();
    short.push('…');
    short
}

/// Format the quoted header shown above a reply's body.
#[must_use]
pub fn quoted_header(reply: &ReplyRef) -> String {
    format!("↳ {}: {}", reply.sender, reply.preview)
}

/// Serialize a snapshot for the `reply_to_json` persistence column.
#[must_use]
pub fn encode(reply: &ReplyRef) -> Option<String> {
    serde_json::to_string(reply).ok()
}

/// Read a snapshot persisted in `reply_to_json`.
#[must_use]
pub fn decode(raw: &str) -> Option<ReplyRef> {
    serde_json::from_str(raw).ok()
}

/// Locate the sender tag and the start of its body in a rendered transcript.
///
/// A transcript row is `{time} {latency}? [{sender}] {body}`, optionally
/// preceded by a `↳ sender: preview` quoted-header line. The body may itself
/// span several lines (multiline messages composed in either frontend). The
/// row carrying the sender tag is therefore the **last** `\n`-separated line
/// containing a `[sender] ` tag; everything from just after that tag to the end
/// of the text is the body, continuation lines included. Returning a byte offset
/// (rather than a body slice) lets callers keep the full multiline remainder
/// without allocating and without indexing into the string.
fn sender_and_body_offset(text: &str) -> Option<(&str, usize)> {
    let mut line_offset = 0usize;
    let mut found = None;
    for line in text.split_inclusive('\n') {
        if let Some((sender, body)) = line
            .split_once('[')
            .and_then(|(_, after_open)| after_open.split_once("] "))
        {
            let body_offset = line_offset
                .saturating_add(line.len())
                .saturating_sub(body.len());
            found = Some((sender, body_offset));
        }
        line_offset = line_offset.saturating_add(line.len());
    }
    found
}

/// Build a snapshot from a formatted TUI transcript line and its wire id.
///
/// TUI lines look like `{time} [sender] body` (an optional latency string sits
/// between the time and the sender tag). Keeping this here makes the quote
/// capture rule shared by Chat, DM and Group tabs.
#[must_use]
pub fn from_display_line(line: &str, msg_id: &str) -> Option<ReplyRef> {
    if msg_id.trim().is_empty() {
        return None;
    }
    let (sender, offset) = sender_and_body_offset(line)?;
    let body = line.get(offset..).unwrap_or(line);
    Some(ReplyRef::new(msg_id.to_string(), sender.to_string(), body))
}

/// Split a TUI transcript line into `(sender tag, body)`.
///
/// Any quoted header above it is ignored. The body includes every continuation
/// line, so the local-edit editor sees the full multiline body rather than only
/// its last line.
#[must_use]
pub fn split_display_line(line: &str) -> (&str, &str) {
    sender_and_body_offset(line).map_or(("", line), |(sender, offset)| {
        (sender, line.get(offset..).unwrap_or(line))
    })
}

/// The body of a TUI transcript line (see [`split_display_line`]).
#[must_use]
pub fn display_body(line: &str) -> String {
    split_display_line(line).1.to_string()
}

/// Replace the body of a TUI transcript line.
///
/// Keeps its timestamp, sender tag and quoted header. Used after a local edit
/// (US-005). When the original body spanned several lines, the entire block is
/// replaced, not just its final line.
#[must_use]
pub fn with_body(line: &str, body: &str) -> String {
    sender_and_body_offset(line).map_or_else(
        || body.to_string(),
        |(_, offset)| format!("{}{}", line.get(..offset).unwrap_or(line), body),
    )
}

/// Prepend the canonical quoted header to a rendered transcript line.
#[must_use]
pub fn with_quoted_header(text: String, reply: Option<&ReplyRef>) -> String {
    match reply {
        Some(reply) => format!("{}\n{text}", quoted_header(reply)),
        None => text,
    }
}

#[cfg(test)]
#[path = "../tests/unit/unit_reply.rs"]
mod tests;
