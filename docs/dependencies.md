# Dependencies Documentation

This document lists all project dependencies, their purpose, and update status.

**Last checked:** 2026-04-06T20:00:00Z
**Last updated:** 2026-04-06T20:00:00Z

## Core Dependencies

### libp2p (=0.56.0)

- **Purpose:** Peer-to-peer networking library - handles P2P connections, protocol negotiation, and message routing
- **Features used:** tokio, gossipsub, noise, macros, tcp, yamux, quic, request-response, serde, json
- **Version in use:** 0.56.0
- **Latest version:** 0.56.0
- **Latest release date:** 2025-06-28
- **Status:** Up to date

### tokio (=1.51.0)

- **Purpose:** Async runtime for Rust - provides event-driven, non-blocking I/O for async operations
- **Features used:** macros, rt-multi-thread, io-util, io-std
- **Version in use:** 1.51.0
- **Latest version:** 1.51.0
- **Latest release date:** 2026-04-03
- **Status:** Up to date

### diesel (=2.3.7)

- **Purpose:** ORM and Query Builder for SQLite database operations
- **Features used:** sqlite, returning_clauses_for_sqlite_3_35, chrono
- **Version in use:** 2.3.7
- **Latest version:** 2.3.7
- **Latest release date:** 2026-03-13
- **Status:** Up to date

### libsqlite3-sys (0.36.0, optional)

- **Purpose:** Native SQLite bindings - provides FFI to SQLite library
- **Features used:** bundled (embed SQLite statically)
- **Version in use:** 0.36.0
- **In use release date:** 2025-12-20
- **Latest version:** 0.37.0
- **Latest release date:** 2026-03-15
- **Status:** Behind latest - 0.37.0 has compatibility issues with diesel 2.3.7

### diesel_migrations (=2.3.1)

- **Purpose:** Database migration management for Diesel - embeds and runs SQL migrations at runtime
- **Version in use:** 2.3.1
- **Latest version:** 2.3.1
- **Latest release date:** 2025-11-26
- **Status:** Up to date

### chrono (0.4.44)

- **Purpose:** Date and time library - used for TIMESTAMP columns in database
- **Features used:** clock
- **Version in use:** 0.4.44
- **Latest version:** 0.4.44
- **Latest release date:** 2026-02-23
- **Status:** Up to date

## Networking

### libp2p-quic (=0.13.0)

- **Purpose:** TLS based QUIC transport implementation for libp2p
- **Version in use:** 0.13.0
- **Latest version:** 0.13.0
- **Latest release date:** 2025-06-27
- **Status:** Up to date

### libp2p-identity (0.2.13)

- **Purpose:** Cryptographic keypairs and peer identity management for libp2p
- **Version in use:** 0.2.13
- **Latest version:** 0.2.13
- **Latest release date:** 2023-03-12
- **Status:** Up to date

### libp2p-request-response (0.29)

- **Purpose:** Generic Request/Response protocols - used for direct messages
- **Features used:** json
- **Version in use:** 0.29.0
- **Latest version:** 0.29.0
- **Status:** Up to date

## Logging & Error Handling

### tracing (=0.1.44)

- **Purpose:** Application-level tracing/logging framework (used by libp2p)
- **Version in use:** 0.1.44
- **Latest version:** 0.1.44
- **Latest release date:** 2026-01-09
- **Status:** Up to date

### tracing-subscriber (=0.3.23)

- **Purpose:** Utilities for implementing tracing subscribers - enables formatted log output
- **Version in use:** 0.3.23
- **Latest version:** 0.3.23
- **Latest release date:** 2026-03-13
- **Status:** Up to date

### color-eyre (=0.6.5)

- **Purpose:** Error report handler for colorful, consistent error messages
- **Version in use:** 0.6.5
- **Latest version:** 0.6.5
- **Latest release date:** 2025-05-30
- **Status:** Up to date

## Configuration

### dotenvy (=0.15.7)

- **Purpose:** Loads environment variables from .env file - allows DATABASE_URL configuration
- **Version in use:** 0.15.7
- **Latest version:** 0.15.7
- **Latest release date:** 2023-03-22
- **Status:** Up to date

## UI

### ratatui (=0.28)

- **Purpose:** Terminal UI framework - renders the 4-tab TUI interface
- **Version in use:** 0.28.1
- **In use release date:** 2024-08-25
- **Latest version:** 0.30.0
- **Latest release date:** 2025-12-26
- **Status:** Behind latest - pinned to 0.28.x

### crossterm (=0.5)

- **Purpose:** Terminal manipulation - handles raw mode, alternate screen, key events
- **Version in use:** 0.5.5
- **In use release date:** 2019-01-26
- **Latest version:** 0.29.0
- **Latest release date:** 2025-04-05
- **Status:** Behind latest - pinned to 0.5.x (very old, but compatible with ratatui 0.28)

### atty (=0.2)

- **Purpose:** TTY detection - determines if stdout is a terminal to decide TUI vs headless
- **Version in use:** 0.2.14
- **Latest version:** 0.2.14
- **Status:** Up to date

## Serialization

### serde (1.0)

- **Purpose:** Serialization/deserialization framework - used for DirectMessage JSON codec
- **Features used:** derive
- **Version in use:** 1.0.228
- **Latest version:** 1.0.228
- **Status:** Up to date

### serde_json (1.0)

- **Purpose:** JSON serialization file format - used for request-response message encoding
- **Version in use:** 1.0.149
- **Latest version:** 1.0.149
- **Status:** Up to date

### futures (0.3)

- **Purpose:** Async stream and future utilities - used for StreamExt in event loops
- **Version in use:** 0.3.32
- **Latest version:** 0.3.32
- **Status:** Up to date

## Summary

| Dependency | In Use | In Use Release | Latest | Latest Release | Status |
|------------|--------|-----------------|--------|-----------------|--------|
| libp2p | 0.56.0 | 2025-06-28 | 0.56.0 | 2025-06-28 | OK |
| tokio | 1.51.0 | 2026-04-03 | 1.51.0 | 2026-04-03 | OK |
| diesel | 2.3.7 | 2026-03-13 | 2.3.7 | 2026-03-13 | OK |
| libsqlite3-sys | 0.36.0 | 2025-12-20 | 0.37.0 | 2026-03-15 | Behind* |
| diesel_migrations | 2.3.1 | 2025-11-26 | 2.3.1 | 2025-11-26 | OK |
| chrono | 0.4.44 | 2026-02-23 | 0.4.44 | 2026-02-23 | OK |
| libp2p-identity | 0.2.13 | 2023-03-12 | 0.2.13 | 2023-03-12 | OK |
| libp2p-quic | 0.13.0 | 2025-06-27 | 0.13.0 | 2025-06-27 | OK |
| libp2p-request-response | 0.29.0 | - | 0.29.0 | - | OK |
| tracing | 0.1.44 | 2026-01-09 | 0.1.44 | 2026-01-09 | OK |
| tracing-subscriber | 0.3.23 | 2026-03-13 | 0.3.23 | 2026-03-13 | OK |
| color-eyre | 0.6.5 | 2025-05-30 | 0.6.5 | 2025-05-30 | OK |
| dotenvy | 0.15.7 | 2023-03-22 | 0.15.7 | 2023-03-22 | OK |
| serde | 1.0.228 | - | 1.0.228 | - | OK |
| serde_json | 1.0.149 | - | 1.0.149 | - | OK |
| futures | 0.3.32 | - | 0.3.32 | - | OK |
| ratatui | 0.28.1 | 2024-08-25 | 0.30.0 | 2025-12-26 | Behind |
| crossterm | 0.5.5 | 2019-01-26 | 0.29.0 | 2025-04-05 | Behind |
| atty | 0.2.14 | - | 0.2.14 | - | OK |

\*libsqlite3-sys 0.37.0 has compatibility issues with diesel 2.3.7

## Notes on Behind Dependencies

### crossterm (0.5.5 vs 0.29.0)
Pinned to `=0.5` in Cargo.toml. This is a very old version from 2019. The ratatui 0.28 ecosystem may have been tested with this version. Updating requires checking ratatui compatibility.

### ratatui (0.28.1 vs 0.30.0)
Pinned to `=0.28` in Cargo.toml. Two minor versions behind. Upgrading to 0.30.0 may require API changes in TUI code.

### libsqlite3-sys (0.36.0 vs 0.37.0)
Not pinned (uses `0.36.0` without `=`). 0.37.0 exists but has known compatibility issues with diesel 2.3.7. Stay on 0.36.0 until diesel updates.
