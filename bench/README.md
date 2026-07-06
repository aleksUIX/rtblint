# rtblint benchmark

Measures end-to-end validation speed of the `rtblint` CLI: one process spawn
per validation, exactly the way the CLI is used in scripts and CI. Every
sample includes process start, file read, JSON parse, and validation.

## Quick start

```bash
# full benchmark: 50 fixtures x 2,000 runs = 100,000 validations
python3 bench/run_bench.py

# smoke run: 50 per fixture (2,500 validations, about a minute)
python3 bench/run_bench.py --quick
```

The runner needs only Python 3 (stdlib). It builds `target/release/rtblint`
automatically if missing, and generates the fixtures on first run.

## Fixtures

`generate_fixtures.py` deterministically produces 50 OpenRTB payloads into
`bench/fixtures/` (gitignored, about 21 MB total), spanning:

- 22 B to 4 MB, tiered as tiny / small / medium / large / xlarge by size
- bid requests and bid responses (CTV pods, mobile banner with consent,
  DOOH, multi-format, native)
- structural stress shapes: 1,000+ imps, 5,000 EIDs, 50,000 user segments,
  10,000 deals, 100,000-entry blocklists, deep `ext` trees, a single 4 MB
  VAST `adm` string, 100 seats x 50 bids
- five intentionally invalid payloads (unknown fields, bad enums,
  deprecated and moved paths) so issue-generation cost is measured too

`manifest.json` records each fixture's kind (request/response), byte size,
and expected validity. Before timing, the runner validates every fixture
against the CLI and aborts on any expectation mismatch, so a regression in
the validator can't silently skew the benchmark.

## Options

| Flag | Meaning |
|------|---------|
| `--iterations N` | total validations across all fixtures (default 100,000) |
| `--per-fixture N` | runs per fixture, overrides `--iterations` |
| `--quick` | shorthand for `--per-fixture 50` |
| `--jobs N` | parallel CLI processes (default 1) |
| `--filter STR` | only fixtures whose name contains STR |
| `--warmup N` | untimed warmup runs per fixture (default 3) |
| `--cli PATH` | benchmark a specific binary (compare builds) |
| `--no-save` | skip writing the JSON result file |

## Reading the numbers

- The report prints per-fixture mean / p50 / p95 / p99 / max latency and
  ops/s, plus per-tier and overall rollups.
- A process-spawn baseline (mean wall time of `rtblint --version`) is
  printed at the end. On a typical machine this is 3-6 ms; small-payload
  numbers are dominated by it. Subtract the baseline to approximate pure
  parse+validate cost, or compare tiers to see how cost scales with payload
  size and shape.
- `--jobs 1` (default) gives clean latency numbers. `--jobs N` raises
  throughput and is the right mode for "validations per second on this
  machine" questions; percentiles then include scheduler contention.
- Results are saved to `bench/results/bench-<timestamp>.json` (gitignored)
  with platform info and the CLI version, so runs can be diffed over time.

## Comparing two builds

```bash
cargo build --release -p rtblint
cp target/release/rtblint /tmp/rtblint-before
# ...make changes, rebuild...
python3 bench/run_bench.py --quick --cli /tmp/rtblint-before
python3 bench/run_bench.py --quick
```

A full 100k run takes roughly 20-40 minutes sequentially depending on the
machine (large fixtures dominate); use `--jobs 8` to cut wall time when
per-sample latency purity matters less.
