#!/usr/bin/env bash
# Push `.github/ci-results/` to a dedicated results branch (default: ci-results).
#
# Called from `.github/workflows/dependencies.yml`. Never touches `main`: CI findings
# are written to this auto-generated branch as a rolling "latest run" record
# (overwritten, not appended). Safe to force-push because the branch is
# machine-generated; nobody should branch work off it.
#
# Usage: push-ci-results.sh [branch]
set -euo pipefail

BRANCH="${1:-ci-results}"
RESULTS_DIR=".github/ci-results"

# Nothing produced on this run? Nothing to push. Logs still surface in the Actions
# UI, so a missing dir is not an error.
if [[ ! -d "$RESULTS_DIR" ]] || ! find "$RESULTS_DIR" -type f -print -quit 2>/dev/null | grep -q .; then
  echo "No result files in $RESULTS_DIR; skipping."
  exit 0
fi

# Operate on a detached worktree of the results branch so we never disturb the
# runner's checkout of `main`.
WORKTREE="$(mktemp -d)"
trap 'rm -rf "$WORKTREE"' EXIT

if git ls-remote --heads "origin" "$BRANCH" | grep -q "$BRANCH"; then
  # Branch already exists on the remote: fetch it into a local ref, then work from it.
  git fetch -q "origin" "refs/heads/$BRANCH:refs/remotes/origin/$BRANCH"
  git worktree add --detach "$WORKTREE" "origin/$BRANCH"
else
  # Branch does not exist yet: seed it empty so the results are the first commit.
  git worktree add --detach "$WORKTREE" HEAD
  git -C "$WORKTREE" read-tree --empty && git -C "$WORKTREE" commit --allow-empty -m "init ci-results branch" >/dev/null
fi
cd "$WORKTREE"

# Replace the results wholesale so stale files from prior runs never accumulate.
rm -rf "$RESULTS_DIR"
mkdir -p "$RESULTS_DIR"
cp -R "$OLDPWD/$RESULTS_DIR/." "$RESULTS_DIR/"

git config user.email "ci-pipeline@users.noreply.github.com"
git config user.name "CI Pipeline"

git add "$RESULTS_DIR"
if ! git diff --cached --quiet; then
  git commit -q -m "ci: dependency check results (${GITHUB_RUN_ID:-unknown run})"
else
  echo "No changes to commit on $BRANCH."
  exit 0
fi

git push --force "origin" "HEAD:refs/heads/$BRANCH"
echo "Pushed dependency check results to branch: $BRANCH"