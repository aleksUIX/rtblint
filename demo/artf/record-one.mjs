#!/usr/bin/env node
// Episode 1 picture. Each body beat is held to script-one.mjs seconds so
// the reel, prompter, and VO-ONE.md share one clock. wrap-cards adds the
// 13s intro and 15s outro. Compose must be up on :8080 and Grafana :3001.

import { copyFileSync, mkdirSync, readdirSync, renameSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { chromium } from "playwright";
import { wrapReel } from "./wrap-cards.mjs";
import { bodyBeats, INTRO_S, OUTRO_S, totalSeconds } from "./script-one.mjs";

const root = dirname(fileURLToPath(import.meta.url));
const outDir = join(root, "record-out");
const mp4 = join(root, "artf-sim-one.mp4");
const consoleUrl = process.env.ARTF_DEMO_URL || "http://localhost:8080/?ep=1";
const grafanaUrl =
  process.env.ARTF_GRAFANA_URL ||
  "http://localhost:3001/d/artf-sim-hop?orgId=1&refresh=1s&kiosk&from=now-1m&to=now";
const promUrl = process.env.ARTF_PROM_URL || "http://localhost:9092";
const bodyS = totalSeconds() - INTRO_S - OUTRO_S;

const sleep = (ms) => new Promise((r) => setTimeout(r, Math.max(0, ms)));

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

{
  const up = await fetch(consoleUrl).catch(() => null);
  if (!up || !up.ok) {
    console.error("console not up at", consoleUrl);
    process.exit(1);
  }
}

await fetch(new URL("/stream/stop", consoleUrl), { method: "POST" }).catch(() => {});
await fetch(new URL("/reset", consoleUrl), { method: "POST" }).catch(() => {});
{
  const runUrl = new URL("/run", consoleUrl);
  for (let i = 0; i < 2; i += 1) {
    await fetch(runUrl, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({ beat: "clean", hopOnly: true }),
    }).catch(() => {});
  }
}
await fetch(new URL("/reset", consoleUrl), { method: "POST" }).catch(() => {});

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  viewport: { width: 1920, height: 1080 },
  deviceScaleFactor: 1,
  recordVideo: { dir: outDir, size: { width: 1920, height: 1080 } },
});
const page = await context.newPage();
const recT0 = Date.now();

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

async function fill(seconds, work) {
  const t0 = Date.now();
  if (work) await work();
  const left = seconds * 1000 - (Date.now() - t0);
  if (left < -250) {
    console.warn(`overran ${(-left / 1000).toFixed(1)}s`);
  }
  await sleep(left);
}

async function beat(id, needle) {
  await page.locator(`[data-beat="${id}"]`).click();
  try {
    await page.waitForFunction(
      (text) => {
        const punch = document.getElementById("punch")?.textContent || "";
        const busy = [...document.querySelectorAll(".beat")].some((b) => b.disabled);
        return !busy && punch.includes(text);
      },
      needle,
      { timeout: 10_000 },
    );
  } catch {
    const punch = await page.locator("#punch").textContent();
    throw new Error(`beat ${id} timeout. punch=${JSON.stringify(punch)}`);
  }
}

async function closeMap() {
  await page.evaluate(() => {
    const map = document.getElementById("hop-map");
    map?.classList.remove("on");
    map?.setAttribute("aria-hidden", "true");
    document.querySelector("[data-beat=map]")?.classList.remove("on");
  });
}

await page.goto(consoleUrl, { waitUntil: "domcontentloaded", timeout: 60_000 });
await page.waitForSelector(".beat");
await page.waitForFunction(() => (document.getElementById("t-deals")?.textContent || "").includes("deal-premium"));
const prefixSec = (Date.now() - recT0) / 1000;
console.log(`prefix ${prefixSec.toFixed(2)}s (crop before clock)`);

async function stampStats(snap) {
  if (!snap) return;
  await page.evaluate((s) => {
    const set = (id, v) => {
      const el = document.getElementById(id);
      if (el) el.textContent = v;
    };
    set("s-auctions", s.auctions);
    set("s-qps", s.qps);
    set("s-fwd", s.forwarded);
    set("s-drop", s.dropped);
    set("s-p99", (s.p99_ms || 0).toFixed(1) + " ms");
    set("s-fly", s.inFlight);
  }, snap);
}

async function returnConsole() {
  const origin = new URL(consoleUrl).origin;
  const snap = await fetch(`${origin}/stats`).then((r) => r.json()).catch(() => null);
  await page.goto(consoleUrl, { waitUntil: "domcontentloaded", timeout: 8_000 }).catch(() => {});
  await page.waitForSelector("#s-auctions", { timeout: 3_000 }).catch(() => {});
  await stampStats(snap);
  console.log(`back auctions=${snap?.auctions ?? "?"} qps=${snap?.qps ?? "?"}`);
}

let secondsBudgetMs = 12_000;
let backJob = null;
const beats = bodyBeats();
for (let i = 0; i < beats.length; i += 1) {
  const beatSpec = beats[i];
  const next = beats[i + 1];
  const { seconds, cue, text, silent, where } = beatSpec;
  secondsBudgetMs = Math.max(800, Math.round(seconds * 1000) - 200);
  await fill(seconds, async () => {
    if (cue === "who") {
      await hideCard();
      return;
    }
    if (cue === "map-open") {
      await page.locator("[data-beat=map]").click();
      await page.waitForSelector("#hop-map.on");
      await card(text, where);
      return;
    }
    if (cue === "map-last") {
      await card(text, where);
      return;
    }
    if (cue === "hop-clean") {
      await hideCard();
      await beat("clean", "auction the agent actually wrote");
      await page.waitForFunction(() => document.getElementById("tv")?.dataset.mode === "ad", null, {
        timeout: Math.min(2500, secondsBudgetMs),
      }).catch(() => {});
      return;
    }
    if (cue === "hop-suppress") {
      await hideCard();
      await beat("suppress", "deal-standard is off the wire");
      return;
    }
    if (cue === "hop-shade") {
      await hideCard();
      await beat("shade", "The bid is still a bid");
      return;
    }
    if (cue === "mixer-start") {
      await hideCard();
      await page.locator("#qps").selectOption("5000");
      await page.locator("#secs").selectOption("180");
      await page.locator("#go").click();
      await page.waitForFunction(() => Number(document.getElementById("s-qps")?.textContent || 0) > 400, null, {
        timeout: secondsBudgetMs,
      }).catch(() => {});
      return;
    }
    if (cue === "hose") {
      await hideCard();
      await waitHose(800, secondsBudgetMs);
      return;
    }
    if (cue === "grafana") {
      await hideCard();
      try {
        const probe = await fetch(grafanaUrl, { signal: AbortSignal.timeout(1500) });
        if (!probe.ok) return;
      } catch {
        return;
      }
      await page.goto(grafanaUrl, { waitUntil: "domcontentloaded", timeout: secondsBudgetMs }).catch(() => {});
      await page.getByText("Hop p99", { exact: false }).first().waitFor({ timeout: 2_500 }).catch(() => {});
      return;
    }
    if (cue === "back") {
      await hideCard();
      await closeMap();
      if (backJob) {
        await backJob;
        backJob = null;
      } else {
        await returnConsole();
      }
      return;
    }
    if (cue === "say" || !cue) {
      if (silent) await hideCard();
      else await card(text, where || "console");
      if (next?.cue === "back") {
        backJob = (async () => {
          await sleep(Math.max(0, seconds * 1000 - 6500));
          await returnConsole();
        })();
      }
    }
  });
  if (cue === "map-last") await closeMap();
}

await fetch(new URL("/stream/stop", consoleUrl), { method: "POST" }).catch(() => {});

const video = page.video();
await page.close();
await context.close();
await browser.close();

const webm = await video.path();
const trimmed = join(outDir, "body.mp4");
console.log(`trim webm -ss ${prefixSec.toFixed(3)} -t ${bodyS.toFixed(3)}`);
const ffmpeg = spawnSync(
  "ffmpeg",
  [
    "-y",
    "-i",
    webm,
    "-ss",
    prefixSec.toFixed(3),
    "-t",
    bodyS.toFixed(3),
    "-an",
    "-c:v",
    "libx264",
    "-profile:v",
    "high",
    "-level",
    "4.0",
    "-pix_fmt",
    "yuv420p",
    "-r",
    "25",
    "-video_track_timescale",
    "12800",
    "-movflags",
    "+faststart",
    trimmed,
  ],
  { stdio: "inherit" },
);
if (ffmpeg.status !== 0) {
  renameSync(webm, join(root, "artf-sim-one.webm"));
  console.error("ffmpeg failed; left webm");
  process.exit(1);
}

await wrapReel(trimmed, { episode: "one" });
copyFileSync(trimmed, mp4);

const want = totalSeconds();
const got = Number(
  spawnSync("ffprobe", ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", mp4], {
    encoding: "utf8",
  }).stdout.trim(),
);
const drift = Math.abs(got - want);
console.log(mp4);
console.log(`clock ${want.toFixed(1)}s  reel ${got.toFixed(1)}s  drift ${drift.toFixed(1)}s  (intro ${INTRO_S}s + outro ${OUTRO_S}s)`);
if (drift > 1.5) {
  console.error("reel is off the prompter clock");
  process.exit(1);
}
for (const name of readdirSync(outDir)) {
  const p = join(outDir, name);
  try {
    unlinkSync(p);
  } catch {
    // leftover dirs from old burn scripts
  }
}
