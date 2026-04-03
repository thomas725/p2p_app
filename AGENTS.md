# AGENTS.md - Developer Guidelines for p2p_app

This is a Rust project using Diesel (SQLite), libp2p (P2P networking), and tokio.

## Build & Run Commands

```bash
# Build the project
cargo build

# Build release version
cargo build --release

# Run the binary
cargo run

# Run with custom database
DATABASE_URL=my.db cargo run
```

### Linting & Formatting

```bash
# Format code (Rustfmt)
cargo fmt

# Run Clippy lints
cargo clippy

# Run Clippy with fix suggestions
cargo clippy --fix
```

### Testing

```bash
# Run all tests
cargo test

# Run a single test
cargo test test_name

# Run tests with output
cargo test -- --nocapture
```

## Code Style Guidelines

### Imports

- Use underscore suffixes for trait imports used for methods: `use tokio::io::AsyncBufReadExt as _`
- Group imports: std first, then external crates, then crate modules
- Use `use` statements at module level, not inline

### Formatting

- Run `cargo fmt` before committing
- Use 4 spaces for indentation (Rust default)
- Maximum line length: 100 characters (default rustfmt)

### Types

- Use explicit return types on public functions
- Prefer `Result<T, color_eyre::Report>` for error handling
- Use the type system; avoid `unwrap()` in production code

### Naming Conventions

- `snake_case` for variables, functions, modules
- `PascalCase` for types, traits, enums
- `SCREAMING_SNAKE_CASE` for constants
- Prefix private fields with underscore: `struct Foo { _private: T }`

### Error Handling

- Use `color_eyre` for errors (already configured in project)
- Use `wrap_err_with()` or `eyre!()` for context
- Use `tracing` for logging errors: `tracing::error!("description: {e}")`
- Avoid silent failures: use `.ok()` or `.map_err()` explicitly

### Database (Diesel)

- Models are in `src/models_queryable.rs` and `src/models_insertable.rs`
- Schema is auto-generated in `src/schema.rs` - do not edit manually
- Run `diesel_generate.sh` to regenerate after schema changes
- Migrations live in `./migrations/` directory

### Async Code (tokio)

- Use `#[tokio::main]` for async main functions
- Prefer `tokio::select!` for concurrent operations
- Use `StreamExt` for stream processing

### libp2p

- Custom behaviours must derive `NetworkBehaviour`
- Use `SwarmEvent` for handling swarm events
- Topic subscription: use `gossipsub::IdentTopic`

### Project Structure

```
src/
├── lib.rs           # Library root, database logic
├── schema.rs        # Auto-generated Diesel schema
├── models_*.rs      # Auto-generated model structs
└── bin/
    └── p2p_chat_example.rs  # Binary entry point
migrations/          # SQL migration files
```

## Development Environment

The project uses Nix for reproducible dev environments:

```bash
# Enter development shell (if flake is enabled)
nix develop

# Or use direnv (auto-loaded via .envrc)
```

Required system packages (see flake.nix): cargo, rustc, rustfmt, clippy, pkg-config, openssl, sqlite.

## Notes

- SQLite database file: defaults to `sqlite.db` or set via `DATABASE_URL`
- Embedded migrations run automatically on startup
- Identity keypair is generated and stored in database on first run