# Build Feature Sizes

This document shows the binary sizes for different feature combinations.

## Feature Flags

| Feature | Description |
|---------|-------------|
| `basic` | libp2p with TCP, yamux, gossipsub, noise (minimum viable) |
| `mdns` | Local network peer discovery |
| `quic` | UDP-based QUIC transport |
| `tracing` | Debug logging and tracing support |

## Size Table

| Features | Uncompressed | Compressed (UPX) |
|----------|--------------|------------------|
| `basic` | 2.3MB | 741KB |
| `mdns` | 2.6MB | 841KB |
| `tracing` | 2.6MB | 825KB |
| `mdns,tracing` | 3.0MB | 930KB |
| `quic` | 3.4MB | 1.3MB |
| `quic,mdns` | 3.7MB | 1.4MB |
| `quic,tracing` | 3.9MB | 1.4MB |
| `quic,mdns,tracing` | 4.3MB | 1.5MB |

## Usage

```bash
# Minimum build (no mdns, no tracing)
target=x86_64-unknown-linux-gnu features=basic bash build_release.sh

# Full build (default)
target=x86_64-unknown-linux-gnu bash build_release.sh

# Custom combinations
target=x86_64-unknown-linux-gnu features="quic,mdns" bash build_release.sh
target=x86_64-unknown-linux-gnu features="mdns,tracing" bash build_release.sh
```

## Notes

- All features are independent and can be combined freely
- Default build includes: `basic`, `mdns`, `tracing`
- `quic` adds significant size (~1.7MB uncompressed) due to QUIC protocol stack
- UPX compression achieves ~30-35% size reduction