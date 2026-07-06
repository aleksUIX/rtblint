# Changelog

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
  [--format human|json]`, exit codes 0/1/2, `rtblint --version`
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
