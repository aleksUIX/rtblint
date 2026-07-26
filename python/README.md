# RTBlint

**OpenRTB linter.** Validates OpenRTB 2.x bid requests and bid responses against versioned IAB spec snapshots.

> The Python binding is not implemented yet; `rtblint.validate()` raises `NotImplementedError`. The working surfaces today are the Rust CLI and library ([crates.io/crates/rtblint](https://crates.io/crates/rtblint)), the npm WASM package, and the MCP server ([crates.io/crates/rtblint-mcp](https://crates.io/crates/rtblint-mcp)).

Website and playground: [rtblint.org](https://rtblint.org)

## In the meantime

```bash
cargo install rtblint
rtblint validate request.json
rtblint validate --type response response.json
```

## License

Apache-2.0. See LICENSE and NOTICE in the repository.
