# Build Feature Sizes

This document shows the binary sizes for different feature combinations.

## Feature Flags

| Feature | Description |
|---------|-------------|
| `basic` | libp2p with TCP, yamux, gossipsub, noise (minimum viable) |
| `mdns` | Local network peer discovery |
| `quic` | UDP-based QUIC transport |
| `tracing` | Debug logging and tracing support |
| `sqlite_bundled` | Embed SQLite library (no system dependency) |

## Size Table

| Features | Uncompressed | Compressed (UPX) |
|----------|--------------|------------------|
| `basic` | 2.3MB | 741KB |
| `mdns` | 2.6MB | 841KB |
| `tracing` | 2.6MB | 815KB |
| `mdns,tracing` | 2.9MB | 920KB |
| `quic` | 3.4MB | 1.3MB |
| `quic,mdns` | 3.7MB | 1.4MB |
| `quic,tracing` | 3.8MB | 1.4MB |
| `quic,mdns,tracing` | 4.1MB | 1.5MB |
| `sqlite_bundled` | 3.3MB | 1.2MB |
| `mdns,sqlite_bundled` | 3.6MB | 1.3MB |
| `tracing,sqlite_bundled` | 3.5MB | 1.3MB |
| `quic,sqlite_bundled` | 4.4MB | 1.7MB |
| `quic,mdns,sqlite_bundled` | 4.7MB | 1.8MB |
| `quic,mdns,tracing,sqlite_bundled` | 5.2MB | 1.9MB |

## Usage

```bash
# Minimum build (no mdns, no tracing)
target=x86_64-unknown-linux-gnu features=basic bash build_release.sh

# Full build (default)
target=x86_64-unknown-linux-gnu bash build_release.sh

# Custom combinations
target=x86_64-unknown-linux-gnu features="quic,mdns" bash build_release.sh
target=x86_64-unknown-linux-gnu features="sqlite_bundled" bash build_release.sh
target=x86_64-unknown-linux-gnu features="quic,mdns,tracing,sqlite_bundled" bash build_release.sh
```

## Notes

- All features are independent and can be combined freely
- Default build includes: `basic`, `mdns`, `tracing`
- `quic` adds significant size (~1.1MB uncompressed) due to QUIC protocol stack
- `sqlite_bundled` adds ~1MB uncompressed for embedded SQLite
- UPX compression achieves ~30-35% size reduction