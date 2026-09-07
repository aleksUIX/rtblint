# ARTF simulator

A slowed-down ARTF hop with a real RTBlint in the middle.

Host (exchange) and agent (curator / shader) are fiction. The payloads are the
fixtures in `crates/rtblint-core/tests/fixtures/artf/` plus two demo scenes in
`demo/artf/fixtures/`. The verdicts are the same engine the CLI, MCP, gRPC, and
WASM surfaces already ship.

The console is a CTV mid-roll: living-room dummy on the left, OpenRTB ticket in
the middle, mutation cards on the right. The TV is CSS. It is not a player.
ARTF stops at the auction.

Five named hops (episode 2 screenplay: `SCREENPLAY.md`). Episode 1
(`SCREENPLAY-ONE.md`) is the intro: where the hop sits, then curation,
suppress, shade, and a clean production mix. Console: `/?ep=1`. Episode 3
(`SCREENPLAY-THREE.md`) is the envelope: lifecycle, intents, semantic paths,
independent apply, then the same RPC after the bid. Console: `/?ep=3`.

1. SSP. Looks legal, is not. Wrong extension-point id, wrong lifecycle intent, missing paths, document-vs-proto vocabulary, `ADJUST_DEAL_MARGIN` with nowhere to write. Drop. TV: no bid.
2. Engineer. Mutation is fine, auction is not. `ADD_METRICS` with `"value": "high"`. Static pass, apply fails. TV: invalid auction.
3. DSP. Curation lands. Segments, deal, floor, numeric metric. Forward. Dummy mid-roll.
4. Publisher. `SUPPRESS_DEALS` pulls `deal-standard` off the wire. Forward. Different dummy.
5. DSP bid. `LIFECYCLE_DSP_BID_RESPONSE` + `BID_SHADE`. $18.40 becomes $12.88. Same dummy, new price.

## Picture it in four containers

Stub SSP, stub agent, stub DSP, real `rtblint-grpc`. Delivery / player / VAST
is the hop after a win. It is not in this compose.

From the rtblint repo root:

```
docker compose -f demo/artf/compose.yaml up --build
```

Or from this directory: `docker compose up --build`.

Open http://localhost:8080/ for the console. Grafana (anonymous, dark):
http://localhost:3001/

Click a named hop for the story. Click **Start** on the mixer for a synthetic
firehose (default 40/s, 90% clean / 8% illegal / 2% breaking). Mixer uses
scenes 1–3 only. Concurrent auctions, real gRPC. The tape samples; Grafana is
the full view.

Prometheus scrapes `rtblint-grpc :9091` (real process metrics: RPC rate,
latency histogram, shed, adaptive concurrency) and the SSP auction counters.
Prometheus itself is on http://localhost:9092/

First image build compiles Rust. Give it several minutes. Later `up` is fast.

Same hops without the browser:

```
curl -sS -X POST http://localhost:8080/run \
  -H 'content-type: application/json' \
  -d '{"beat":"illegal"}'
curl -sS -X POST http://localhost:8080/run \
  -H 'content-type: application/json' \
  -d '{"beat":"shade"}'
curl -sS http://localhost:8080/inbox
```

Mixer, 15 seconds at 40/s:

```
curl -sS -X POST http://localhost:8080/stream \
  -H 'content-type: application/json' \
  -d '{"qps":5000,"seconds":15}'
```

gRPC only, no SSP:

```
docker run --rm -p 50061:50061 -p 9091:9091 \
  $(docker compose -f demo/artf/compose.yaml build -q rtblint)
```

Then `grpcurl` against `ValidateArtfMutations`.

Re-record the console (compose must be up):

```
npm install
node record.mjs
```

Writes `artf-sim-demo.mp4` in this directory.

90s send-ahead (scenes 1–3, 5k/s mixer, Grafana). Compose and Grafana must be up:

```
npm install
node record-pitch.mjs
```

Writes `artf-sim-pitch.mp4`.

Five-minute teaching cut (living-room open, five hops, mixer, Grafana):

```
npm install
node record-five.mjs
```

Writes `artf-sim-five.mp4`. Spoken script: `SCREENPLAY.md`.

Episode 1 intro (living-room, hop map, curation / suppress / shade, production mixer, Grafana):

```
npm install
node record-one.mjs
```

Writes `artf-sim-one.mp4`. Spoken script: `SCREENPLAY-ONE.md`. Console must be
reachable as `http://localhost:8080/?ep=1`. Grafana: `:3001/d/artf-sim-hop`.

Episode 3 envelope (envelope pane, host intent list, semantic paths, one skipped
write, shade, production mixer, Grafana):

```
npm install
node record-three.mjs
```

Writes `artf-sim-three.mp4`. Spoken script: `SCREENPLAY-THREE.md`. Console:
`http://localhost:8080/?ep=3`. Prompter: `npm run prompter:three`.

Episode 4 fan-in (lab hop graph, apply table, isolated shaders, Play VAST, Grafana):

```
npm install
node record-four.mjs
```

Writes `artf-sim-four.mp4`. Spoken script: `SCREENPLAY-FOUR.md`. Lab must be
up at `http://localhost:8080/lab/`. Grafana: `:3000`. Prompter: `npm run prompter:four`.

## Picture it in the browser (WASM, no Docker)

From `rtblint-infra/apps/rtblint-web`:

```
npm run dev
```

Open http://localhost:3000/demo/artf/

Live URL once this tree is deployed: https://rtblint.org/demo/artf/

Browser demo is still the original three beats. The five-hop CTV stage lives
in this compose.

## Picture it in the terminal

From this directory:

```
./run.sh
```

Needs `rtblint` on PATH, or a working `cargo` in the repo root. Three beats,
same core fixtures, `would_forward` printed from whether any error fired.

This is the reel-friendly version if Docker is not running: one command, no
browser.
