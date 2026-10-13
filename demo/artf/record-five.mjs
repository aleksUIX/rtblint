#!/usr/bin/env node
// Five-minute pitch reel. Follows SCREENPLAY.md: living-room open, five named
// hops, mixer, Grafana. Compose must already be up on :8080 and Grafana :3001.

import { mkdirSync, readdirSync, renameSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { chromium } from "playwright";
import { wrapReel } from "./wrap-cards.mjs";

const root = dirname(fileURLToPath(import.meta.url));
const outDir = join(root, "record-out");
const mp4 = join(root, "artf-sim-five.mp4");
const consoleUrl = process.env.ARTF_DEMO_URL || "http://localhost:8080/";
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

await fetch(new URL("/stream/stop", consoleUrl), { method: "POST" }).catch(() => {});
await fetch(new URL("/reset", consoleUrl), { method: "POST" }).catch(() => {});

await page.goto(consoleUrl, { waitUntil: "domcontentloaded", timeout: 60_000 });
await page.waitForSelector(".beat");
await page.waitForFunction(() => (document.getElementById("t-deals")?.textContent || "").includes("deal-premium"));

await say([
  "This is the auction a CTV app is about to run: a mid-roll, two private deals, a floor already on the impression.",
  "OpenRTB is the object every SSP and DSP already speaks. ARTF lets a container beside the exchange rewrite that object before any buyer sees it.",
  "What v1.0 never asks is whether the auction is still valid OpenRTB afterwards. That hop, slowed down, is what we are about to walk.",
]);

await say([
  "If you sit at an SSP, this is the nightmare in slow motion: an agent that speaks fluent gRPC and still cannot point at the impression you handed it.",
]);
await beat("illegal", "The auction was never asked");
await page.waitForFunction(() => document.getElementById("tv")?.dataset.mode === "unsold", null, { timeout: 10_000 });
await say([
  "Wrong lifecycle, wrong ids, paths that are not even on this ticket. One of the writes would have landed. We still drop the whole set.",
  "The framework accepted the RPC. The auction was never asked, so we never called a DSP, and the living room never left the show.",
]);

await say([
  "From the exchange engineer seat, the next failure is quieter. The mutation can be well-formed ARTF and still wreck the auction.",
]);
await beat("breaking", "still-valid OpenRTB");
await page.waitForFunction(() => document.getElementById("tv")?.dataset.mode === "poison", null, { timeout: 10_000 });
await say([
  "ADD_METRICS is allowed, the path resolves, the payload member matches. After apply, OpenRTB wanted a number and it got a string.",
  "Well-formed mutation and still-valid OpenRTB are different questions. If you forward because the RPC succeeded, you just shipped a broken bid request.",
]);

await say([
  "This is the hop a buyer actually feels. Watch the ticket, not the dummy, while the patch lands.",
]);
await beat("clean", "auction the agent actually wrote");
await page.waitForFunction(() => document.getElementById("tv")?.dataset.mode === "ad", null, { timeout: 10_000 });
await say([
  "The DSP did not bid on the publisher's original ticket. It bid on the one the agent wrote: extra segments, a new deal, a higher floor, a numeric viewability.",
  "The dummy on the glass is what that bid is holding. It is not VAST, and ARTF never gets as far as delivery.",
]);

await say([
  "From the publisher seat, curation is allowed to take a deal off the auction. That can be the product. It can also be the wrong id.",
]);
await beat("suppress", "deal-standard is off the wire");
await say([
  "deal-standard just left the wire, and the DSP never saw it. The path still has to resolve against this auction, and the result still has to be OpenRTB, or you find out at reporting.",
]);

await say([
  "ARTF is not only a request-side curator. After the DSP answers, a container can shade the price.",
]);
await beat("shade", "The bid is still a bid");
await say([
  "$18.40 comes in, $12.88 goes out. The creative did not change. The bid object did.",
  "The first scene tried to shade at the wrong lifecycle and bounced. This is the legal stage, and the number still has to remain a bid.",
]);

await say([
  "One slot is the story. A bidstream is the job. This is a laptop running the same binary, not a colo POP.",
]);
await page.locator("#qps").selectOption("5000");
await page.locator("#secs").selectOption("180");
await page.locator("#go").click();
await page.waitForFunction(() => Number(document.getElementById("s-qps")?.textContent || 0) > 400, null, {
  timeout: 20_000,
});
await say([
  "Five thousand auctions a second through real gRPC, mostly clean, with some planted dirt and a little legal curation.",
  "Green on the barcode is a forward. Red is a drop. The log is sampled so you can still read it.",
]);
await waitHose(2000, 20_000);
await say([
  "Grafana is the unsampled view of the same hop. That is where you actually operate this check.",
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
      "This is the same hop without sampling. The auctions-per-second figure is the mixer, as a raw count, not a claim about anyone's production.",
      "Drop rate should sit near ten percent, because that is the dirt we planted. Suppress and shade still forward; those are legal curation.",
      "Mutations p99 should sit far under the 150 millisecond tmax. Headroom is whatever is left for the rest of the hop.",
      "Shed has to stay at zero. If it ticks up, the limiter is refusing work rather than lying about latency.",
      "The stack is the mix by beat. The heatmap should stay in the first milliseconds; a ridge walking right is queueing.",
      "On the drop gate, mutations means the agent never pointed at this auction. Applied means the patch wrote illegal OpenRTB, so it must not fan out.",
      "The rule list is what on-call reads when the rate turns. You call ValidateArtfMutations with apply true before you forward, and before you accept a shaded bid.",
    ],
    "grafana",
  );
}

await fetch(new URL("/stream/stop", consoleUrl), { method: "POST" }).catch(() => {});
await sleep(800);
await hideCard();
await page.goto(consoleUrl, { waitUntil: "domcontentloaded", timeout: 60_000 });
await page.waitForSelector(".beat");
await say([
  "Those are the three passes: envelope, mutation, and applied. VASTlint was the creative hop. This is the bid hop, an independent implementation, not an IAB product.",
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
  renameSync(webm, join(root, "artf-sim-five.webm"));
  console.error("ffmpeg failed; left webm");
  process.exit(1);
}

await wrapReel(mp4);
console.log(mp4);
for (const name of readdirSync(outDir)) {
  const p = join(outDir, name);
  try {
    unlinkSync(p);
  } catch {
    // leftover dirs from old burn scripts
  }
}
