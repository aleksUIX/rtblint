# Contributing to RTBlint

Thanks for helping make OpenRTB integrations less painful.

## Dev setup

Rust 1.74+ and cargo. The whole workspace builds with:

```bash
cargo test --workspace
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all --check
```

All three must pass; CI enforces them.

## Repo layout

- `crates/rtblint-core`: the validation engine and embedded spec catalogs
- `crates/rtblint`: the CLI
- `crates/rtblint-mcp`: MCP server over stdio
- `crates/rtblint-wasm`: wasm-bindgen bindings
- `npm/`: the Node package, wrapping a committed WASM build in `npm/wasm/`
- `python/`, `go/`: not-implemented-yet stubs

## What contributions land easily

- New semantic rules (cross-field checks the catalogs can't express) in
  `crates/rtblint-core/src/validator.rs`, with fixtures under
  `crates/rtblint-core/tests/fixtures/`
- Version delta rules in `crates/rtblint-core/src/version_rules.rs`
- AdCOM list additions in `crates/rtblint-core/src/adcom_lists.rs`
- Bug reports with a payload that validates wrong (see the issue template)

Every new rule needs a stable dotted id (`openrtb.<area>.<name>`), a fixture
exercising it, and a spec section reference.

## Spec catalogs

The JSON catalogs in `crates/rtblint-core/specs/` are generated from archived
IAB spec sources that are not part of this repository, so contributors cannot
regenerate them; treat them as maintainer-managed data. If a catalog entry
looks wrong, open an issue with the field, the version, and the spec section
instead of editing the JSON by hand.

The catalogs deliberately contain no spec prose, only structured metadata
(names, type notations, value sets, section references). Do not add
description text to them.

## WASM artifact

`npm/wasm/` is a committed build. If your change touches `rtblint-core` or
`rtblint-wasm`, rebuild it:

```bash
wasm-pack build crates/rtblint-wasm --target nodejs --out-dir ../../npm/wasm --out-name rtblint_wasm
rm -f npm/wasm/package.json && : > npm/wasm/.gitignore
```

CI verifies the WASM crate still builds.

## Commit style

Short imperative subject lines. One logical change per commit.
