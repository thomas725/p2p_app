// Node server for the web frontend (`docs/web_frontend.md`).
//

// Starts the p2p node with the same headless lifecycle as the mobile path

// (DB selection, identity provisioning, group re-subscription, attachment

// GC) and then serves the JSON-over-WebSocket API — plus, with `--assets`,

// the `flutter build web` output — until Ctrl-C.

//

// ```text

// p2p_chat_server [--bind <ip>] [--port <u16>] [--user <name>] \

//                 [--password <secret>] [--assets <dir>] [--db <sqlite path>]

// ```

//

// Defaults to loopback-only with no credentials (the TUI's trust model).

// Expose `--bind 0.0.0.0` to other devices only together with HTTP Basic

// credentials (`--password`, preferably `P2P_WEB_PASSWORD`).


use clap::Parser;
use p2p_app::web::{Credentials, WebConfig};
use std::path::PathBuf;




#[derive(Parser, Debug)]
#[command(name = "p2p_chat_server", version, about = "p2p chat node + web frontend API")]
struct Cli {
    #[arg(long, default_value = "127.0.0.1")]
    bind: String,
    #[arg(long, default_value_t = 8080)]
    port: u16,
    #[arg(long, env = "P2P_WEB_USER", default_value = "p2p")]
    user: String,
    #[arg(long, env = "P2P_WEB_PASSWORD")]
    password: Option<String>,
    #[arg(long)]
    assets: Option<PathBuf>,
    #[arg(long)]
    db: Option<String>,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    public_dht: bool,
    #[arg(long, action = clap::ArgAction::SetTrue)]
    no_public_dht: bool,
}

/// Reject credentials the Basic auth layer could not represent.
fn validate(cli: &Cli) -> Result<(), String> {
    let Some(password) = cli.password.as_deref() else {
        return Ok(());
    };
    if password.is_empty() {
        return Err("--password must not be empty".to_owned());
    }
    if cli.user.is_empty() {
        return Err("--user must not be empty when a password is set".to_owned());
    }
    if cli.user.contains(':') {
        return Err("--user must not contain ':' (RFC 7617 splits on it)".to_owned());
    }
    Ok(())
}

fn main() -> color_eyre::Result<()> {
    run_server()
}

/// Run the web server frontend.
///
/// # Errors
/// Returns an error if argument parsing or serving fails.
pub fn run_server() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();
    validate(&cli).map_err(|e| color_eyre::eyre::eyre!("{e}"))?;

    // Public DHT opt-in override.
    if cli.no_public_dht {
        unsafe { std::env::set_var("P2P_PUBLIC_DHT", "0") };
    } else if cli.public_dht {
        unsafe { std::env::set_var("P2P_PUBLIC_DHT", "1") };
    }

    let addr: std::net::SocketAddr = format!("{}:{}", cli.bind, cli.port)
        .parse()
        .map_err(|e| color_eyre::eyre::eyre!("invalid --bind/--port: {e}"))?;
    let credentials = cli.password.map(|password| Credentials {
        user: cli.user,
        password,
    });
    if credentials.is_some() && std::env::var_os("P2P_WEB_PASSWORD").is_none() {
        eprintln!(
            "NOTE: the password came from the command line, where `ps` and your shell \
             history can see it — prefer P2P_WEB_PASSWORD."
        );
    }
    if !addr.ip().is_loopback() && credentials.is_none() {
        eprintln!(
            "WARNING: binding non-loopback {addr} without --password — anyone on the network \
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
                credentials,
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
