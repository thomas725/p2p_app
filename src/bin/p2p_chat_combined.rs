//! Unified binary — runs as the TUI or the web server.
//!
//! Select the mode with `P2P_MODE=tui|server` (default: tui if available).

#[cfg(feature = "tui")]
#[allow(dead_code)]
mod tui_bin {
    include!("p2p_chat_tui.rs");
}

#[cfg(feature = "web")]
#[allow(dead_code)]
mod server_bin {
    include!("p2p_chat_server.rs");
}

fn main() {
    let mode = std::env::var("P2P_MODE").unwrap_or_else(|_| {
        #[cfg(feature = "tui")]
        {
            "tui".to_string()
        }
        #[cfg(all(not(feature = "tui"), feature = "web"))]
        {
            "server".to_string()
        }
        #[cfg(all(not(feature = "tui"), not(feature = "web")))]
        {
            "none".to_string()
        }
    });

    match mode.as_str() {
        #[cfg(feature = "tui")]
        "tui" => {
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create runtime: {e}");
                    std::process::exit(1);
                }
            };
            if let Err(e) = rt.block_on(tui_bin::run_tui()) {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        #[cfg(feature = "web")]
        "server" => {
            if let Err(e) = server_bin::run_server() {
                eprintln!("{e}");
                std::process::exit(1);
            }
        }
        other => {
            eprintln!("Unknown or unavailable mode: {other}");
            std::process::exit(2);
        }
    }
}