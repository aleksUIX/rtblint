# RTBlint

**OpenRTB linter.** Validates OpenRTB 2.x bid requests and bid responses against versioned IAB Tech Lab spec snapshots, from 2.0 through the monthly 2.6 releases (currently up to 2.6-202606).

[![Crates.io](https://img.shields.io/crates/v/rtblint.svg)](https://crates.io/crates/rtblint)
[![CI](https://github.com/aleksUIX/rtblint/actions/workflows/rust.yml/badge.svg)](https://github.com/aleksUIX/rtblint/actions)
[![smithery badge](https://smithery.ai/badge/aleksander/rtblint)](https://smithery.ai/servers/aleksander/rtblint)
[![OpenSSF Scorecard](https://api.scorecard.dev/projects/github.com/aleksUIX/rtblint/badge)](https://scorecard.dev/viewer/?uri=github.com/aleksUIX/rtblint)

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
- JSON dialect: spec JSON types flag fields such as `imp.secure` and `regs.coppa` as integers, while the IAB OpenRTB protobuf schema declares 28 of them `bool`. Either encoding is correct on its own transport and wrong on the other, so the caller declares which one it meant
- Exchange profiles: documented protocol extras on top of the spec. `--profile google-ab` accepts `at: 3` (FIXED_PRICE) and requires `Imp.ext.billing_id`. `--profile prebid-server` requires each Imp to name a bidder or stored request and refuses `wseat`/`bseat`. `--profile xandr` requires `ext.appnexus.seller_member_id` and video `ext.appnexus.context`. `--profile magnite` requires xAPI identity fields (`imp.ext.rp.zone_id`, site/app `ext.rp.site_id`, `publisher.ext.rp.account_id`). Business policy stays out
- Nested specs OpenRTB carries as strings or opaque `ext`: Native Ads 1.2 markup (`imp.native.request` and native `bid.adm`, including required-asset pairing), GPP header vs `gpp_sid` and TCF 2 shape, `${AUCTION_*}` macros on billing and loss URLs, EID/SUA structure, SKAdNetwork `ext.skadn`
- ARTF envelopes and mutation sets, including applying the mutations and revalidating what comes out

Every finding carries a stable rule id, a severity, a message, and a JSON path.

## ARTF

[ARTF](https://iabtechlab.com/standards/artf/), the IAB Tech Lab Agentic Real Time Framework, hands an agent an OpenRTB payload inside an `RTBRequest` envelope and takes back *mutations*: proposed changes the orchestrator may accept or reject one at a time. Nothing in the framework checks that the auction still validates once they are applied, and a mutation is only meaningful relative to the request it targets.

```bash
# The envelope, plus full OpenRTB validation of what it carries
rtblint validate --type artf-request rtb-request.json

# The mutation set against the auction it targets
rtblint validate --type artf-response --request rtb-request.json rtb-response.json

# Apply the mutations, revalidate, and report only what the mutations broke
rtblint validate --type artf-response --apply --request rtb-request.json rtb-response.json
```

Three passes:

- **Envelope.** Required members, `lifecycle` against the payloads actually carried, `tmax` plausibility for an in-auction call, `originator` and `applicable_intents` enum values, and the carried bid request and bid response validated as protobuf JSON.
- **Mutations.** The response id echoes the extension point request id (not the bid request id), each declared intent is in `applicable_intents`, the operation and payload oneof member match the intent, and every semantic path (`/imp/{id}`, `/imp/{id}/pmp/deals/{id}`, `/user/data/segment`, `/seatbid/{seat}/bid/{id}`) resolves to something the auction carries. `ADJUST_DEAL_MARGIN` is reported as having no OpenRTB field to write to, because it does not.
- **Applied.** The mutations are written in and the result revalidated, reporting the OpenRTB findings the mutations introduced with pre-existing findings filtered out. What the agent broke, not what arrived broken.

The ARTF v1.0 document and its `.proto` use different vocabularies for the same mutation (`activateSegments` and a `value: {IDsPayload: ...}` wrapper against `ACTIVATE_SEGMENTS` and top-level oneof members). Payloads written from the document are mapped and reported as `artf.mutation.legacy_spec_encoding` rather than dismissed as unknown.

## Surfaces

| Surface | Package | Status |
|---------|---------|--------|
| Rust CLI | [`rtblint`](https://crates.io/crates/rtblint) | Working |
| Rust library | [`rtblint-core`](https://crates.io/crates/rtblint-core) | Working |
| MCP server | [`rtblint-mcp`](https://crates.io/crates/rtblint-mcp) | Working |
| Node (WASM) | `rtblint-core` on npm | Working |
| GitHub Action | [`aleksUIX/rtblint`](https://github.com/aleksUIX/rtblint) | Working |
| Python | `rtblint` on PyPI | Not implemented yet |
| Go | `github.com/aleksUIX/rtblint/go` | Not implemented yet |

OpenRTB 3.0 validates through its layered envelope: the transport objects (Openrtb, Request, Item, Deal, Source, Response, Seatbid, Bid) and the AdCOM 1.0 domain objects under `item.spec` (Placement), `bid.media` (Ad), and `request.context`. A 2.x payload sent to a 3.0 validator gets a migration diagnostic rather than a bare parse error. The 2.6-202204 snapshot has no extracted catalog and reports itself as unsupported instead of passing payloads silently. See [ROADMAP.md](ROADMAP.md) for what's next and [CHANGELOG.md](CHANGELOG.md) for release history.

## CLI

```bash
cargo install rtblint

rtblint validate request.json
rtblint validate --type response response.json
rtblint validate --type response --request request.json response.json
rtblint validate --version 2.5 --format json request.json
rtblint validate --dialect proto-json grpc-bid-request.json
rtblint validate --profile google-ab google-bid-request.json
rtblint validate --profile prebid-server pbs-auction.json
rtblint validate --profile xandr xandr-bid-request.json
rtblint validate --profile magnite magnite-bid-request.json
rtblint validate --resolve --cache ./supply-cache request.json
rtblint validate --summary bids.ndjson
rtblint validate --batch --summary bids.ndjson
cat request.json | rtblint validate --stdin
```

`--request` supplies the originating bid request so the response is also cross-validated against it (works with `--batch` too: one request, many response lines). `--dialect proto-json` validates a payload that came off a gRPC bidstream integration. `--profile google-ab` applies Google Authorized Buyers' documented protocol extras (`at: 3` FIXED_PRICE, required `Imp.ext.billing_id`) on top of the spec. `--profile prebid-server` applies Prebid Server `/openrtb2/auction` extras (bidder or stored request on each Imp, no `wseat`/`bseat`). `--profile xandr` applies Microsoft Monetize extras (`ext.appnexus.seller_member_id`, video `ext.appnexus.context`). `--profile magnite` applies Magnite xAPI identity fields. `--resolve --cache <dir>` checks SupplyChain hops against sellers.json and the publisher's ads.txt / app-ads.txt from a local directory:

```text
<dir>/sellers/<asi>/sellers.json
<dir>/ads/<site.domain>/ads.txt
<dir>/app-ads/<app.bundle>/app-ads.txt
```

Nothing is fetched; populate the cache yourself. `--batch` lints one JSON object per line from a file or stdin. `--summary` adds rule-frequency totals for a captured stream (`--summary bids.ndjson` for the histogram alone). See [ARTF](#artf) for `--type artf-request` and `--type artf-response`.

Exit codes: 0 valid, 1 validation errors, 2 usage or I/O error.

## GitHub Action

The Action lives in this repo. Pin a release tag so CI downloads that CLI tarball:

```yaml
- uses: aleksUIX/rtblint@v0.11.0
  with:
    path: fixtures/bid-request.json
    spec-version: 2.6-202505
```

`version` selects the CLI release (`auto` follows the action's own `v*` tag). `spec-version` is the OpenRTB snapshot. Linux and macOS runners, x86_64 and aarch64.

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

For gRPC bidstream payloads and ARTF:

```js
import {
  validateDialect,
  validateProfile,
  validateArtfRequest,
  validateArtfResponseApplied,
  protoBoolDivergences,
} from "rtblint-core";

validateDialect(JSON.stringify(bidRequest), "proto-json");
validateProfile(JSON.stringify(bidRequest), "google-ab");
validateProfile(JSON.stringify(bidRequest), "prebid-server");
validateArtfRequest(JSON.stringify(rtbRequest));

// { result, application }: what the mutations broke, and the payloads they produced
const { result, application } = validateArtfResponseApplied(
  JSON.stringify(rtbResponse),
  JSON.stringify(rtbRequest)
);

protoBoolDivergences(); // the 28 fields the two schemas type differently
```

## MCP server

Hosted Streamable HTTP (no install): [https://rtblint.org/mcp](https://rtblint.org/mcp). Smithery listing: [aleksander/rtblint](https://smithery.ai/servers/aleksander/rtblint) (same account as vastlint).

`rtblint-mcp` also speaks MCP over stdio. Tools: `validate_bid_request`, `validate_bid_response` (optional `bid_request` for cross-validation), `validate_artf_request`, `validate_artf_response` (`apply` writes the mutations and revalidates), `list_openrtb_versions`, `get_adcp_capabilities`. Validation tools take optional `dialect` and `profile` arguments.

The ARTF tools are the guardrail an agent calls around its own work: check the envelope it was handed, then check the mutation set it is about to propose, before the orchestrator sees it.

```json
{
  "mcpServers": {
    "rtblint": { "url": "https://rtblint.org/mcp" }
  }
}
```

Local stdio instead of the hosted endpoint:

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

`validate_bid_request_with_profile` applies an exchange profile (`Profile::GoogleAuthorizedBuyers`, `Profile::PrebidServer`) on top of the spec. `validate_bid_request_with_dialect` selects spec JSON vs protobuf JSON.

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

## Documentation

Beyond this README, [rtblint.org](https://rtblint.org) hosts the reference material:

- [Diagnostic code reference](https://rtblint.org/docs/rule-reference/): every stable issue id, each with a page covering what it means, why it matters, and how to fix it
- [Versioned rule catalog](https://rtblint.org/docs/rules/): what every OpenRTB release added, deprecated, moved, or removed
- [OpenRTB versions](https://rtblint.org/docs/openrtb-versions/): a page per tracked version, 2.0 through 3.0
- [Common OpenRTB mistakes](https://rtblint.org/guides/common-openrtb-mistakes/) and [validating in CI](https://rtblint.org/guides/openrtb-validation-in-ci/)
- [Bid request anatomy](https://rtblint.org/docs/openrtb/bid-request/), [bid response anatomy](https://rtblint.org/docs/openrtb/bid-response/), and the [OpenRTB FAQ](https://rtblint.org/faq/)

## Spec data and provenance

The validator runs on structured catalogs extracted from the IAB Tech Lab OpenRTB specifications: object names, field names, type notations, enumerated value sets, and section citations. The catalogs carry no spec prose. The dialect table is derived the same way, by comparing those catalogs against the field types the IAB OpenRTB protobuf schema declares. RTBlint is not affiliated with or endorsed by IAB Tech Lab. See NOTICE for attribution.

## Supply chain

[OpenSSF Scorecard](https://scorecard.dev/viewer/?uri=github.com/aleksUIX/rtblint) runs weekly and publishes a public score. Dependabot covers Cargo, npm, and GitHub Actions. CodeQL scans Rust and JavaScript on every push and PR.

Three [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) targets (`validate`, `validate_response`, `validate_artf`) run for 30 seconds each on every CI push. The validator must not panic on arbitrary input.

```bash
cargo +nightly fuzz run validate -- -max_total_time=60
```

## License

Apache-2.0. See LICENSE and NOTICE.

## Research

Sekowski, A. (2026). *How Machine-Checkable Is OpenRTB? Classifying the Normative Content of the Protocol That Clears Real-Time Advertising*. Preprint.
DOI: [10.13140/RG.2.2.27937.57448](https://doi.org/10.13140/RG.2.2.27937.57448)

Sekowski, A. (2026). *Measuring OpenRTB Dialects in Client-Side Header Bidding*. Preprint.
DOI: [10.13140/RG.2.2.26572.78720](https://doi.org/10.13140/RG.2.2.26572.78720)
