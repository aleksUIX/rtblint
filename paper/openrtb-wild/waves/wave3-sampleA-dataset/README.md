# OpenRTB in-the-wild conformance dataset

Structural findings from live OpenRTB traffic captured at a residential
browser vantage point. Derived from a private raw corpus that is **not**
released: bid payloads contain user identifiers, extended IDs, geolocation,
and commercial terms (floor prices, deal IDs). This dataset contains only
rule identifiers and JSON paths, never payload values, and reproduces every
table and figure in the paper.

- Validator: rtblint 0.6.0 (pinned; catalogs ship with the crate)
- Payloads: 4,281 across 79 sites and 129 endpoint/side pairs
- Findings: 6,665
- Redacted paths (defence-in-depth filter): 0

## Files

`payloads.csv` one row per captured payload
: `payload_id`, `site`, `endpoint`, `side` (request = built by the Prebid
  adapter and publisher config; response = built by the SSP server),
  `best_fit_version`, `valid`, `n_issues`, `n_errors`, `n_warnings`.
  Payloads cluster by site and by auction: use `site` for clustering-aware
  statistics rather than treating rows as independent.

`issues.csv` one row per validation finding
: `payload_id` (joins to payloads.csv), `site`, `endpoint`, `side`,
  `best_fit_version`, `rule` (stable RTBlint rule id), `severity`, `path`
  (structural JSON locator), `section` (OpenRTB spec section).

`endpoints.csv` per endpoint and side aggregate
: `payloads`, `invalid`, `invalid_pct`, `issues`.

## Method notes

Best-fit version: each (endpoint, side) is validated against all 16
cataloged OpenRTB versions and assigned the one minimizing its error count,
ties to newest. This is deliberately charitable to implementers. The
selection is only as sound as the weakest catalog in the candidate set, so
the validator version is pinned above and must be reported with any reuse.

Capture and analysis code accompanies this dataset.
