# CI/CD Workflow Quick Reference

## For Developers: Making Changes

### Typical Development Workflow

```bash
# 1. Create a feature branch
git checkout -b feature/my-feature

# 2. Make your changes and commit
git add .
git commit -m "feat: add my feature"

# 3. Push to GitHub
git push origin feature/my-feature

# 4. Create a Pull Request on GitHub
# GitHub Actions will automatically:
#    ✓ Check formatting (cargo fmt)
#    ✓ Run linter (cargo clippy)
#    ✓ Run all tests (cargo test)
#    ✓ Build release binaries for multiple platforms
#    ✓ Generate code coverage report

# 5. Review CI results in the Pull Request
# If any checks fail:
#    - Click "Details" to see the full logs
#    - Fix the issues locally
#    - Commit and push again (CI re-runs automatically)

# 6. Merge the Pull Request once all checks pass
# - Locally: git checkout main && git merge feature/my-feature
# - Or: Use GitHub's "Merge" button in the PR
```

### What Happens on Every Push

The **CI workflow** (`ci.yml`) automatically runs:

| Job | Purpose | Fails If |
|-----|---------|----------|
| **fmt** | Check code formatting | Code doesn't match `cargo fmt` |
| **clippy** | Lint code | Warnings are treated as errors |
| **test** | Run all tests | Any test fails |
| **build** | Build binaries | Compilation fails on any platform |
| **build-minimal** | Minimal feature set | Build fails without default features |

**Time:** Usually 3-8 minutes

### Local Pre-Push Checklist

Before pushing, you can run locally to catch issues early:

```bash
# Format code
cargo fmt

# Check formatting
cargo fmt -- --check

# Run clippy
cargo clippy --all-targets --all-features

# Run tests
cargo test --all-features

# Build release
cargo build --release
```

Or run all at once:
```bash
cargo fmt && cargo clippy --all-targets --all-features && cargo test && cargo build --release
```

## For Maintainers: Creating Releases

### Creating a Release

```bash
# 1. Ensure everything is committed
git status

# 2. Update version in Cargo.toml
# Change: version = "0.1.0" → version = "0.2.0"
git add Cargo.toml
git commit -m "bump: version to 0.2.0"

# 3. (Optional) Create a CHANGELOG entry
# Edit CHANGELOG.md:
# ## [0.2.0] - 2024-04-14
# ### Added
# - New feature 1
# - New feature 2
git add CHANGELOG.md
git commit -m "docs: add changelog for v0.2.0"

# 4. Create a git tag
git tag -a v0.2.0 -m "Release v0.2.0"

# 5. Push the tag to GitHub
git push origin v0.2.0

# 6. GitHub Actions will automatically:
#    ✓ Build optimized release binaries for all platforms
#    ✓ Create a GitHub Release
#    ✓ Upload binaries with checksums
#    ✓ (Optional) Deploy documentation to GitHub Pages
```

**Time to complete release:** ~10-15 minutes

### What Gets Released

The Release workflow builds:
- ✅ Linux x86_64 (GNU)
- ✅ macOS x86_64
- ✅ macOS ARM64 (Apple Silicon)
- ✅ Linux ARM64 (aarch64)
- ✅ Minimal binary (embedded Linux)

Each binary includes:
- Stripped symbols (smaller size)
- SHA256 checksums
- Release notes from CHANGELOG

### Verifying a Release

```bash
# Download binary from GitHub Release page
# Verify checksum
sha256sum -c p2p_chat_example.sha256

# Make executable and run
chmod +x p2p_chat_example
./p2p_chat_example
```

## Viewing Results

### GitHub Actions Dashboard

1. Go to **Actions** tab in your repository
2. Click on a workflow name (e.g., "CI")
3. Click on a run to see details
4. Click on a job to see step-by-step logs

### PR Status Checks

When you create a PR:
1. GitHub automatically runs CI
2. Scroll to the bottom of the PR to see status
3. Click "Details" next to any failed check to see logs
4. Once all checks pass, you can merge (if approved)

### Monitoring

#### For New PRs:
- Keep an eye on the **Checks** section
- Fix and push again if something fails

#### For Main Branch:
- Visit the **Actions** tab to monitor builds
- All commits to main should have passing CI

#### For Releases:
- Go to **Releases** page to download binaries
- Check that binaries exist for all platforms

## Troubleshooting

### "Checks Failed" on Pull Request

1. Click "Details" next to the failed check
2. Scroll through logs to find the error
3. Common issues:
   - **Formatting:** Run `cargo fmt` locally and push
   - **Clippy warnings:** Run `cargo clippy --fix` and push
   - **Test failures:** Run `cargo test` locally to debug
   - **Build errors:** Check dependencies, run `cargo build` locally

### CI Takes Too Long

- First build of a branch is slow (builds from scratch)
- Subsequent pushes are faster (uses cache)
- Each platform builds independently (can take 5-10 min)

### Coverage Report Not Uploading

- This is non-critical; coverage still runs
- To use codecov: Add `CODECOV_TOKEN` secret to GitHub Settings

## Environment Files

The CI workflows use these key files:

| File | Purpose |
|------|---------|
| `Cargo.toml` | Package metadata and dependencies |
| `Cargo.lock` | Dependency lock file (commit this!) |
| `.github/workflows/*.yml` | GitHub Actions workflows |
| `CHANGELOG.md` | Release notes (optional) |
| `.rustfmt.toml` | Code formatting rules (auto-used) |

## Pro Tips

### 1. Catch Errors Before Pushing

```bash
# Create a pre-commit hook
cat > .git/hooks/pre-commit << 'EOF'
#!/bin/bash
cargo fmt --check || exit 1
cargo clippy --all-targets -- -D warnings || exit 1
exit 0
EOF
chmod +x .git/hooks/pre-commit
```

### 2. Speed Up Local Testing

```bash
# Test only changed files
cargo test --lib

# Skip slow integration tests initially
cargo test --lib --all-features
```

### 3. Build Multiple Targets Locally

```bash
# Requires cross: cargo install cross
cross build --release --target aarch64-unknown-linux-gnu
```

### 4. Monitor in Real-Time

```bash
# Watch for CI updates (requires 'gh' CLI)
gh run watch
```

## Getting Help

- **GitHub Actions Docs:** https://docs.github.com/en/actions
- **Rust Ecosystem:** https://www.rust-lang.org/what/wasm/
- **libp2p Docs:** https://docs.libp2p.io/
- **Project Issues:** Create an issue on GitHub

## Summary

| Scenario | Action | Time | Result |
|----------|--------|------|--------|
| Push to PR | Auto-runs CI | 3-8 min | Green/red status |
| Merge to main | Auto-runs CI + deploys docs | 3-8 min | Tests + deploy |
| Create release tag | Auto-builds + releases | 10-15 min | GitHub Release page |
| Daily check | Auto-checks deps | Daily | Audit report |

**The goal:** All tedious testing, building, and deployment is automated. You focus on code! 🚀
