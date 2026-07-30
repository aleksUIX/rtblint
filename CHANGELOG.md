# Changelog

## Unreleased

### Fixed

- AdCOM value sets now cover AdCOM 1.0-202607 (tagged 2026-07-16), which
  added the CTV Ad Portfolio enumerations. Before this, a valid pause,
  screensaver, overlay, squeezeback or in-scene bid request was rejected with
  four `openrtb.value.invalid` errors. Extended: List: Plcmt Subtypes - Video
  with 5 through 9, List: Placement Positions with 8 through 17, List:
  Playback Methods with 8 through 11, and List: Creative Attributes with 19
  through 23. Values outside the new ranges are still rejected.

## 0.6.0 (2026-07-25)

### Added

- OpenRTB 3.0 layered validation. A 3.0 payload is validated through its
  `openrtb` envelope: `ver`, `domainspec`, `domainver`, and the whole
  transport tree (Request, Source, Item, Deal, Metric, Response, Seatbid,
  Bid, Macro) with the usual required-field, unknown-field, type, and value
  set checks. The AdCOM domain objects under `item.spec` and `bid.media`
  are accepted as opaque until an AdCOM catalog ships. New rule ids:
  `openrtb.envelope.missing`, `openrtb.envelope.ver_mismatch`,
  `openrtb.envelope.domainspec_unsupported`, and
  `openrtb.pair.unsupported_version` (cross-validation stays 2.x-only and
  now says so rather than skipping silently). The spec's "required *"
  footnote on `request`/`response` is enforced as exactly-one-of, and a 2.x
  payload sent to a 3.0 validator gets a migration diagnostic naming where
  `imp`, `bid.impid`, and `bid.adm` moved. 3.0 also gains JSON Schemas,
  rooted at the envelope and pinned to one payload member
- Published JSON Schemas: `schemas/` holds a draft 2020-12 schema per
  tracked version for both payload types, generated from the same catalogs
  the validator uses and served at
  `https://rtblint.org/schemas/openrtb-<version>-<payload>.schema.json`.
  They carry structure, types, required fields, documented value sets, and
  AdCOM enum lists; deprecations, moved paths, and the semantic rules stay
  with the linter. Regenerate with
  `cargo run -p rtblint-core --example export_json_schemas`; CI fails if the
  committed files drift from the catalogs. `adcom_list_values` is public so
  callers can resolve a field's `adcom_list` reference to concrete values
- Validated fixture coverage for every tracked version, on both payload
  types. Bid responses were covered on 6 of 18 versions and bid requests
  had a parse-only tier that asserted nothing about validation; every
  fixture now asserts a verdict, and a test fails if a new version ships
  without one. Twelve bid response fixtures and one bid request fixture
  were added, and fixtures that used fields postdating their own version
  (2.0/2.1 `device.ifa`, 202303 `imp.refresh`, 202409 `EID.inserter`)
  were corrected

### Fixed

Catalog extraction: a regeneration now reproduces the shipped catalogs
instead of quietly changing them, and several extraction gaps that made
older versions validate less than they claimed are closed.

- `Source.schain` keeps its `SupplyChain` wiring through a regeneration.
  The hint parser only understood "Details via a/an X object", so the
  spec's "Details via the `SupplyChain` object" resolved to nothing and
  every 2.6 catalog would have lost SupplyChain validation on the next
  export
- Mis-extracted prose entries (sentences, headings, notes read as objects
  or fields) are dropped in the exporter rather than by a manual pass, and
  field names are normalized to the lowercase identifiers every OpenRTB
  version uses. This removes 40-70 junk fields per catalog on 2.3-2.5
- Bid response objects are extracted for 2.0-2.2. Their tables have no
  Default column, which the parser required, so `BidResponse`, `SeatBid`,
  and `Bid` shipped with zero fields and **any** bid response validated
  clean on those three versions
- Field tables that continue past a page break are no longer truncated at
  the first cross-reference in a wrapped description ("6.9 Video Start
  Delay ..."), and a field name split across two lines is stitched back
  together. 2.0-2.2 gain 113, 128, and 137 fields respectively
- HTML tables in the 2.6 snapshots are read as a cell stream rather than
  by `<tr>` grouping. The IAB sources misnest those tags, which silently
  dropped 25-51 fields per snapshot (2.6-202409's `Site` lost 6 of 18,
  including `inventorypartnerdomain`; `User` lost `eids`)
- `ExpectedShape` understands the legacy "array of objects" phrasing and
  the 2.6 "string, array" phrasing, and reads the type column only, so
  arrays are no longer typed as scalars and scope prose mentioning
  "object" no longer types a string field as an object. `dooh.venuetype`
  is an array on every 2.6 snapshot and used to report a type mismatch

## 0.5.0 (2026-07-25)

### Added

Response-side markup validation: `bid.adm` is now checked, both standalone
and cross-referenced against the originating bid request.

- Standalone `mtype`/`adm` coherence on every bid response: native bids
  (mtype 4) must carry a JSON Native Markup Response in `adm`
  (double-encoded and non-JSON payloads are flagged), video and audio bids
  (mtype 2/3) must carry markup with a VAST or DAAST document root, banner
  bids (mtype 1) must not carry a JSON payload, and an `adm` without any
  `mtype` warns on versions that define it
- Two-pass request/response cross-validation:
  `validate_bid_response_against_request` indexes the request's Imps in
  pass one, then checks every bid in pass two: `impid` must resolve to a
  request Imp, `mtype` and sniffed `adm` markup must match a media subtype
  that Imp offers, `dealid` is checked against the Imp's pmp deals
  (warning; out-of-band deals exist), the response id must echo the
  request id, and `wseat`/`bseat`/`cur` constraints are enforced
- CLI: `--request <request.json>` cross-validates a response (single and
  `--batch` modes) against its originating request
- MCP: `validate_bid_response` accepts an optional `bid_request` argument
- npm: `validateResponseAgainstRequest(response, request, version?)`

## 0.4.0 (2026-07-05)

### Added

Semantic rule pack: cross-field checks the object catalogs alone cannot
express.

- SupplyChain node hygiene: duplicate adjacent nodes, missing `hp`, empty
  `asi`/`sid` identifiers (`source.schain` is now wired into the recursive
  validator; it was previously opaque to field-level checks)
- GPP/GPP-SID and US Privacy string coherence on `Regs`
- CTV pod duration sanity on `Video`: empty `rqddurs`, `mincpmpersec` used
  outside a dynamic pod context
- Native request encoding: double-encoded JSON, unparseable content, and the
  deprecated pre-1.1 `{"native": {...}}` wrapper, all on `imp.native.request`
- Plausibility checks: non-positive or implausible `tmax`, malformed
  currency codes on `cur`/`bidfloorcur`, negative `bidfloor` (`Imp` and
  `Deal`)

### Known limits

- Native markup encoding checks cover `imp.native.request` only; a
  `bid.adm` cross-check against its Imp's media type would need a two-pass
  validator architecture and is not implemented

## 0.3.0 (2026-07-05)

### Changed

- Cut validator cost on structure-heavy payloads 4-6x

## 0.2.0 (2026-07-05)

### Added

- CLI `--batch` mode: one JSON payload per stdin line, one result per stdout
  line, spec catalogs loaded once per process
- Spec catalogs compile to static Rust data instead of parsing embedded JSON
  at first use

### Changed

- npm package published as `rtblint-core` (npm's similarity filter blocks the
  bare `rtblint` name); API unchanged
- First versions actually on registries: crates.io rtblint / rtblint-core /
  rtblint-mcp 0.2.0, npm rtblint-core 0.2.0

## 0.1.0 (2026-07-05)

First real release. Everything before this was a name reservation.

### Added

- OpenRTB bid request validation for 2.0 through 2.6-202606 (every monthly
  2.6 snapshot tracked as its own version target)
- OpenRTB bid response validation across the same versions
- Checks: malformed JSON, required fields (including required non-empty
  arrays), unknown fields, type mismatches, documented enum values (AdCOM
  lists plus inline value sets with vendor ranges), deprecated / moved /
  removed / not-yet-available fields, and semantic rules (site/app/dooh
  exclusivity, imp media type presence, skippable-video dependencies,
  duration exclusivity, seatbid/nbr presence on responses)
- Every finding carries a stable rule id, typed severity, JSON path, and the
  OpenRTB spec section it derives from
- CLI: `rtblint validate [--type request|response] [--version <id>]
  [--format human|json]`, exit codes 0/1/2, `RTBlint --version`
- MCP server (`rtblint-mcp`) over stdio with `validate_bid_request`,
  `validate_bid_response`, and `list_openrtb_versions` tools
- npm package backed by the Rust core compiled to WASM (CJS + ESM), with
  `validate`, `validateResponse`, `versions`, `rules`, `coreVersion`
- Apache-2.0 LICENSE and NOTICE with IAB Tech Lab attribution

### Changed

- Spec catalogs carry structured validation metadata only; no spec prose
- Versions without a usable catalog (2.6-202204, 3.0) report
  `openrtb.version.unsupported` instead of silently passing payloads
- `Issue.severity` is a typed enum serializing as "error" / "warning";
  `Issue`, `ValidationResult`, and `Severity` are non_exhaustive

### Known limits

- OpenRTB 3.0 payload validation is not implemented (catalog ships for
  introspection only)
- Python and Go packages are explicit not-implemented stubs

## 0.0.1 / 0.0.2

Name-reservation stubs across registries; no functional validator.
