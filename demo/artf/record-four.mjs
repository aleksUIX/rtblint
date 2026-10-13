#!/usr/bin/env node
// Episode 4: Fan-in. Follows SCREENPLAY-FOUR.md. Lab hop graph on :8080,
// Grafana :3000 (ARTF lab). Not the five-button teaching console.

import { mkdirSync, readdirSync, renameSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { chromium } from "playwright";
import { wrapReel } from "./wrap-cards.mjs";

const root = dirname(fileURLToPath(import.meta.url));
const outDir = join(root, "record-out");
const mp4 = join(root, "artf-sim-four.mp4");
const labUrl = process.env.ARTF_LAB_URL || "http://localhost:8080/lab/";
const grafanaUrl =
  process.env.ARTF_GRAFANA_URL ||
  "http://localhost:3000/d/artf-lab/artf-lab?orgId=1&refresh=1s&kiosk&from=now-5m&to=now";
const promUrl = process.env.ARTF_PROM_URL || "http://localhost:9090";
const adUrl = new URL("/ad?beat=clean&format=json", labUrl).href;

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function waitHose(min, ms) {
  const deadline = Date.now() + ms;
  const q = encodeURIComponent("sum(rate(artf_auctions_total[1m]))");
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
    bottom: where === "grafana" ? "48px" : "36px",
    top: where === "top" ? "18px" : "",
  });
}

async function say(lines, where = "top") {
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

async function focusNode(id) {
  await page.evaluate((n) => window.artfFocusNode?.(n), id);
}

async function focusOutcome(i) {
  await page.evaluate((n) => window.artfFocusOutcome?.(n), i);
}

await page.goto(labUrl, { waitUntil: "domcontentloaded", timeout: 60_000 });
await page.waitForSelector("button[data-beat=clean]");
await page.addStyleTag({
  content: `
    button[data-beat="suppress"], button[data-beat="poison"], button[data-beat="dirty"], #ssai-btn { display: none !important; }
    .node.focus rect { stroke: #fff !important; stroke-width: 2.6 !important; }
    #outcomes tr.focus td { background: #2a2416; }
  `,
});
await page.evaluate(() => {
  window.artfFocusNode = (id) => {
    document.querySelectorAll(".node.focus").forEach((n) => n.classList.remove("focus"));
    document.getElementById("n-" + id)?.classList.add("focus");
  };
  window.artfFocusOutcome = (i) => {
    const rows = [...document.querySelectorAll("#outcomes tr")];
    rows.forEach((tr, idx) => tr.classList.toggle("focus", idx === i));
    rows[i]?.scrollIntoView({ block: "center" });
  };
});

await say([
  "Same mid-roll as the last three films. Different picture.",
  "Episode 3 was one envelope, one agent. Production is not that.",
  "Production is several containers answering the same envelope in parallel, then one apply onto one ticket, then a second envelope after each DSP bids. The gold nodes are GetMutations.",
]);

await page.locator("button[data-beat=clean]").click();
await page.waitForFunction(
  () => (document.getElementById("outcomes")?.innerText || "").includes("ACTIVATE_SEGMENTS"),
  null,
  { timeout: 25_000 },
);
await page.evaluate(() => {
  const rows = [...document.querySelectorAll("#outcomes tr")];
  if (rows.some((r) => /quality/i.test(r.textContent || ""))) {
    document.getElementById("n-quality")?.classList.add("on");
  }
});

await focusNode("ssp");
await say([
  "The host fills one envelope: publisher-bid-request lifecycle, one tmax, one applicable_intents list that covers the jobs these containers are for. Then it fans that envelope out.",
]);
await focusNode("audience");
await say([
  "Audience does not wait for curator. Metrics does not wait for quality. They answer independently because mutations are independently acceptable.",
]);
await focusNode("curator");
await say([
  "The auction clock is one budget for the set, not a budget per vendor. If you serialise these calls you have already lost the reason the working group put them in-process.",
]);

await page.evaluate(() => document.getElementById("outcomes")?.scrollIntoView({ block: "center" }));
await focusOutcome(0);
await say([
  "This table is the whole contract. Each row is an intent, the agent that proposed it, and the OpenRTB field the host wrote.",
]);
await focusOutcome(1);
await focusOutcome(2);
await say([
  "The ticket on the right is BidRequest-prime. The ticket on the left is what the publisher sent. Agents proposed. They never held the bid.",
]);
await focusOutcome(3);
await focusOutcome(5);
await say([
  "If two specialists touch different paths, the merge is the union. Fan-in is not a mash-up. It is apply.",
]);

await page.evaluate(() => document.getElementById("graph")?.scrollIntoView({ block: "center" }));
await focusNode("dsp-alpha");
await say([
  "Buyers bid on what you forwarded. Then each DSP runs the same RPC at the later stage, with a different intent list, against its own bid.",
]);
await focusNode("shader-alpha");
await say([
  "Isolation is the design: a shader beside alpha must not see beta's price, and it must not see the request-side agents.",
]);
await focusNode("shader-beta");
await say([
  "You get shade without a second bid protocol and without a shared god-agent that can observe every seat. The host still applies BID_SHADE. The auction still clears on OpenRTB.",
]);

await page.locator("#play-btn").click();
await page.waitForFunction(
  () => {
    const summary = document.getElementById("summary")?.innerText || "";
    const events = document.getElementById("events")?.innerText || "";
    return /vast played/i.test(summary) || /\bimp\b/i.test(events) || /\bstart\b/i.test(events);
  },
  null,
  { timeout: 25_000 },
).catch(() => {});
await focusNode("cdn");
await say([
  "ARTF stopped at apply. What lights now is the rest of CTV: VAST InLine, a MediaFile from a CDN stub, win and billing notices, quartile pixels.",
]);
await focusNode("tracker");
await say([
  "Useful so you can see where the framework ends. Not a claim that the lab is a player, an encoder, or a measurement company. If a room asks where delivery lives, this is the answer: after the hop, on purpose.",
]);

let hose = setInterval(() => {
  fetch(adUrl).catch(() => {});
}, 80);

await say([
  "At volume you watch specialists, not a blended hop. Audience p95 is not curator p95.",
]);
await waitHose(0.2, 12_000);
await say([
  "Grafana is the unsampled view of the same hop, split by agent.",
]);

let grafanaOk = false;
try {
  const probe = await fetch(grafanaUrl, { signal: AbortSignal.timeout(2500) });
  grafanaOk = probe.ok;
} catch {
  grafanaOk = false;
}
await hideCard();
try {
  if (grafanaOk) {
    await page.goto(grafanaUrl, { waitUntil: "domcontentloaded" });
    await page.getByText("Auctions per second", { exact: false }).first().waitFor({ timeout: 20_000 }).catch(() => {});
    await say(
      [
        "A shader that blows tmax on alpha does not excuse a stall on the request fan-in. Applied vs skipped is host policy under load.",
        "This is a laptop loop of the same hop, not a colo. The new number is fan-in: five request RPCs and two shade RPCs inside one auction id.",
      ],
      "grafana",
    );
  }
} finally {
  clearInterval(hose);
}
await hideCard();
await page.goto(labUrl, { waitUntil: "domcontentloaded", timeout: 60_000 });
await page.waitForSelector("button[data-beat=clean]");
await say([
  "That is ARTF as a host actually runs it: several containers on the way out, an isolated shader per buyer on the way back, one apply onto OpenRTB, and a hard stop before VAST. Package specialists. Keep isolation. Keep the ticket. Independent simulation. Not an IAB product.",
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
  renameSync(webm, join(root, "artf-sim-four.webm"));
  console.error("ffmpeg failed; left webm");
  process.exit(1);
}

await wrapReel(mp4, { episode: "four" });
console.log(mp4);
for (const name of readdirSync(outDir)) {
  const p = join(outDir, name);
  try {
    unlinkSync(p);
  } catch {
    // leftover dirs from old burn scripts
  }
}
