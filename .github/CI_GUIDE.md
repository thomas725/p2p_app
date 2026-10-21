# CI/CD quick reference

What actually runs, and what to do before you push.

The technical reference lives in [`docs/ci_pipeline.md`](../docs/ci_pipeline.md).

## Before you push

```bash
cargo fmt --all
cargo ct                                  # alias for: cargo clippy --all-targets --all-features
cargo test --all-features
python3 scripts/check_docs_links.py       # if you touched any Markdown
```

Plus, if you touched the Flutter app or the Android native code:

```bash
cd apps/flutter_app
flutter analyze
flutter test
```

`cargo ct` must exit 0 with **no warnings**. The project denies clippy's
`pedantic`, `nursery`, and `restriction` groups, plus `unwrap_used`,
`expect_used`, `indexing_slicing`, `as_conversions`, and friends. The
allowance policy is in [`AGENTS.md`](../AGENTS.md) — do not add new allows
without following that policy.

## What runs when

| Workflow | Trigger | Runs |
|---|---|---|
| `main.yml` | push to `main`/`develop` | fmt, clippy, tests, build, audit, rustdoc |
| `dependencies.yml` | daily 02:00 UTC | `cargo outdated` (informational), `cargo audit` (**gates**) |
| `metrics.yml` | daily 02:30 UTC | Rust/Dart/Kotlin coverage + codebase metrics → commits to `main` |
| `nightly-apk.yml` | daily 03:17 UTC | Android APK → rolling nightly GitHub release |
| `release.yml` | tag `v*.*.*` | Cross-platform binaries → GitHub Release |

**Nothing gates a push.** Every step in `main.yml` is `continue-on-error`, so
the workflow stays green even when a test fails; the results are the artifact.
The one hard gate in the repo is `cargo audit` in `dependencies.yml`. This is
a known gap — see "Known gaps" in
[`docs/ci_pipeline.md`](../docs/ci_pipeline.md).

## Where to read the results

- **Actions tab** → pick a run → expand the step for the full log.
- **Push-pipeline runs** also write a consolidated file to the **`ci-results`
  branch** (`.github/ci-results/results_<timestamp>_<run_id>.txt`). That branch
  is machine-generated and force-pushed; never branch work off it.
- **Dependency runs** write `.github/ci-results/dependency-check_<timestamp>_<run_id>.txt`.

## Making a release

```bash
# 1. Everything committed and green locally
git status

# 2. Bump the version
#    Cargo.toml: version = "0.1.0" → "0.2.0"
git add Cargo.toml
git commit -m "bump: version to 0.2.0"

# 3. (Optional) Add a CHANGELOG.md entry.
#    release.yml reads the section for the tag from CHANGELOG.md; without the
#    file the release notes are a single placeholder line.

# 4. Tag and push
git tag -a v0.2.0 -m "Release v0.2.0"
git push origin v0.2.0
```

The release workflow then builds `p2p_chat_tui` for Linux x86-64, Linux
aarch64, macOS x86-64, and macOS aarch64, plus a minimal headless `p2p_chat`
binary, and publishes them with SHA-256 checksums.

To check a published binary:

```bash
sha256sum -c p2p_chat_tui-x86_64-unknown-linux-gnu.sha256
```

## Scheduled runs

The three scheduled workflows skip themselves when no new human commit has
landed on the default branch since they last succeeded — the nightly APK
build takes ~25 minutes and the coverage run ~13, so there is no point paying
for them when nothing changed. CI's own commits (the metrics push) do not
count as new work, and a manual **Run workflow** always forces a full run.

## Troubleshooting

**Clippy fails with a lint you did not expect.** The project denies the
`restriction` group, which includes opinionated lints. Fix the code rather
than adding an allow — see the allowance policy in `AGENTS.md`. The one
existing allow is documented there.

**A test fails only in CI.** The database tests share one SQLite file and are
serialized by `shared_db_test_lock()`; a test that touches the DB without
holding that lock can cascade. If you added a test, make sure it takes the
lock.

**The metrics workflow pushes twice.** It has a redundancy guard for this,
but if it happens, check that the `ci-skip-no-new-commits.sh` call still has
`--commit-baseline="ci: refresh coverage and codebase metrics"` and that the
workflow's `permissions` block still contains `actions: read`.

**The dependency workflow runs every night despite no changes.** The token is
missing `actions: read`, so the runs API returns 403 and the check cannot
tell whether anything landed. Add `actions: read` to the workflow's
`permissions`.

## Local pre-commit hook

```bash
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
cargo fmt --all -- --check || exit 1
cargo ct || exit 1
exit 0
EOF
chmod +x .git/hooks/pre-commit
```

## Useful links

- [GitHub Actions documentation](https://docs.github.com/en/actions)
- [Rust on GitHub Actions](https://github.com/actions-rs/toolchain)
- [cargo-audit](https://github.com/rustsec/rustsec)
- [libp2p docs](https://docs.libp2p.io/)
