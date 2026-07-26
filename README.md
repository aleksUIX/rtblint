# RTBlint

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
- Response markup coherence: `bid.adm` content vs the declared `bid.mtype` (native JSON encoding, VAST/DAAST roots, double-encoded payloads)
- Request/response cross-validation: with the originating request supplied, every bid's `impid`, `mtype`, `adm` markup, `dealid`, seat, and currency are checked against what the request actually offered

Every finding carries a stable rule id, a severity, a message, and a JSON path.

## Surfaces

| Surface | Package | Status |
|---------|---------|--------|
| Rust CLI | [`rtblint`](https://crates.io/crates/rtblint) | Working |
| Rust library | [`rtblint-core`](https://crates.io/crates/rtblint-core) | Working |
| MCP server | [`rtblint-mcp`](https://crates.io/crates/rtblint-mcp) | Working |
| Node (WASM) | `rtblint-core` on npm | Working |
| Python | `rtblint` on PyPI | Not implemented yet |
| Go | `github.com/aleksUIX/rtblint/go` | Not implemented yet |

OpenRTB 3.0 validates through its layered envelope: the transport objects (Openrtb, Request, Item, Deal, Source, Response, Seatbid, Bid) are checked in full, and the AdCOM domain objects under `item.spec` and `bid.media` are accepted as opaque until an AdCOM catalog ships. A 2.x payload sent to a 3.0 validator gets a migration diagnostic rather than a bare parse error. The 2.6-202204 snapshot has no extracted catalog and reports itself as unsupported instead of passing payloads silently. See [ROADMAP.md](ROADMAP.md) for what's next and [CHANGELOG.md](CHANGELOG.md) for release history.

## CLI

```bash
cargo install rtblint

rtblint validate request.json
rtblint validate --type response response.json
rtblint validate --type response --request request.json response.json
rtblint validate --version 2.5 --format json request.json
cat request.json | rtblint validate --stdin
```

`--request` supplies the originating bid request so the response is also cross-validated against it (works with `--batch` too: one request, many response lines).

Exit codes: 0 valid, 1 validation errors, 2 usage or I/O error.

## Node

```js
import { validate, validateResponse, validateResponseAgainstRequest } from "rtblint-core";

const report = validate(JSON.stringify(bidRequest), "2.6-202505");
if (!report.valid) {
  for (const issue of report.issues) {
    console.log(`[${issue.severity}] ${issue.path}: ${issue.message} (${issue.id})`);
  }
}

// Cross-validate a response against the request it answers.
const paired = validateResponseAgainstRequest(
  JSON.stringify(bidResponse),
  JSON.stringify(bidRequest)
);
```

## MCP server

`rtblint-mcp` speaks MCP over stdio and exposes `validate_bid_request`, `validate_bid_response` (with an optional `bid_request` argument for cross-validation), and `list_openrtb_versions`.

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

## JSON Schemas

`schemas/` holds a JSON Schema (draft 2020-12) per tracked version, for both payload types, generated from the same catalogs the validator uses. IAB Tech Lab publishes no JSON Schema for 2.6 or 3.0, so these are the machine-readable contract for each monthly snapshot:

```
https://rtblint.org/schemas/openrtb-2.6-202606-bid-request.schema.json
https://rtblint.org/schemas/openrtb-2.6-202606-bid-response.schema.json
```

They cover structure, types, required fields, documented value sets, and AdCOM enum lists. What they cannot express is what the linter adds: deprecated and moved paths, version-specific removals, and the semantic rules. Validating against a schema is not the same as linting.

Regenerate after any catalog change (CI fails if they drift):

```bash
cargo run -p rtblint-core --example export_json_schemas
```

## Spec data and provenance

The validator runs on structured catalogs extracted from the IAB Tech Lab OpenRTB specifications: object names, field names, type notations, enumerated value sets, and section citations. The catalogs carry no spec prose. RTBlint is not affiliated with or endorsed by IAB Tech Lab. See NOTICE for attribution.

## License

Apache-2.0. See LICENSE and NOTICE.
