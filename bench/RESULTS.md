# Benchmark results

Committed baselines live in `bench/baselines/`; each JSON file is a full
run of `bench/run_bench.py` (see bench/README.md). Latest summary below.

## 2026-07-06 · rtblint 0.0.3 · Apple M4, macOS 26.5

100,000 validations (50 fixtures x 2,000), sequential (jobs=1), one CLI
process per validation. Process-spawn baseline 1.69 ms
(`rtblint --version`). Raw data: `bench/baselines/2026-07-06-v0.0.3-m4.json`.

| Tier | Size span | Mean | p50 | p99 (worst fixture) | Ops/s |
|------|-----------|------|-----|---------------------|-------|
| tiny | 22 B - 1 KB | 6.0 ms | 5.7 ms | 20 ms (tiny-banner-request) | 165 |
| small | 4 KB - 9 KB | 6.1 ms | 6.1 ms | 12 ms (medium-eids-50-request) | 163 |
| medium | 11 KB - 95 KB | 8.2 ms | 8.0 ms | 17 ms (large-metrics-request) | 122 |
| large | 106 KB - 940 KB | 24.9 ms | 20.5 ms | 82 ms (xlarge-deals-10000-request) | 40 |
| xlarge | 1.1 MB - 3.8 MB | 64.9 ms | 90.3 ms | 130 ms (xlarge-invalid-many-issues-request) | 15 |

Overall: 100,000 validations in 1,929 s (51.8/s sequential).

Key observations:

- Fixed per-invocation cost is about 5.7 ms: 1.7 ms process spawn plus
  roughly 4 ms of CLI startup, dominated by parsing all embedded version
  catalogs (3.7 MB of JSON) on first use. Actual parse+validate work for
  a typical 1-30 KB payload is about 1-2.5 ms on top.
- Cost tracks structure, not bytes: a 2.4 MB blocklist array validates in
  10.5 ms and a 4 MB single adm string in 7.5 ms, while 1.2 MB of 500
  fully-loaded imps takes 90.6 ms (video imps cost ~33 us each).
- Invalid payloads cost about the same as valid ones; issue generation
  stays proportional even at ~15,000 issues (107 ms).
- p99 stays within 5-15% of p50 on nearly every fixture.
