# Benchmark results

Committed baselines live in `bench/baselines/`; each JSON file is a full
run of `bench/run_bench.py` (see bench/README.md). All runs below: Apple M4,
macOS 26.5, 100,000 validations (50 fixtures x 2,000), sequential (jobs=1).
Newest first.

## 2026-07-06 · RTBlint 0.3.0

Validator walk optimizations: field shape/required/deprecated flags
precomputed at build time, version-rule matching gated on a static set of
rule path leaves, value sets binary-searched in place, and instance paths
built with a push/truncate cursor so path strings are only materialized
when an issue is pushed. Validation output is byte-identical to 0.2.0
across the fixture set.

Per-process (spawn baseline 2.27 ms), raw data
`2026-07-06-v0.3.0-m4-perprocess.json`:

| Tier | Mean | Ops/s | vs 0.2.0 mean | vs 0.0.3 mean |
|------|------|-------|---------------|---------------|
| tiny | 2.33 ms | 429 | 2.43 ms | 6.05 ms |
| small | 2.41 ms | 415 | 2.72 ms | 6.13 ms |
| medium | 2.78 ms | 360 | 4.89 ms | 8.20 ms |
| large | 5.98 ms | 167 | 21.88 ms | 24.85 ms |
| xlarge | 14.50 ms | 69 | 63.23 ms | 64.88 ms |

Overall: 100,000 validations in 502 s (199/s, was 1,619 s / 62/s on 0.2.0).

Batch mode (`--batch`, one process per fixture), raw data
`2026-07-06-v0.3.0-m4-batch.json`:

| Tier | Per item | Ops/s (1 core) | vs 0.2.0 per item |
|------|----------|----------------|-------------------|
| tiny | 0.009 ms | 111,501 | 0.042 ms |
| small | 0.070 ms | 14,298 | 0.332 ms |
| medium | 0.385 ms | 2,600 | 2.492 ms |
| large | 3.22 ms | 310 | 19.43 ms |
| xlarge | 12.21 ms | 82 | 61.88 ms |

Overall batch: 100,000 validations in 258 s (388/s single core).

Key observations:

- Structure-heavy giants dropped 4-8x: 500 fully-loaded imps 92 → 15 ms,
  10,000 deals 68 → 9 ms, 50,000 user segments 107 → 24 ms,
  100 seats x 50 bids 94 → 19 ms, 15,000-issue invalid payload
  107 → 24 ms (all per-process means vs 0.2.0).
- Byte-heavy but structure-light payloads were already parse-bound and
  stay put: 4 MB adm 3.3 ms, 100,000-entry blocklist 6.9 ms.
- The floor is now serde_json DOM construction plus process spawn. In
  batch mode a typical small request validates in ~10 us and medium
  payloads in under 0.4 ms.
- Profile that motivated the work (50k-segment fixture on 0.2.0): under
  10% JSON parsing; ~30% rule-matching allocations, ~30% malloc/free
  churn from per-field path strings, ~13% re-parsing type_spec strings.
  All three are gone from the 0.3.0 profile.

## 2026-07-06 · RTBlint 0.2.0

Static catalog codegen (no runtime JSON parsing of spec catalogs) plus new
CLI `--batch` mode. Per-process (spawn baseline 2.25 ms), raw data
`2026-07-06-v0.2.0-m4-perprocess.json`:

| Tier | Mean | Ops/s | vs 0.0.3 mean |
|------|------|-------|---------------|
| tiny | 2.43 ms | 412 | 6.05 ms |
| small | 2.72 ms | 368 | 6.13 ms |
| medium | 4.89 ms | 204 | 8.20 ms |
| large | 21.88 ms | 46 | 24.85 ms |
| xlarge | 63.23 ms | 16 | 64.88 ms |

Overall: 100,000 validations in 1,619 s (62/s, was 1,929 s / 52/s).

Batch mode, raw data `2026-07-06-v0.2.0-m4-batch.json`: tiny 0.042 ms
(23,876/s), small 0.332 ms, medium 2.49 ms, large 19.4 ms, xlarge 61.9 ms;
100,000 validations in 1,393 s. Small-payload per-process cost became
essentially pure process spawn; large payloads remained walk-bound (fixed
in 0.3.0).

## 2026-07-06 · RTBlint 0.0.3

One CLI process per validation, process-spawn baseline 1.69 ms. Raw data:
`bench/baselines/2026-07-06-v0.0.3-m4.json`.

| Tier | Size span | Mean | p50 | p99 (worst fixture) | Ops/s |
|------|-----------|------|-----|---------------------|-------|
| tiny | 22 B - 1 KB | 6.0 ms | 5.7 ms | 20 ms (tiny-banner-request) | 165 |
| small | 4 KB - 9 KB | 6.1 ms | 6.1 ms | 12 ms (medium-eids-50-request) | 163 |
| medium | 11 KB - 95 KB | 8.2 ms | 8.0 ms | 17 ms (large-metrics-request) | 122 |
| large | 106 KB - 940 KB | 24.9 ms | 20.5 ms | 82 ms (xlarge-deals-10000-request) | 40 |
| xlarge | 1.1 MB - 3.8 MB | 64.9 ms | 90.3 ms | 130 ms (xlarge-invalid-many-issues-request) | 15 |

Overall: 100,000 validations in 1,929 s (51.8/s sequential).

Key observations:

- Fixed per-invocation cost was about 5.7 ms: 1.7 ms process spawn plus
  roughly 4 ms of CLI startup, dominated by parsing all embedded version
  catalogs (3.7 MB of JSON) on first use. Removed in 0.2.0 by compiling
  catalogs to static data.
- Cost tracks structure, not bytes: a 2.4 MB blocklist array validated in
  10.5 ms and a 4 MB single adm string in 7.5 ms, while 1.2 MB of 500
  fully-loaded imps took 90.6 ms.
- Invalid payloads cost about the same as valid ones; issue generation
  stays proportional even at ~15,000 issues.
- p99 stays within 5-15% of p50 on nearly every fixture.
