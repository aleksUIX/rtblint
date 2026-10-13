#!/usr/bin/env node
// Pitch reel. Follows SCREENPLAY.md: three beats, 200/s mixer, Grafana kiosk.
// Compose must already be up on :8080 and Grafana on :3001.

import { mkdirSync, readdirSync, renameSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { chromium } from "playwright";

const root = dirname(fileURLToPath(import.meta.url));
const outDir = join(root, "record-out");
const mp4 = join(root, "artf-sim-pitch.mp4");
const consoleUrl = process.env.ARTF_DEMO_URL || "http://localhost:8080/";
const grafanaUrl =
  process.env.ARTF_GRAFANA_URL ||
  "http://localhost:3001/d/artf-sim?orgId=1&refresh=1s&kiosk&from=now-3m&to=now";

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({
  viewport: { width: 1920, height: 1080 },
  deviceScaleFactor: 1,
  recordVideo: { dir: outDir, size: { width: 1920, height: 1080 } },
});
const page = await context.newPage();

async function card(line) {
  await page.evaluate((line) => {
    let el = document.getElementById("pitch-card");
    if (!el) {
      el = document.createElement("div");
      el.id = "pitch-card";
      el.style.cssText =
        "position:fixed;left:50%;bottom:72px;transform:translateX(-50%);" +
        "z-index:2147483647;max-width:min(68vw,820px);padding:10px 20px 12px;" +
        "background:#000;color:#fff;text-align:center;" +
        "font:500 26px/1.35 Helvetica,Arial,sans-serif;pointer-events:none;";
      document.body.appendChild(el);
    }
    el.textContent = line;
  }, line);
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
    { timeout: 20_000 },
  );
}

await fetch(new URL("/reset", consoleUrl), { method: "POST" }).catch(() => {});

await page.goto(consoleUrl, { waitUntil: "networkidle" });
await page.waitForSelector(".beat");
await card(
  "An agent mutates the bidstream. v1.0 never asks if the auction is still OpenRTB.",
);
await sleep(4000);

await card(
  "Wrong extension-point id, wrong intent, missing paths. The framework accepted the RPC.",
);
await beat("illegal", "The auction was never asked");
await sleep(11000);

await card(
  "ADD_METRICS value \"high\". Well-formed ARTF. After apply, OpenRTB is not.",
);
await beat("breaking", "still-valid OpenRTB");
await sleep(9000);

await card(
  "Segments, deal, floor, numeric metric. Empty findings. The DSP bids on the rewritten ticket.",
);
await beat("clean", "auction the agent actually wrote");
await sleep(7000);

await card(
  "Synthetic mix at 5,000/s. 90% clean, 8% illegal, 2% breaking. Same gRPC hop. Laptop, not a colo POP.",
);
await page.locator("#qps").selectOption("5000");
await page.locator("#secs").selectOption("90");
await page.locator("#go").click();
await page.waitForFunction(() => Number(document.getElementById("s-qps")?.textContent || 0) > 400, null, {
  timeout: 20_000,
});
await sleep(14000);

await page.goto(grafanaUrl, { waitUntil: "domcontentloaded" });
await page.getByText("Auctions / s", { exact: false }).first().waitFor({ timeout: 20_000 }).catch(() => {});
await sleep(2500);
await card(
  "p99 against a 150ms tmax. Drops are the dirty mix. Shed should sit at zero.",
);
await sleep(20000);

await page.goto(consoleUrl, { waitUntil: "networkidle" });
await page.waitForSelector(".beat");
await card(
  "VASTlint was the creative hop. This is the bid hop. Independent implementation. Not an IAB product.",
);
await sleep(8000);

await fetch(new URL("/stream/stop", consoleUrl), { method: "POST" }).catch(() => {});
await sleep(1200);

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
  renameSync(webm, join(root, "artf-sim-pitch.webm"));
  console.error("ffmpeg failed; left webm");
  process.exit(1);
}

console.log(mp4);
for (const name of readdirSync(outDir)) {
  unlinkSync(join(outDir, name));
}
