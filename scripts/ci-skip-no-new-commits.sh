#!/usr/bin/env bash
# Detects whether a scheduled workflow's heavy tasks can be skipped because the
# default branch has not received any new commits since that workflow last
# completed successfully.
#
# Commits authored by the CI pipeline itself (CI Pipeline
# <ci-pipeline@users.noreply.github.com>) are IGNORED, so self-generated commits
# (the nightly metrics/coverage push to main, the ci-results pushes) never count
# as new work. Manual workflow_dispatch always forces a full run.
#
# Usage: ci-skip-no-new-commits.sh <workflow-file-name>
#   e.g.    ci-skip-no-new-commits.sh dependencies.yml
#
# Output: prints  skip=true|false  to stdout, and appends  skip=...  to
#   $GITHUB_OUTPUT when running as an Actions step (env var set by the runner).
set -euo pipefail

wf="${1:-}"
if [ -z "$wf" ]; then
  echo "error: workflow file name required" >&2
  exit 2
fi

if [ "${GITHUB_EVENT_NAME:-}" = "workflow_dispatch" ]; then
  echo "manual dispatch; running full check"
  echo "skip=false"
  [ -n "${GITHUB_OUTPUT:-}" ] && echo "skip=false" >> "$GITHUB_OUTPUT"
  exit 0
fi

repo="${GITHUB_REPOSITORY:-}"
sha="${GITHUB_SHA:-}"
token="${GITHUB_TOKEN:-}"
ci_author_email="ci-pipeline@users.noreply.github.com"

skip="false"

if [ -z "$repo" ] || [ -z "$sha" ] || [ -z "$token" ]; then
  echo "info: missing GITHUB_REPOSITORY/GITHUB_SHA/GITHUB_TOKEN; running conservatively"
else
  api="https://api.github.com/repos/${repo}"
  auth="Authorization: Bearer ${token}"

  last_sha="$(curl -sS -H "$auth" \
    "${api}/actions/workflows/${wf}/runs?status=success&per_page=1&exclude_pull_requests=true" \
    | jq -r '.workflow_runs[0].head_sha // empty' 2>/dev/null || true)"

  if [ "$last_sha" = "$sha" ]; then
    skip="true"
    echo "no commits at all since last successful run ($last_sha); skipping"
  elif [ -n "$last_sha" ]; then
    compare="$(curl -sS -H "$auth" "${api}/compare/${last_sha}...${sha}" 2>/dev/null || true)"
    status_code="$(printf '%s' "$compare" | jq -r '.status // empty' 2>/dev/null || true)"
    if [ -n "$status_code" ]; then
      non_ci="$(printf '%s' "$compare" \
        | jq "[.commits[] | select((.commit.author.email // \"\") != \"${ci_author_email}\")] | length" \
        2>/dev/null || echo 1)"
      if [ "${non_ci:-1}" -eq 0 ]; then
        skip="true"
        echo "only CI-authored commits since last successful run ($last_sha); skipping"
      else
        echo "${non_ci} human-authored commit(s) since last successful run ($last_sha); running"
      fi
    else
      echo "could not compare $last_sha...$sha (unrelated/rewritten history); running conservatively"
    fi
  else
    echo "no prior successful run of $wf; running"
  fi
fi

echo "skip=${skip}"
[ -n "${GITHUB_OUTPUT:-}" ] && echo "skip=${skip}" >> "$GITHUB_OUTPUT"