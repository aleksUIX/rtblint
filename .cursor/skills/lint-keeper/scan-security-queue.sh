#!/usr/bin/env bash
# Inventory open Dependabot PRs (GitHub pull requests / merge requests),
# Dependabot alerts, and code scanning alerts.
#
# Lists all open pulls over REST and keeps dependabot[bot] plus dependabot/*
# head refs. `gh pr list --author app/dependabot` uses GraphQL and can miss PRs.
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

# Exact repo name only. REPO=vastlint does not include vastlint-java.
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

list_dependabot_prs() {
  local full="$1"
  local prs
  if prs=$(gh api --paginate "repos/${full}/pulls?state=open&per_page=100" \
    --jq '.[] | select(
      ((.user.login // "") | test("dependabot"; "i"))
      or ((.head.ref // "") | startswith("dependabot/"))
    ) | "- #\(.number) \(.title) \(.html_url)"' 2>/dev/null); then
    if [[ -z "$prs" ]]; then
      echo "- none"
    else
      printf '%s\n' "$prs"
    fi
  else
    echo "- (pr list failed)"
  fi
}

echo "# Security queue $(date -u +%Y-%m-%dT%H:%M:%SZ)"
echo

for repo in "${REPOS[@]}"; do
  full="${OWNER}/${repo}"
  echo "## ${full}"

  echo "### Dependabot PRs"
  list_dependabot_prs "$full"

  echo "### Dependabot alerts"
  if alerts=$(gh api --paginate "repos/${full}/dependabot/alerts?state=open" \
    --jq '.[] | "- #\(.number) \(.security_advisory.severity // "?") \(.security_advisory.summary) \(.html_url)"' 2>/dev/null); then
    if [[ -z "$alerts" ]]; then
      echo "- none"
    else
      printf '%s\n' "$alerts"
    fi
  else
    echo "- (alerts API unavailable or not enabled)"
  fi

  echo "### Code scanning"
  if scanning=$(gh api --paginate "repos/${full}/code-scanning/alerts?state=open" \
    --jq '.[] | "- #\(.number) \(.tool.name // "?") \(.rule.id // "?") \(.most_recent_instance.location.path // "-"):\(.most_recent_instance.location.start_line // 0) \(.html_url)"' 2>/dev/null); then
    if [[ -z "$scanning" ]]; then
      echo "- none"
    else
      printf '%s\n' "$scanning"
    fi
  else
    echo "- (code scanning API unavailable or not enabled)"
  fi

  echo
done
