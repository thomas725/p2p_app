# Dependencies Documentation

This document lists all project dependencies, their purpose, and the date they were last checked for updates.

**Last checked:** 2026-04-04T13:18:50Z
**Last updated:** 2026-04-04T14:00:00Z

## Core Dependencies

### libp2p (=0.56.0)

- **Purpose:** Peer-to-peer networking library - handles P2P connections, protocol negotiation, and message routing
- **Features used:** tokio, gossipsub, noise, macros, tcp, yamux, quic
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
- **Status:** Up to date (updated from 1.47.1)

### diesel (=2.3.7)

- **Purpose:** ORM and Query Builder for SQLite database operations
- **Features used:** sqlite, returning_clauses_for_sqlite_3_35, chrono
- **Version in use:** 2.3.7
- **Latest version:** 2.3.7
- **Latest release date:** 2026-03-13
- **Status:** Up to date (updated from 2.2.12)

### libsqlite3-sys (=0.36.0)

- **Purpose:** Native SQLite bindings - provides FFI to SQLite library
- **Features used:** bundled (embed SQLite statically)
- **Version in use:** 0.36.0
- **Latest version:** 0.37.0
- **Latest release date:** 2026-03-15
- **Version 0.36.0 release date:** 2025-12-20
- **Status:** Up to date (updated from 0.35.0) - Note: 0.37.0 has compatibility issues with diesel 2.3.7

### diesel_migrations (=2.3.1)

- **Purpose:** Database migration management for Diesel - embeds and runs SQL migrations at runtime
- **Version in use:** 2.3.1
- **Latest version:** 2.3.1
- **Latest release date:** 2025-11-26
- **Status:** Up to date (updated from 2.2.0)

### chrono (=0.4.44)

- **Purpose:** Date and time library - used for TIMESTAMP columns in database
- **Version in use:** 0.4.44
- **Latest version:** 0.4.44
- **Latest release date:** 2026-02-23
- **Status:** Up to date (updated from 0.4.41)

## Networking

### libp2p-quic (=0.13.0)

- **Purpose:** QUIC transport protocol for libp2p
- **Version in use:** 0.13.0
- **Latest version:** 0.13.0
- **Latest release date:** 2025-06-27
- **Status:** Up to date

### libp2p-identity (=0.2.13)

- **Purpose:** Cryptographic keypairs and peer identity management for libp2p
- **Version in use:** 0.2.13
- **Latest version:** 0.2.13
- **Latest release date:** 2023-03-12
- **Status:** Up to date (updated from 0.2.12)

## Logging & Error Handling

### tracing (=0.1.44)

- **Purpose:** Application-level tracing/logging framework (used by libp2p)
- **Version in use:** 0.1.44
- **Latest version:** 0.1.44
- **Latest release date:** 2026-01-09
- **Status:** Up to date (updated from 0.1.41)

### tracing-subscriber (=0.3.23)

- **Purpose:** Utilities for implementing tracing subscribers - enables formatted log output
- **Version in use:** 0.3.23
- **Latest version:** 0.3.23
- **Latest release date:** 2026-03-13
- **Status:** Up to date (updated from 0.3.20)

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

## Summary

| Dependency | In Use | In Use Release | Latest | Latest Release | Status |
|------------|--------|-----------------|--------|-----------------|--------|
| libp2p | 0.56.0 | 2025-06-28 | 0.56.0 | 2025-06-28 | OK |
| tokio | 1.51.0 | 2026-04-03 | 1.51.0 | 2026-04-03 | OK |
| diesel | 2.3.7 | 2026-03-13 | 2.3.7 | 2026-03-13 | OK |
| libsqlite3-sys | 0.36.0 | 2025-12-20 | 0.37.0 | 2026-03-15 | OK* |
| diesel_migrations | 2.3.1 | 2025-11-26 | 2.3.1 | 2025-11-26 | OK |
| chrono | 0.4.44 | 2026-02-23 | 0.4.44 | 2026-02-23 | OK |
| libp2p-identity | 0.2.13 | 2023-03-12 | 0.2.13 | 2023-03-12 | OK |
| libp2p-quic | 0.13.0 | 2025-06-27 | 0.13.0 | 2025-06-27 | OK |
| tracing | 0.1.44 | 2026-01-09 | 0.1.44 | 2026-01-09 | OK |
| tracing-subscriber | 0.3.23 | 2026-03-13 | 0.3.23 | 2026-03-13 | OK |
| color-eyre | 0.6.5 | 2025-05-30 | 0.6.5 | 2025-05-30 | OK |
| dotenvy | 0.15.7 | 2023-03-22 | 0.15.7 | 2023-03-22 | OK |

*libsqlite3-sys 0.37.0 has compatibility issues with diesel 2.3.7
