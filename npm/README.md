# RTBlint

**OpenRTB linter for Node.** Validates OpenRTB 2.x bid requests and bid responses against versioned IAB spec snapshots, from 2.0 through the monthly 2.6 releases. Backed by the RTBlint Rust core compiled to WASM; no native dependencies.

Website and playground: [rtblint.org](https://rtblint.org)

## Install

```bash
npm install rtblint-core
```

## Usage

```js
import { validate, validateResponse, versions, rules } from "rtblint-core";
// or: const { validate } = require("rtblint-core");

const report = validate(JSON.stringify(bidRequest));            // latest tracked 2.6
const legacy = validate(JSON.stringify(bidRequest), "2.5");     // version-aware
const response = validateResponse(JSON.stringify(bidResponse));

if (!report.valid) {
  for (const issue of report.issues) {
    console.log(`[${issue.severity}] ${issue.path}: ${issue.message} (${issue.id})`);
  }
}

console.log(versions()); // every tracked OpenRTB version id
```

## API

- `validate(input, version?)` validates a bid request JSON string
- `validateResponse(input, version?)` validates a bid response JSON string
- `validateProfile(input, profile, version?, dialect?)` selects a documented vendor contract
- `validateResponseProfile(input, profile, version?, dialect?)` selects that contract for a response
- `validateResponseAgainstRequestProfile(response, request, profile, version?, dialect?)` checks a response against its originating request and the selected contract
- `versions()` lists every tracked OpenRTB version id
- `rules()` returns the versioned rule catalog (codes, paths, summaries, spec sections)
- `coreVersion()` reports the rtblint-core build version

Findings carry a stable rule id, a severity (`error` or `warning`), a message, and a JSON path into the payload.

Profile functions accept `spec-json` (the default) or `proto-json` as the dialect.

```js
import { validateResponseAgainstRequestProfile } from "rtblint-core";

const paired = validateResponseAgainstRequestProfile(
  JSON.stringify(bidResponse),
  JSON.stringify(bidRequest),
  "google-ab",
  "2.6-202606",
);
```

## Exchange profiles

Choose the vendor and traffic direction for the captured JSON. Optional extension fields remain optional unless the primary protocol requires them in that context. Microsoft Monetize uses `xandr`; Magnite DV+ xAPI uses `magnite`. Buyer, supplier and SDK contracts have separate profile IDs.

`spec`, `google-ab`, `prebid-server`, `xandr`, `magnite`, `dv360`, `index-exchange`, `index-exchange-seller`, `unity`, `vungle`, `bidswitch`, `bidswitch-supplier`, `inmobi`, `inmobi-supplier`, `mobilefuse`, `mobilefuse-sdk`, `applovin-alx`, `commerce-grid`, `digital-turbine`, `sovrn`, `equativ`, `equativ-supplier`, `triplelift-supplier`, `adform-handler`, `yandex-sdk-bidding`, `pubmatic-openwrap`, `pubmatic-openwrap-ctv`.

`applovin-alx` covers responses. `index-exchange-seller`, `commerce-grid` and `sovrn` cover requests. Other profiles cover their documented request and response directions. An unsupported direction or OpenRTB 3.0 receives canonical specification checks and a scope warning. OpenWrap profiles cover the pinned public producer implementation, not the private PubMatic exchange acceptance contract. See [coverage and source limits](https://github.com/aleksUIX/rtblint/tree/main/docs/exchange-profiles).

## Other surfaces

Rust CLI and library on [crates.io](https://crates.io/crates/rtblint), MCP server as [rtblint-mcp](https://crates.io/crates/rtblint-mcp).

## License

Apache-2.0. See LICENSE and NOTICE.
