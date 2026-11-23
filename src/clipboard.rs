//! OSC 52 clipboard sequences for the TUI.
//!
//! The render loop owns every TUI stdout write, so clipboard content is turned
//! into an escape sequence here (a pure function) and emitted *after* the frame
//! alongside the OSC 777 notifications in [`crate::notifications`]. That keeps
//! terminal writes single-threaded and prevents a clipboard payload from
//! interleaving with ratatui's screen drawing.
//!
//! OSC 52 support is terminal-dependent (kitty, iTerm2, `WezTerm`, Alacritty,
//! foot and modern xterm work; tmux needs `set-clipboard` passthrough enabled).
//! The same caveat applies to the desktop notifications already in the app.

use crossterm::Command;

/// Build the OSC 52 sequence that copies `content` to the terminal clipboard.
///
/// [`crossterm::clipboard::CopyToClipboard::to_clipboard_from`] targets the `c`
/// (clipboard) selection, the most widely supported destination across
/// emulators. The content is base64-encoded by crossterm, so message text —
/// including newlines, tabs and ANSI bytes — can never inject a second OSC
/// command into the sequence.
#[must_use]
pub fn copy_sequence(content: &str) -> String {
    let command = crossterm::clipboard::CopyToClipboard::to_clipboard_from(content.as_bytes());
    let mut out = String::new();
    // Writing an ANSI sequence into a `String` cannot fail.
    let _ = command.write_ansi(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// crossterm terminates the OSC with the canonical `ST` (`ESC \`) rather
    /// than BEL, so there is one ESC for the introducer and one for the end.
    const ST: &str = "\u{1b}\\";

    #[test]
    fn copy_sequence_targets_clipboard_with_known_base64() {
        let seq = copy_sequence("hello");
        assert!(seq.starts_with("\u{1b}]52;c;"), "{seq:?}");
        assert!(seq.ends_with(ST), "{seq:?}");
        assert!(seq.contains("aGVsbG8="), "{seq:?}");
    }

    #[test]
    fn copy_sequence_encodes_control_characters_without_osc_injection() {
        let seq = copy_sequence("line one\nline two\ttab\u{1b}[31mred");
        assert!(!seq.contains('\n'), "{seq:?}");
        assert!(!seq.contains('\t'), "{seq:?}");
        assert_eq!(seq.matches('\u{1b}').count(), 2, "{seq:?}");
        assert!(seq.ends_with(ST), "{seq:?}");
    }

    #[test]
    fn copy_sequence_of_empty_content_is_still_a_valid_sequence() {
        let seq = copy_sequence("");
        assert_eq!(seq, format!("\u{1b}]52;c;{ST}"));
    }

    #[test]
    fn copy_sequence_differs_for_different_content() {
        assert_ne!(copy_sequence("one"), copy_sequence("two"));
    }
}
