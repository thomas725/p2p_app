# CI/CD pipeline

The six GitHub Actions workflows, what each one does, and which of them can
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
| [`nightly-desktop.yml`](../.github/workflows/nightly-desktop.yml) | daily 03:47 UTC + dispatch | Builds the Ubuntu desktop set (TUI, minimal CLI, GTK app, web bundle) and publishes a rolling nightly release | No |
| [`release.yml`](../.github/workflows/release.yml) | tag `v*.*.*` + dispatch (`tag`) | Cross-platform release binaries + a GitHub Release | Yes |

### `main.yml` — the push pipeline

One job, `pipeline`. It installs the Rust toolchain, `libsqlite3-dev` and
`pkg-config`, then runs, in order:

> **Toolchain mismatch, by design:** every workflow uses
> `dtolnay/rust-toolchain@stable`, while a dev box has `rustup override set
> nightly` in the repo root (`AGENTS.md`). The crate uses no `#![feature]`, so
> both compile the same code; what can differ is the clippy/rustfmt lint set,
> which moves between releases. A lint that only exists on nightly will not be
> seen by CI, and vice versa — treat the local nightly `cargo ct` as the
> stricter gate.

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

The prune step matches only `^nightly-[0-9]{8}-`, so it cannot delete the
desktop nightly's `nightly-desktop-*` releases or the `nightly-latest` pointer.

### `nightly-desktop.yml` — the Ubuntu desktop nightly

One `gate` job runs the skip check once; the build jobs hang off it so the
expensive work happens at most once per new commit. It produces:

| Artifact | What it is |
|---|---|
| `p2p_chat_tui-x86_64-unknown-linux-gnu` | Terminal frontend (`cargo build --release`) |
| `p2p_chat` | Minimal headless CLI (`--no-default-features --features basic,mdns,sqlite_bundled`) |
| `p2p_app_flutter-linux-x64.tar.gz` | Flutter Linux desktop app (GTK), built with `flutter build linux --release` |
| `p2p-web-linux-x64.tar.gz` | `p2p_chat_server` + the `flutter build web` bundle, plus a `RUN.txt` |

`build-rust` compiles once and hands two extra artifacts to the Flutter jobs so
they can run in parallel: the `libp2p_app.so` cdylib (which
`apps/flutter_app/linux/CMakeLists.txt` installs into the bundle's `lib/`) and
the `p2p_chat_server` binary (packaged with the web assets).

Releases are tagged `nightly-desktop-<timestamp>`, with a
`nightly-desktop-latest` pointer and the same keep-15 retention. The prune step
matches only `^nightly-desktop-[0-9]{8}-`, which excludes its own pointer and
the APK releases.

Targets are x86_64 only: the desktop bundle is a native GTK build. The aarch64
TUI binary stays in tagged releases, where `cross` builds it.

### `release.yml` — tagged releases

A `prepare` job resolves the release tag and validates it (`v1.2.3`, optionally
with a `-rc1`-style suffix) before any build machine is spent. On a tag push it
uses the pushed ref; on a manual **Run workflow** it uses the required `tag`
input, so dispatching publishes a real release instead of uploading artifacts
nobody can find.

A four-target matrix (`x86_64-unknown-linux-gnu`, `x86_64-apple-darwin`,
`aarch64-apple-darwin`, `aarch64-unknown-linux-gnu`; the last through
`cross`) builds the TUI binary, strips it, and produces a SHA-256 file. A
second job publishes a GitHub Release with the binaries; a third builds a
*minimal* headless binary (`--no-default-features --features
basic,mdns,sqlite_bundled`).

`create-release` needs **both** build jobs: `download-artifact` only sees
artifacts that have already been uploaded, so without that dependency the
minimal binary raced the release and was usually missing from it.

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

`dependencies.yml`, `metrics.yml`, `nightly-apk.yml`, and
`nightly-desktop.yml` all call `scripts/ci-skip-no-new-commits.sh` before doing
any expensive work. If no new human-authored commit has landed on the default
branch since the workflow last succeeded, the run skips. Commits authored by CI
itself are ignored, so the nightly metrics push does not count as new work.

Three details matter:

- The check step needs **`GITHUB_TOKEN` in its step environment**
  (`env: GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}`). GitHub does **not** export
  `GITHUB_TOKEN` to `run:` steps on its own — `permissions:` alone grants the
  token but does not put it in the environment. The script is **fail-closed**,
  so a missing token reads as "cannot tell" and skips every scheduled run. Both
  nightlies shipped without that `env:` line, which is why they never ran on
  their cron.
- The token needs **`actions: read`**. Without it the runs API returns 403 and
  the check skips for the same reason. (Before the script was made fail-closed,
  the failure mode was the opposite: every scheduled run executed, producing
  duplicate `ci: refresh coverage and codebase metrics` commits on `main`.)
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
- **A tagged release is narrower than a nightly.** `release.yml` ships the TUI
  (four targets) and the minimal CLI; the Flutter Linux app, the web frontend
  and the Android APK only exist as nightly builds. A `v*.*.*` release
  therefore does not contain the desktop or web deliverables yet.
- **No `CHANGELOG.md`**, so release notes are placeholder text.
