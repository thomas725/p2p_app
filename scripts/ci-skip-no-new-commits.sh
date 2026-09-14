#!/usr/bin/env bash
# Detects whether a scheduled workflow's heavy tasks can be skipped because the
# default branch has not received any new commits since that workflow last
# completed "real" work.
#
# Commits authored by the CI pipeline itself (CI Pipeline
# <ci-pipeline@users.noreply.github.com>) are IGNORED, so self-generated commits
# (the nightly metrics/coverage push to main, the ci-results pushes) never count
# as new work. Manual workflow_dispatch always forces a full run.
#
# TWO MODES
# ---------
# 1. Run-history mode (default): the baseline is the head SHA of the last
#    `status=success` run of the workflow file named in $1. Requires `actions:
#    read` on the GITHUB_TOKEN; querying run history over the API otherwise
#    returns 403. This mode is used by workflows whose completion is not
#    reflected by a commit on the default branch (e.g. dependencies.yml,
#    nightly-apk.yml).
#
# 2. Commit-baseline mode (--commit-baseline=<git-log-grep>): the baseline is
#    the HEAD of the newest commit on the current branch whose subject matches
#    <git-log-grep> AND whose author is the CI pipeline. Used by workflows whose
#    "real work" is marked by a commit on the default branch (metrics.yml: the
#    `ci: refresh coverage and codebase metrics` commit). This mode is purely
#    local git: it never touches the GitHub API, so it cannot be tripped by
#    rate limits or 403s. Crucially it also keeps a SKIPPED run from becoming
#    the baseline: a skipped run pushes no commit, so the baseline stays at the
#    last run that actually did work and can never reseed the
#    "no commits at all" skip loop.
#
# RUNTIME REQUIREMENTS
# --------------------
# - Mode 1 needs the workflow permission `actions: read` (run-history query).
# - Mode 2 needs the checkout to carry full history (actions/checkout with
#   fetch-depth: 0) so the baseline and the rev-list compare resolve locally.
#
# This script is FAIL-CLOSED in both modes: in mode 1 any error querying the
# GitHub API (403, rate limit, network failure, unparseable response) results in
# skip=true. Mode 2 has no API round-trip at all; its only failure path is an
# unreadable local rev-list, which also results in skip=true. A skipped
# scheduled refresh is harmless (the next scheduled run or a manual
# workflow_dispatch recovers it); an unsolicited heavy run is the failure mode
# this exists to prevent.
#
# Usage: ci-skip-no-new-commits.sh <workflow-file-name> [--commit-baseline=<git-log-grep>]
#   e.g.    ci-skip-no-new-commits.sh dependencies.yml
#           ci-skip-no-new-commits.sh metrics.yml --commit-baseline="ci: refresh coverage and codebase metrics"
#
# Output: prints  skip=true|false  to stdout, and appends  skip=...  to
#   $GITHUB_OUTPUT when running as an Actions step (env var set by the runner).
set -euo pipefail

wf="${1:-}"
if [ -z "$wf" ]; then
  echo "error: workflow file name required" >&2
  exit 2
fi

commit_baseline_grep=""
for arg in "${@:2}"; do
  case "$arg" in
    --commit-baseline=*) commit_baseline_grep="${arg#*=}" ;;
    *) echo "error: unknown argument '$arg'" >&2; exit 2 ;;
  esac
done

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

if [ -z "$repo" ] || [ -z "$sha" ]; then
  echo "error: missing GITHUB_REPOSITORY/GITHUB_SHA; skipping conservatively"
  skip="true"
elif [ -n "$commit_baseline_grep" ]; then
  # ── Commit-baseline mode: purely local git, no API. ──
  # The baseline is the newest CI-authored commit on the current branch whose
  # subject matches the marker (e.g. the metrics "ci: refresh" commit). A full
  # run that produced nothing new does not push, so the baseline stays put and
  # the compare below still reflects "what is not yet covered by a refresh".
  baseline="$(git log -1 --format=%H --author="${ci_author_email}" \
    --grep="${commit_baseline_grep}" HEAD 2>/dev/null || true)"
  head_sha="$(git rev-parse --verify HEAD 2>/dev/null || true)"

  if [ -z "$baseline" ] || [ -z "$head_sha" ]; then
    echo "no prior commit matching '${commit_baseline_grep}'; running"
  elif [ "$baseline" = "$head_sha" ]; then
    skip="true"
    echo "no commits at all since last '${commit_baseline_grep}' commit (${baseline}); skipping"
  else
    new_human="$(git rev-list "${baseline}..${head_sha}" 2>/dev/null \
      | while read -r c; do git show -s --format=%ae "$c" 2>/dev/null; done \
      | grep -vc "^${ci_author_email}$" || true)"
    if [ -z "$new_human" ]; then
      echo "error: could not count commits since '${commit_baseline_grep}' (${baseline}); skipping (use workflow_dispatch to force)"
      skip="true"
    elif [ "$new_human" -eq 0 ]; then
      skip="true"
      echo "only CI-authored commits since last '${commit_baseline_grep}' commit (${baseline}); skipping"
    else
      echo "${new_human} human-authored commit(s) since last '${commit_baseline_grep}' commit (${baseline}); running"
    fi
  fi
elif [ -z "$token" ]; then
  echo "error: missing GITHUB_TOKEN; skipping conservatively (run-history mode)"
  skip="true"
else
  # ── Run-history mode: baseline = head of the last successful run. ──
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