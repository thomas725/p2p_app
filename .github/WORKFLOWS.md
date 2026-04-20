# GitHub Actions CI/CD Setup

This project uses GitHub Actions for continuous integration and deployment. The workflows are defined in `.github/workflows/`.

## Workflows

### 1. **CI (`ci.yml`)** - Main Continuous Integration
Runs on every push and pull request to `main`, `master`, or `develop` branches.

**Jobs:**
- **fmt** - Check code formatting with `rustfmt`
- **clippy** - Run linter with strict warning-as-error policy
- **test** - Run all tests with all features enabled
- **build** - Build release binaries for:
  - Linux x86_64 (GNU)
  - macOS x86_64
  - macOS ARM64 (Apple Silicon)
- **build-minimal** - Build minimal feature set for embedded systems

**Status Badge:**
```markdown
[![CI](https://github.com/thomas725/p2p_app/actions/workflows/ci.yml/badge.svg)](https://github.com/thomas725/p2p_app/actions/workflows/ci.yml)
```

### 2. **Release (`release.yml`)** - Release Build Pipeline
Triggered automatically when a git tag matching `v*.*.*` is pushed, or manually via `workflow_dispatch`.

**Jobs:**
- **build-release** - Build optimized release binaries for multiple platforms
- **create-release** - Create GitHub Release with binaries and checksums
- **build-minimal-release** - Build minimal binary for embedded Linux

**Creating a Release:**
```bash
# Tag the current commit
git tag -a v0.2.0 -m "Release v0.2.0: New features"
git push origin v0.2.0

# GitHub Actions will automatically:
# 1. Build optimized binaries for all platforms
# 2. Create a GitHub Release
# 3. Upload binaries with checksums
```

### 3. **Code Coverage (`coverage.yml`)** - Coverage Reports
Runs on every push and pull request to track code coverage over time.

**Jobs:**
- **coverage** - Generate coverage report with `cargo-tarpaulin`
- Uploads to Codecov (requires `CODECOV_TOKEN` secret if desired)

**Status Badge:**
```markdown
[![codecov](https://codecov.io/github/thomas725/p2p_app/badge.svg)](https://codecov.io/github/thomas725/p2p_app)
```

### 4. **Documentation (`documentation.yml`)** - Build & Deploy Docs
Runs on every push and pull request. Auto-deploys to GitHub Pages on main branch.

**Jobs:**
- **docs** - Build Rust documentation with `cargo doc`
- Auto-deploys to GitHub Pages (main branch only)

**Setup GitHub Pages:**
1. Go to repository Settings → Pages
2. Set source to "Deploy from a branch"
3. Select `gh-pages` branch and `/root` folder
4. Save

**Note:** The `cname` field in the workflow is disabled. Update if you have a custom domain.

### 5. **Dependency Updates (`dependencies.yml`)** - Monitoring
Runs daily at 2 AM UTC and on manual trigger to check for outdated dependencies and vulnerabilities.

**Jobs:**
- **update-dependencies** - Check outdated packages with `cargo-outdated`
- **security-audit** - Check for known vulnerabilities with `cargo-audit`

## Setting Up Secrets

Some workflows use GitHub secrets for enhanced functionality:

### Optional Secrets:
1. **CODECOV_TOKEN** (for codecov.io integration)
   - Go to Settings → Secrets and variables → Actions
   - Click "New repository secret"
   - Name: `CODECOV_TOKEN`
   - Value: Your codecov.io token

2. **GITHUB_TOKEN** (auto-provided by GitHub Actions)
   - Used for releases and GitHub Pages deployments
   - No manual setup needed

## Caching

All workflows use `Swatinem/rust-cache@v2` for efficient build caching:
- Caches cargo registry, index, and compiled artifacts
- Separate caches per platform/target to avoid conflicts
- Automatically invalidated when Cargo.lock changes

## Customization

### Adding Custom Targets
Edit `ci.yml` and `release.yml` to add more build targets:

```yaml
- target: armv7-unknown-linux-gnueabihf
  os: ubuntu-latest
  binary_name: p2p_chat_example
```

### Changing Build Profiles
Modify the build commands to use different feature combinations:

```yaml
run: cargo build --release --no-default-features --features mdns,tracing
```

### Disabling Workflows
To temporarily disable a workflow without deleting it:
1. Rename the `.yml` file to `.yml.disabled`
2. Or add `if: false` to the `on:` section

## Troubleshooting

### Tests Failing in CI But Passing Locally
1. Check for platform-specific issues (Linux vs macOS vs Windows)
2. Review CI logs for full error messages
3. Check environment variables (e.g., `DATABASE_URL`)

### Build Failures
1. Check system dependencies installation in CI workflow
2. Ensure Cargo.lock is committed to git
3. Review platform-specific build issues in logs

### Coverage Not Uploading
1. Verify `CODECOV_TOKEN` is set if using codecov.io
2. Check codecov.io account is linked to repository
3. Note: Coverage still runs even if upload fails

## Performance Tips

1. **Faster Builds:** Use `--no-default-features` for minimal feature sets
2. **Parallel Jobs:** All jobs run in parallel by default
3. **Cache:** First build takes longer, subsequent builds use cache
4. **Nightly Rust:** Consider adding a nightly build job for experimental features

## Monitoring

### View Workflow Status
1. Go to repository → Actions tab
2. Click on workflow name to see recent runs
3. Click on a run to see detailed job logs

### Workflow Status in README
Add badges to your README.md:

```markdown
## Status

[![CI](https://github.com/thomas725/p2p_app/actions/workflows/ci.yml/badge.svg)](https://github.com/thomas725/p2p_app/actions/workflows/ci.yml)
[![Release](https://github.com/thomas725/p2p_app/actions/workflows/release.yml/badge.svg)](https://github.com/thomas725/p2p_app/actions/workflows/release.yml)
[![codecov](https://codecov.io/github/thomas725/p2p_app/badge.svg)](https://codecov.io/github/thomas725/p2p_app)
```

## Next Steps

1. **Commit and push** the `.github/workflows/` directory
2. **Monitor the first CI run** in the Actions tab
3. **Fix any issues** if tests fail
4. **Configure secrets** if desired (CODECOV_TOKEN, etc.)
5. **Enable GitHub Pages** for documentation deployment
6. **Create first release** with git tag `v0.1.0`

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Rust on GitHub Actions](https://github.com/actions-rs/toolchain)
- [Cargo Documentation](https://doc.rust-lang.org/cargo/)
- [dtolnay Rust Toolchain](https://github.com/dtolnay/rust-toolchain)
