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
