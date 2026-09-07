#!/usr/bin/env node
// SSP / exchange orchestrator. Simulated host: holds the auction, calls the
// agent, calls real RTBlint over gRPC, forwards to the DSP only on a clean
// applied pass. Shade scenes never call the DSP: the bid is already in the
// envelope. Serves the console at /. Quiet mixer for volume; verbose hops
// for named scenes.

import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import grpc from "@grpc/grpc-js";
import protoLoader from "@grpc/proto-loader";
import {
  annotateMutations,
  fromBidRequest,
  fromBidResponse,
  parseJson,
  pickDummy,
} from "./ticket.mjs";

const __dirname = dirname(fileURLToPath(import.meta.url));
const PUBLIC = join(__dirname, "public");
const PORT = Number(process.env.PORT || 8080);
const RTBLINT_ADDR = process.env.RTBLINT_ADDR || "127.0.0.1:50061";
const AGENT_URL = process.env.AGENT_URL || "http://127.0.0.1:8091";
const DSP_URL = process.env.DSP_URL || "http://127.0.0.1:8092";
const GRAFANA_URL = process.env.GRAFANA_URL || "http://localhost:3001/d/artf-sim";
const FIX = process.env.FIXTURES || "/fixtures";
const PROTO = process.env.PROTO || "/proto/openadtech/rtblint/v1/rtblint.proto";
const PROTO_ROOT = process.env.PROTO_ROOT || "/proto";
const TMAX_MS = 150;
const PACE_MS = Number(process.env.PACE_MS || 700);

const SCENES = {
  illegal: {
    title: "Looks legal. Is not.",
    blurb: "Wrong id, wrong intent, missing paths. Drop.",
    perspective: "ssp",
    perspectiveLabel: "SSP / publisher",
    punchline: "The framework accepted the RPC. The auction was never asked.",
    envelope: "publisher",
    file: "invalid-mutations.json",
    hop: "request",
    tvFail: "unsold",
  },
  breaking: {
    title: "Mutation fine. Auction not.",
    blurb: "ADD_METRICS value \"high\". Apply fails.",
    perspective: "exchange",
    perspectiveLabel: "Exchange engineer",
    punchline: "Well-formed mutation and still-valid OpenRTB are different questions.",
    envelope: "publisher",
    file: "breaking-mutations.json",
    hop: "request",
    tvFail: "poison",
  },
  clean: {
    title: "Curation lands. DSP bids.",
    blurb: "Segments, deal, floor, metric. Dummy mid-roll.",
    perspective: "dsp",
    perspectiveLabel: "DSP / buyer",
    punchline: "The DSP is bidding on the auction the agent actually wrote.",
    envelope: "publisher",
    file: "valid-mutations.json",
    hop: "request",
    tvOk: "ad",
  },
  policy: {
    title: "The host still chooses.",
    blurb: "Four writes land. One intent was not offered.",
    perspective: "ssp",
    perspectiveLabel: "Host / SSP",
    punchline: "Independently acceptable means the host still chooses.",
    envelope: "publisher",
    file: "policy-mutations.json",
    hop: "request",
    tvOk: "ad",
    hostIntents: [
      "ACTIVATE_SEGMENTS",
      "ACTIVATE_DEALS",
      "ADJUST_DEAL_FLOOR",
      "ADD_METRICS",
    ],
    notOffered: ["BID_SHADE"],
  },
  suppress: {
    title: "A deal leaves the wire.",
    blurb: "SUPPRESS_DEALS pulls deal-standard. DSP never sees it.",
    perspective: "publisher",
    perspectiveLabel: "Publisher / PMP",
    punchline: "deal-standard is off the wire. The DSP never saw it.",
    envelope: "publisher",
    file: "suppress-mutations.json",
    hop: "request",
    tvOk: "ad",
  },
  shade: {
    title: "The bid is shaded.",
    blurb: "DSP response hop. $18.40 becomes $12.88.",
    perspective: "dsp",
    perspectiveLabel: "DSP / bid response",
    punchline: "The bid is still a bid. The price is not.",
    envelope: "dsp",
    file: "shade-mutations.json",
    hop: "response",
    tvOk: "ad",
  },
};

const MIXER_BEATS = ["illegal", "breaking", "clean", "suppress", "shade"];
function seedOutcomeZeros() {
  for (const beat of MIXER_BEATS) {
    if (counters.byOutcome[`${beat}:forward:forward`] == null) {
      counters.byOutcome[`${beat}:forward:forward`] = 0;
    }
    if (counters.byOutcome[`${beat}:drop:mutations`] == null) {
      counters.byOutcome[`${beat}:drop:mutations`] = 0;
    }
  }
}
const MIXES = {
  dirt: { illegal: 0.08, breaking: 0.02, suppress: 0.04, shade: 0.01, clean: 0.85 },
  production: { illegal: 0, breaking: 0, suppress: 0.08, shade: 0.04, clean: 0.88 },
};
const HIST_LE = [0.001, 0.002, 0.004, 0.008, 0.016, 0.032, 0.064, 0.128, 0.256, 0.5, 1, 2.5];
const POPS = [
  { id: "us-west-2", w: 0.46 },
  { id: "us-east-1", w: 0.34 },
  { id: "eu-west-1", w: 0.2 },
];
const INVS = [
  { id: "ctv", app: "streaming.example", w: 0.58 },
  { id: "app", app: "com.example.sports", w: 0.27 },
  { id: "site", app: "news.example", w: 0.15 },
];

const envelopes = {
  publisher: JSON.parse(readFileSync(join(FIX, "valid-publisher-request.json"), "utf8")),
  dsp: JSON.parse(readFileSync(join(FIX, "ctv-dsp-request.json"), "utf8")),
};
const mutationsJson = Object.fromEntries(
  Object.entries(SCENES).map(([id, meta]) => [id, readFileSync(join(FIX, meta.file), "utf8")]),
);
const mutationsDoc = Object.fromEntries(
  Object.entries(mutationsJson).map(([id, raw]) => [id, JSON.parse(raw)]),
);

const clients = new Set();
const RPC_POOL = 4;
let rpcPool = [];
let rpcCursor = 0;
let ready = false;
let inFlight = 0;
let stream = null;
const counters = {
  auctions: 0,
  forwarded: 0,
  dropped: 0,
  errors: 0,
  byBeat: Object.fromEntries(Object.keys(SCENES).map((k) => [k, 0])),
  byOutcome: {},
  byGate: {},
  byPop: {},
  byInv: {},
  byRule: {},
};
const hist = { buckets: new Array(HIST_LE.length + 1).fill(0), sum: 0, count: 0 };
const latRing = [];
const LAT_RING = 800;
const windowTs = [];
seedOutcomeZeros();
let logTokens = 0;
let logLast = Date.now();

function takeLog(priority) {
  const now = Date.now();
  logTokens += (32 * (now - logLast)) / 1000;
  logLast = now;
  if (logTokens > 48) logTokens = 48;
  const cost = priority ? 0.55 : 1;
  if (logTokens < cost) return false;
  logTokens -= cost;
  return true;
}

function bump(map, key) {
  map[key] = (map[key] || 0) + 1;
}

function bumpOutcome(beat, outcome, gate) {
  bump(counters.byOutcome, `${beat}:${outcome}:${gate}`);
}

function pickWeighted(list) {
  const r = Math.random();
  let acc = 0;
  for (const row of list) {
    acc += row.w;
    if (r < acc) return row;
  }
  return list[list.length - 1];
}

function reqId() {
  return `br-${Math.random().toString(16).slice(2, 10)}`;
}

function facadeFor(verbose) {
  if (verbose) {
    return { pop: "us-west-2", inv: "ctv", app: "streaming.example", req: reqId() };
  }
  const pop = pickWeighted(POPS);
  const inv = pickWeighted(INVS);
  return { pop: pop.id, inv: inv.id, app: inv.app, req: reqId() };
}

function hopName(meta) {
  return meta.envelope === "dsp" ? "DSP_BID_RESPONSE" : "PUBLISHER_BID_REQUEST";
}

function envelopeView(doc, extras = {}) {
  const attached = doc.bid_response && Object.keys(doc.bid_response).length ? "bid_response" : "bid_request";
  return {
    epId: doc.id || "",
    tmax: doc.tmax ?? TMAX_MS,
    lifecycle: doc.lifecycle || "",
    originatorType: doc.originator?.type || "",
    originatorId: doc.originator?.id || "",
    intents: [...(doc.applicable_intents || [])],
    attached,
    bidId: doc.bid_request?.id || "",
    notOffered: [...(extras.notOffered || [])],
  };
}

function applyHostPolicy(meta, proposed, mutationsRaw) {
  if (!meta.hostIntents) {
    return {
      mutations: mutationsRaw,
      hostSkipped: [],
      keptIdx: proposed.map((_, i) => i),
    };
  }
  const offered = new Set(meta.hostIntents);
  const kept = [];
  const keptIdx = [];
  const hostSkipped = [];
  for (let i = 0; i < proposed.length; i += 1) {
    if (offered.has(proposed[i].intent)) {
      keptIdx.push(i);
      kept.push(proposed[i]);
    } else {
      hostSkipped.push(i);
    }
  }
  const parsed = JSON.parse(mutationsRaw);
  parsed.mutations = kept;
  return { mutations: JSON.stringify(parsed), hostSkipped, keptIdx };
}

function remapIndexes(grpcIndexes, keptIdx) {
  return (grpcIndexes || [])
    .map((i) => keptIdx[Number(i)])
    .filter((n) => typeof n === "number");
}

function remapIssuePaths(issues, keptIdx) {
  return (issues || []).map((iss) => {
    const m = String(iss.path || "").match(/^mutations\[(\d+)\](.*)$/);
    if (!m) return iss;
    const orig = keptIdx[Number(m[1])];
    if (orig == null) return iss;
    return { ...iss, path: `mutations[${orig}]${m[2]}` };
  });
}

function classifyGate(forwarded, mutIssues) {
  if (forwarded) return "forward";
  const appliedFail = (mutIssues.issues || []).some(
    (i) => i.severity === "error" && String(i.id).startsWith("openrtb."),
  );
  return appliedFail ? "applied" : "mutations";
}

function promLabel(value) {
  return String(value || "none")
    .replace(/\\/g, "\\\\")
    .replace(/"/g, "\\\"")
    .slice(0, 96);
}

function broadcast(event) {
  const line = `data: ${JSON.stringify({ t: Date.now(), ...event })}\n\n`;
  for (const res of clients) res.write(line);
}

function observe(seconds) {
  hist.sum += seconds;
  hist.count += 1;
  for (let i = 0; i < HIST_LE.length; i += 1) {
    if (seconds <= HIST_LE[i]) hist.buckets[i] += 1;
  }
  hist.buckets[HIST_LE.length] += 1;
  latRing.push(seconds);
  if (latRing.length > LAT_RING) latRing.shift();
}

function percentile(p) {
  if (!latRing.length) return 0;
  const sorted = [...latRing].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.floor(p * (sorted.length - 1)))];
}

function qpsNow() {
  const cut = Date.now() - 1000;
  while (windowTs.length && windowTs[0] < cut) windowTs.shift();
  return windowTs.length;
}

function snapshot() {
  return {
    ready,
    grafana: GRAFANA_URL,
    tmax_ms: TMAX_MS,
    stream: stream
      ? { on: true, qps: stream.qps, until: stream.until, mix: stream.mix }
      : { on: false },
    auctions: counters.auctions,
    forwarded: counters.forwarded,
    dropped: counters.dropped,
    errors: counters.errors,
    byBeat: { ...counters.byBeat },
    inFlight,
    qps: qpsNow(),
    p50_ms: percentile(0.5) * 1000,
    p99_ms: percentile(0.99) * 1000,
  };
}

function issuesOf(verdict) {
  const issues = verdict?.issues || [];
  const errors = issues.filter((i) => String(i.severity).includes("ERROR")).length;
  const warnings = issues.filter((i) => String(i.severity).includes("WARNING")).length;
  return {
    valid: Boolean(verdict?.valid),
    errors,
    warnings,
    issues: issues.map((i) => ({
      id: i.rule_id,
      severity: String(i.severity).replace(/^SEVERITY_/, "").toLowerCase(),
      message: i.message,
      path: i.path || "",
    })),
  };
}

function rpc(name, req) {
  const stub = rpcPool[rpcCursor++ & (rpcPool.length - 1)] || rpcPool[0];
  const fn = stub[name] || stub[name.charAt(0).toLowerCase() + name.slice(1)];
  if (typeof fn !== "function") {
    throw new Error(`rpc ${name} is not on the client`);
  }
  return new Promise((resolve, reject) => {
    fn.call(stub, req, (err, res) => (err ? reject(err) : resolve(res)));
  });
}

async function waitForRtblint() {
  const def = protoLoader.loadSync(PROTO, {
    keepCase: true,
    longs: String,
    enums: String,
    defaults: true,
    oneofs: true,
    includeDirs: [PROTO_ROOT],
  });
  const pack = grpc.loadPackageDefinition(def);
  const Client =
    pack?.openadtech?.rtblint?.v1?.RtblintService ||
    pack?.openadtech?.rtblint?.v1?.rtblintService;
  if (!Client) throw new Error("RtblintService missing from proto");
  const make = () =>
    new Client(RTBLINT_ADDR, grpc.credentials.createInsecure(), {
      "grpc.keepalive_time_ms": 8000,
      "grpc.keepalive_timeout_ms": 2000,
      "grpc.max_concurrent_streams": 2048,
    });
  const first = make();
  for (let i = 0; i < 90; i += 1) {
    const ok = await new Promise((resolve) => {
      first.waitForReady(Date.now() + 2000, (err) => resolve(!err));
    });
    if (ok) {
      rpcPool = [first];
      for (let n = 1; n < RPC_POOL; n += 1) {
        const extra = make();
        await new Promise((resolve) => extra.waitForReady(Date.now() + 4000, () => resolve()));
        rpcPool.push(extra);
      }
      ready = true;
      console.log(`ssp connected to rtblint at ${RTBLINT_ADDR} · ${rpcPool.length} channels`);
      return;
    }
  }
  throw new Error(`rtblint not reachable at ${RTBLINT_ADDR}`);
}

function pickBeat(mix) {
  const r = Math.random();
  let acc = 0;
  for (const beat of MIXER_BEATS) {
    acc += Number(mix[beat] || 0);
    if (r < acc) return beat;
  }
  return "clean";
}

function cloneEnvelope(kind, auctionId) {
  const doc = structuredClone(envelopes[kind]);
  if (doc.bid_request) doc.bid_request.id = auctionId;
  if (doc.bid_response) doc.bid_response.id = auctionId;
  return doc;
}

function restStage() {
  const doc = envelopes.publisher;
  return {
    scene: "open",
    phase: "open",
    perspective: "publisher",
    perspectiveLabel: "Publisher / CTV",
    hop: "request",
    tv: "content",
    punchline: "",
    before: fromBidRequest(doc.bid_request),
    after: fromBidRequest(doc.bid_request),
    bidBefore: null,
    bidAfter: null,
    mutations: [],
    findings: [],
    dummy: null,
    forwarded: false,
    envelope: envelopeView(
      {
        ...doc,
        applicable_intents: [
          "ACTIVATE_SEGMENTS",
          "ACTIVATE_DEALS",
          "ADJUST_DEAL_FLOOR",
          "ADD_METRICS",
        ],
      },
      { notOffered: ["BID_SHADE"] },
    ),
  };
}

function makeHopClock() {
  let acc = 0n;
  let t0 = process.hrtime.bigint();
  return {
    pause() {
      acc += process.hrtime.bigint() - t0;
    },
    resume() {
      t0 = process.hrtime.bigint();
    },
    seconds() {
      return Number(acc + (process.hrtime.bigint() - t0)) / 1e9;
    },
  };
}

function pace(verbose, hop) {
  if (!verbose) return Promise.resolve();
  hop.pause();
  return new Promise((r) => {
    setTimeout(() => {
      hop.resume();
      r();
    }, PACE_MS);
  });
}

async function runAuction(beat, { verbose, hopOnly = false }) {
  const meta = SCENES[beat];
  if (!meta) throw Object.assign(new Error(`unknown beat: ${beat}`), { status: 400 });
  if (!ready) throw Object.assign(new Error("rtblint not ready"), { status: 503 });

  const hop = makeHopClock();
  inFlight += 1;

  const facade = facadeFor(verbose);
  const doc = cloneEnvelope(meta.envelope, facade.req);
  if (meta.hostIntents) {
    doc.applicable_intents = [...meta.hostIntents];
  }
  const envSnap = envelopeView(doc, { notOffered: meta.notOffered || [] });
  let envelope = JSON.stringify(doc);
  const before = fromBidRequest(doc.bid_request);
  const bidBefore = fromBidResponse(doc.bid_response);
  let mutations = mutationsJson[beat];
  let proposed = mutationsDoc[beat].mutations || [];
  let intentCount = proposed.length;
  let dummy = null;
  let hostSkipped = [];
  let keptIdx = proposed.map((_, i) => i);

  const emitStage = (phase, extra = {}) => {
    if (!verbose) return;
    broadcast({
      type: "stage",
      scene: beat,
      phase,
      perspective: meta.perspective,
      perspectiveLabel: meta.perspectiveLabel,
      hop: meta.hop,
      title: meta.title,
      before,
      bidBefore,
      envelope: envSnap,
      ...extra,
    });
  };

  try {
    if (verbose) {
      broadcast({
        actor: "ssp",
        kind: "info",
        text: `Auction start · ${beat}: ${meta.title}`,
      });
      emitStage("open", { tv: "content", after: before, bidAfter: bidBefore, mutations: [], findings: [] });
      await pace(true, hop);
      broadcast({
        actor: "ssp",
        kind: "info",
        text: `Envelope ${doc.id} · ${doc.lifecycle} · auction ${doc.bid_request.id}`,
      });
      emitStage("envelope", { tv: "content", after: before, bidAfter: bidBefore, mutations: [], findings: [] });
      await pace(true, hop);
      if (meta.hostIntents) {
        emitStage("intents", { tv: "content", after: before, bidAfter: bidBefore, mutations: [], findings: [] });
        await pace(true, hop);
      }
      broadcast({ actor: "agent", kind: "info", text: "Calling agent" });
      const agentRes = await fetch(`${AGENT_URL}/mutate?beat=${encodeURIComponent(beat)}`);
      if (!agentRes.ok) throw new Error(`agent ${agentRes.status}`);
      const agentBody = await agentRes.json();
      mutations = JSON.stringify(agentBody.mutations);
      proposed = agentBody.mutations.mutations || [];
      intentCount = proposed.length;
      const filtered = applyHostPolicy(meta, proposed, JSON.stringify(agentBody.mutations));
      mutations = filtered.mutations;
      hostSkipped = filtered.hostSkipped;
      keptIdx = filtered.keptIdx;
      envelope = JSON.stringify(doc);
      broadcast({
        actor: "agent",
        kind: "info",
        text: `${agentBody.agent} proposed ${intentCount} mutation(s)`,
      });
      if (hostSkipped.length) {
        broadcast({
          actor: "ssp",
          kind: "ok",
          text: `Host skipped ${hostSkipped.length} mutation(s) not on this hop's list`,
        });
      }
      emitStage("agent", {
        tv: "content",
        after: before,
        bidAfter: bidBefore,
        mutations: annotateMutations(proposed, [], [], hostSkipped).map((card, i) =>
          hostSkipped.includes(i) ? { ...card, reason: "not offered" } : card,
        ),
        findings: [],
      });
      await pace(true, hop);
      broadcast({
        actor: "rtblint",
        kind: "info",
        text: hopOnly ? "Apply · envelope" : "Pass 1 · ValidateArtfEnvelope",
      });
    } else {
      const filtered = applyHostPolicy(meta, proposed, mutations);
      mutations = filtered.mutations;
      hostSkipped = filtered.hostSkipped;
      keptIdx = filtered.keptIdx;
      envelope = JSON.stringify(doc);
    }

    let rpcNs = 0n;
    let envOut = { verdict: { valid: true, issues: [] } };
    if (verbose) {
      const envStarted = process.hrtime.bigint();
      envOut = await rpc("ValidateArtfEnvelope", { rtb_request: envelope });
      rpcNs += process.hrtime.bigint() - envStarted;
    }
    const envIssues = issuesOf(envOut.verdict);
    if (verbose) {
      broadcast({
        actor: "rtblint",
        kind: envIssues.errors ? "error" : "ok",
        text: hopOnly
          ? `Envelope ${envIssues.valid ? "ok" : "failed"}`
          : `Envelope ${envIssues.valid ? "ok" : "failed"} · ${envIssues.errors} error(s), ${envIssues.warnings} warning(s)`,
        extra: envIssues.issues,
      });
      await pace(true, hop);
      broadcast({
        actor: "rtblint",
        kind: "info",
        text: hopOnly ? "Apply · mutations" : "Pass 2+3 · ValidateArtfMutations apply=true",
      });
    }

    const mutStarted = process.hrtime.bigint();
    const mutOut = await rpc("ValidateArtfMutations", {
      rtb_request: envelope,
      rtb_response: mutations,
      apply: true,
    });
    rpcNs += process.hrtime.bigint() - mutStarted;
    const seconds = Number(rpcNs) / 1e9;
    observe(seconds);
    const mutIssuesRaw = issuesOf(mutOut.verdict);
    const mutIssues = {
      ...mutIssuesRaw,
      issues: remapIssuePaths(mutIssuesRaw.issues, keptIdx),
    };
    const applied = remapIndexes(mutOut.application?.applied || [], keptIdx);
    const skipped = [...hostSkipped, ...remapIndexes(mutOut.application?.skipped || [], keptIdx)];
    const rewrittenReq = parseJson(mutOut.application?.bid_request) || doc.bid_request;
    const rewrittenRes = parseJson(mutOut.application?.bid_response) || doc.bid_response || null;
    const after = fromBidRequest(rewrittenReq);
    const bidAfter = fromBidResponse(rewrittenRes);
    const cards = annotateMutations(proposed, mutIssues.issues, applied, skipped);
    for (const i of hostSkipped) {
      if (cards[i]) {
        cards[i].status = "skipped";
        cards[i].reason = "not offered";
      }
    }

    if (verbose) {
      broadcast({
        actor: "rtblint",
        kind: mutIssues.errors ? "error" : "ok",
        text: hopOnly
          ? `Apply ${mutIssues.valid ? "ok" : "failed"} · ${applied.length} write(s)`
          : `Mutations ${mutIssues.valid ? "ok" : "failed"} · ${mutIssues.errors} error(s), ${mutIssues.warnings} warning(s) · applied [${applied}] skipped [${skipped}]`,
        extra: mutIssues.issues,
      });
      emitStage("lint", {
        tv: "content",
        after,
        bidAfter,
        mutations: cards,
        findings: mutIssues.issues,
        applied,
        skipped,
      });
      await pace(true, hop);
    }

    let dsp = null;
    const forwarded = mutIssues.errors === 0 && mutIssues.valid;
    let tv = "content";
    if (forwarded) {
      if (meta.hop === "response") {
        dummy = bidAfter?.dummy || pickDummy(after);
        tv = meta.tvOk || "ad";
        if (verbose) {
          broadcast({
            actor: "ssp",
            kind: "ok",
            text: `Accepting shaded bid · ${bidBefore?.price} → ${bidAfter?.price}`,
          });
          broadcast({
            actor: "dsp",
            kind: "ok",
            text: `Bid still in · seat ${bidAfter?.seat} · price ${bidAfter?.price}`,
          });
        }
      } else if (verbose) {
        const body = JSON.stringify(rewrittenReq);
        broadcast({ actor: "ssp", kind: "ok", text: "Forwarding rewritten bid request to DSP" });
        const dspRes = await fetch(`${DSP_URL}/bid`, {
          method: "POST",
          headers: { "content-type": "application/json" },
          body,
        });
        dsp = await dspRes.json();
        dummy = dsp.dummy || pickDummy(after);
        tv = meta.tvOk || "ad";
        const price = dsp.seatbid?.[0]?.bid?.[0]?.price;
        broadcast({
          actor: "dsp",
          kind: "ok",
          text: `Bid in · seat ${dsp.seatbid?.[0]?.seat} · price ${price} · ${dummy?.title || "dummy"}`,
        });
      } else {
        // Mixer: lint is the hop. Named scenes still call the DSP.
        dummy = pickDummy(after);
        tv = meta.tvOk || "ad";
      }
    } else {
      tv = meta.tvFail || "unsold";
      if (verbose) {
        broadcast({ actor: "ssp", kind: "error", text: "Dropping mutation set. DSP is not called." });
        broadcast({ actor: "dsp", kind: "info", text: "No bid request arrived" });
      }
    }

    const outcome = forwarded ? "forward" : "drop";
    const gate = classifyGate(forwarded, mutIssues);
    const first = mutIssues.issues.find((i) => i.severity === "error");
    const rule = first?.id || "";
    counters.auctions += 1;
    counters.byBeat[beat] = (counters.byBeat[beat] || 0) + 1;
    bumpOutcome(beat, outcome, gate);
    bump(counters.byGate, gate);
    bump(counters.byPop, `${facade.pop}:${outcome}`);
    bump(counters.byInv, `${facade.inv}:${outcome}`);
    if (rule && outcome === "drop") bump(counters.byRule, rule);
    if (forwarded) counters.forwarded += 1;
    else counters.dropped += 1;
    windowTs.push(Date.now());

    const row = {
      id: facade.req,
      beat,
      outcome,
      gate,
      ms: seconds * 1000,
      rule,
      applied: applied.length,
      intents: intentCount,
      pop: facade.pop,
      inv: facade.inv,
      app: facade.app,
      hop: hopName(meta),
      tmax: TMAX_MS,
    };

    const stage = {
      scene: beat,
      phase: "verdict",
      perspective: meta.perspective,
      perspectiveLabel: meta.perspectiveLabel,
      hop: meta.hop,
      title: meta.title,
      tv,
      punchline: meta.punchline,
      before,
      after,
      bidBefore,
      bidAfter,
      mutations: cards,
      findings: mutIssues.issues,
      dummy,
      forwarded,
      applied,
      skipped,
      envelope: envSnap,
      dspPrice: dsp?.seatbid?.[0]?.bid?.[0]?.price ?? bidAfter?.price ?? null,
    };

    if (verbose) {
      broadcast({
        actor: "ssp",
        kind: forwarded ? "ok" : "error",
        text: meta.punchline,
        extra: { forwarded, punchline: meta.punchline },
      });
      broadcast({ type: "stage", ...stage });
      broadcast({ type: "log", ...row });
    } else {
      const qps = stream?.qps || 1;
      const sampleEvery = Math.max(1, Math.round(qps / 8));
      if (counters.auctions % sampleEvery === 0) {
        broadcast({ type: "auction", ...row });
      }
      if (takeLog(outcome !== "forward")) {
        broadcast({ type: "log", ...row });
      }
    }

    return {
      beat,
      forwarded,
      punchline: meta.punchline,
      envelope: envIssues,
      mutations: mutIssues,
      applied,
      skipped,
      dsp,
      auction: row,
      stage,
    };
  } catch (err) {
    counters.errors += 1;
    counters.auctions += 1;
    counters.byBeat[beat] = (counters.byBeat[beat] || 0) + 1;
    bumpOutcome(beat, "error", "error");
    windowTs.push(Date.now());
    throw err;
  } finally {
    inFlight = Math.max(0, inFlight - 1);
  }
}

function stopStream() {
  if (stream) stream.stop = true;
  stream = null;
}

function resolveMix(mix) {
  if (typeof mix === "string" && MIXES[mix]) return { ...MIXES[mix] };
  const base = MIXES.dirt;
  if (!mix || typeof mix !== "object") return { ...base };
  return {
    illegal: Number(mix.illegal ?? base.illegal),
    breaking: Number(mix.breaking ?? base.breaking),
    suppress: Number(mix.suppress ?? base.suppress),
    shade: Number(mix.shade ?? base.shade),
    clean: Number(mix.clean ?? base.clean),
  };
}

function startStream({ qps = 2000, seconds = 45, mix, concurrency } = {}) {
  const q = Math.max(1, Math.min(20000, Number(qps) || 2000));
  const secs = Math.max(5, Math.min(180, Number(seconds) || 45));
  const m = resolveMix(mix);
  const cap = Math.max(32, Math.min(2048, Number(concurrency) || 96));
  stopStream();
  const handle = { stop: false, qps: q, until: Date.now() + secs * 1000, mix: m };
  stream = handle;
  broadcast({ type: "stream", on: true, qps: q, seconds: secs, mix: m });

  (async () => {
    let carried = 0;
    let last = Date.now();
    while (!handle.stop && Date.now() < handle.until) {
      const now = Date.now();
      const jitter = 0.72 + Math.random() * 0.56;
      const burst = Math.random() < 0.012 ? 1.8 : 1;
      carried += (q * jitter * burst * (now - last)) / 1000;
      last = now;
      if (carried > q * 0.35) carried = q * 0.35;
      const room = cap - inFlight;
      const n = Math.min(Math.floor(carried), room, 256);
      if (n > 0) {
        carried -= n;
        for (let i = 0; i < n; i += 1) {
          const beat = pickBeat(m);
          runAuction(beat, { verbose: false }).catch((err) => {
            broadcast({ type: "error", text: String(err.message || err) });
          });
        }
      }
      await new Promise((r) => setTimeout(r, 1));
    }
    if (stream === handle) stream = null;
    broadcast({ type: "stream", on: false });
  })();

  return { qps: q, seconds: secs, mix: m, concurrency: cap, beats: MIXER_BEATS };
}

function prometheusText() {
  const lines = [
    "# HELP artf_sim_auctions_total Simulated auctions through the SSP, by beat, outcome, and gate.",
    "# TYPE artf_sim_auctions_total counter",
  ];
  for (const [key, value] of Object.entries(counters.byOutcome)) {
    const [beat, outcome, gate] = key.split(":");
    lines.push(
      `artf_sim_auctions_total{beat="${promLabel(beat)}",outcome="${promLabel(outcome)}",gate="${promLabel(gate)}"} ${value}`,
    );
  }
  lines.push("# HELP artf_sim_pop_auctions_total Simulated auctions by pop and outcome.");
  lines.push("# TYPE artf_sim_pop_auctions_total counter");
  for (const [key, value] of Object.entries(counters.byPop)) {
    const [pop, outcome] = key.split(":");
    lines.push(`artf_sim_pop_auctions_total{pop="${promLabel(pop)}",outcome="${promLabel(outcome)}"} ${value}`);
  }
  lines.push("# HELP artf_sim_inv_auctions_total Simulated auctions by inventory class and outcome.");
  lines.push("# TYPE artf_sim_inv_auctions_total counter");
  for (const [key, value] of Object.entries(counters.byInv)) {
    const [inv, outcome] = key.split(":");
    lines.push(`artf_sim_inv_auctions_total{inv="${promLabel(inv)}",outcome="${promLabel(outcome)}"} ${value}`);
  }
  lines.push("# HELP artf_sim_drop_rules_total First error rule on dropped auctions.");
  lines.push("# TYPE artf_sim_drop_rules_total counter");
  for (const [rule, value] of Object.entries(counters.byRule)) {
    lines.push(`artf_sim_drop_rules_total{rule="${promLabel(rule)}"} ${value}`);
  }
  lines.push("# HELP artf_sim_in_flight Auctions currently in the hop.");
  lines.push("# TYPE artf_sim_in_flight gauge");
  lines.push(`artf_sim_in_flight ${inFlight}`);
  lines.push("# HELP artf_sim_stream_qps Configured mixer launch rate. Zero when idle.");
  lines.push("# TYPE artf_sim_stream_qps gauge");
  lines.push(`artf_sim_stream_qps ${stream ? stream.qps : 0}`);
  lines.push("# HELP artf_sim_auction_duration_seconds End-to-end SSP hop (agent cache + two gRPC RPCs + optional DSP).");
  lines.push("# TYPE artf_sim_auction_duration_seconds histogram");
  for (let i = 0; i < HIST_LE.length; i += 1) {
    lines.push(`artf_sim_auction_duration_seconds_bucket{le="${HIST_LE[i]}"} ${hist.buckets[i]}`);
  }
  lines.push(`artf_sim_auction_duration_seconds_bucket{le="+Inf"} ${hist.buckets[HIST_LE.length]}`);
  lines.push(`artf_sim_auction_duration_seconds_sum ${hist.sum}`);
  lines.push(`artf_sim_auction_duration_seconds_count ${hist.count}`);
  return `${lines.join("\n")}\n`;
}

function scenesPublic() {
  return Object.entries(SCENES).map(([id, meta]) => ({
    id,
    title: meta.title,
    blurb: meta.blurb,
    perspective: meta.perspective,
    perspectiveLabel: meta.perspectiveLabel,
    hop: meta.hop,
  }));
}

const server = createServer(async (req, res) => {
  const url = new URL(req.url || "/", `http://${req.headers.host}`);
  res.setHeader("access-control-allow-origin", "*");

  if (req.method === "GET" && url.pathname === "/events") {
    res.writeHead(200, {
      "content-type": "text/event-stream",
      "cache-control": "no-cache",
      connection: "keep-alive",
    });
    res.write("\n");
    clients.add(res);
    req.on("close", () => clients.delete(res));
    return;
  }

  if (req.method === "GET" && url.pathname === "/health") {
    json(res, 200, { ok: ready, role: "ssp", rtblint: RTBLINT_ADDR });
    return;
  }

  if (req.method === "GET" && url.pathname === "/metrics") {
    const body = prometheusText();
    res.writeHead(200, { "content-type": "text/plain; version=0.0.4; charset=utf-8" });
    res.end(body);
    return;
  }

  if (req.method === "GET" && url.pathname === "/stats") {
    json(res, 200, snapshot());
    return;
  }

  if (req.method === "GET" && url.pathname === "/scenes") {
    json(res, 200, { scenes: scenesPublic() });
    return;
  }

  if (req.method === "GET" && url.pathname === "/stage") {
    json(res, 200, restStage());
    return;
  }

  if (req.method === "GET" && url.pathname === "/inbox") {
    try {
      const r = await fetch(`${DSP_URL}/inbox`);
      json(res, r.status, await r.json());
    } catch (err) {
      json(res, 502, { error: String(err.message || err) });
    }
    return;
  }

  if (req.method === "POST" && url.pathname === "/reset") {
    stopStream();
    counters.auctions = 0;
    counters.forwarded = 0;
    counters.dropped = 0;
    counters.errors = 0;
    counters.byBeat = Object.fromEntries(Object.keys(SCENES).map((k) => [k, 0]));
    counters.byOutcome = {};
    seedOutcomeZeros();
    counters.byGate = {};
    counters.byPop = {};
    counters.byInv = {};
    counters.byRule = {};
    hist.buckets.fill(0);
    hist.sum = 0;
    hist.count = 0;
    latRing.length = 0;
    windowTs.length = 0;
    inFlight = 0;
    json(res, 200, snapshot());
    return;
  }

  if (req.method === "POST" && url.pathname === "/run") {
    let beat = url.searchParams.get("beat");
    let hopOnly = false;
    if (!beat) {
      try {
        const body = JSON.parse(await readBody(req));
        beat = body.beat;
        hopOnly = Boolean(body.hopOnly);
      } catch {
        beat = "clean";
      }
    }
    stopStream();
    try {
      const result = await runAuction(beat, { verbose: true, hopOnly });
      json(res, 200, result);
    } catch (err) {
      const status = err.status || 500;
      broadcast({ actor: "ssp", kind: "error", text: String(err.message || err) });
      json(res, status, { error: String(err.message || err) });
    }
    return;
  }

  if (req.method === "POST" && url.pathname === "/stream") {
    let body = {};
    try {
      body = JSON.parse((await readBody(req)) || "{}");
    } catch {
      body = {};
    }
    if (!ready) {
      json(res, 503, { error: "rtblint not ready" });
      return;
    }
    const started = startStream(body);
    json(res, 200, started);
    return;
  }

  if (req.method === "POST" && url.pathname === "/stream/stop") {
    stopStream();
    json(res, 200, { on: false });
    return;
  }

  if (req.method === "GET" && (url.pathname === "/" || url.pathname === "/index.html")) {
    const html = readFileSync(join(PUBLIC, "index.html"));
    res.writeHead(200, { "content-type": "text/html; charset=utf-8" });
    res.end(html);
    return;
  }

  json(res, 404, { error: "not found" });
});

function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    req.on("error", reject);
  });
}

function json(res, status, body) {
  res.writeHead(status, { "content-type": "application/json" });
  res.end(JSON.stringify(body));
}

setInterval(() => {
  if (clients.size) broadcast({ type: "tick", ...snapshot() });
}, 250).unref();

waitForRtblint()
  .then(() => {
    server.listen(PORT, "0.0.0.0", () => {
      console.log(`ssp console on ${PORT}`);
    });
  })
  .catch((err) => {
    console.error(err);
    process.exit(1);
  });
