# How the Error Visibility System Works

## Overview

The system captures CI errors at multiple layers and stores them permanently in the git repository, making debugging transparent and independent of external services.

## The Flow

```
1. GitHub Actions CI runs
   ↓
2. Jobs execute (fmt, clippy, tests, build)
   ↓
3. On failure: capture output to files
   ↓
4. Artifacts created with error details
   ↓
5. store-results.yml triggered on completion
   ↓
6. Extracts artifacts from CI job
   ↓
7. Commits error files to .github/ci-errors/
   ↓
8. Pushes to repository
   ↓
9. Errors permanently visible in git history
```

## Key Components

### 1. Enhanced CI Workflow (ci.yml)

**What it does:**
- Runs fmt, clippy, tests, and builds
- Captures all output to text files
- On failure, aggregates errors into FAILURE_REPORT.txt
- Creates artifacts with error details

**Error Capture:**
```bash
# Clippy errors captured:
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee clippy-output.txt

# Test output captured:
cargo test --all-features 2>&1 | tee test-results/test-output.txt

# If failure detected:
echo "=== FAILURE REPORT ===" > FAILURE_REPORT.txt
cat clippy-output.txt >> FAILURE_REPORT.txt
tail -100 test-results/test-output.txt >> FAILURE_REPORT.txt
```

**Result:** Artifacts created with full error details

### 2. Error Persistence Workflow (store-results.yml)

**What it does:**
- Triggered when CI workflow completes (workflow_run trigger)
- Downloads artifacts from ci.yml
- Extracts error files
- Commits them to repository
- Creates timestamped records

**Process:**
```bash
# Download artifacts
download-artifact

# Extract to repository
mkdir -p .github/ci-errors/
cp FAILURE_REPORT.txt .github/ci-errors/latest-failure.txt
cp FAILURE_REPORT.txt .github/ci-errors/failure_TIMESTAMP_RUNID.txt

# Commit to git
git add .github/ci-errors/
git commit -m "ci: capture error details from run $RUN_ID"
git push origin main
```

**Result:** Errors committed permanently to git history

### 3. Error Storage Structure

```
.github/
├── ci-errors/
│   ├── latest-failure.txt          # Latest error summary
│   ├── failure_2026-04-22_04-07_24759578039.txt
│   └── failure_2026-04-22_04-09_24759579621.txt
│
├── build-outputs/
│   ├── test-output.txt             # Full 176KB compilation log
│   └── FAILURE_REPORT.txt
│
├── workflow-logs/
│   └── 2026-04-23_20-05-32_24856170077.md  # Metadata
│
└── ci-results/
    ├── latest-run.txt              # Job status
    └── latest.txt                  # Detailed results
```

### 4. System Dependencies

The CI workflow automatically installs:
```bash
sudo apt-get install -y \
  libsqlite3-dev \
  pkg-config \
  libglib2.0-dev \
  libgtk-3-dev
```

## Why This Approach?

✅ **Works Around Blocked API**
- Can't use api.github.com
- Solution: Commit to repository instead
- Result: No external API dependency

✅ **Permanent Storage**
- Artifacts are transient (deleted after 90 days)
- Git history is permanent
- Result: Never lose error information

✅ **Fully Searchable**
- Use `git grep` to search all errors
- Use `git log` to see error history
- Result: Complete audit trail

✅ **Offline Compatible**
- Works with git clones
- Works with git mirrors
- Result: Portable and reproducible

## How to Use It

### Check Latest Errors
```bash
cat .github/ci-errors/latest-failure.txt
```

### View Full Compilation Log
```bash
tail -200 .github/build-outputs/test-output.txt
```

### Search for Specific Errors
```bash
git grep "error:" -- .github/ci-errors/
```

### See Error History
```bash
git log --oneline -- .github/ci-errors/
git log -p -- .github/ci-errors/ | grep "error:"
```

### Find When an Error First Appeared
```bash
git log --reverse --oneline -- .github/ci-errors/ | head -1
```

