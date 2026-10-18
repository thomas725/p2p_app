//! Node server for the web frontend (docs/2026-10-18_web_frontend.md).
//!
//! Starts the p2p node with the same headless lifecycle as the mobile path
//! (DB selection, identity provisioning, group re-subscription, attachment
//! GC) and then serves the JSON-over-WebSocket API — plus, with `--assets`,
//! the `flutter build web` output — until Ctrl-C.
//!
//! ```text
//! p2p_chat_server [--bind <ip>] [--port <u16>] [--token <t>] \
//!                 [--assets <dir>] [--db <sqlite path>]
//! ```
//!
//! Defaults to loopback-only with no token (the TUI's trust model). Expose
//! `--bind 0.0.0.0` to other devices only together with `--token`.

use p2p_app::web::WebConfig;
use std::path::PathBuf;

const USAGE: &str = "p2p_chat_server — p2p chat node + web frontend API

USAGE:
    p2p_chat_server [OPTIONS]

OPTIONS:
    --bind <ip>       Address to bind (default 127.0.0.1)
    --port <u16>      Port to listen on (default 8080)
    --token <token>   Bearer token guarding /ws and /api/* (recommended
                      whenever --bind is not loopback)
    --assets <dir>    Serve a `flutter build web` output directory
    --db <path>       SQLite database path (default: auto-select, same
                      lock-based behavior as the TUI)
    -h, --help        Show this help
";

struct Cli {
    bind: String,
    port: u16,
    token: Option<String>,
    assets: Option<PathBuf>,
    db: Option<String>,
}

/// Parse argv into a [`Cli`]. `Ok(None)` = help was printed.
fn parse_args() -> Result<Option<Cli>, String> {
    let mut cli = Cli {
        bind: "127.0.0.1".to_owned(),
        port: 8080,
        token: None,
        assets: None,
        db: None,
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{USAGE}");
                return Ok(None);
            }
            "--bind" => cli.bind = value_of(&arg, &mut args)?,
            "--port" => {
                let raw = value_of(&arg, &mut args)?;
                cli.port = raw
                    .parse()
                    .map_err(|e| format!("invalid --port {raw:?}: {e}"))?;
            }
            "--token" => cli.token = Some(value_of(&arg, &mut args)?),
            "--assets" => cli.assets = Some(PathBuf::from(value_of(&arg, &mut args)?)),
            "--db" => cli.db = Some(value_of(&arg, &mut args)?),
            other => return Err(format!("unknown argument {other:?}\n\n{USAGE}")),
        }
    }
    Ok(Some(cli))
}

fn value_of(flag: &str, args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    args.next()
        .ok_or_else(|| format!("{flag} requires a value\n\n{USAGE}"))
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = match parse_args() {
        Ok(Some(cli)) => cli,
        Ok(None) => return Ok(()),
        Err(e) => {
            eprintln!("{e}");
            return Err(color_eyre::eyre::eyre!("invalid arguments"));
        }
    };

    let addr: std::net::SocketAddr = format!("{}:{}", cli.bind, cli.port)
        .parse()
        .map_err(|e| color_eyre::eyre::eyre!("invalid --bind/--port: {e}"))?;
    if !addr.ip().is_loopback() && cli.token.is_none() {
        eprintln!(
            "WARNING: binding non-loopback {addr} without --token — anyone on the network \
             can drive this node (send messages, read history, download attachments)."
        );
    }

    // Start the node on this plain (non-async) thread: the mobile lifecycle
    // builds its own tokio runtime for the swarm, which must not happen
    // inside an async context.
    let peer_id = cli
        .db
        .as_deref()
        .map_or_else(p2p_app::mobile_node::start_node_auto, |db| {
            p2p_app::mobile_node::start_node(db.to_owned())
        })
        .map_err(|e| color_eyre::eyre::eyre!(e))?;

    // Mirror the TUI's console logging: the log callback feeds the in-memory
    // buffer the Log tab polls; also print to stderr for the server console.
    let callback: std::sync::Arc<dyn Fn(String) + Send + Sync> =
        std::sync::Arc::new(|msg| eprintln!("{msg}"));
    p2p_app::logging::register_log_callback(callback);
    eprintln!("p2p chat server online — peer id: {peer_id}");

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime
        .block_on(async {
            let served = p2p_app::web::serve(WebConfig {
                bind: addr,
                token: cli.token,
                assets: cli.assets,
            })
            .await;
            // Release the DB lock file on the way out so the next start can pick
            // the same database (mirrors the TUI's shutdown path).
            p2p_app::db::release_db_lock();
            served
        })
        .map_err(|e| color_eyre::eyre::eyre!(e))
}
