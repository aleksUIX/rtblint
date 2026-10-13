#!/usr/bin/env bash
# Simulated ARTF hop. Host and agent are print statements. RTBlint is real.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
FIX="$ROOT/crates/rtblint-core/tests/fixtures/artf"

if command -v rtblint >/dev/null 2>&1; then
  RTB=(rtblint)
else
  RTB=(cargo run -q -p rtblint --manifest-path "$ROOT/Cargo.toml" --)
fi

run() {
  "${RTB[@]}" "$@"
}

# Validate can exit 1 on findings. Capture stdout anyway.
capture() {
  local out="$1"
  shift
  run "$@" >"$out" || true
}

errors() {
  # Count "error" severities in JSON output. Compatible with the CLI's --format json.
  python3 -c '
import json, sys
doc = json.load(sys.stdin)
issues = doc.get("issues") or doc.get("result", {}).get("issues") or []
print(sum(1 for i in issues if str(i.get("severity", "")).lower() == "error"))
'
}

hr() {
  printf '\n── %s ──\n' "$1"
}

beat_header() {
  printf '\n\n======== BEAT %s: %s ========\n' "$1" "$2"
  printf '%s\n' "$3"
}

forward_line() {
  local n="$1"
  if [ "$n" -gt 0 ]; then
    printf 'would_forward: false  (%s error(s); host drops the mutation set)\n' "$n"
  else
    printf 'would_forward: true   (no errors; host forwards)\n'
  fi
}

# -- Beat 1 ---------------------------------------------------------------
beat_header 1 "Looks legal. Is not." \
  "Host opens extension-point-001. Agent answers with extension-point-999 and a pile of unresolvable mutations."

hr "HOST  emits RTBRequest envelope"
capture /tmp/rtblint-artf-env.json validate --type artf-request --format json "$FIX/valid-publisher-request.json"
printf 'envelope errors: %s\n' "$(errors < /tmp/rtblint-artf-env.json)"

hr "AGENT proposes mutations"
printf 'payload: %s\n' "$FIX/invalid-mutations.json"

hr "RTBLINT pass 2: mutations vs auction"
capture /tmp/rtblint-artf-static.json validate --type artf-response --request "$FIX/valid-publisher-request.json" --format json \
  "$FIX/invalid-mutations.json"
n="$(errors < /tmp/rtblint-artf-static.json)"
printf 'mutation errors: %s\n' "$n"
run validate --type artf-response --request "$FIX/valid-publisher-request.json" \
  "$FIX/invalid-mutations.json" || true
hr "HOST"
forward_line "$n"
printf 'punchline: The framework accepted the RPC. The auction was never asked.\n'

# -- Beat 2 ---------------------------------------------------------------
beat_header 2 "Mutation is fine. Auction is not." \
  "ADD_METRICS with value \"high\". Static ARTF checks pass. Apply writes a string into a numeric field."

hr "RTBLINT pass 2: static"
capture /tmp/rtblint-artf-b2-static.json validate --type artf-response --request "$FIX/valid-publisher-request.json" --format json \
  "$FIX/breaking-mutations.json"
printf 'static errors: %s\n' "$(errors < /tmp/rtblint-artf-b2-static.json)"

hr "RTBLINT pass 3: apply then revalidate"
capture /tmp/rtblint-artf-b2-apply.json validate --type artf-response --apply --request "$FIX/valid-publisher-request.json" --format json \
  "$FIX/breaking-mutations.json"
n="$(errors < /tmp/rtblint-artf-b2-apply.json)"
printf 'applied errors: %s\n' "$n"
run validate --type artf-response --apply --request "$FIX/valid-publisher-request.json" \
  "$FIX/breaking-mutations.json" || true
hr "HOST"
forward_line "$n"
printf 'punchline: Well-formed mutation and still-valid OpenRTB are different questions.\n'

# -- Beat 3 ---------------------------------------------------------------
beat_header 3 "Clean pass. Forward it." \
  "Activate segments and a deal, raise deal-premium to \$14.50, add a numeric viewability metric."

hr "RTBLINT pass 3: apply then revalidate"
capture /tmp/rtblint-artf-b3.json validate --type artf-response --apply --request "$FIX/valid-publisher-request.json" --format json \
  "$FIX/valid-mutations.json"
n="$(errors < /tmp/rtblint-artf-b3.json)"
printf 'applied errors: %s\n' "$n"
run validate --type artf-response --apply --request "$FIX/valid-publisher-request.json" \
  "$FIX/valid-mutations.json"
hr "HOST"
forward_line "$n"
printf 'punchline: Empty finding list. The host can forward.\n'
