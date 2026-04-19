            }
        }
    }


        #[derive(Debug, Clone)]
        pub struct RunningState {
            pub messages: VecDeque<(String, Option<String>)>,
            pub dm_messages: HashMap<String, VecDeque<String>>,
            pub peers: VecDeque<(String, String, String)>,
            pub dynamic_tabs: DynamicTabs,
            pub active_tab: usize,
            pub dm_inputs: HashMap<String, TextArea<'static>>,
            pub chat_input: TextArea<'static>,
            pub concurrent_peers: usize,
            pub peer_selection: usize,
            pub debug_scroll_offset: usize,
            pub debug_auto_scroll: bool,
            pub chat_scroll_offset: usize,
            pub chat_auto_scroll: bool,
            pub own_nickname: String,
            pub local_nicknames: HashMap<String, String>,
            pub received_nicknames: HashMap<String, String>,
            pub unread_broadcasts: u32,
            pub unread_dms: HashMap<String, u32>,
            pub topic_str: String,
            pub logs: Arc<Mutex<VecDeque<String>>>,
        }

        impl RunningState {
            pub fn new(
                topic_str: String,
                logs: Arc<Mutex<VecDeque<String>>>,
                own_nickname: String,
                local_nicknames: HashMap<String, String>,
                received_nicknames: HashMap<String, String>,
                initial_messages: VecDeque<(String, Option<String>)>,
                initial_peers: VecDeque<(String, String, String)>,
            ) -> Self {
                Self {
                    messages: initial_messages,
                    dm_messages: HashMap::new(),
                    peers: initial_peers,
                    dynamic_tabs: DynamicTabs::new(),
                    active_tab: 0,
                    dm_inputs: HashMap::new(),
                    chat_input: TextArea::default(),
                    concurrent_peers: 0,
                    peer_selection: 0,
                    debug_scroll_offset: 0,
                    debug_auto_scroll: true,
                    chat_scroll_offset: 0,
                    chat_auto_scroll: true,
                    own_nickname,
                    local_nicknames,
                    received_nicknames,
                    unread_broadcasts: 0,
                    unread_dms: HashMap::new(),
                    topic_str,
                    logs,
                }
            }
        }

        #[derive(Debug)]
        pub enum UiCommand {
            ShowMessage(String, Option<String>),
            UpdatePeers(VecDeque<(String, String, String)>),
            AddDmTab(String),
            RemoveDmTab(String),
            SetActiveTab(usize),
            ShowDmMessages(String, VecDeque<String>),
            ClearInput,
            SetChatInput(String),
            ToggleMouseCapture,
            Exit,
        }

        #[derive(Debug)]
        pub enum SwarmCommand {
            SendBroadcast(String),
            SendDm(String, libp2p::PeerId, DirectMessage),
            SetNickname(String),
            SetPeerNickname(String, String),
        }

        #[derive(Debug)]
        pub enum InputEvent {
            Key(Event),
            Mouse(Event),
        }

        #[derive(Debug)]
        pub enum SwarmEvent {
            MessageReceived(String, String, Option<String>),
            PeerConnected(libp2p::PeerId),
            PeerDisconnected(libp2p::PeerId),
            ListenAddr(String),
            ConnectionEstablished(libp2p::PeerId),
            ConnectionClosed(libp2p::PeerId),
            Dialing(libp2p::PeerId),
            OutgoingConnectionError(libp2p::PeerId, String),
            ExpiredListenAddr(String),
            ListenerError(usize, String),
            ListenerClosed(usize, String, Vec<libp2p::Multiaddr>),
            IncomingConnection,
            IncomingConnectionError(usize, String, libp2p::Multiaddr, libp2p::Multiaddr),
            MdnsDiscovered(Vec<(libp2p::PeerId, libp2p::Multiaddr)>),
            MdnsExpired(Vec<(libp2p::PeerId, libp2p::Multiaddr)>),
            GossipsubMessage { propagation_source: libp2p::PeerId, message: gossipsub::Message },
            RequestResponseRequest { peer: libp2p::PeerId, message: libp2p::request_response::Message<DirectMessage, ()>, channel: libp2p::request_response::Channel<DirectMessage, ()> },
            RequestResponseResponse { request_id: libp2p::request_response::RequestId, response: DirectMessage },
        }

        pub type UiCommandTx = mpsc::Sender<UiCommand>;
        pub type UiCommandRx = mpsc::Receiver<UiCommand>;
        pub type SwarmCommandTx = mpsc::Sender<SwarmCommand>;
        pub type SwarmCommandRx = mpsc::Receiver<SwarmCommand>;
        pub type InputEventTx = mpsc::Sender<InputEvent>;
        pub type InputEventRx = mpsc::Receiver<InputEvent>;
        pub type SwarmEventTx = mpsc::Sender<SwarmEvent>;
        pub type SwarmEventRx = mpsc::Receiver<SwarmEvent>;

#[cfg(feature = "tui")]
#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    init_logging();

    let logs = std::sync::Arc::new(std::sync::Mutex::new(VecDeque::new()));
    let logs_callback = logs.clone();
    p2p_app::logging::set_tui_log_callback(move |msg| {
        if let Ok(mut l) = logs_callback.lock() {
            l.push_back(msg);
            if l.len() > 1000 {
                l.pop_front();
            }
        }
    });

    let topic = gossipsub::IdentTopic::new("test-net");

    let network_size = match p2p_app::get_network_size() {
        Ok(size) => {
            eprintln!("Network size detected: {:?}", size);
            size
        }
        Err(e) => {
            eprintln!(
                "Could not determine network size, defaulting to Small: {}",
                e
            );
            p2p_app::NetworkSize::Small
        }
    };

    let mut swarm = {
        let base = libp2p::SwarmBuilder::with_existing_identity(p2p_app::get_libp2p_identity()?)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )?;

        #[cfg(feature = "quic")]
        let swarm = base
            .with_quic()
            .with_behaviour(|key| Ok(build_behaviour(key, network_size)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        #[cfg(not(feature = "quic"))]
        let swarm = base
            .with_behaviour(|key| Ok(build_behaviour(key, network_size)))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        swarm
    };

    let listen_addr: libp2p::Multiaddr = "/ip4/0.0.0.0/tcp/0".parse()?;
    swarm.listen_on(listen_addr)?;

    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;

    tui::run_tui(swarm, "test-net".to_string(), logs).await
}
