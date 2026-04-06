Starting two peers simultanously gives me BrokenPipe and TimedOut errors, and the first started peer takes 15 seconds to detect the one started just 1/10th of a second after the first. Please analyze / debug & fix.

```log
[18:22:05.572] Using database: peer1.db
[18:22:05.572] Our peer ID: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:22:05.572] Loaded 17 messages from database
[18:22:05.573] Loaded 1 peers from database
[18:22:05.573] Network size: Small (0-3 peers avg) - optimized for low latency
[18:22:05.573] Listening on: /ip4/127.0.0.1/tcp/38671
[18:22:05.580] Listening on: /ip4/10.201.0.2/tcp/38671
[18:22:05.586] Listening on: /ip4/127.0.0.1/udp/44726/quic-v1
[18:22:05.590] Listening on: /ip4/10.201.0.2/udp/44726/quic-v1
[18:22:30.914] mDNS discovered: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3 at /ip4/10.201.0.2/tcp/42491/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[18:22:30.942] Attempting to dial: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[18:22:30.943] Dial initiated: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[18:22:30.943] mDNS discovered: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3 at /ip4/10.201.0.2/udp/39167/quic-v1/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[18:22:30.947] Attempting to dial: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[18:22:30.948] Dial initiated: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[18:22:30.960] Incoming connection: ConnectionId(5) from /ip4/10.201.0.2/tcp/38671
[18:22:30.964] Incoming connection error: ConnectionId(5) from /ip4/10.201.0.2/tcp/38671 to /ip4/10.201.0.2/tcp/57464 (peer: None): Transport(Other(Custom { kind: Other, error: Other(Left(Left(Right(Select(ProtocolError(IoError(Os { code: 32, kind: BrokenPipe, message: "Broken pipe" }))))))) }))
[18:22:30.968] Listener error: ListenerId(1) - Custom { kind: Other, error: Other(Right(Connection(ConnectionError(TimedOut)))) }
--------------------------------------------
[18:22:15.582] Using database: peer2.db
[18:22:15.582] Our peer ID: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[18:22:15.582] Loaded 23 messages from database
[18:22:15.582] Loaded 2 peers from database
[18:22:15.583] Network size: Small (0-3 peers avg) - optimized for low latency
[18:22:15.583] Listening on: /ip4/127.0.0.1/tcp/42491
[18:22:15.590] mDNS discovered: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1 at /ip4/10.201.0.2/udp/44726/quic-v1/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:22:15.604] Attempting to dial: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:22:15.605] Dial initiated: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:22:15.605] mDNS discovered: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1 at /ip4/10.201.0.2/tcp/38671/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:22:15.608] Attempting to dial: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:22:15.608] Dial initiated: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:22:15.623] Listening on: /ip4/10.201.0.2/tcp/42491
[18:22:15.629] Listening on: /ip4/127.0.0.1/udp/39167/quic-v1
[18:22:15.634] Listening on: /ip4/10.201.0.2/udp/39167/quic-v1
```

----

okey, with your modifications I now only see a single connection attempt, and no errors, but the first started peer never sees the second even after waiting ~10 minutes:

```log
[18:57:45.160] Using database: peer1.db
[18:57:45.160] Our peer ID: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:57:45.161] Loaded 17 messages from database
[18:57:45.161] Loaded 1 peers from database
[18:57:45.161] Network size: Small (0-3 peers avg) - optimized for low latency
[18:57:45.161] Listening on: /ip4/127.0.0.1/tcp/41621
[18:57:45.169] Listening on: /ip4/10.201.0.2/tcp/41621
[18:57:45.174] Listening on: /ip4/127.0.0.1/udp/57695/quic-v1
[18:57:45.179] Listening on: /ip4/10.201.0.2/udp/57695/quic-v1
----
[18:57:51.059] Using database: peer2.db
[18:57:51.059] Our peer ID: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[18:57:51.059] Loaded 23 messages from database
[18:57:51.060] Loaded 2 peers from database
[18:57:51.060] Network size: Small (0-3 peers avg) - optimized for low latency
[18:57:51.060] Listening on: /ip4/127.0.0.1/tcp/45923
[18:57:51.067] mDNS discovered: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1 at /ip4/10.201.0.2/udp/57695/quic-v1/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:57:51.078] Attempting to dial: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:57:51.078] Dial initiated: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:57:51.078] mDNS discovered: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1 at /ip4/10.201.0.2/tcp/41621/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[18:57:51.096] Listening on: /ip4/10.201.0.2/tcp/45923
[18:57:51.101] Listening on: /ip4/127.0.0.1/udp/59271/quic-v1
[18:57:51.106] Listening on: /ip4/10.201.0.2/udp/59271/quic-v1
```

----

that dialing of known peers is a great idea, please limit it to the last N (maybe 10?) seen peers to not bog the network down with hundreds of requests should we ever reach such big network sizes. Sadly we still see errors. Can you please look if we can get more details to log for those errors, and also research the web for possible causes:

```log
[19:15:31.778] Using database: peer1.db
[19:15:31.778] Our peer ID: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:15:31.779] Loaded 17 messages from database
[19:15:31.779] Loaded 1 peers from database
[19:15:31.779] Dialing known peer: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3 at /ip4/10.201.0.2/udp/39167/quic-v1/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:15:31.779] Network size: Small (0-3 peers avg) - optimized for low latency
[19:15:31.780] Listening on: /ip4/127.0.0.1/tcp/33025
[19:15:31.788] Listening on: /ip4/10.201.0.2/tcp/33025
[19:15:31.793] Listening on: /ip4/127.0.0.1/udp/60125/quic-v1
[19:15:31.799] Listening on: /ip4/10.201.0.2/udp/60125/quic-v1
[19:15:43.937] mDNS discovered: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3 at /ip4/10.201.0.2/tcp/46853/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:15:43.948] mDNS discovered: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3 at /ip4/10.201.0.2/udp/59443/quic-v1/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:15:43.973] Incoming connection: ConnectionId(6) from /ip4/10.201.0.2/tcp/33025
[19:15:43.979] Incoming connection: ConnectionId(7) from /ip4/0.0.0.0/udp/60125/quic-v1
[19:15:43.984] Incoming connection error: ConnectionId(7) from /ip4/0.0.0.0/udp/60125/quic-v1 to /ip4/10.201.0.2/udp/59443/quic-v1 (peer: None): Transport(Other(Custom { kind: Other, error:
Other(Right(Connection(ConnectionError(ConnectionClosed(ConnectionClose { error_code: APPLICATION_ERROR, frame_type: None, reason: b"" }))))) }))
----
[19:15:36.978] Using database: peer2.db
[19:15:36.978] Our peer ID: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:15:36.978] Loaded 23 messages from database
[19:15:36.978] Loaded 2 peers from database
[19:15:36.978] Dialing known peer: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1 at /ip4/10.201.0.2/tcp/41621/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:15:36.978] Dialing known peer: 12D3KooWBLEWjn5z8hYMwktTHq9gJ3b2eDZA5EeZw4bLpiDnC9as at /ip4/10.201.0.2/udp/59921/quic-v1/p2p/12D3KooWBLEWjn5z8hYMwktTHq9gJ3b2eDZA5EeZw4bLpiDnC9as
[19:15:36.979] Network size: Small (0-3 peers avg) - optimized for low latency
[19:15:36.987] Listening on: /ip4/127.0.0.1/tcp/46853
[19:15:36.992] mDNS discovered: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1 at /ip4/10.201.0.2/tcp/33025/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:15:37.002] mDNS discovered: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1 at /ip4/10.201.0.2/udp/60125/quic-v1/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:15:37.021] Listening on: /ip4/10.201.0.2/tcp/46853
[19:15:37.026] Listening on: /ip4/127.0.0.1/udp/59443/quic-v1
[19:15:37.031] Listening on: /ip4/10.201.0.2/udp/59443/quic-v1
```

Also a nice feature you could add would be sorting the peers list by last seen timestamp showning latest ones first.

----

nice one! next we'll need to add the ability to scroll the debug tab output and we need to improve the formatting, maybe filter out or unsubscribe from irrelevant libp2p tracing events or remove duplicate manual logging we have added in our own code but can now see in the libp2p events anyway:

```log
[19:52:40.428] Using database: peer1.db
[19:52:40.428] Our peer ID: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:40.429] Loaded 17 messages from database
[19:52:40.429] Loaded 1 peers from database (dialing top 10 by last_seen)
[19:52:40.429] Network size: Small (0-3 peers avg) - optimized for low latency
[19:52:40.430]  INFO+0200 [0m Swarm::poll:         libp2p_mdns::behaviour::iface:                 creating instance on iface address address=10.201.0.2
[19:52:40.430] DEBUG+0200 [0m Swarm::poll:         libp2p_tcp:                 New listen address address=/ip4/127.0.0.1/tcp/44137
[19:52:40.430] DEBUG+0200 [0m Swarm::poll:gs       libp2p_swarm:                 New listener address listener=ListenerId(2)                 address=/ip4/127.0.0.1/tcp/44137
[19:52:40.430] Listening on: /ip4/127.0.0.1/tcp/44137
[19:52:40.440] DEBUG+0200 [0m Swarm::poll:         libp2p_tcp:                 New listen address address=/ip4/10.201.0.2/tcp/44137
[19:52:40.441] DEBUG+0200 [0m Swarm::poll:gf       libp2p_swarm:                 New listener address listener=ListenerId(2)                 address=/ip4/10.201.0.2/tcp/44137
[19:52:40.441] Listening on: /ip4/10.201.0.2/tcp/44137
[19:52:40.446] DEBUG+0200 [0m Swarm::poll:234      libp2p_quic::transport:                 New listen address address=/ip4/127.0.0.1/udp/55352/quic-v1
[19:52:40.446] DEBUG+0200 [0m Swarm::poll:         libp2p_swarm:                 New listener address listener=ListenerId(1)                 address=/ip4/127.0.0.1/udp/55352/quic-v1
[19:52:40.446] Listening on: /ip4/127.0.0.1/udp/55352/quic-v1
[19:52:40.451] DEBUG+0200 [0m Swarm::poll:2 0924   libp2p_quic::transport:                 New listen address address=/ip4/10.201.0.2/udp/55352/quic-v1
[19:52:40.451] DEBUG+0200 [0m Swarm::poll:4        libp2p_swarm:                 New listener address listener=ListenerId(1)                 address=/ip4/10.201.0.2/udp/55352/quic-v1
[19:52:40.451] Listening on: /ip4/10.201.0.2/udp/55352/quic-v1
[19:52:43.497]       INFO     Swarm::poll:         libp2p_mdns::behaviour:                 discovered peer on address peer=12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
address=/ip4/10.201.0.2/tcp/46125/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.497]       INFO     Swarm::poll:         libp2p_mdns::behaviour:                 discovered peer on address peer=12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
address=/ip4/10.201.0.2/udp/52206/quic-v1/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.497] mDNS discovered: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3 at /ip4/10.201.0.2/tcp/46125/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.524] DEBUG          libp2p_swarm:                 discarding addresses from `NetworkBehaviour` because `DialOpts::extend_addresses_through_behaviour is `false` for connection connection=1
discarded_addresses_count=2
[19:52:43.524] DEBUG          libp2p_tcp:                 dialing address address=10.201.0.2:46125
[19:52:43.524] DEBUG          libp2p_gossipsub::behaviour:                 Adding explicit peer peer=12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.524] DEBUG          libp2p_gossipsub::behaviour:                 Connecting to explicit peer peer=12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.524] mDNS discovered: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3 at /ip4/10.201.0.2/udp/52206/quic-v1/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.524] DEBUG          Transport::dial:         multistream_select::dialer_select:                 Dialer: Proposed protocol: /noise
address=/ip4/10.201.0.2/tcp/46125/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.524] DEBUG          Transport::dial:         multistream_select::dialer_select:                 Dialer: Expecting proposed protocol: /noise
address=/ip4/10.201.0.2/tcp/46125/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.532] DEBUG          libp2p_swarm:                 discarding addresses from `NetworkBehaviour` because `DialOpts::extend_addresses_through_behaviour is `false` for connection connection=3
discarded_addresses_count=2
[19:52:43.532] DEBUG          libp2p_core::transport::choice:                 Failed to dial using libp2p_core::transport::map::Map<libp2p_quic::transport::GenTransport<libp2p_quic::provider::tokio::Provider>, libp2p::builder::phase::quic::<impl  
libp2p::builder::SwarmBuilder<libp2p::builder::phase::provider::Tokio,
libp2p::builder::phase::quic::QuicPhase<libp2p_core::transport::map::Map<libp2p_core::transport::upgrade::Multiplexed<libp2p_core::transport::and_then::AndThen<libp2p_core::transport::and_then::AndThen<libp2p_tcp::Transport<libp2p_tcp::provider::t
okio::Tcp>, libp2p_core::transport::upgrade::Builder<libp2p_tcp::Transport<libp2p_tcp::provider::tokio::Tcp>>::authenticate<libp2p_tcp::provider::tokio::TcpStream,
libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>, libp2p_noise::Config, libp2p_noise::Error>::{{closure}}>,
libp2p_core::transport::upgrade::Authenticated<libp2p_core::transport::and_then::AndThen<libp2p_tcp::Transport<libp2p_tcp::provider::tokio::Tcp>,
libp2p_core::transport::upgrade::Builder<libp2p_tcp::Transport<libp2p_tcp::provider::tokio::Tcp>>::authenticate<libp2p_tcp::provider::tokio::TcpStream,
libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>, libp2p_noise::Config,
libp2p_noise::Error>::{{closure}}>>::multiplex<libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>,
libp2p_yamux::Muxer<multistream_select::negotiated::Negotiated<libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>>>, libp2p_yamux::Config, std::io::error::Error>::{{closure}}>>,
libp2p::builder::phase::tcp::<impl libp2p::builder::SwarmBuilder<libp2p::builder::phase::provider::Tokio, libp2p::builder::phase::tcp::TcpPhase>>::with_tcp<libp2p_noise::Config::new,
libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>, libp2p_noise::Error, <libp2p_yamux::Config as core::default::Default>::default,
libp2p_yamux::Muxer<multistream_select::negotiated::Negotiated<libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>>>,
std::io::error::Error>::{{closure}}>>>>::with_quic_config<core::convert::identity<libp2p_quic::config::Config>>::{{closure}}> address=/ip4/10.201.0.2/udp/52206/quic-v1/p2p/12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.532] DEBUG          libp2p_gossipsub::behaviour:                 Adding explicit peer peer=12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.532] DEBUG          libp2p_gossipsub::behaviour:                 Connecting to explicit peer peer=12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:43.553] DEBUG          Swarm::poll:         libp2p_tcp:                 Incoming connection from remote at local remote_address=/ip4/10.201.0.2/tcp/45742                 local_address=/ip4/10.201.0.2/tcp/44137
[19:52:43.553] Incoming connection ConnectionId(5) from /ip4/10.201.0.2/tcp/44137
[19:52:43.554] DEBUG          new_incoming_connection:         multistream_select::listener_select:                 Listener: confirming protocol: /noise remote_addr=/ip4/10.201.0.2/tcp/45742                     id=5
[19:52:43.554] DEBUG          new_incoming_connection:         multistream_select::listener_select:                 Listener: sent confirmed protocol: /noise remote_addr=/ip4/10.201.0.2/tcp/45742                     id=5
[19:52:43.561] Incoming connection ConnectionId(6) from /ip4/0.0.0.0/udp/55352/quic-v1
[19:52:46.507] DEBUG          Swarm::poll:         libp2p_gossipsub::behaviour:                 HEARTBEAT: Mesh low. Topic contains: 0 needs: 1 topic=test-net
[19:52:46.507] DEBUG          Swarm::poll:         libp2p_gossipsub::behaviour:                 RANDOM PEERS: Got 0 peers
[19:52:46.507] DEBUG          Swarm::poll:         libp2p_gossipsub::behaviour:                 Updating mesh, new mesh: {}
[19:52:46.508] DEBUG          Swarm::poll:         libp2p_swarm:                 Connection established peer=12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3                 endpoint=Listener                 { local_addr:
/ip4/0.0.0.0/udp/55352/quic-v1, send_back_addr: /ip4/10.201.0.2/udp/52206/quic-v1 } total_peers=1
[19:52:46.508] DEBUG          Swarm::poll:         libp2p_gossipsub::behaviour:                 New peer connected peer=12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:46.508] Connection established: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3 (conn: ConnectionId(6))
[19:52:46.508] Concurrent peers: 1
[19:52:46.508] DEBUG          libp2p_gossipsub::behaviour:                 Adding explicit peer peer=12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:46.508] DEBUG          new_established_connection:Connection::poll:                 multistream_select::dialer_select:                 Dialer: Proposed protocol: /meshsub/1.2.0

--------------

[19:52:42.008] Using database: peer2.db
[19:52:42.008] Our peer ID: 12D3KooWRGLthXYB1JWz1kttzeV4W8pgNcUvT65gSSRZAVAFwVd3
[19:52:42.009] Loaded 23 messages from database
[19:52:42.010] Loaded 2 peers from database (dialing top 10 by last_seen)
[19:52:42.010] Network size: Small (0-3 peers avg) - optimized for low latency
[19:52:42.010]  INFO+0200 [0m Swarm::poll:         libp2p_mdns::behaviour::iface:                 creating instance on iface address address=10.201.0.2
[19:52:42.011] DEBUG+0200 [0m Swarm::poll:         libp2p_tcp:                 New listen address address=/ip4/127.0.0.1/tcp/46125
[19:52:42.011] 6:34mDEBUG [0m Swarm::poll:         libp2p_swarm:                 New listener address listener=ListenerId(2)                 address=/ip4/127.0.0.1/tcp/46125
[19:52:42.011] Listening on: /ip4/127.0.0.1/tcp/46125
[19:52:42.018] 6:32m INFO [0m Swarm::poll:56       libp2p_mdns::behaviour:                 discovered peer on address peer=12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
address=/ip4/10.207 +0200 [L7VYTKV1.0.2/udp/55352/quic-v1/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.018] 6:32m INFO [0m Swarm::poll:         libp2p_mdns::behaviour:                 discovered peer on address peer=12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
address=/ip4/10.207 +0200 [L7VYTKV1.0.2/tcp/44137/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.018] mDNS discovered: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1 at /ip4/10.201.0.2/udp/55352/quic-v1/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.025] DEBUG+0200 [0m libp2p_swarm:                 discarding addresses from `NetworkBehaviour` because `DialOpts::extend_addresses_through_behaviour is `false` for connection connection=1
discarded_addresses_count=2You] test 2 0924
[19:52:42.025] DEBUG+0200 [0m libp2p_core::transport::choice:                 Failed to dial using libp2p_core::transport::map::Map<libp2p_quic::transport::GenTransport<libp2p_quic::provider::tokio::Provider>, libp2p::builder::phase::quic::<impl  
libp2p::builder::SwarmBuilder<libp2p::builder::phase::provider::Tokio,
libp2p::builder::phase::quic::QuicPhase<libp2p_core::transport::map::Map<libp2p_core::transport::upgrade::Multiplexed<libp2p_core::transport::and_then::AndThen<libp2p_core::transport::and_then::AndThen<libp2p_tcp::Transport<libp2p_tcp::provider::t
okio::Tcp>, libp2p_core::transport::upgrade::Builder<libp2p_tcp::Transport<libp2p_tcp::provider::tokio::Tcp>>::authenticate<libp2p_tcp::provider::tokio::TcpStream,
libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>, libp2p_noise::Config, libp2p_noise::Error>::{{closure}}>,
libp2p_core::transport::upgrade::Authenticated<libp2p_core::transport::and_then::AndThen<libp2p_tcp::Transport<libp2p_tcp::provider::tokio::Tcp>,
libp2p_core::transport::upgrade::Builder<libp2p_tcp::Transport<libp2p_tcp::provider::tokio::Tcp>>::authenticate<libp2p_tcp::provider::tokio::TcpStream,
libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>, libp2p_noise::Config,
libp2p_noise::Error>::{{closure}}>>::multiplex<libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>,
libp2p_yamux::Muxer<multistream_select::negotiated::Negotiated<libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>>>, libp2p_yamux::Config, std::io::error::Error>::{{closure}}>>,
libp2p::builder::phase::tcp::<impl libp2p::builder::SwarmBuilder<libp2p::builder::phase::provider::Tokio, libp2p::builder::phase::tcp::TcpPhase>>::with_tcp<libp2p_noise::Config::new,
libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>, libp2p_noise::Error, <libp2p_yamux::Config as core::default::Default>::default,
libp2p_yamux::Muxer<multistream_select::negotiated::Negotiated<libp2p_noise::io::Output<multistream_select::negotiated::Negotiated<libp2p_tcp::provider::tokio::TcpStream>>>>,
std::io::error::Error>::{{closure}}>>>>::with_quic_config<core::convert::identity<libp2p_quic::config::Config>>::{{closure}}> address=/ip4/10.201.0.2/udp/55352/quic-v1/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.025] DEBUG          libp2p_gossipsub::behaviour:                 Adding explicit peer peer=12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.025] DEBUG          libp2p_gossipsub::behaviour:                 Connecting to explicit peer peer=12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.025] mDNS discovered: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1 at /ip4/10.201.0.2/tcp/44137/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.034] DEBUG          libp2p_swarm:                 discarding addresses from `NetworkBehaviour` because `DialOpts::extend_addresses_through_behaviour is `false` for connection connection=3
discarded_addresses_count=2
[19:52:42.034] DEBUG          libp2p_tcp:                 dialing address address=10.201.0.2:44137
[19:52:42.034] DEBUG          libp2p_gossipsub::behaviour:                 Adding explicit peer peer=12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.034] DEBUG          libp2p_gossipsub::behaviour:                 Connecting to explicit peer peer=12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.035] DEBUG          Transport::dial:         multistream_select::dialer_select:                 Dialer: Proposed protocol: /noise
address=/ip4/10.201.0.2/tcp/44137/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.035] DEBUG          Transport::dial:         multistream_select::dialer_select:                 Dialer: Expecting proposed protocol: /noise
address=/ip4/10.201.0.2/tcp/44137/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:42.050] DEBUG          Swarm::poll:         libp2p_tcp:                 New listen address address=/ip4/10.201.0.2/tcp/46125
[19:52:42.050] DEBUG          Swarm::poll:         libp2p_swarm:                 New listener address listener=ListenerId(2)                 address=/ip4/10.201.0.2/tcp/46125
[19:52:42.050] Listening on: /ip4/10.201.0.2/tcp/46125
[19:52:42.055] DEBUG          Swarm::poll:         libp2p_quic::transport:                 New listen address address=/ip4/127.0.0.1/udp/52206/quic-v1
[19:52:42.055] DEBUG          Swarm::poll:         libp2p_swarm:                 New listener address listener=ListenerId(1)                 address=/ip4/127.0.0.1/udp/52206/quic-v1
[19:52:42.055] Listening on: /ip4/127.0.0.1/udp/52206/quic-v1
[19:52:42.060] DEBUG          Swarm::poll:         libp2p_quic::transport:                 New listen address address=/ip4/10.201.0.2/udp/52206/quic-v1
[19:52:42.060] DEBUG          Swarm::poll:         libp2p_swarm:                 New listener address listener=ListenerId(1)                 address=/ip4/10.201.0.2/udp/52206/quic-v1
[19:52:42.060] Listening on: /ip4/10.201.0.2/udp/52206/quic-v1
[19:52:43.554] DEBUG          Transport::dial:         multistream_select::negotiated:                 Negotiated: Received confirmation for protocol: /noise
address=/ip4/10.201.0.2/tcp/44137/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:43.562] DEBUG          Transport::dial:         multistream_select::dialer_select:                 Dialer: Proposed protocol: /yamux/1.0.0
address=/ip4/10.201.0.2/tcp/44137/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:43.562] DEBUG          Transport::dial:         multistream_select::dialer_select:                 Dialer: Expecting proposed protocol: /yamux/1.0.0
address=/ip4/10.201.0.2/tcp/44137/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:43.562] DEBUG          Transport::dial:         yamux::connection:                 new connection: 75cb6380 (Client) address=/ip4/10.201.0.2/tcp/44137/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:51.533] DEBUG          Swarm::poll:         libp2p_gossipsub::behaviour:                 HEARTBEAT: Mesh low. Topic contains: 0 needs: 1 topic=test-net
[19:52:51.533] DEBUG          Swarm::poll:         libp2p_gossipsub::behaviour:                 RANDOM PEERS: Got 0 peers
[19:52:51.533] DEBUG          Swarm::poll:         libp2p_gossipsub::behaviour:                 Updating mesh, new mesh: {}
[19:52:51.534] DEBUG          Swarm::poll:         libp2p_swarm:                 Connection established peer=12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1                 endpoint=Dialer                 { address:
/ip4/10.201.0.2/tcp/44137/p2p/12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1, role_override: Dialer, port_use: Reuse } total_peers=1
[19:52:51.534] DEBUG          Swarm::poll:         libp2p_gossipsub::behaviour:                 New peer connected peer=12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1
[19:52:51.534] Connection established: 12D3KooWRCx4pgegk5AkVrB4dZA41LSpGU9X96HACsEaL7VYTKV1 (conn: ConnectionId(3))
[19:52:51.534] Concurrent peers: 1
```

----

analyze and debug why we don't see any connection succesful in our logs, even though the second started instance detects and tries to connect to the first one. Is it a problem that it detects and tries to connect to the other instance before it starts listening on all it's network devces or is that just an logging timing artefact? Try to reproduce this with an integration test and iterate using it.

```log
[08:27:17.505] Using database: host1.db
[08:27:17.506] Our peer ID: 12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw
[08:27:17.507] Loaded 0 messages from database
[08:27:17.508] Loaded 0 peers from database (dialing top 10 by last_seen)
[08:27:17.508] Network size: Small (0-3 peers avg) - optimized for low latency
[08:27:17.510] DEBUG Swarm::poll: libp2p_tcp: New listen address address=/ip4/127.0.0.1/tcp/32925
[08:27:17.510] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(2) address=/ip4/127.0.0.1/tcp/32925
[08:27:17.543] DEBUG Swarm::poll: libp2p_tcp: New listen address address=/ip4/192.168.16.99/tcp/32925
[08:27:17.543] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(2) address=/ip4/192.168.16.99/tcp/32925
[08:27:17.554] DEBUG Swarm::poll: libp2p_tcp: New listen address address=/ip4/172.17.0.1/tcp/32925
[08:27:17.554] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(2) address=/ip4/172.17.0.1/tcp/32925
[08:27:17.569] DEBUG Swarm::poll: libp2p_tcp: New listen address address=/ip4/172.30.0.1/tcp/32925
[08:27:17.570] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(2) address=/ip4/172.30.0.1/tcp/32925
[08:27:17.586] DEBUG Swarm::poll: libp2p_tcp: New listen address address=/ip4/172.18.0.1/tcp/32925
[08:27:17.587] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(2) address=/ip4/172.18.0.1/tcp/32925
[08:27:17.602] DEBUG Swarm::poll: libp2p_tcp: New listen address address=/ip4/10.201.0.1/tcp/32925
[08:27:17.602] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(2) address=/ip4/10.201.0.1/tcp/32925
[08:27:17.616] DEBUG Swarm::poll: libp2p_quic::transport: New listen address address=/ip4/127.0.0.1/udp/46272/quic-v1
[08:27:17.616] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(1) address=/ip4/127.0.0.1/udp/46272/quic-v1
[08:27:17.630] DEBUG Swarm::poll: libp2p_quic::transport: New listen address address=/ip4/192.168.16.99/udp/46272/quic-v1
[08:27:17.630] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(1) address=/ip4/192.168.16.99/udp/46272/quic-v1
[08:27:17.642] DEBUG Swarm::poll: libp2p_quic::transport: New listen address address=/ip4/172.17.0.1/udp/46272/quic-v1
[08:27:17.642] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(1) address=/ip4/172.17.0.1/udp/46272/quic-v1
[08:27:17.660] DEBUG Swarm::poll: libp2p_quic::transport: New listen address address=/ip4/172.30.0.1/udp/46272/quic-v1
[08:27:17.660] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(1) address=/ip4/172.30.0.1/udp/46272/quic-v1
[08:27:17.673] DEBUG Swarm::poll: libp2p_quic::transport: New listen address address=/ip4/172.18.0.1/udp/46272/quic-v1
[08:27:17.673] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(1) address=/ip4/172.18.0.1/udp/46272/quic-v1
[08:27:17.686] DEBUG Swarm::poll: libp2p_quic::transport: New listen address address=/ip4/10.201.0.1/udp/46272/quic-v1
[08:27:17.686] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(1) address=/ip4/10.201.0.1/udp/46272/quic-v1

----

[08:27:22.223] Using database: sanbox2.db
[08:27:22.224] Our peer ID: 12D3KooWNbSRcN5oaq4Y8JphCC8dQfFLPYkjHgoiuntzkTYtXzhX
[08:27:22.224] Loaded 0 messages from database
[08:27:22.225] Loaded 1 peers from database (dialing top 10 by last_seen)
[08:27:22.225] Dialing known peer: 12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw at /ip4/10.201.0.1/tcp/33389/p2p/12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw
[08:27:22.225] DEBUG libp2p_tcp: dialing address address=10.201.0.1:33389
[08:27:22.226] Network size: Small (0-3 peers avg) - optimized for low latency
[08:27:22.227] DEBUG Swarm::poll: libp2p_tcp: New listen address address=/ip4/127.0.0.1/tcp/35015
[08:27:22.227] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(2) address=/ip4/127.0.0.1/tcp/35015
[08:27:22.249] INFO Swarm::poll: libp2p_mdns::behaviour: discovered peer on address peer=12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw address=/ip4/10.201.0.1/tcp/32925/p2p/12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw
[08:27:22.249] INFO Swarm::poll: libp2p_mdns::behaviour: discovered peer on address peer=12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw address=/ip4/10.201.0.1/udp/46272/quic-v1/p2p/12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw
[08:27:22.265] DEBUG libp2p_swarm: discarding addresses from `NetworkBehaviour` because `DialOpts::extend_addresses_through_behaviour is `false` for connection connection=2 discarded_addresses_count=2
[08:27:22.265] DEBUG libp2p_tcp: dialing address address=10.201.0.1:32925
[08:27:22.265] DEBUG libp2p_gossipsub::behaviour: Adding explicit peer peer=12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw
[08:27:22.265] DEBUG libp2p_gossipsub::behaviour: Connecting to explicit peer peer=12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw
[08:27:22.271] DEBUG libp2p_swarm: discarding addresses from `NetworkBehaviour` because `DialOpts::extend_addresses_through_behaviour is `false` for connection connection=4 discarded_addresses_count=2
[08:27:22.271] DEBUG libp2p_gossipsub::behaviour: Adding explicit peer peer=12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw
[08:27:22.271] DEBUG libp2p_gossipsub::behaviour: Connecting to explicit peer peer=12D3KooWKwEn4KjozEt4tzNB9jfG2SC2fj6B6ecnpNfF9EtRikMw
[08:27:22.315] DEBUG Swarm::poll: libp2p_tcp: New listen address address=/ip4/10.201.0.2/tcp/35015
[08:27:22.315] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(2) address=/ip4/10.201.0.2/tcp/35015
[08:27:22.327] DEBUG Swarm::poll: libp2p_quic::transport: New listen address address=/ip4/127.0.0.1/udp/57877/quic-v1
[08:27:22.327] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(1) address=/ip4/127.0.0.1/udp/57877/quic-v1
[08:27:22.341] DEBUG Swarm::poll: libp2p_quic::transport: New listen address address=/ip4/10.201.0.2/udp/57877/quic-v1
[08:27:22.341] DEBUG Swarm::poll: libp2p_swarm: New listener address listener=ListenerId(1) address=/ip4/10.201.0.2/udp/57877/quic-v1
```
