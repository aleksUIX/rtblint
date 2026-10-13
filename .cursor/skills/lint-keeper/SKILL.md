---
name: lint-keeper
description: Keep vastlint, rtblint, pixellint, their infra repos, and sub-distributions healthy. Merge Dependabot PRs (patch, minor, major) after local tests, patch-release when runtime deps change, and keep GitHub Actions green without dropping tests, checks, or publish destinations. Use when scanning Dependabot, GitHub security alerts, RUSTSEC, npm audit, red CI, failed workflows, or when asked to patch-release dependency fixes.
---

# Lint keeper

Run end to end. Do not wait for a human unless a rule below says stop.

Owner: `aleksUIX`. Repo list and local test commands: [repos.md](repos.md).

## Inventory

Listing Dependabot PRs (GitHub pull requests / merge requests) is required every tick. Merge each one that passes local tests. Do not skip this step. A missing scan script or an empty injected queue is not proof the queue is empty; re-query GitHub.

Listing GitHub Actions is also required every tick. Fix red workflows. A missing or empty pipeline scan is not proof CI is green; re-list with `gh run list`.

From the vast-master workspace, run `vast-media/agents/lint-keeper/scan-security-queue.sh` and `vast-media/agents/lint-keeper/scan-pipelines.sh`. Dependabot fallbacks: `rtblint/scripts/scan-security-queue.sh`, `pixellint/scripts/scan-security-queue.sh`, `vastlint/scripts/scan-security-queue.sh`. Cover every repo in `repos.md`, including infra and sub-distributions. If this tick is scoped with `--repo`, that name is exact: `vastlint` is not `vastlint-java`.

The Dependabot scan lists all open pulls over REST and keeps `dependabot[bot]` plus `dependabot/` head refs. Do not rely on `gh pr list --author app/dependabot` alone; GraphQL can miss PRs.

The pipeline scan prints the latest run per workflow on the default branch, plus failures in the last 7 days on any ref (tag CI, scheduled jobs). Treat `failure`, `timed_out`, and `startup_failure` as red. Cancelled by a human can wait. In-progress can wait unless it is hung past the job timeout.

Per repo collect:

1. Open Dependabot PRs (`gh api --paginate repos/aleksUIX/<repo>/pulls?state=open&per_page=100`, keep dependabot authors and `dependabot/` heads). Required. Test locally. Merge if tests pass.
2. Red or broken GitHub Actions (injected pipeline queue, then `gh run list --repo aleksUIX/<repo>` and `gh run view <id> --log-failed`). Required. Fix root cause. No decay.
3. Open Dependabot alerts (`gh api repos/aleksUIX/<repo>/dependabot/alerts?state=open`)
4. Open code scanning alerts (`gh api repos/aleksUIX/<repo>/code-scanning/alerts?state=open`)
5. Open GitHub security advisories if the alerts API is empty
6. Local scanners when the tree is checked out: `cargo audit` (Rust), `npm audit --omit=dev` (published npm), `pnpm audit` / `npm audit` (infra)

Skip `vastlint-java-rebase`. It is not a published repo.

## What to merge

| Update | Action |
|--------|--------|
| Dependabot patch, minor, or major | Test the PR branch locally. If it fails to compile because our call sites use the old API, port them, re-test, then merge. Merge if tests pass. |
| Alert / GHSA / RUSTSEC / npm advisory, any bump size | Test locally. Merge or apply the bump if tests pass. |
| Red GitHub Actions (default branch, tag CI, scheduled) | Fix the root cause. Keep the same tests and publish destinations. Merge workflow-only fixes to main. Do not cut a product tag for Actions-only diffs. |
| Code scanning with a source path (CodeQL unused-variable, similar) | Fix in code. Test locally. Push. |
| Code scanning with no file (Scorecard branch protection, code review, CII badge) | Skip. Repo settings, not a patch. |
| Scorecard Binary-Artifacts on a committed wasm/binary we still ship | Skip and report. Do not delete the artifact. |

Merge with `gh pr merge <n> --squash --delete-branch`. If main requires a review and local tests plus required checks already passed, add `--admin`. Pull `main` after each merge.

## Local tests (required)

Never push or merge on CI green alone. Check out the PR branch (or apply the bump on a local branch), run the commands in [repos.md](repos.md), then merge or push.

If a Dependabot bump fails to compile because our call sites use the old API, port the call sites, re-run the local tests, then merge. Do not skip on the first compile error from that API break.

If tests still fail after that port: comment on the PR with the failure, skip that PR, continue other PRs and repos.

After a workflow fix, run the local tests in repos.md that still apply (typecheck, unit, e2e, site checks). Then push and re-run the failed workflow.

## No decay

Pipelines must stay green by preserving full functionality. Going green by weakening the suite is a failure.

Do not:

- Delete, skip, comment out, or `continue-on-error` test, lint, fmt, clippy, typecheck, e2e, audit, or smoke jobs
- Drop required status checks or branch protection
- Remove or `if: false` publish destinations (npm, crates.io, PyPI, Docker, Homebrew, VS Code, Open VSX, bindings, Chrome)
- Skip descendant jobs that should still run after a parent succeeds
- Lower rustc/clippy/eslint so old tags pass on new toolchains by hiding warnings
- Swap a real test for a stub that always succeeds
- Upload directories as release assets when the uploader expects files (filter files instead of dropping the upload)

Do:

- Fix labels, secrets, `if:` conditions, tag vs `workflow_dispatch` context, GITHUB_OUTPUT nulls, cosign flags, User-Agent headers, asset globs
- Port call sites when a dependency API broke
- Recreate missing GitHub labels the workflow writes
- Keep Chrome / binding / crates publish; fix how the job gets a tag or token
- After the fix, confirm the same jobs still exist and still run the same commands

Actions-only CI fixes: merge to main, no product tag.

## Patch release

Cut a patch only when published artifacts or runtime deps changed (Cargo.lock, crate/npm/gem/wheel/jar, FFI, NIF, action runtime). Not for GitHub Actions-only diffs.

**Base repos** (`vastlint`, `rtblint`, `pixellint`): bump the patch component, write CHANGELOG, tag `vX.Y.Z`, push commit and tag. Follow `vastlint/RELEASE.md` for vastlint. For rtblint: workspace `Cargo.toml` version, inter-crate version pins, `npm/package.json`, `crates/rtblint-mcp/server.json`, CHANGELOG heading `## X.Y.Z (YYYY-MM-DD)`, then `scripts/rebuild-npm-wasm.sh` before the tag. A version bump without rebuilding `npm/wasm` fails CI (`coreVersion()` drift) and can publish a stale blob to npm. For pixellint: workspace `Cargo.toml` version, inter-crate `pixellint-core` version pins, `npm/package.json`, CHANGELOG heading `## X.Y.Z - YYYY-MM-DD`, then rebuild `npm/wasm` before the tag (`wasm-pack build crates/pixellint-wasm --target nodejs --out-dir ../../npm/wasm --out-name pixellint`, drop generated `npm/wasm/{package.json,README.md,.gitignore}`, `node npm/test.mjs`). After the pixellint tag, refresh pixellint-infra (`PIXELLINT_REPO=../pixellint npm run refresh:wasm`) and push; a vendored engine ahead of crates.io is allowed. pixellint crates/npm publish waits on the `production` environment. Do not remove that gate.

**Version-locked sub-repos** (`vastlint-go`, `vastlint-python`, `vastlint-java`, `vastlint-erlang`, `vastlint-ruby`): merge to `main` only. Parent `sync-*` jobs tag them. Independent tags desync from the core release.

**Deploy-on-main** (`vastlint-infra`, `rtblint-infra`, `pixellint-infra`, `vastlint-action`, `homebrew-tap`): merge to `main`. Do not retag Homebrew formulas; parent `sync-homebrew-tap` owns `Formula/vastlint.rb`.

Do not bump the product version minor or major. Dependency bumps of any semver size are in scope. If the only product fix requires a minor/major version, stop and report.

vastlint patch tags publish without the `production` environment. Minor and major still wait for approval. Do not change that.

## Git

Conventional commits: `chore: bump <dep> to <version>` or `chore: release vX.Y.Z`. Workflow-only: `ci: ...`.

After every `git commit`, run `git log -1 --format=%B`. If the body contains `Co-authored-by: Cursor` or `Made-with: Cursor`, strip it with a message-only rewrite before push. Never add AI attribution trailers.

Do not `--force` to `main`. Do not skip hooks. Do not `git config`.

## Schedule prompt

When this runs unattended (daily cron):

First list every open Dependabot PR (GitHub pull request / merge request) and every red GitHub Actions run on every repo in repos.md, including vastlint-infra, rtblint-infra, and pixellint-infra. Re-query GitHub if a scan is missing, failed, or empty. Follow `.cursor/skills/lint-keeper/SKILL.md`. Test each Dependabot PR locally, then merge (patch, minor, major) when tests pass. If the bump fails to compile on an API break in our code, port call sites and merge. Fix red pipelines at the root cause with no decay (same tests, checks, and publish destinations). Fix in-code scanning findings (CodeQL with a file path). Cut a patch release on vastlint, rtblint, and pixellint when runtime deps changed. Infra is deploy-on-main: merge, do not tag. If a PR still fails tests after a port, comment, skip that PR, continue. Stop only on a required product minor/major bump. Reply with a per-repo list: merged, released, pipelines, skipped, blocked.
