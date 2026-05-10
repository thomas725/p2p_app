#[cfg(feature = "dioxus-desktop")]
fn main() {
    eprintln!("Dioxus desktop support requires fixing API compatibility.");
    eprintln!("Run with: cargo run --features tui for TUI mode");
}

#[cfg(not(feature = "dioxus-desktop"))]
fn main() {
    eprintln!("This binary requires the 'dioxus-desktop' feature.");
    eprintln!("Run with: cargo run --bin p2p_chat_dioxus --features dioxus-desktop");
}
