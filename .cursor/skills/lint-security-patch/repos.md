# Repos

All under `aleksUIX`. Prefer sibling checkouts in `vast-master` when present; otherwise clone into `/tmp/lint-security-patch/<repo>`.

## Base (patch-tag on runtime dep changes)

| Repo | Local tests |
|------|-------------|
| `vastlint` | `cargo fmt --all -- --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --all`, `cargo audit` |
| `rtblint` | `cargo fmt --all --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo audit`. If proto or `package-lock.json` changed: `npm ci && npx buf lint && npx buf format --diff --exit-code`. **Before any version tag:** `scripts/rebuild-npm-wasm.sh` (committed `npm/wasm` must report the new `coreVersion()`). |
| `pixellint` | `cargo fmt --all --check`, `cargo test --workspace`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo audit`. If `npm/` changed: `node npm/test.mjs`. **Before any version tag:** `wasm-pack build crates/pixellint-wasm --target nodejs --out-dir ../../npm/wasm --out-name pixellint`, then `rm -f npm/wasm/package.json npm/wasm/README.md npm/wasm/.gitignore` and `node npm/test.mjs`. **After the tag:** refresh pixellint-infra (`PIXELLINT_REPO=../pixellint npm run refresh:wasm`) and push. |

## Sub-distributions (merge to main; no independent product tag)

| Repo | Local tests |
|------|-------------|
| `vastlint-go` | `go test ./...` |
| `vastlint-python` | `python -m pip install pytest`, then `PYTHONPATH=src python -m pytest tests/ -q` |
| `vastlint-java` | `./gradlew --no-daemon test` |
| `vastlint-erlang` | `mix deps.get && VASTLINT_BUILD=true mix test` (needs `native/vastlint_nif` → `vastlint/crates/vastlint-nif`) |
| `vastlint-ruby` | `ruby -Ilib:test test/vastlint_test.rb` |

## Deploy-on-main (merge; no product tag)

| Repo | Local tests |
|------|-------------|
| `vastlint-infra` | `pnpm install --frozen-lockfile`, `pnpm audit`, `pnpm --filter vastlint-web typecheck`, `pnpm --filter vastlint-web build` |
| `rtblint-infra` | `npm ci` in `apps/rtblint-web`, `npm audit`, `npm run typecheck`, `npm run build`. If root lockfile changed: `npm ci` and `npm run typecheck` at repo root |
| `pixellint-infra` | `npm ci`, `npm audit`, `npm run check:engine`, `npm run check:site` |
| `vastlint-action` | `action.yml` still parses; no unit suite |
| `homebrew-tap` | `ruby -c Formula/vastlint.rb`. Do not edit formula version/SHA; that is `sync-homebrew-tap` |

## Out of scope

`vastlint-java-rebase`
