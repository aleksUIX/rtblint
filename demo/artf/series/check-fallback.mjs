import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { output } from "./build.mjs";

const docker = args => {
  const r = spawnSync("docker", args, { encoding: "utf8" });
  if (r.status) throw new Error(r.stderr || r.stdout);
  return r.stdout;
};
const request = async () => {
  const r = await fetch("http://localhost:8081/ad?beat=clean&format=json", { signal: AbortSignal.timeout(15000) });
  assert(r.ok, `Auction HTTP ${r.status}: ${await r.clone().text()}`);
  const doc = await r.json();
  assert(doc.auction, "Missing auction JSON");
  return doc;
};
const counters = () => {
  const raw = docker(["exec", "artf-recording-gateway-1", "wget", "-qO-", "http://ssp:8080/metrics"]);
  const count = result => Number(raw.match(new RegExp(`^artf_mutations_total\\{result="${result}"\\} (\\d+)`, "m"))?.[1] || 0);
  return { proposed: count("proposed"), applied: count("applied") };
};
await request(); // Warm reused connections before the measured clean auction.
const before = counters();
const initial = await request();
const after = counters();
assert.equal(initial.auction.winner.seat, "dsp-beta");
assert.equal(after.proposed - before.proposed, 6);
assert.equal(after.applied - before.applied, 6);

const agents = ["audience", "curator", "metrics", "quality", "cids"].map(role => `artf-recording-agent-${role}-1`);
let missing;
try {
  docker(["pause", ...agents]);
  missing = await request();
  assert.equal(missing.hops.filter(h => h.stage === "request-artf" && h.error).length, 5);
  assert.equal(missing.auction.applied.length, 0);
  assert.equal(missing.auction.bid_request.imp[0].pmp.deals.find(d => d.id === "deal-premium").bidfloor, 12);
  assert.equal(missing.auction.winner.seat, "dsp-beta");
} finally {
  docker(["unpause", ...agents]);
}
let shade;
try {
  docker(["pause", "artf-recording-agent-shader-beta-1"]);
  shade = await request();
  assert(shade.hops.some(h => h.name === "shader-dsp-beta" && h.error));
  assert.equal(shade.auction.winner.seat, "dsp-beta");
  assert.equal(shade.auction.winner.price, 18.5);
} finally {
  docker(["unpause", "artf-recording-agent-shader-beta-1"]);
}
const recovered = await request();
assert(!recovered.hops.some(h => h.error));
assert.equal(recovered.auction.winner.price, 15.725);
writeFileSync(join(output, "episode-3.fallback.verification.json"), JSON.stringify({ checkedAt: new Date().toISOString(), sameScopeCleanAuction: { before, after, proposedDelta: 6, appliedDelta: 6 }, allRequestAgentsMissing: { failedHops: 5, applied: 0, preservedPremiumFloor: 12, winner: missing.auction.winner.seat }, betaShaderMissing: { winner: shade.auction.winner.seat, preservedOriginalPrice: 18.5 }, recovered: { errors: 0, winnerPrice: 15.725 }, method: "Paused the actual local containers, verified the real host response, then restored every container in finally blocks." }, null, 2) + "\n");
console.log("Real missing-agent and failed-shader fallbacks passed; clean SSP counters add 6 proposed and 6 applied. All agents restored.");
