# CI/CD pipeline

The five GitHub Actions workflows, what each one does, and which of them can
actually fail a build.

For the contributor-facing checklist see
[../.github/CI_GUIDE.md](../.github/CI_GUIDE.md).

## The workflows

| Workflow | Trigger | What it does | Can it fail? |
|---|---|---|---|
| [`main.yml`](../.github/workflows/main.yml) | push to `main` / `develop` | fmt, clippy, tests, build, audit, rustdoc → a consolidated results file | **No** — every step is `continue-on-error` |
| [`dependencies.yml`](../.github/workflows/dependencies.yml) | daily 02:00 UTC + dispatch | `cargo outdated` (informational) + `cargo audit` | **Yes** — the audit is a hard gate |
| [`metrics.yml`](../.github/workflows/metrics.yml) | daily 02:30 UTC + dispatch | Rust/Dart/Kotlin coverage + codebase metrics, pushed to `main` | No (all steps are `continue-on-error`) |
| [`nightly-apk.yml`](../.github/workflows/nightly-apk.yml) | daily 03:17 UTC + dispatch | Builds the Android APK and publishes a rolling nightly release | No |
| [`release.yml`](../.github/workflows/release.yml) | tag `v*.*.*` + dispatch | Cross-platform release binaries + a GitHub Release | Yes |

### `main.yml` — the push pipeline

One job, `pipeline`. It installs the Rust toolchain, `libsqlite3-dev` and
`pkg-config`, then runs, in order:

1. `cargo fmt -- --check`
2. `cargo clippy --all-targets --all-features`
3. `cargo test --all-features`
4. `cargo build --release --all-features`
5. `cargo audit`
6. `cargo doc --no-deps --all-features` (with `RUSTDOCFLAGS="-W missing-docs"`)

Each step tees its output to `.github/ci-logs/<name>.log`. The steps are
all `continue-on-error: true`, so **the workflow never turns red** — the
results are the artifact, not the exit status. A final step concatenates the
logs into one timestamped file and pushes it to the **`ci-results` branch**
(via `scripts/push-ci-results.sh`), never to `main`.

> The `ci-results` branch is a rolling "latest run" record that CI force-pushes.
> Do not branch work off it.

### `dependencies.yml` — the only hard dependency gate

Runs `cargo outdated` (informational — it exits non-zero whenever *any* row
exists, including transitive pins no repo change can move, so it must not
gate) and `cargo audit`, which **does** gate: the workflow fails on any
advisory not ignored in `.cargo/audit.toml`.

`.cargo/audit.toml` is comment-only — nothing is silently ignored. `cargo
audit` currently exits 0, with two allowed *warnings* (the unmaintained
`paste` 1.0.15 and the yanked `yoke-derive` 0.8.3). See
[dependencies.md](dependencies.md) for the full picture.

### `metrics.yml` — coverage and codebase metrics

Produces Rust coverage (`cargo tarpaulin --all-features`), Dart coverage
(`flutter test --coverage`), and Kotlin coverage (Gradle's `jacoco` task),
then regenerates `docs/codebase_metrics.md` with `scripts/generate_metrics.py`
and commits the result to `main` as `ci: refresh coverage and codebase
metrics`.

The commit is made in a **detached worktree** of `origin/main`, so the
runner's primary checkout (which holds the generated coverage files) is never
disturbed. A redundancy guard skips the push when `origin/main` already ends
in a CI metrics commit with no new human commits behind it.

> `docs/codebase_metrics.md` is **generated**. Never hand-edit it — regenerate
> with `python3 scripts/generate_metrics.py > docs/codebase_metrics.md`.

### `nightly-apk.yml` — the Android nightly

Builds the Rust native library for Android
(`apps/flutter_app/build_rust_android.sh`), then `flutter build apk --release`,
and publishes it as a rolling prerelease tagged `nightly-<timestamp>`. A
`nightly-latest` tag always points at the most recent build, and the 15 most
recent nightlies are kept; older ones are deleted.

### `release.yml` — tagged releases

A four-target matrix (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`,
`aarch64-apple-darwin`, `aarch64-unknown-linux-gnu`; the last through
`cross`) builds the TUI binary, strips it, and produces a SHA-256 file. A
second job publishes a GitHub Release with the binaries; a third builds a
*minimal* headless binary (`--no-default-features --features
basic,mdns,sqlite_bundled`).

Because `p2p_chat_tui` and `p2p_chat_server` declare `required-features`, the
minimal build produces only the headless `p2p_chat` CLI. Release binaries are
published as `p2p_chat_tui-<target>` so the platform builds cannot overwrite
each other, and each checksum is generated against the published name. The
minimal build is published too, as `p2p_chat` plus `p2p_chat.minimal.sha256`.

A released archive therefore contains:

```
p2p_chat_tui-x86_64-unknown-linux-gnu        p2p_chat_tui-x86_64-unknown-linux-gnu.sha256
p2p_chat_tui-x86_64-apple-darwin             p2p_chat_tui-x86_64-apple-darwin.sha256
p2p_chat_tui-aarch64-apple-darwin            p2p_chat_tui-aarch64-apple-darwin.sha256
p2p_chat_tui-aarch64-unknown-linux-gnu       p2p_chat_tui-aarch64-unknown-linux-gnu.sha256
p2p_chat                                    p2p_chat.minimal.sha256
```

Every `.sha256` names the file it belongs to, so `sha256sum -c` works on a
downloaded release.

Release notes come from `CHANGELOG.md` if it exists; the repository does not
have one yet, so the notes fall back to a single line naming the tag.

## The skip check

`dependencies.yml`, `metrics.yml`, and `nightly-apk.yml` all call
`scripts/ci-skip-no-new-commits.sh` before doing any expensive work. If no
new human-authored commit has landed on the default branch since the workflow
last succeeded, the run skips. Commits authored by CI itself are ignored, so
the nightly metrics push does not count as new work.

Two details matter:

- The token needs **`actions: read`**. Without it the runs API returns 403 and
  the check would silently run on every schedule, producing duplicate
  `ci: refresh coverage and codebase metrics` commits on `main`. The script is
  **fail-closed**: any API error results in a skip.
- `metrics.yml` uses **commit-baseline mode**
  (`--commit-baseline="ci: refresh coverage and codebase metrics"`). The
  baseline is the newest CI-authored commit with that subject on the branch
  (resolved with local git, no API call), **not** the last workflow run. With
  run-history mode a skipped run would become the baseline and reseed the
  "no commits at all" skip loop forever. Only the metrics workflow uses this
  mode.

A manual `workflow_dispatch` always forces a full run.

## Reading the results

The newest `.github/ci-results/results_<timestamp>_<run_id>.txt` file is the
most recent push-pipeline run. It contains, in order: the formatting check,
clippy output, the test suite, the build output, the security audit, and the
doc-build warnings.

Dependency runs write `.github/ci-results/dependency-check_<timestamp>_<run_id>.txt`.

## Known gaps

- **Nothing gates a push.** Every step in `main.yml` is
  `continue-on-error: true`, so a failing test or a clippy warning does not
  block a merge. The policy in `AGENTS.md` ("`cargo ct` must exit 0 with no
  warnings") is enforced by convention and by the local checklist, not by
  CI. The dependency workflow's `cargo audit` is the only hard gate.
- **Flutter and Kotlin tests are nightly-only.** They run in `metrics.yml`,
  not on push.
- **No `CHANGELOG.md`**, so release notes are placeholder text.
