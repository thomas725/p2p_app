#[cfg(feature = "dioxus-desktop")]
fn main() {
    dioxus_desktop::launch_cfg(
        dioxus_desktop::Config::new().with_window(
            dioxus_desktop::WindowBuilder::new()
                .with_title("P2P Chat")
                .with_inner_size(dioxus_desktop::LogicalSize::new(800.0, 600.0)),
        ),
        app,
    );
}

#[cfg(not(feature = "dioxus-desktop"))]
fn main() {
    eprintln!("This binary requires the 'dioxus-desktop' feature.");
    eprintln!("Run with: cargo run --bin p2p_chat_dioxus --features dioxus-desktop");
}

use dioxus::prelude::*;

fn app() -> Element {
    let mut messages = use_signal(Vec::<Message>::new);
    let mut peers = use_signal(Vec::<Peer>::new);
    let mut input_text = use_signal(String::new);
    let mut selected_tab = use_signal(|| Tab::Chat);

    rsx! {
        div {
            style: "display: flex; flex-direction: column; height: 100vh; font-family: system-ui, sans-serif; margin: 0; padding: 0;",

            // Header
            div {
                style: "background: #2d2d2d; color: white; padding: 10px 20px;",
                h1 { "P2P Chat" }
            }

            // Tab bar
            div {
                style: "display: flex; background: #3d3d3d; border-bottom: 1px solid #555;",
                button {
                    style: "flex: 1; padding: 10px; border: none; background: {if *selected_tab.read() == Tab::Chat { \"#555\" } else { \"transparent\" }}; color: white; cursor: pointer;",
                    onclick: move |_| selected_tab.set(Tab::Chat),
                    "Chat"
                }
                button {
                    style: "flex: 1; padding: 10px; border: none; background: {if *selected_tab.read() == Tab::Peers { \"#555\" } else { \"transparent\" }}; color: white; cursor: pointer;",
                    onclick: move |_| selected_tab.set(Tab::Peers),
                    "Peers"
                }
                button {
                    style: "flex: 1; padding: 10px; border: none; background: {if *selected_tab.read() == Tab::Log { \"#555\" } else { \"transparent\" }}; color: white; cursor: pointer;",
                    onclick: move |_| selected_tab.set(Tab::Log),
                    "Log"
                }
            }

            // Content area
            div {
                style: "flex: 1; display: flex; flex-direction: column; overflow: hidden;",

                match *selected_tab.read() {
                    Tab::Chat => rsx! { chat_view { messages: messages, input_text: input_text } },
                    Tab::Peers => rsx! { peers_view { peers: peers } },
                    Tab::Log => rsx! { log_view {} },
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
enum Tab {
    Chat,
    Peers,
    Log,
}

#[derive(Clone)]
struct Message {
    id: usize,
    sender: String,
    content: String,
    timestamp: String,
}

#[derive(Clone)]
struct Peer {
    id: String,
    nickname: String,
    connected: bool,
}

#[component]
fn chat_view(messages: Signal<Vec<Message>>, input_text: Signal<String>) -> Element {
    let scroll_ref = use_node_ref();

    rsx! {
        div {
            style: "flex: 1; display: flex; flex-direction: column; overflow: hidden;",

            // Messages area
            div {
                style: "flex: 1; overflow-y: auto; padding: 10px; background: #1a1a1a;",

                for msg in messages.read().iter() {
                    div {
                        style: "padding: 8px; margin-bottom: 4px; background: #2d2d2d; border-radius: 4px;",
                        span {
                            style: "color: #888; font-size: 0.8em;",
                            "{msg.timestamp} "
                        }
                        strong {
                            style: "color: #4da6ff;",
                            "{msg.sender}"
                        }
                        span { ": " }
                        span { "{msg.content}" }
                    }
                }
            }

            // Input area
            div {
                style: "display: flex; padding: 10px; background: #2d2d2d; border-top: 1px solid #555;",
                input {
                    style: "flex: 1; padding: 10px; border: 1px solid #555; border-radius: 4px; background: #1a1a1a; color: white;",
                    placeholder: "Type a message...",
                    value: "{input_text}",
                    oninput: move |evt| input_text.set(evt.value().clone()),
                    onkeydown: move |evt| {
                        if evt.key() == Key::Enter && !input_text.read().is_empty() {
                            let new_msg = Message {
                                id: messages.read().len(),
                                sender: "You".to_string(),
                                content: input_text.read().clone(),
                                timestamp: "Now".to_string(),
                            };
                            messages.write().push(new_msg);
                            input_text.set(String::new());
                        }
                    },
                }
                button {
                    style: "padding: 10px 20px; margin-left: 10px; background: #4da6ff; color: white; border: none; border-radius: 4px; cursor: pointer;",
                    onclick: move |_| {
                        if !input_text.read().is_empty() {
                            let new_msg = Message {
                                id: messages.read().len(),
                                sender: "You".to_string(),
                                content: input_text.read().clone(),
                                timestamp: "Now".to_string(),
                            };
                            messages.write().push(new_msg);
                            input_text.set(String::new());
                        }
                    },
                    "Send"
                }
            }
        }
    }
}

#[component]
fn peers_view(peers: Signal<Vec<Peer>>) -> Element {
    rsx! {
        div {
            style: "flex: 1; overflow-y: auto; padding: 10px; background: #1a1a1a;",

            div {
                style: "color: #888; padding: 10px; text-align: center;",
                "No peers connected yet."
                br {}
                "Peers will appear here when discovered via mDNS."
            }
        }
    }
}

#[component]
fn log_view() -> Element {
    let mut logs = use_signal(|| {
        vec![
            "[12:00:00] Application started".to_string(),
            "[12:00:01] Network initialized".to_string(),
            "[12:00:02] Listening for peers...".to_string(),
        ]
    });

    rsx! {
        div {
            style: "flex: 1; overflow-y: auto; padding: 10px; background: #1a1a1a; font-family: monospace;",

            for (i, log) in logs.read().iter().enumerate() {
                div {
                    key: "{i}",
                    style: "padding: 4px; color: #ddd; font-size: 0.9em;",
                    "{log}"
                }
            }
        }
    }
}
