# Repo Scripts

## build-all.sh

Builds **both** desktop frontends in one command: the TUI binary and the Flutter
web bundle (served by the Rust node server).

```bash
scripts/build-all.sh              # release build of everything
scripts/build-all.sh --debug      # fast debug build
scripts/build-all.sh --tui-only   # only the TUI binary
scripts/build-all.sh --web-only   # only the server binary + web bundle
```

Why one script: the web frontend is a **display client** — the node stays native
inside the Rust web server (`p2p_chat_server`) and the browser talks
JSON-over-WebSocket to it (`docs/web_frontend.md`). Nothing is
compiled to WASM, so the web half is a normal Rust build plus `flutter build web`.
Both Rust binaries come out of a **single** `cargo build --bin p2p_chat_tui --bin
p2p_chat_server`, so the shared dependency graph is compiled once and the two
frontends get an identical feature set (same migrations, same attachment GC, same
node behaviour).

Then run (from the repo root):

```bash
./target/release/p2p_chat_tui
./target/release/p2p_chat_server --assets apps/flutter_app/build/web
# LAN exposure (auth required off loopback): the browser prompts for these
export P2P_WEB_PASSWORD='<secret>'   # P2P_WEB_USER defaults to `p2p`
./target/release/p2p_chat_server --bind 0.0.0.0 --assets apps/flutter_app/build/web
```

`scripts/build-web.sh` is the web-only script (same web half, without the TUI).

## build-web.sh

Web frontend only: `cargo build --bin p2p_chat_server` + `flutter build web`.
See `build-all.sh` above for the combined build.

## check_docs_links.py

Verifies that every relative link in the repository's Markdown files resolves.
Moving or renaming a document is the easiest way to leave a broken link
behind, and a broken link in a doc nobody opens stays invisible until someone
does open it.

```bash
python3 scripts/check_docs_links.py            # every tracked *.md
python3 scripts/check_docs_links.py docs/      # only the docs tree
```

Exits 0 when all links resolve, 1 otherwise. It checks inline links
(`[text](target)`) and reference definitions (`[id]: target`), skips fenced
code blocks and inline code spans, and ignores external URLs and pure
anchors. The machine-generated history in `.github/workflow-logs/` is
excluded.

Run it in CI: `.github/workflows/main.yml` has a "Check documentation links"
step.

# Metrics Generation Script

## generate_metrics.py

Automatically generates codebase metrics by analyzing all Rust source files.

### Features

- **Line counting**: Counts total lines in each source file
- **Character counting**: Counts total characters including whitespace
- **Nesting depth calculation**: Determines maximum nesting depth by tracking brace nesting levels
  - Correctly ignores strings and comments
  - Handles escape sequences in strings
  - Counts `{}[]()` brace pairs

### Usage

```bash
python3 scripts/generate_metrics.py
```

### Output

Generates two sections:

1. **Summary**: Statistics about the entire codebase
   - Total files, lines, characters
   - Average metrics per file

2. **All Source Files**: Markdown table with columns:
   - Folder: Directory location (/, src, src/bin, src/generated, src/bin/tui)
   - File: Filename
   - Lines: Total lines of code
   - Characters: Total characters
   - Nesting: Maximum nesting depth
   - Purpose: Brief description of file purpose

### File Scope

The script analyzes:
- `build.rs` (project root)
- All files under `src/` (main library code)
- Recursively includes subdirectories: `src/bin/`, `src/generated/`, etc.

The script excludes:
- Test files (anything under `tests/`)
- Build artifacts (`target/` directory)
- Git metadata (`.git/` directory)

### Updating Documentation

To update the metrics in `docs/codebase_metrics.md`:

1. Fast (no coverage): `python3 scripts/generate_metrics.py > docs/codebase_metrics.md`
2. With coverage: `python3 scripts/generate_metrics.py --with-coverage > docs/codebase_metrics.md`
3. Force fresh coverage: `python3 scripts/generate_metrics.py --force-coverage > docs/codebase_metrics.md`

### Nesting Depth Algorithm

The script calculates nesting depth by counting **maximum indentation level** in the file:

1. For each non-empty, non-comment line, count leading whitespace (spaces and tabs)
2. Convert tabs to 4 spaces for consistent measurement
3. Calculate nesting depth: `depth = leading_spaces / 4`
4. Track and report the maximum depth found

**Why indentation?** This directly measures what developers see in their editor. The 
visual indentation level is the most accurate representation of code nesting complexity
because:

- It counts actual code structure (not just parentheses from type signatures)
- It correlates with cognitive load and readability
- It's what code formatters enforce (in Rust, `cargo fmt` enforces consistent indentation)
- It's language-independent and tool-agnostic

This approach is simpler, more accurate, and more intuitive than counting braces,
since indentation is what humans use to understand nesting depth at a glance.
