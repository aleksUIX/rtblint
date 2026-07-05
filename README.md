# rtblint

**OpenRTB linter.** Validates OpenRTB 2.x bid requests and bid responses against versioned IAB Tech Lab spec snapshots, from 2.0 through the monthly 2.6 releases (currently up to 2.6-202606).

[![Crates.io](https://img.shields.io/crates/v/rtblint.svg)](https://crates.io/crates/rtblint)
[![CI](https://github.com/aleksUIX/rtblint/actions/workflows/rust.yml/badge.svg)](https://github.com/aleksUIX/rtblint/actions)

Website and playground: [rtblint.org](https://rtblint.org)

## What it checks

- Malformed JSON and wrong top-level shape
- Required fields, including required non-empty arrays
- Unknown objects and fields per version catalog (`ext` subtrees stay open)
- Type mismatches (string, integer, float, boolean, object, and array forms)
- Documented enum values, including AdCOM lists and vendor ranges (500+)
- Deprecated, moved, removed, and not-yet-available fields across versions
- Semantic rules: site/app/dooh exclusivity, imp media type presence, skippable video dependencies, duration exclusivity, seatbid/nbr presence on responses, and more

Every finding carries a stable rule id, a severity, a message, and a JSON path.

## Surfaces

| Surface | Package | Status |
|---------|---------|--------|
| Rust CLI | [`rtblint`](https://crates.io/crates/rtblint) | Working |
| Rust library | [`rtblint-core`](https://crates.io/crates/rtblint-core) | Working |
| MCP server | [`rtblint-mcp`](https://crates.io/crates/rtblint-mcp) | Working |
| Node (WASM) | `rtblint` on npm | Built, publish pending |
| Python | `rtblint` on PyPI | Not implemented yet |
| Go | `github.com/aleksUIX/rtblint/go` | Not implemented yet |

OpenRTB 3.0 payload validation is not implemented yet (the 3.0 catalog ships for introspection only). The 2.6-202204 snapshot has no extracted catalog and reports itself as unsupported instead of passing payloads silently.

## CLI

```bash
cargo install rtblint

rtblint validate request.json
rtblint validate --type response response.json
rtblint validate --version 2.5 --format json request.json
cat request.json | rtblint validate --stdin
```

Exit codes: 0 valid, 1 validation errors, 2 usage or I/O error.

## Node

```js
import { validate, validateResponse, versions } from "rtblint";

const report = validate(JSON.stringify(bidRequest), "2.6-202505");
if (!report.valid) {
  for (const issue of report.issues) {
    console.log(`[${issue.severity}] ${issue.path}: ${issue.message} (${issue.id})`);
  }
}
```

## MCP server

`rtblint-mcp` speaks MCP over stdio and exposes `validate_bid_request`, `validate_bid_response`, and `list_openrtb_versions`.

```json
{
  "mcpServers": {
    "rtblint": { "command": "rtblint-mcp" }
  }
}
```

## Rust library

```rust
use rtblint_core::{validate_bid_request_for_version, OpenRtbVersion};

let result = validate_bid_request_for_version(OpenRtbVersion::V2_6_202505, payload);
for issue in &result.issues {
    println!("{} {} {:?}", issue.severity, issue.id, issue.path);
}
```

## Spec data and provenance

The validator runs on structured catalogs extracted from the IAB Tech Lab OpenRTB specifications: object names, field names, type notations, enumerated value sets, and section citations. The catalogs carry no spec prose. rtblint is not affiliated with or endorsed by IAB Tech Lab. See NOTICE for attribution.

## License

Apache-2.0. See LICENSE and NOTICE.
