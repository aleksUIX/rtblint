# Roadmap

What rtblint does today and where it's heading. Not a promise of dates.

## Shipped (0.1.0)

- OpenRTB 2.x bid request and bid response validation, versions 2.0 through
  every monthly 2.6 snapshot (currently 2.6-202606)
- Stable rule ids, typed severities, JSON paths, and spec section citations
  on every finding
- Rust library and CLI, MCP server over stdio, WASM-backed npm package
- Web playground at [rtblint.org](https://rtblint.org)

## Next (0.2.x)

- Semantic rule pack for the failure modes JSON Schemas can't catch:
  - SupplyChain (schain) integrity: node completeness, asi/sid sanity,
    complete-flag consistency
  - Privacy signal coherence: GPP presence vs legacy regs.gdpr /
    us_privacy, contradictory signals
  - CTV pod coherence: podid / slotinpod / poddur / rqddurs interplay
  - Native adm encoding: double-encoded JSON, wrapper presence
  - Plausibility checks: tmax realism, price/currency sanity
- Validated fixture coverage for every tracked version, not just 2.6
- Published JSON Schemas per version, generated from the catalogs

## Later

- OpenRTB 3.0 / AdCOM layered payload validation and 2.x-to-3.0 migration
  diagnostics
- NDJSON stream mode: lint captured bid streams, aggregate rule frequencies
- Exchange dialect profiles (validate against a specific platform's
  documented requirements on top of the spec)
- GitHub Action and pre-commit hook
- Homebrew tap, Docker image, prebuilt static binaries
- Python and Go bindings over the Rust core (the current packages are
  explicit stubs)

## Non-goals

- Enforcing exchange-specific business policy beyond documented protocol
  requirements
- Anything requiring redistribution of IAB spec prose; catalogs stay
  structured-metadata only
