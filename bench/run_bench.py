#!/usr/bin/env python3
"""RTBlint CLI validation speed benchmark.

Spawns the RTBlint CLI once per validation (so numbers reflect real CLI
usage: process start + file read + parse + validate) and reports latency
percentiles and throughput per fixture, per size tier, and overall.

Typical runs:
    python3 bench/run_bench.py                         # full run: 100,000 validations
    python3 bench/run_bench.py --quick                 # smoke run: 50 per fixture
    python3 bench/run_bench.py --iterations 10000      # custom total
    python3 bench/run_bench.py --jobs 8                # parallel (throughput mode)
    python3 bench/run_bench.py --batch                 # one process per fixture (--batch CLI mode)
    python3 bench/run_bench.py --filter xlarge         # only matching fixtures

Fixtures are generated on first run (deterministic; see generate_fixtures.py).
Results are printed as a table and saved as JSON under bench/results/.
"""

import argparse
import json
import platform
import statistics
import subprocess
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime, timezone
from pathlib import Path

BENCH_DIR = Path(__file__).parent
REPO_ROOT = BENCH_DIR.parent
DEFAULT_CLI = REPO_ROOT / "target" / "release" / "rtblint"


def human_bytes(n: int) -> str:
    if n >= 1024 * 1024:
        return f"{n / 1024 / 1024:.1f} MB"
    if n >= 1024:
        return f"{n / 1024:.1f} KB"
    return f"{n} B"


def tier_of(size: int) -> str:
    if size < 2_000:
        return "tiny"
    if size < 10_000:
        return "small"
    if size < 100_000:
        return "medium"
    if size < 1_000_000:
        return "large"
    return "xlarge"


def percentile(sorted_samples: list, p: float) -> float:
    if not sorted_samples:
        return 0.0
    k = (len(sorted_samples) - 1) * p
    lo = int(k)
    hi = min(lo + 1, len(sorted_samples) - 1)
    return sorted_samples[lo] + (sorted_samples[hi] - sorted_samples[lo]) * (k - lo)


def ensure_cli(cli_arg: str | None) -> Path:
    if cli_arg:
        cli = Path(cli_arg)
        if not cli.exists():
            sys.exit(f"error: --cli path not found: {cli}")
        return cli
    if not DEFAULT_CLI.exists():
        print("release binary not found, building (cargo build --release -p rtblint)...")
        result = subprocess.run(
            ["cargo", "build", "--release", "-p", "rtblint"], cwd=REPO_ROOT
        )
        if result.returncode != 0:
            sys.exit("error: cargo build failed")
    return DEFAULT_CLI


def ensure_fixtures(fixtures_dir: Path) -> list:
    manifest_path = fixtures_dir / "manifest.json"
    if not manifest_path.exists():
        print("fixtures not found, generating...")
        subprocess.run(
            [sys.executable, str(BENCH_DIR / "generate_fixtures.py"), "--out", str(fixtures_dir)],
            check=True,
        )
    return json.loads(manifest_path.read_text())


def cli_command(cli: Path, fixture: dict, fixtures_dir: Path) -> list:
    return [
        str(cli),
        "validate",
        "--type",
        fixture["kind"],
        "--format",
        "json",
        str(fixtures_dir / fixture["file"]),
    ]


def verify_fixture(cli: Path, fixture: dict, fixtures_dir: Path) -> None:
    proc = subprocess.run(
        cli_command(cli, fixture, fixtures_dir),
        stdout=subprocess.DEVNULL,
        stderr=subprocess.PIPE,
    )
    if proc.returncode >= 2:
        sys.exit(
            f"error: CLI failed on fixture {fixture['name']} "
            f"(rc={proc.returncode}): {proc.stderr.decode()[:200]}"
        )
    actually_valid = proc.returncode == 0
    if actually_valid != fixture["expected_valid"]:
        sys.exit(
            f"error: fixture {fixture['name']} expected "
            f"{'valid' if fixture['expected_valid'] else 'invalid'} but CLI says "
            f"{'valid' if actually_valid else 'invalid'}; "
            "regenerate fixtures or fix the fixture generator before benchmarking"
        )


def measure_baseline(cli: Path, runs: int = 100) -> float:
    """Mean wall time of the cheapest possible CLI invocation (--version):
    approximates pure process spawn + startup overhead."""
    samples = []
    for _ in range(runs):
        start = time.perf_counter()
        subprocess.run([str(cli), "--version"], stdout=subprocess.DEVNULL)
        samples.append(time.perf_counter() - start)
    return statistics.mean(samples)


def bench_fixture_batch(cli: Path, fixture: dict, fixtures_dir: Path, iterations: int) -> dict:
    """One CLI process, `iterations` payloads on stdin. Measures amortized
    per-validation cost with spawn and startup paid once."""
    payload = (fixtures_dir / fixture["file"]).read_text().strip() + "\n"
    cmd = [str(cli), "validate", "--batch", "--type", fixture["kind"], "--format", "json"]

    wall_start = time.perf_counter()
    proc = subprocess.Popen(
        cmd, stdin=subprocess.PIPE, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, text=True
    )
    for _ in range(iterations):
        proc.stdin.write(payload)
    proc.stdin.close()
    proc.wait()
    wall = time.perf_counter() - wall_start

    per_ms = wall / iterations * 1000
    return {
        "name": fixture["name"],
        "kind": fixture["kind"],
        "tier": tier_of(fixture["bytes"]),
        "bytes": fixture["bytes"],
        "expected_valid": fixture["expected_valid"],
        "mode": "batch",
        "iterations": iterations,
        "wall_seconds": wall,
        "mean_ms": per_ms,
        "ops_per_sec": iterations / wall,
    }


def print_batch_table(rows: list) -> None:
    header = f"{'fixture':<38} {'kind':<8} {'size':>9} {'n':>6} {'per-item':>10} {'ops/s':>10}"
    print()
    print(header)
    print("-" * len(header))
    for row in rows:
        print(
            f"{row['name']:<38} {row['kind']:<8} {human_bytes(row['bytes']):>9} "
            f"{row['iterations']:>6} {row['mean_ms']:>8.3f}ms {row['ops_per_sec']:>10,.0f}"
        )
    print("-" * len(header))
    tiers = {}
    for row in rows:
        tiers.setdefault(row["tier"], []).append(row)
    for tier in ("tiny", "small", "medium", "large", "xlarge"):
        if tier not in tiers:
            continue
        tier_rows = tiers[tier]
        total_n = sum(r["iterations"] for r in tier_rows)
        total_wall = sum(r["wall_seconds"] for r in tier_rows)
        print(f"{'tier ' + tier:<38} {'':<8} {'':>9} {total_n:>6} "
              f"{total_wall / total_n * 1000:>8.3f}ms {total_n / total_wall:>10,.0f}")
    total_n = sum(r["iterations"] for r in rows)
    total_wall = sum(r["wall_seconds"] for r in rows)
    print()
    print(f"total: {total_n:,} validations in {total_wall:,.1f}s "
          f"({total_n / total_wall:,.1f} validations/s, batch mode, single process per fixture)")


def bench_fixture(cli: Path, fixture: dict, fixtures_dir: Path,
                  iterations: int, warmup: int, jobs: int) -> dict:
    cmd = cli_command(cli, fixture, fixtures_dir)

    def one_run(_: int) -> float:
        start = time.perf_counter()
        subprocess.run(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        return time.perf_counter() - start

    for _ in range(warmup):
        one_run(0)

    wall_start = time.perf_counter()
    if jobs > 1:
        with ThreadPoolExecutor(max_workers=jobs) as pool:
            samples = list(pool.map(one_run, range(iterations)))
    else:
        samples = [one_run(i) for i in range(iterations)]
    wall = time.perf_counter() - wall_start

    samples.sort()
    return {
        "name": fixture["name"],
        "kind": fixture["kind"],
        "tier": tier_of(fixture["bytes"]),
        "bytes": fixture["bytes"],
        "expected_valid": fixture["expected_valid"],
        "iterations": iterations,
        "wall_seconds": wall,
        "mean_ms": statistics.mean(samples) * 1000,
        "p50_ms": percentile(samples, 0.50) * 1000,
        "p95_ms": percentile(samples, 0.95) * 1000,
        "p99_ms": percentile(samples, 0.99) * 1000,
        "min_ms": samples[0] * 1000,
        "max_ms": samples[-1] * 1000,
        "ops_per_sec": iterations / wall,
    }


def print_table(rows: list, baseline_ms: float, jobs: int) -> None:
    header = (
        f"{'fixture':<38} {'kind':<8} {'size':>9} {'n':>6} "
        f"{'mean':>9} {'p50':>9} {'p95':>9} {'p99':>9} {'max':>10} {'ops/s':>8}"
    )
    print()
    print(header)
    print("-" * len(header))
    for row in rows:
        print(
            f"{row['name']:<38} {row['kind']:<8} {human_bytes(row['bytes']):>9} "
            f"{row['iterations']:>6} "
            f"{row['mean_ms']:>7.2f}ms {row['p50_ms']:>7.2f}ms {row['p95_ms']:>7.2f}ms "
            f"{row['p99_ms']:>7.2f}ms {row['max_ms']:>8.2f}ms {row['ops_per_sec']:>8.1f}"
        )
    print("-" * len(header))

    tiers = {}
    for row in rows:
        tiers.setdefault(row["tier"], []).append(row)
    for tier in ("tiny", "small", "medium", "large", "xlarge"):
        if tier not in tiers:
            continue
        tier_rows = tiers[tier]
        total_n = sum(r["iterations"] for r in tier_rows)
        total_wall = sum(r["wall_seconds"] for r in tier_rows)
        mean = sum(r["mean_ms"] * r["iterations"] for r in tier_rows) / total_n
        print(
            f"{'tier ' + tier:<38} {'':<8} {'':>9} {total_n:>6} "
            f"{mean:>7.2f}ms {'':>9} {'':>9} {'':>9} {'':>10} {total_n / total_wall:>8.1f}"
        )

    total_n = sum(r["iterations"] for r in rows)
    total_wall = sum(r["wall_seconds"] for r in rows)
    print()
    print(f"total: {total_n:,} validations in {total_wall:,.1f}s "
          f"({total_n / total_wall:,.1f} validations/s, jobs={jobs})")
    print(f"process spawn baseline (rtblint --version): {baseline_ms:.2f} ms mean; "
          "every sample above includes roughly this much fixed process-start cost")


def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument("--iterations", type=int, default=100_000,
                        help="total validations across all fixtures (default 100000)")
    parser.add_argument("--per-fixture", type=int, default=None,
                        help="validations per fixture (overrides --iterations)")
    parser.add_argument("--quick", action="store_true",
                        help="shorthand for --per-fixture 50")
    parser.add_argument("--batch", action="store_true",
                        help="use the CLI's --batch mode: one process per fixture, all "
                             "iterations through stdin; measures amortized per-payload cost")
    parser.add_argument("--jobs", type=int, default=1,
                        help="parallel CLI processes (default 1; >1 measures throughput, "
                             "latency percentiles will include contention)")
    parser.add_argument("--warmup", type=int, default=3,
                        help="untimed warmup runs per fixture (default 3)")
    parser.add_argument("--filter", default=None,
                        help="only run fixtures whose name contains this substring")
    parser.add_argument("--cli", default=None,
                        help="path to rtblint binary (default: target/release/rtblint, "
                             "built automatically if missing)")
    parser.add_argument("--fixtures", default=str(BENCH_DIR / "fixtures"),
                        help="fixtures directory")
    parser.add_argument("--no-save", action="store_true",
                        help="do not write a JSON result file")
    args = parser.parse_args()

    cli = ensure_cli(args.cli)
    fixtures_dir = Path(args.fixtures)
    manifest = ensure_fixtures(fixtures_dir)

    if args.filter:
        manifest = [f for f in manifest if args.filter in f["name"]]
        if not manifest:
            sys.exit(f"error: no fixtures match filter {args.filter!r}")

    if args.quick:
        per_fixture = 50
    elif args.per_fixture:
        per_fixture = args.per_fixture
    else:
        per_fixture = max(1, args.iterations // len(manifest))

    version = subprocess.run(
        [str(cli), "--version"], capture_output=True, text=True
    ).stdout.strip() or subprocess.run(
        [str(cli), "-V"], capture_output=True, text=True
    ).stdout.strip()

    mode = "batch" if args.batch else "per-process"
    print(f"RTBlint benchmark · {version or cli} · mode: {mode}")
    print(f"fixtures: {len(manifest)} · per fixture: {per_fixture} "
          f"· total: {per_fixture * len(manifest):,} validations · jobs: {args.jobs}")

    print("verifying fixtures against the CLI...")
    for fixture in manifest:
        verify_fixture(cli, fixture, fixtures_dir)

    baseline_ms = measure_baseline(cli) * 1000

    rows = []
    for index, fixture in enumerate(manifest, 1):
        if args.batch:
            row = bench_fixture_batch(cli, fixture, fixtures_dir, per_fixture)
        else:
            row = bench_fixture(cli, fixture, fixtures_dir, per_fixture, args.warmup, args.jobs)
        rows.append(row)
        print(f"[{index:>2}/{len(manifest)}] {row['name']:<38} "
              f"{human_bytes(row['bytes']):>9}  mean {row['mean_ms']:.3f} ms  "
              f"({row['ops_per_sec']:,.0f}/s)", flush=True)

    if args.batch:
        print_batch_table(rows)
    else:
        print_table(rows, baseline_ms, args.jobs)

    if not args.no_save:
        results_dir = BENCH_DIR / "results"
        results_dir.mkdir(exist_ok=True)
        stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
        out_path = results_dir / f"bench-{stamp}.json"
        out_path.write_text(json.dumps(
            {
                "timestamp": stamp,
                "cli_version": version,
                "platform": platform.platform(),
                "machine": platform.machine(),
                "python": platform.python_version(),
                "mode": "batch" if args.batch else "per-process",
                "jobs": args.jobs,
                "per_fixture": per_fixture,
                "warmup": args.warmup,
                "baseline_spawn_ms": baseline_ms,
                "results": rows,
            },
            indent=2,
        ) + "\n")
        print(f"saved {out_path.relative_to(REPO_ROOT)}")


if __name__ == "__main__":
    main()
