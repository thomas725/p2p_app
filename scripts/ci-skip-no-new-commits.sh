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
# RUNTIME REQUIREMENT: the workflow must grant the token `actions: read` in its
# `permissions:` block. Without it the run-history query returns HTTP 403 and the
# skip decision would be based on no data at all.
#
# This script is FAIL-CLOSED: any error while querying the GitHub API (403, rate
# limit, network failure, unknown workflow, unparseable response) results in
# skip=true. A skipped scheduled refresh is harmless (the next scheduled run or a
# manual workflow_dispatch recovers it); an unsolicited heavy run is the failure
# mode this exists to prevent.
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
  echo "error: missing GITHUB_REPOSITORY/GITHUB_SHA/GITHUB_TOKEN; skipping conservatively"
  skip="true"
else
  api="https://api.github.com/repos/${repo}"
  auth="Authorization: Bearer ${token}"

  # Query the last successful run of this workflow. An authenticated token without
  # `actions: read` gets a 403 even on public repos, so the HTTP code separates
  # "no prior successful run" (200, empty list: genuinely run) from "cannot know"
  # (anything else: skip).
  runs_resp="$(curl -sS -w '\n%{http_code}' -H "$auth" \
    "${api}/actions/workflows/${wf}/runs?status=success&per_page=1&exclude_pull_requests=true" \
    2>/dev/null || true)"
  runs_code="$(printf '%s' "$runs_resp" | tail -n1)"
  runs_body="$(printf '%s' "$runs_resp" | sed '$d')"

  if [ "$runs_code" != "200" ]; then
    echo "error: could not read runs for workflow '${wf}' (HTTP ${runs_code:-none}); skipping (grant 'actions: read' if this persists)"
    skip="true"
  else
    last_sha="$(printf '%s' "$runs_body" | jq -r '.workflow_runs[0].head_sha // empty' 2>/dev/null || true)"

    if [ -z "$last_sha" ]; then
      echo "no prior successful run of ${wf}; running"
    elif [ "$last_sha" = "$sha" ]; then
      skip="true"
      echo "no commits at all since last successful run ($last_sha); skipping"
    else
      compare_resp="$(curl -sS -w '\n%{http_code}' -H "$auth" "${api}/compare/${last_sha}...${sha}" \
        2>/dev/null || true)"
      compare_code="$(printf '%s' "$compare_resp" | tail -n1)"
      compare_body="$(printf '%s' "$compare_resp" | sed '$d')"

      if [ "$compare_code" != "200" ]; then
        echo "error: could not compare ${last_sha}...${sha} (HTTP ${compare_code:-none}); skipping (use workflow_dispatch to force)"
        skip="true"
      else
        status_code="$(printf '%s' "$compare_body" | jq -r '.status // empty' 2>/dev/null || true)"
        non_ci="$(printf '%s' "$compare_body" \
          | jq "[.commits[] | select((.commit.author.email // \"\") != \"${ci_author_email}\")] | length" \
          2>/dev/null || true)"

        if [ -z "$status_code" ] || [ -z "$non_ci" ]; then
          echo "error: invalid compare response for ${last_sha}...${sha}; skipping (use workflow_dispatch to force)"
          skip="true"
        elif [ "$non_ci" -eq 0 ]; then
          skip="true"
          echo "only CI-authored commits since last successful run ($last_sha); skipping"
        else
          echo "${non_ci} human-authored commit(s) since last successful run ($last_sha); running"
        fi
      fi
    fi
  fi
fi

echo "skip=${skip}"
[ -n "${GITHUB_OUTPUT:-}" ] && echo "skip=${skip}" >> "$GITHUB_OUTPUT"