# Changelog

## 0.9.0 (2026-08-16)

### Added

- AdCOM 1.0 object catalog for the OpenRTB 3.0 domain layer. `item.spec`,
  `bid.media`, and `request.context` are no longer opaque: they validate as
  AdCOM Placement, Ad, and Context (Appendix C wrappers), including nested
  media, placement, and context objects. Site, App, and Dooh inherit
  DistributionChannel. New rule ids: `adcom.placement.subtype_required`,
  `adcom.ad.subtype_required`, `adcom.asset.subtype_required`,
  `adcom.assetformat.subtype_required`. A non-AdCOM `domainspec` still
  leaves the domain objects unchecked.
- Opt-in `--resolve` on the CLI, implemented in the `rtblint-resolve` crate
  so `rtblint-core` stays a pure function of the payload. `--cache <dir>`
  holds `sellers/<asi>/sellers.json`, `ads/<domain>/ads.txt`, and
  `app-ads/<bundle>/app-ads.txt`. Payment hops (`hp` not 0) are checked
  against sellers.json; the first payment hop must appear as DIRECT or
  RESELLER in the publisher's ads.txt or app-ads.txt. New rule ids:
  `openrtb.resolve.sellers_json_unavailable`,
  `openrtb.resolve.sellers_json_unparseable`,
  `openrtb.resolve.sid_not_in_sellers`,
  `openrtb.resolve.ads_txt_unavailable`,
  `openrtb.resolve.ads_txt_unauthorized`,
  `openrtb.resolve.app_ads_txt_unavailable`,
  `openrtb.resolve.app_ads_txt_unauthorized`. Nothing is fetched from the
  network.
- `Issue::new` and `Default` for `ValidationResult`, so crates outside
  `rtblint-core` can construct findings. Both types stay non-exhaustive.

- OpenSSF Scorecard weekly workflow, CodeQL on Rust and JavaScript, Dependabot
  for Cargo / npm / GitHub Actions, and three cargo-fuzz targets
  (`validate`, `validate_response`, `validate_artf`). CI Actions are pinned by
  hash.
- `SECURITY.md` now has a private disclosure path, acknowledgement SLA, and
  coordinated-disclosure timeline. Dockerfile base images are pinned by digest.

## 0.8.0 (2026-08-14)

### Added

- JSON dialects. The OpenRTB specification types a family of flag fields as
  integers with the value set {0, 1}; the IAB OpenRTB protobuf schema
  (`com.iabtechlab.openrtb.v2`, what every gRPC bidstream integration compiles
  against) declares 28 of those same fields `bool`. The encodings are
  incompatible in both directions: protojson writes `"secure": true`, which a
  spec-JSON reader rejects, and spec JSON writes `"secure": 1`, which a
  protojson parser refuses to unmarshal. Neither side is wrong for its own
  transport and the payload cannot settle it, so the caller declares the
  dialect and the validator reports against that:
  `openrtb.dialect.bool_for_integer` when protobuf flags arrive in spec JSON,
  `openrtb.dialect.integer_for_bool` when spec flags arrive in protobuf JSON,
  and `openrtb.dialect.camel_case_name` for the lowerCamelCase spellings
  protojson emits unless the serializer sets `UseProtoNames`. Exposed as
  `validate_bid_request_with_dialect` and siblings in the library, `--dialect`
  on the CLI, a `dialect` argument on the MCP validation tools, and
  `validateDialect` / `validateResponseDialect` on npm. The divergence set
  itself is published: `proto_bool_fields()` in Rust,
  `protoBoolDivergences()` on npm.
- ARTF (IAB Tech Lab Agentic Real Time Framework) validation, in three passes.
  `validate_artf_request` checks the `RTBRequest` envelope (required members,
  `lifecycle` against the payloads actually carried, `tmax` plausibility for an
  in-auction call, `originator` and `applicable_intents` enums) and validates
  the OpenRTB payloads it carries as protobuf JSON.
  `validate_artf_response_against_request` checks each mutation against the
  auction it targets: the response id echoes the extension point request id
  rather than the bid request id, the declared intent is in
  `applicable_intents`, operation and payload oneof member match the intent,
  and every semantic path (`/imp/{id}`, `/imp/{id}/pmp/deals/{id}`,
  `/user/data/segment`, `/seatbid/{seat}/bid/{id}`, both the document's and the
  example docs' deal spellings) resolves to something that exists.
  `validate_artf_mutations_applied` writes the mutations in and revalidates,
  reporting the OpenRTB findings the mutations introduced with pre-existing
  findings filtered out, which is the question an orchestrator actually has.
  `ADJUST_DEAL_MARGIN` reports `artf.mutation.no_openrtb_target`: margin is not
  part of the OpenRTB Deal object, so no validator can check the result of
  applying it. Mutations written in the vocabulary of the ARTF v1.0 document's
  examples rather than its own `.proto` (`activateSegments`, `op: "add"`, a
  `value: {IDsPayload: ...}` wrapper) are mapped and reported as
  `artf.mutation.legacy_spec_encoding`. Surfaced on the CLI as `--type
  artf-request` / `--type artf-response` with `--apply`, as the
  `validate_artf_request` and `validate_artf_response` MCP tools, and as
  `validateArtfRequest` / `validateArtfResponse` /
  `validateArtfResponseApplied` on npm.

- gRPC: `ValidateArtfEnvelope` and `ValidateArtfMutations` on
  `openadtech.rtblint.v1`, plus `ValidationContext.dialect`. ARTF mandates gRPC
  for the extension point itself, so an orchestrator checking what an agent
  proposed does it in band rather than shelling out. The mutation RPC is split
  from the envelope RPC on the same reasoning that split `ValidatePair` from
  `Validate`: well formed and coherent with the auction are different
  questions. With `apply` set it returns the rewritten payloads and which
  mutation indexes went in. `dialect` is refused on the ARTF RPCs even when it
  names the correct dialect, because the framework decides that, not the
  caller. Additive on the wire: new RPCs, new messages, one new field, one new
  enum, all past `buf breaking`.

### Fixed

- Fields the spec tables type `int` rather than `integer`, plus the source
  typos `inpteger` and `srting`, were mapped to no shape at all, which
  silently disabled type checking on them: `Content.livestream`,
  `Content.realtime`, `Content.gtax`, `EID.mm`, `Video.minbitrate`,
  `Site.page`, and the garbled 2.0-2.2 entries for `BidRequest.at` and
  `Imp.secure`. They are now typed, so a boolean or a string where an integer
  belongs is reported, and the published JSON Schemas carry their types and
  value sets instead of an empty object. Regenerated `schemas/` accordingly.
- `openrtb.bid.dealid_unknown` read "dealid X references a deal, but imp Y is
  not among the deals its pmp object enumerates", swapping subject and object.
  It is the deal id that is absent, not the impression.

## 0.7.0 (2026-08-09)

### Added

- `rtblint-grpc`, a gRPC server for `openadtech.rtblint.v1`, the sibling of
  vastlint-grpc. Three contract decisions inverted under OpenRTB's facts and
  are argued in the proto where they occur: payload kind is supplied rather
  than sniffed, because a bid request and a bid response are both JSON
  objects with an `id` and any heuristic fails hardest on the malformed
  payloads the service exists to diagnose; the spec version is a string
  rather than an enum, the opposite of the VAST call, because 2.6 alone has
  had ten dated revisions since 2022 and an enum would mean a wire release
  per IAB erratum; severity has two levels rather than three, because
  rtblint emits no advisory level and a field the server never sends is one
  consumers can branch on and never reach.
- `get_adcp_capabilities` on the MCP server, for AdCP protocol discovery.
  Declares the AdCP releases the agent speaks (3.1 at release precision)
  and the bid-stream conformance metrics it computes:
  `openrtb_error_count`, `openrtb_warning_count`, and
  `openrtb_conformance_rate`. A pin outside `supported_versions` returns a
  typed `VERSION_UNSUPPORTED` error naming the releases that would work,
  rather than silently serving a version the caller did not ask for.
  `adcp_major_version` is accepted and deprecated in favour of
  `adcp_version`.
- Seven SupplyChain hygiene rules, all offline and all warnings. On the
  chain: `openrtb.schain.ver_unexpected` (`ver` other than the only
  published version, 1.0), `openrtb.schain.incomplete` (`complete` is 0, so
  the declared path knowingly omits an upstream node), and
  `openrtb.schain.length_implausible` (more than ten nodes, usually a
  forwarder splicing two chains together). On each node:
  `openrtb.schain.node.hp_unexpected` (`hp` other than 1, which SupplyChain
  1.0 expects on every node), `openrtb.schain.node.asi_not_domain` (`asi`
  carrying a scheme, path, port, or whitespace instead of a bare domain),
  and `openrtb.schain.node.asi_not_lowercase`. On Source:
  `openrtb.schain.duplicate_location`, for a chain declared at both
  `source.schain` and `source.ext.schain`, where the two copies can
  disagree and receivers differ on which they read.

  `asi` is a lookup key, not a label: verifying a hop means fetching that
  domain's sellers.json and matching the sid inside it. The two `asi` rules
  exist because a value that is not a bare lowercase domain fails that
  lookup, so the node cannot be authorised even when the chain is honest.

  An empty `nodes` array is deliberately not a new rule. The required-field
  check already errors on it, and a second finding for one defect is noise.

### Changed

- A field the selected version's catalog does not define now reports why,
  when the version rules know. Previously every such field reported
  `openrtb.field.undefined`, which is the same answer for a typo, for a
  field deleted by a later revision, and for a field that has not shipped
  yet. Those want different fixes.

  `openrtb.field.not_yet_available` now fires for a field that arrives in a
  later version, and names it: `regs.gpp` against 2.6-202210 reports "not
  available in OpenRTB 2.6-202210. It arrives in 2.6-202211." That is the
  code to look for when a partner calls a field unknown, because it usually
  means the two of you pinned different 2.6 snapshots.
  `openrtb.field.removed` now fires for a field deleted by a revision, such
  as `banner.wmax` against any 2.6.

  Both ids were documented and neither could fire. The catalog lookup
  pushed `openrtb.field.undefined` and returned before the path-state check
  ran, and both states describe fields absent from that catalog, so their
  arms were unreachable. A second mismatch sat underneath: the walker builds
  a path from the document root (`imp.banner.wmax`) while the version rules
  name a field relative to its owning object (`banner.wmax`), which is how
  the spec's change appendices are written, so full-path lookups only landed
  when the object happened to sit at the root. Lookups now try the full path
  and then the trailing `object.field` pair, both exact comparisons.

  **Migration.** Severity stays `Error` and `valid` is unchanged, so exit
  codes do not move. If you gate CI on rule ids, payloads that reported
  `openrtb.field.undefined` for a version-shifted field now report
  `openrtb.field.not_yet_available` or `openrtb.field.removed`. Fields no
  version rule knows, ordinary typos included, still report
  `openrtb.field.undefined`.

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
