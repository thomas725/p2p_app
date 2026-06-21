# CI/CD Pipeline

## Workflows (`.github/workflows/`)

- **`main.yml`** — runs on every push to `main`/`develop`. A single job that:
  checks out the repo, installs the Rust toolchain and system deps (SQLite,
  GTK/WebKit for the Dioxus desktop build), runs `cargo fmt --check`,
  `cargo clippy`, `cargo build`, `cargo audit`, builds the docs, and runs
  `cargo tarpaulin` for coverage. It then writes one timestamped results file
  to `.github/ci-results/results_<timestamp>_<run_id>.txt` (containing all
  step output plus a coverage summary) and regenerates
  `docs/codebase_metrics.md`, committing both back to the repo.
- **`dependencies.yml`** — periodic dependency audit (`cargo outdated`,
  `cargo audit`). Writes a timestamped file to
  `.github/ci-results/dependency-check_<timestamp>_<run_id>.txt`.
- **`release.yml`** — builds and publishes release binaries on tag push.

## Reading results

The latest pipeline run is always the most recent
`.github/ci-results/results_*.txt` file (sort by timestamp/run ID). It
contains, in order: formatting check, clippy output, build output, security
audit, doc build warnings, per-test-suite results, and a tarpaulin coverage
summary (overall percentage plus per-file coverage).

`.github/ci-results/dependency-check_*.txt` holds the latest dependency audit.

Because results are committed to the repo, CI history is reviewable with
`git log` / `git grep` without needing GitHub API access.

## Known issues

- `cargo audit` reports two outstanding advisories in transitive
  dependencies pulled in via libp2p's mdns/DNS stack (`hickory-proto`) and
  diesel's SQLite backend. No compatible fixed versions are currently
  available; these are tracked as accepted risk (see the `audit` step's
  allow-list in `main.yml`).
- `cargo outdated` in the dependency-check workflow currently fails with a
  version-resolution conflict on `libsqlite3-sys` between `cargo-outdated`'s
  own dependencies and this project's. This is a tooling issue in the audit
  job itself (exit code 0, doesn't fail CI) and does not reflect a problem
  with the project's dependencies.
