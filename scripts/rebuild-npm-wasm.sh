#!/usr/bin/env bash
# Rebuild the committed Node WASM package. Required before every rtblint version tag.
# See CONTRIBUTING.md.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
wasm-pack build crates/rtblint-wasm --target nodejs --out-dir ../../npm/wasm --out-name rtblint_wasm
rm -f npm/wasm/package.json
: > npm/wasm/.gitignore
workspace_version=$(grep '^version = ' Cargo.toml | head -1 | cut -d'"' -f2)
node <<EOF
const r = require("./npm/index.cjs");
const pkg = require("./npm/package.json");
const out = r.validate(JSON.stringify({ id: "ci", imp: [{ id: "i", banner: {} }] }));
if (!out.valid) {
  throw new Error("wasm smoke failed: " + JSON.stringify(out));
}
if (r.coreVersion() !== "$workspace_version" || pkg.version !== "$workspace_version") {
  throw new Error("version drift: workspace $workspace_version, wasm " + r.coreVersion() + ", package.json " + pkg.version);
}
console.log("npm wasm ok, core", r.coreVersion());
EOF
