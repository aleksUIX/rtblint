---
name: lint-security-patch
description: Daily Dependabot and vulnerability sweep for vastlint, rtblint, pixellint, their infra repos (vastlint-infra, rtblint-infra, pixellint-infra), and their sub-distribution repos. Merges Dependabot PRs (patch, minor, and major) after local tests, then cuts a patch release. Use when scanning Dependabot PRs, GitHub security alerts, RUSTSEC, npm audit, or when asked to patch-release dependency fixes.
---

# Lint security patch

Run end to end. Do not wait for a human unless a rule below says stop.

Owner: `aleksUIX`. Repo list and local test commands: [repos.md](repos.md).

## Inventory

Listing Dependabot PRs (GitHub pull requests / merge requests) is required every tick. Merge each one that passes local tests. Do not skip this step. A missing scan script or an empty injected queue is not proof the queue is empty; re-query GitHub.

From the vast-master workspace, run `vast-media/agents/lint-security-patch/scan-security-queue.sh`. Fallbacks: `rtblint/scripts/scan-security-queue.sh`, `pixellint/scripts/scan-security-queue.sh`, `vastlint/scripts/scan-security-queue.sh`. Cover every repo in `repos.md`, including infra and sub-distributions. If this tick is scoped with `--repo`, that name is exact: `vastlint` is not `vastlint-java`.

The scan lists all open pulls over REST and keeps `dependabot[bot]` plus `dependabot/` head refs. Do not rely on `gh pr list --author app/dependabot` alone; GraphQL can miss PRs.

Per repo collect:

1. Open Dependabot PRs (`gh api --paginate repos/aleksUIX/<repo>/pulls?state=open&per_page=100`, keep dependabot authors and `dependabot/` heads). Required. Test locally. Merge if tests pass.
2. Open Dependabot alerts (`gh api repos/aleksUIX/<repo>/dependabot/alerts?state=open`)
3. Open code scanning alerts (`gh api repos/aleksUIX/<repo>/code-scanning/alerts?state=open`)
4. Open GitHub security advisories if the alerts API is empty
5. Local scanners when the tree is checked out: `cargo audit` (Rust), `npm audit --omit=dev` (published npm), `pnpm audit` / `npm audit` (infra)

Skip `vastlint-java-rebase`. It is not a published repo.

## What to merge

| Update | Action |
|--------|--------|
| Dependabot patch, minor, or major | Test the PR branch locally. If it fails to compile because our call sites use the old API, port them, re-test, then merge. Merge if tests pass. |
| Alert / GHSA / RUSTSEC / npm advisory, any bump size | Test locally. Merge or apply the bump if tests pass. |
| Code scanning with a source path (CodeQL unused-variable, similar) | Fix in code. Test locally. Push. |
| Code scanning with no file (Scorecard branch protection, code review, CII badge) | Skip. Repo settings, not a patch. |
| Scorecard Binary-Artifacts on a committed wasm/binary we still ship | Skip and report. Do not delete the artifact. |

Merge with `gh pr merge <n> --squash --delete-branch`. If main requires a review and local tests plus required checks already passed, add `--admin`. Pull `main` after each merge.

## Local tests (required)

Never push or merge on CI green alone. Check out the PR branch (or apply the bump on a local branch), run the commands in [repos.md](repos.md), then merge or push.

If a Dependabot bump fails to compile because our call sites use the old API, port the call sites, re-run the local tests, then merge. Do not skip on the first compile error from that API break.

If tests still fail after that port: comment on the PR with the failure, skip that PR, continue other PRs and repos.

## Patch release

Cut a patch only when published artifacts or runtime deps changed (Cargo.lock, crate/npm/gem/wheel/jar, FFI, NIF, action runtime). Not for GitHub Actions-only diffs.

**Base repos** (`vastlint`, `rtblint`, `pixellint`): bump the patch component, write CHANGELOG, tag `vX.Y.Z`, push commit and tag. Follow `vastlint/RELEASE.md` for vastlint. For rtblint: workspace `Cargo.toml` version, inter-crate version pins, `npm/package.json`, `crates/rtblint-mcp/server.json`, CHANGELOG heading `## X.Y.Z (YYYY-MM-DD)`, then `scripts/rebuild-npm-wasm.sh` before the tag. A version bump without rebuilding `npm/wasm` fails CI (`coreVersion()` drift) and can publish a stale blob to npm. For pixellint: workspace `Cargo.toml` version, inter-crate `pixellint-core` version pins, `npm/package.json`, CHANGELOG heading `## X.Y.Z - YYYY-MM-DD`, then rebuild `npm/wasm` before the tag (`wasm-pack build crates/pixellint-wasm --target nodejs --out-dir ../../npm/wasm --out-name pixellint`, drop generated `npm/wasm/{package.json,README.md,.gitignore}`, `node npm/test.mjs`). After the pixellint tag, refresh pixellint-infra (`PIXELLINT_REPO=../pixellint npm run refresh:wasm`) and push; a vendored engine ahead of crates.io is allowed. pixellint crates/npm publish waits on the `production` environment. Do not remove that gate.

**Version-locked sub-repos** (`vastlint-go`, `vastlint-python`, `vastlint-java`, `vastlint-erlang`, `vastlint-ruby`): merge to `main` only. Parent `sync-*` jobs tag them. Independent tags desync from the core release.

**Deploy-on-main** (`vastlint-infra`, `rtblint-infra`, `pixellint-infra`, `vastlint-action`, `homebrew-tap`): merge to `main`. Do not retag Homebrew formulas; parent `sync-homebrew-tap` owns `Formula/vastlint.rb`.

Do not bump the product version minor or major. Dependency bumps of any semver size are in scope. If the only product fix requires a minor/major version, stop and report.

vastlint patch tags publish without the `production` environment. Minor and major still wait for approval. Do not change that.

## Git

Conventional commits: `chore: bump <dep> to <version>` or `chore: release vX.Y.Z`.

After every `git commit`, run `git log -1 --format=%B`. If the body contains `Co-authored-by: Cursor` or `Made-with: Cursor`, strip it with a message-only rewrite before push. Never add AI attribution trailers.

Do not `--force` to `main`. Do not skip hooks. Do not `git config`.

## Schedule prompt

When this runs unattended (daily cron):

First list every open Dependabot PR (GitHub pull request / merge request) on every repo in repos.md, including vastlint-infra, rtblint-infra, and pixellint-infra. Re-query GitHub if the scan is missing, failed, or empty. Follow `.cursor/skills/lint-security-patch/SKILL.md`. Test each Dependabot PR locally, then merge (patch, minor, major) when tests pass. If the bump fails to compile on an API break in our code, port call sites and merge. Fix in-code scanning findings (CodeQL with a file path). Cut a patch release on vastlint, rtblint, and pixellint when runtime deps changed. Infra is deploy-on-main: merge, do not tag. If a PR still fails tests after a port, comment, skip that PR, continue. Stop only on a required product minor/major bump. Reply with a per-repo list: merged, released, skipped, blocked.
