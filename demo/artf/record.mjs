#!/usr/bin/env node
// Record the three-beat compose console to MP4.
// Compose must already be up on :8080.

import { mkdirSync, readdirSync, renameSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { chromium } from "playwright";

const root = dirname(fileURLToPath(import.meta.url));
const outDir = join(root, "record-out");
const mp4 = join(root, "artf-sim-demo.mp4");
const url = process.env.ARTF_DEMO_URL || "http://localhost:8080/";

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
mkdirSync(outDir, { recursive: true });

const browser = await chromium.launch({ headless: true, slowMo: 180 });
const context = await browser.newContext({
  viewport: { width: 1440, height: 900 },
  deviceScaleFactor: 2,
  recordVideo: { dir: outDir, size: { width: 1440, height: 900 } },
});
const page = await context.newPage();

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

await page.goto(url, { waitUntil: "networkidle" });
await page.waitForSelector(".beat");
await sleep(2200);

await beat("illegal", "The auction was never asked");
await sleep(6500);

await beat("breaking", "still-valid OpenRTB");
await sleep(5500);

await beat("clean", "auction the agent actually wrote");
await page.waitForFunction(
  () => (document.getElementById("inbox")?.textContent || "").includes("receipt"),
  null,
  { timeout: 10_000 },
);
await sleep(6500);

const video = page.video();
await page.close();
await context.close();
await browser.close();

const webm = await video.path();
const ffmpeg = spawnSync(
  "ffmpeg",
  ["-y", "-i", webm, "-c:v", "libx264", "-pix_fmt", "yuv420p", "-movflags", "+faststart", mp4],
  { stdio: "inherit" },
);
if (ffmpeg.status !== 0) {
  renameSync(webm, join(root, "artf-sim-demo.webm"));
  console.error("ffmpeg failed; left webm");
  process.exit(1);
}

console.log(mp4);
for (const name of readdirSync(outDir)) {
  unlinkSync(join(outDir, name));
}
