# rtblint

**OpenRTB linter** — the repository currently contains an early Rust validator for versioned OpenRTB bid requests, with the strongest coverage in the tracked 2.6 snapshots.

> Published `0.0.1` registry packages are still stub releases. Repository HEAD now includes a working Rust core and local CLI, but MCP, WASM, Go, and Python runtime surfaces are still placeholders.

[![Crates.io](https://img.shields.io/crates/v/rtblint.svg)](https://crates.io/crates/rtblint)
[![npm](https://img.shields.io/npm/v/rtblint.svg)](https://www.npmjs.com/package/rtblint)
[![PyPI](https://img.shields.io/pypi/v/rtblint.svg)](https://pypi.org/project/rtblint/)

## Current Repo Status

- `rtblint-core` validates OpenRTB `2.6-202505` bid request payloads for malformed JSON, required fields, unknown fields, moved and deprecated paths, basic type mismatches, documented enum values, and a narrow set of semantic rules.
- The Rust CLI supports `rtblint validate <file.json>` and `rtblint validate --stdin`, plus `--version <openrtb-version>` and `--format json` for version-aware machine output.
- External bid request fixtures now cover representative valid, warning, and error cases for the current `2.6` validator slice.
- A parallel bid response fixture inventory now exists, but response validation itself is not wired yet.
- Rust tests and CI currently cover the Rust workspace.
- Bid response validation, OpenRTB `3.0` payload validation, the MCP server, and non-Rust bindings are not implemented yet.

## Packages

| Ecosystem | Package | Registry |
|-----------|---------|----------|
| Rust CLI  | `rtblint` | [crates.io/crates/rtblint](https://crates.io/crates/rtblint) |
| Rust lib  | `rtblint-core` | [crates.io/crates/rtblint-core](https://crates.io/crates/rtblint-core) |
| Rust MCP  | `rtblint-mcp` | [crates.io/crates/rtblint-mcp](https://crates.io/crates/rtblint-mcp) |
| Node/WASM | `rtblint` | [npmjs.com/package/rtblint](https://www.npmjs.com/package/rtblint) |
| Python    | `rtblint` | [pypi.org/project/rtblint](https://pypi.org/project/rtblint/) |
| Go        | `github.com/aleksUIX/rtblint/go` | [pkg.go.dev](https://pkg.go.dev/github.com/aleksUIX/rtblint/go) |

## Roadmap

- [ ] 0.1.0 — first non-stub Rust release for OpenRTB 2.6 bid request validation
- [ ] 0.2.0 — OpenRTB 2.6 bid response validation and broader CLI coverage
- [ ] 0.3.0 — OpenRTB 3.0 + AdCOM support
- [ ] 0.4.0 — MCP server, WASM, Go/Python bindings

## Local Usage

```bash
cargo run -p rtblint -- validate path/to/request.json
cat request.json | cargo run -p rtblint -- validate --stdin
cargo run -p rtblint -- validate --version 2.5 --format json path/to/request.json
```

## License

Apache-2.0
