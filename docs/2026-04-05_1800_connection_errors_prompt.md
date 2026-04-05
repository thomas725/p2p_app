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