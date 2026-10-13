#!/usr/bin/env bash
# Latest GitHub Actions result per workflow on the default branch, plus
# failures in the last 7 days on any ref (tag CI, scheduled jobs).
set -euo pipefail

OWNER="${OWNER:-aleksUIX}"
REPOS=(
  vastlint
  vastlint-go
  vastlint-python
  vastlint-java
  vastlint-erlang
  vastlint-ruby
  vastlint-action
  vastlint-infra
  homebrew-tap
  rtblint
  rtblint-infra
  pixellint
  pixellint-infra
)

if [[ -n "${REPO:-}" ]]; then
  matched=0
  for repo in "${REPOS[@]}"; do
    if [[ "$repo" == "$REPO" ]]; then
      matched=1
      break
    fi
  done
  if [[ "$matched" -ne 1 ]]; then
    echo "unknown REPO=${REPO} (exact name from repos.md)" >&2
    exit 1
  fi
  REPOS=("$REPO")
fi

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing $1" >&2
    exit 1
  }
}

need gh
need jq

SINCE="$(date -u -v-7d +%Y-%m-%dT%H:%M:%SZ 2>/dev/null || date -u -d '7 days ago' +%Y-%m-%dT%H:%M:%SZ)"

echo "# Pipeline queue $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo
echo "since ${SINCE}"
echo

for repo in "${REPOS[@]}"; do
  full="${OWNER}/${repo}"
  echo "## ${full}"

  default="$(gh api "repos/${full}" --jq .default_branch 2>/dev/null || echo main)"

  echo "### Latest on ${default} (per workflow)"
  if latest=$(gh run list --repo "$full" --branch "$default" --limit 40 \
    --json workflowName,conclusion,status,url,event,createdAt,displayTitle 2>/dev/null); then
    grouped=$(printf '%s' "$latest" | jq -r '
      unique_by(.workflowName)
      | .[]
      | "- \(.workflowName) \(.conclusion // .status) \(.event) \(.url)"
    ')
    if [[ -z "$grouped" ]]; then
      echo "- none"
    else
      printf '%s\n' "$grouped"
    fi
  else
    echo "- (run list failed)"
  fi

  echo "### Red in 7d (any ref)"
  if red=$(gh run list --repo "$full" --limit 40 \
    --json workflowName,conclusion,status,url,event,createdAt,headBranch,displayTitle 2>/dev/null); then
    failed=$(printf '%s' "$red" | jq -r --arg since "$SINCE" '
      .[]
      | select(.createdAt >= $since)
      | select(.conclusion == "failure" or .conclusion == "timed_out" or .conclusion == "startup_failure")
      | "- \(.workflowName) \(.conclusion) \(.event) \(.headBranch) \(.url)"
    ')
    if [[ -z "$failed" ]]; then
      echo "- none"
    else
      printf '%s\n' "$failed"
    fi
  else
    echo "- (run list failed)"
  fi

  echo
done
