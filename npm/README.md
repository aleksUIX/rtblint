# rtblint

**OpenRTB linter for Node.** Validates OpenRTB 2.x bid requests and bid responses against versioned IAB spec snapshots, from 2.0 through the monthly 2.6 releases. Backed by the rtblint Rust core compiled to WASM; no native dependencies.

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
- `versions()` lists every tracked OpenRTB version id
- `rules()` returns the versioned rule catalog (codes, paths, summaries, spec sections)
- `coreVersion()` reports the rtblint-core build version

Findings carry a stable rule id, a severity (`error` or `warning`), a message, and a JSON path into the payload.

## Other surfaces

Rust CLI and library on [crates.io](https://crates.io/crates/rtblint), MCP server as [rtblint-mcp](https://crates.io/crates/rtblint-mcp).

## License

Apache-2.0. See LICENSE and NOTICE.
