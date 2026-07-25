# Roadmap

What RTBlint does today and where it's heading. Not a promise of dates.

## Shipped

- OpenRTB 2.x bid request and bid response validation, versions 2.0 through
  every monthly 2.6 snapshot (currently 2.6-202606)
- Stable rule ids, typed severities, JSON paths, and spec section citations
  on every finding
- Semantic rule pack for the failure modes catalogs alone can't catch:
  SupplyChain node hygiene, GPP/US Privacy string coherence, CTV pod
  duration sanity, native request encoding, tmax/currency/bidfloor
  plausibility
- CLI batch mode; spec catalogs compiled to static Rust data
- Rust library and CLI, MCP server over stdio, WASM-backed npm package
- Web playground at [rtblint.org](https://rtblint.org)
- Response-side markup validation: `bid.mtype`/`bid.adm` coherence on any
  bid response, plus two-pass request/response cross-validation (impid
  resolution, markup vs the referenced Imp's media subtypes, dealid, seat
  and currency constraints) via the library, CLI `--request`, MCP, and npm

- Validated fixture coverage on both payload types for every tracked
  version, with a test that fails when a new snapshot ships without one

## Next

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
