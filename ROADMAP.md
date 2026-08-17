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
- JSON Schemas (draft 2020-12) per version and payload type, generated from
  the catalogs and published at
  [rtblint.org/schemas](https://rtblint.org/docs/json-schemas/)
- OpenRTB 3.0 layered validation: the envelope and every transport object,
  exactly-one-of request/response, and a migration diagnostic when a 2.x
  payload is sent to a 3.0 validator
- JSON dialects: spec JSON and the protobuf JSON mapping of the IAB OpenRTB
  protobuf schema, which declares 28 of the spec's integer flag fields as
  bool. Both encodings are validated against the dialect the caller declares,
  in the library, the CLI (`--dialect`), the MCP tools, and npm
- ARTF (Agentic Real Time Framework) support: the `RTBRequest` envelope, the
  `RTBResponse` mutation set cross-validated against the auction it targets,
  and an apply-then-revalidate pass that reports only the OpenRTB findings
  the mutations introduced
- AdCOM 1.0 object catalog: OpenRTB 3.0 `item.spec`, `bid.media`, and
  `request.context` are validated as Placement, Ad, and Context rather than
  accepted as opaque. Site/App/Dooh inherit DistributionChannel. Subtype
  rules fire when a Placement or Ad has none of display/video/audio.
- Opt-in `--resolve`: each SupplyChain payment hop is checked against that
  domain's sellers.json, and `app.bundle` or `site.domain` against the
  publisher's ads.txt or app-ads.txt, from a locally cached directory
  (`--cache`). The offline core stays a pure function of the payload.
- NDJSON stream mode: `--batch` lints one payload per line from a file or
  stdin; `--summary` prints how often each rule id fired across the capture
- Exchange profiles: documented protocol requirements on top of the spec.
  `--profile google-ab` (Google Authorized Buyers) accepts `at: 3`
  (FIXED_PRICE) and requires `Imp.ext.billing_id`. Orthogonal to `--dialect`.
  Business policy stays out.

## Later

- ARTF beyond v1.0: the intent set is growing in the reference repository, and
  each new intent brings a payload shape and a target vocabulary to check
- GitHub Action and pre-commit hook
- Homebrew tap, Docker image, prebuilt static binaries
- Python and Go bindings over the Rust core (the current packages are
  explicit stubs)

## Non-goals

- Enforcing exchange-specific business policy beyond documented protocol
  requirements
- Anything requiring redistribution of IAB spec prose; catalogs stay
  structured-metadata only
