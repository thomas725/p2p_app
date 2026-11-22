//! Raw key-byte probe for diagnosing `Tab` vs `Ctrl+I` (and other modifier
//! key) encoding differences across terminals.
//!
//! Run with: `p2p_chat_tui key-probe`
//!
//! Reads raw bytes straight off the terminal (bypassing crossterm's event
//! parser, which can silently mis-decode or reject unusual CSI sequences) and
//! prints each byte it receives. This gives ground truth for what a terminal
//! actually sends for a given physical key under each keyboard-encoding mode:
//!
//! - Pressing `1` selects **Legacy mode** (no requests sent): `Ctrl+I` is
//!   expected to be byte-identical to `Tab` (`0x09`).
//! - Pressing `2` requests the **Kitty protocol** (`CSI > 1 u`,
//!   `DISAMBIGUATE_ESCAPE_CODES`): a working terminal must send `Ctrl+I` as
//!   `ESC [ 1 0 5 ; 4 u` distinct from a bare `Tab` (`0x09` or `ESC [ 9 u`).
//! - Pressing `3` requests **modifyOtherKeys mode 2** (`CSI > 4;2 m`): xterm
//!   style, `Ctrl+I` as `ESC [ 2 7 ; 5 ; 1 0 5 ~`.
//!
//! Press `q` to quit (restores the terminal).

use crossterm::{
    event::{KeyboardEnhancementFlags, PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{Read, Write};

#[derive(Debug)]
enum ProbeMode {
    Legacy,
    Kitty,
    ModifyOtherKeys,
}

impl ProbeMode {
    const fn banner(&self) -> &'static str {
        match self {
            Self::Legacy => "LEGACY (no request). Press Tab then Ctrl+I.",
            Self::Kitty => "KITTY (`CSI > 1 u`). Press Tab then Ctrl+I.",
            Self::ModifyOtherKeys => "modifyOtherKeys 2 (`CSI > 4;2 m`). Press Tab then Ctrl+I.",
        }
    }
}

fn byte_label(b: u8) -> String {
    match b {
        0x1B => "ESC".to_string(),
        0x09 => "TAB".to_string(),
        0x0D => "CR".to_string(),
        0x0A => "LF".to_string(),
        0x7F => "DEL".to_string(),
        b if b.is_ascii_graphic() || b == b' ' => format!("[{}]", char::from(b)),
        b => format!("\\x{b:02x}"),
    }
}

/// Apply one keyboard-encoding mode and print its banner.
///
/// Keeping the three protocol switches in one place makes the near-identical
/// `1`/`2`/`3` arms in [`run`] one line each and guarantees every switch emits
/// the same banner. `stdout` is passed by mutable reference so `execute!`'s
/// `by_ref()` flush leaves the caller's handle usable.
fn switch_mode(stdout: &mut impl Write, mode: &ProbeMode) -> color_eyre::Result<()> {
    match mode {
        ProbeMode::Legacy => {
            execute!(stdout, PopKeyboardEnhancementFlags)?;
            write!(stdout, "\x1b[>4;0m")?;
        }
        ProbeMode::Kitty => {
            write!(stdout, "\x1b[>4;0m")?;
            execute!(
                stdout,
                PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
            )?;
        }
        ProbeMode::ModifyOtherKeys => {
            execute!(stdout, PopKeyboardEnhancementFlags)?;
            write!(stdout, "\x1b[>4;2m")?;
        }
    }
    write!(stdout, "\r\nMode: {}\r\n", mode.banner())?;
    Ok(())
}

/// Runs the probe until the user presses `q`. Restores the terminal on exit.
pub fn run() -> color_eyre::Result<()> {
    enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    write!(stdout, "\r\n")?; // leading blank line, then scroll like a normal app
    write!(
        stdout,
        "Byte probe. Modes: 1=legacy 2=kitty 3=modifyOtherKeys   q=quit\r\n"
    )?;
    stdout.flush()?;

    let mut mode = ProbeMode::Legacy;
    write!(stdout, "Mode: {}\r\n", mode.banner())?;
    stdout.flush()?;

    let mut stdin = std::io::stdin();
    let mut buf = [0u8; 128];
    let mut in_escape = false;
    // True while we are still expecting the single-byte introducer that must
    // follow ESC (`[` for CSI, `]` for OSC, ...). The introducer itself is
    // never a final byte, so without this a CSI sequence would wrongly
    // terminate on its own `[` (0x5B is inside 0x40..=0x7E).
    let mut expect_intro = false;
    let mut seq_len = 0usize;

    let mut quit = false;
    while !quit {
        let n = stdin.read(&mut buf).map_err(color_eyre::Report::from)?;
        if n == 0 {
            break;
        }
        for &b in buf.iter().take(n) {
            if !in_escape {
                match b {
                    b'1' => {
                        mode = ProbeMode::Legacy;
                        switch_mode(&mut stdout, &mode)?;
                    }
                    b'2' => {
                        mode = ProbeMode::Kitty;
                        switch_mode(&mut stdout, &mode)?;
                    }
                    b'3' => {
                        mode = ProbeMode::ModifyOtherKeys;
                        switch_mode(&mut stdout, &mode)?;
                    }
                    b'q' => {
                        quit = true;
                        break;
                    }
                    0x1B => {
                        write!(stdout, "ESC")?;
                        in_escape = true;
                        expect_intro = true;
                        seq_len = 1;
                    }
                    b => {
                        write!(stdout, " {}\r\n", byte_label(b))?;
                    }
                }
            } else if b == 0x1B {
                // New escape inside an unfinished one: close the line first.
                write!(stdout, "\r\n")?;
                write!(stdout, "ESC")?;
                expect_intro = true;
                seq_len = 1;
            } else {
                write!(stdout, " {}", byte_label(b))?;
                seq_len = seq_len.saturating_add(1);
                let final_byte = !expect_intro && (0x40..=0x7E).contains(&b);
                expect_intro = false;
                if final_byte || seq_len > 32 {
                    in_escape = false;
                    write!(stdout, "\r\n")?;
                    seq_len = 0;
                }
            }
        }
        stdout.flush()?;
    }

    // Restore the terminal: reset modifyOtherKeys, pop the kitty flags.
    write!(stdout, "\x1b[>4;0m")?;
    let _ = execute!(stdout, PopKeyboardEnhancementFlags);
    disable_raw_mode()?;
    println!("Key probe finished. Terminal restored.");
    Ok(())
}
