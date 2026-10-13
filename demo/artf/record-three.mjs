#!/usr/bin/env node
// Episode 3: The envelope. Follows SCREENPLAY-THREE.md. Compose must already
// be up on :8080 and Grafana :3001. Console is ?ep=3 (production mix).

import { mkdirSync, readdirSync, renameSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { chromium } from "playwright";
import { wrapReel } from "./wrap-cards.mjs";

const root = dirname(fileURLToPath(import.meta.url));
const outDir = join(root, "record-out");
const mp4 = join(root, "artf-sim-three.mp4");
const consoleUrl = process.env.ARTF_DEMO_URL || "http://localhost:8080/?ep=3";
const grafanaUrl =
  process.env.ARTF_GRAFANA_URL ||
  "http://localhost:3001/d/artf-sim?orgId=1&refresh=1s&kiosk&from=now-1m&to=now";
const promUrl = process.env.ARTF_PROM_URL || "http://localhost:9092";

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitHose(min, ms) {
  const deadline = Date.now() + ms;
  const q = encodeURIComponent("sum(rate(artf_sim_auctions_total[10s]))");
  while (Date.now() < deadline) {
    try {
      const d = await (await fetch(`${promUrl}/api/v1/query?query=${q}`)).json();
      const v = Number(d?.data?.result?.[0]?.value?.[1] || 0);
      if (v >= min) return v;
    } catch {
      // Prometheus not up yet
    }
    await sleep(400);
  }
  return 0;
}
mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  viewport: { width: 1920, height: 1080 },
  deviceScaleFactor: 1,
  recordVideo: { dir: outDir, size: { width: 1920, height: 1080 } },
});
const page = await context.newPage();

async function hideCard() {
  await page.evaluate(() => {
    document.getElementById("pitch-card")?.remove();
  });
}

async function card(line, where) {
  await page.evaluate(({ line, bottom, top }) => {
    let el = document.getElementById("pitch-card");
    if (!el) {
      el = document.createElement("div");
      el.id = "pitch-card";
      document.body.appendChild(el);
    }
    el.style.cssText =
      "position:fixed;left:50%;" +
      (top ? "top:" + top + ";bottom:auto;" : "bottom:" + bottom + ";") +
      "transform:translateX(-50%);" +
      "z-index:2147483647;max-width:min(74vw,960px);padding:10px 22px 12px;" +
      "background:#000;color:#fff;text-align:center;" +
      "font:500 22px/1.38 Helvetica,Arial,sans-serif;pointer-events:none;";
    el.textContent = line;
  }, {
    line,
    bottom: where === "grafana" ? "48px" : "292px",
    top: where === "grafana-top" ? "18px" : "",
  });
}

async function say(lines, where = "console") {
  for (const item of lines) {
    const line = Array.isArray(item) ? item[0] : item;
    const ms = Array.isArray(item) ? item[1] : readMs(line);
    if (!line) await hideCard();
    else await card(line, where);
    await sleep(ms);
  }
}

function readMs(line) {
  const words = line.trim().split(/\s+/).filter(Boolean).length;
  return Math.min(17000, Math.max(8500, Math.round(words * 400 + 1800)));
}

async function beat(id, needle) {
  await page.locator(`[data-beat="${id}"]`).click();
  await page.waitForFunction(
    (text) => {
      const punch = document.getElementById("punch")?.textContent || "";
      const busy = [...document.querySelectorAll(".beat")].some((b) => b.disabled);
      return !busy && punch.includes(text);
    },
    needle,
    { timeout: 25_000 },
  );
}

async function envFocus(key) {
  await page.evaluate((k) => window.artfEnvFocus?.(k), key);
}

async function mutFocus(index) {
  await page.evaluate((i) => window.artfMutFocus?.(i), index);
}

await fetch(new URL("/stream/stop", consoleUrl), { method: "POST" }).catch(() => {});
await fetch(new URL("/reset", consoleUrl), { method: "POST" }).catch(() => {});

await page.goto(consoleUrl, { waitUntil: "domcontentloaded", timeout: 60_000 });
await page.waitForSelector("[data-beat=envelope]");
await page.waitForFunction(() => (document.getElementById("t-deals")?.textContent || "").includes("deal-premium"));
await page.waitForFunction(() => document.getElementById("env-lifecycle")?.textContent.includes("PUBLISHER"));

await say([
  "Episode 1 was the stack: a container inside the host, a patch on OpenRTB, a DSP bidding on what you forwarded.",
  "This is the call. ARTF does not invent a new bid object.",
  "It wraps the one you already speak in an envelope, hands that envelope to an agent over gRPC, and takes back mutations. The host stays the host. The envelope is how.",
]);

await page.locator("[data-beat=envelope]").click();
await envFocus("lifecycle");
await say([
  "This is RTBRequest. Lifecycle names the stage you are in, so the agent knows which intents are meaningful. It does not inject the matching OpenRTB member.",
]);
await envFocus("attached");
await say([
  "You still attach bid_request on a publisher hop, and you still attach bid_response later when you actually have a bid.",
]);
await envFocus("id");
await say([
  "The id is the extension-point id, for correlation under concurrency. It is not the bid request id.",
]);
await envFocus("tmax");
await say([
  "tmax is the SLA of the hop. One hundred fifty milliseconds, inside the auction clock.",
]);
await envFocus("originator");
await say([
  "Originator is who is calling. TYPE_EXCHANGE on this seat.",
]);
await envFocus("intents");
await say([
  "applicable_intents is the host's vocabulary for this call: what you are willing to evaluate, not what the agent wishes you would.",
  "The agent never gets a copy of the bidstream to take elsewhere. It gets this envelope, inside your infra, on your clock.",
]);

await page.locator("[data-beat=offer]").click();
await say([
  "The same container can sit beside an SSP and beside a DSP. What changes is not the agent binary. What changes is this list.",
  "On a publisher hop you offer deals, segments, floors, metrics: the jobs that have always been tight on time. You do not offer BID_SHADE, because there is no bid yet.",
  "The spec is built this way so you can package the agent once and keep policy on the host. If the agent returns an intent you did not offer, that mutation is dead on arrival. That is how an exchange stays an exchange.",
]);

await say([
  "A mutation is not a blob of JSON to merge. It is an intent, an operation, a semantic path, and a typed payload. Watch the ticket, not the dummy.",
]);
await beat("policy", "Independently acceptable means the host still chooses");
await page.waitForFunction(() => document.getElementById("tv")?.dataset.mode === "ad", null, { timeout: 10_000 });

await mutFocus(0);
await say(["/user/data/segment. ACTIVATE_SEGMENTS. seg-sports stays. seg-cord-cutter and seg-premium-viewer arrive."]);
await mutFocus(2);
await say(["/imp/imp-1/pmp/deals/deal-premium. ADJUST_DEAL_FLOOR. Twelve dollars becomes fourteen fifty."]);
await mutFocus(1);
await say(["/imp/imp-1. ACTIVATE_DEALS. deal-curated was not on the wire. It is now."]);
await mutFocus(3);
await say(["/imp/imp-1. ADD_METRICS. viewability 0.82, a number."]);
await mutFocus(4);
await say([
  "ADD_CIDS was not on this hop's list. The host skipped it. Four writes landed. The DSP still bid. That is independently acceptable: the orchestrator still chooses, change by change.",
  "Control never left the exchange. The DSP bid on BidRequest-prime: whatever you actually forwarded, not whatever the agent proposed in full.",
]);

await say([
  "The spec did not invent a second protocol for shading. Same gRPC, same response shape, later in the auction.",
]);
await beat("shade", "The bid is still a bid");
await envFocus("lifecycle");
await say([
  "Lifecycle is now LIFECYCLE_DSP_BID_RESPONSE. The request-side list is gone. applicable_intents is BID_SHADE.",
]);
await envFocus("attached");
await say([
  "You attach bid_response because BID_SHADE addresses a seat and a bid id that only exist once a buyer has answered.",
]);
await mutFocus(0);
await say([
  "The path is /seatbid/dsp-1/bid/bid-ctv. Seat id, bid id. Eighteen forty comes in, twelve eighty-eight lands. Same dummy. One container model. Two moments.",
]);

await say([
  "tmax is not a footnote. Identity, deals, segments, a shade on the way back: those jobs have always been tight on time.",
  "Five thousand auctions a second is a laptop slice of that clock, not a colo.",
]);
await page.locator("#qps").selectOption("5000");
await page.locator("#secs").selectOption("180");
await page.locator("#go").click();
await page.waitForFunction(() => Number(document.getElementById("s-qps")?.textContent || 0) > 400, null, {
  timeout: 20_000,
});
await say([
  "Green on the barcode is a forward. Drop should sit near zero on this mix. The number that matters on this film is whether the envelope budget still holds.",
]);
await waitHose(2000, 20_000);
await say([
  "Grafana is the unsampled view of the same hop.",
]);

let grafanaOk = false;
try {
  const probe = await fetch(grafanaUrl, { signal: AbortSignal.timeout(2500) });
  grafanaOk = probe.ok;
} catch {
  grafanaOk = false;
}
await hideCard();
if (grafanaOk) {
  await page.goto(grafanaUrl, { waitUntil: "domcontentloaded" });
  await page.getByText("Auctions / s", { exact: false }).first().waitFor({ timeout: 20_000 }).catch(() => {});
  await page.waitForFunction(
    () => /\b([3-9]\d{3}|[1-9]\d{4,})\b/.test(document.body.innerText),
    null,
    { timeout: 25_000 },
  ).catch(() => {});
  await say(
    [
      "Mutations p99 has to live far under the 150 millisecond tmax, because this hop sits inside the auction.",
      "Headroom is whatever is left for the rest of the bid. Shed at zero means the limiter is not refusing work to protect that clock.",
      "The stack is still the mix. The number that matters here is not drop rate. It is whether the envelope budget still holds when the hop is concurrent.",
    ],
    "grafana",
  );
}

await fetch(new URL("/stream/stop", consoleUrl), { method: "POST" }).catch(() => {});
await sleep(800);
await hideCard();
await page.goto(consoleUrl, { waitUntil: "domcontentloaded", timeout: 60_000 });
await page.waitForSelector("[data-beat=envelope]");
await say([
  "That is how ARTF is built: an envelope the host fills, a vocabulary the host publishes, paths that name ids in this auction, and mutations the orchestrator may refuse one by one. Package the agent once. Keep policy and timing where the bid already lives. Independent simulation. Not an IAB product.",
]);

const video = page.video();
await page.close();
await context.close();
await browser.close();

const webm = await video.path();
const ffmpeg = spawnSync(
  "ffmpeg",
  [
    "-y",
    "-i",
    webm,
    "-c:v",
    "libx264",
    "-pix_fmt",
    "yuv420p",
    "-movflags",
    "+faststart",
    mp4,
  ],
  { stdio: "inherit" },
);
if (ffmpeg.status !== 0) {
  renameSync(webm, join(root, "artf-sim-three.webm"));
  console.error("ffmpeg failed; left webm");
  process.exit(1);
}

await wrapReel(mp4, { episode: "three" });
console.log(mp4);
for (const name of readdirSync(outDir)) {
  const p = join(outDir, name);
  try {
    unlinkSync(p);
  } catch {
    // leftover dirs from old burn scripts
  }
}
