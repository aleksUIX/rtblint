import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { chromium } from "playwright";
import { episodes, timeline, totalSeconds } from "./episodes.mjs";
import { build, output, clock } from "./build.mjs";
import { action, reset } from "./driver.mjs";
import { encodePicture } from "./media.mjs";
import { finishEpisode } from "./finish.mjs";
import { focusRegion } from "./framing.mjs";

const episode = episodes[Number(process.argv[2])];
if (!episode) throw new Error("Use node series/record.mjs 2 or 3 [--smoke]");
build();
const smoke = process.argv.includes("--smoke");
const work = mkdtempSync(join(tmpdir(), `artf-episode-${episode.number}-`));
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1, ...(smoke ? {} : { recordVideo: { dir: work, size: { width: 1920, height: 1080 } } }) });
const page = await context.newPage();
const errors = [];
page.on("pageerror", e => errors.push(e.message));
page.setDefaultTimeout(25000);
const rows = timeline(episode);
let started, elapsed = 0;
let videoAlignment;
let presentation;
let completed = false;
const actual = [];
const foci = [];
const pending = join(output, `episode-${episode.number}.pending.mp4`);
try {
  await reset(episode);
  await action(page, rows[0], episode);
  started = performance.now();
  mkdirSync(join(output, "qa"), { recursive: true });
  for (const row of rows) {
    const actionStart = performance.now();
    const evidence = row.index !== 0 ? await action(page, row, episode) : undefined;
    if (row.action.type !== "card") foci.push(await focusRegion(page, row, episode));
    const actionSeconds = (performance.now() - actionStart) / 1000;
    if (errors.length) throw new Error(errors.join("; "));
    if (episode.number === 2 && row.action.type !== "card") {
      const layout = await page.evaluate(() => {
        const pane = document.getElementById("flyout").getBoundingClientRect();
        const keys = document.getElementById("fly-keys").getBoundingClientRect();
        return { paneBottom: pane.bottom, textBottom: keys.bottom, viewport: innerHeight };
      });
      if (layout.textBottom > layout.paneBottom - 4 || layout.paneBottom > layout.viewport) throw new Error(`Inspector does not fit beat ${row.index}: ${JSON.stringify(layout)}`);
    }
    if (actionSeconds > row.seconds - 0.2) throw new Error(`Action exceeded beat ${row.index}: ${actionSeconds.toFixed(2)}s for ${row.seconds}s`);
    if (row.action.type === "card") await page.screenshot({ path: join(output, `episode-${episode.number}.${row.action.kind}.png`) });
    if (smoke || ["scene", "lab-view", "lab-play", "lab-traffic"].includes(row.action.type)) {
      await page.screenshot({ path: join(output, "qa", `episode-${episode.number}-${String(row.index).padStart(2,"0")}.png`) });
    }
    actual.push({ index: row.index, plannedAt: row.at, observedAt: (actionStart - started) / 1000, actionSeconds, ...(evidence ? { evidence } : {}) });
    console.log(`${smoke ? "Check" : "Record"} ${episode.number} ${clock(row.at)} · ${row.chapter} (${actionSeconds.toFixed(2)}s action)`);
    if (!smoke) {
      const remaining = (started + row.end * 1000) - performance.now();
      if (remaining < -250) throw new Error(`Picture fell behind at beat ${row.index}`);
      await page.waitForTimeout(Math.max(0, remaining));
    }
  }
  elapsed = (performance.now() - started) / 1000;
  const video = page.video();
  await context.close();
  writeFileSync(join(output, `episode-${episode.number}.framing.json`), JSON.stringify({ episode: episode.number, seconds: totalSeconds(episode), foci }, null, 2) + "\n");
  if (!smoke) {
    const raw = await video.path();
    const encoded = join(work, "picture.mp4");
    videoAlignment = encodePicture({ source: raw, destination: encoded, materials: output, episode: episode.number, rows, seconds: totalSeconds(episode) });
    presentation = await finishEpisode(episode, { source: encoded });
  }
  writeFileSync(join(output, `episode-${episode.number}.${smoke ? "smoke" : "recording"}.json`), JSON.stringify({ episode: episode.number, smoke, seconds: totalSeconds(episode), elapsed, errors, actual, videoAlignment, ...(presentation ? { presentation } : {}) }, null, 2) + "\n");
  completed = true;
  console.log(`Ready: episode ${episode.number} ${smoke ? "all picture actions passed" : clock(totalSeconds(episode)) + " picture reel"}`);
} finally {
  // Stop traffic even when a recording fails before its normal closing cue.
  await page.evaluate(() => window.artfTrafficStop?.()).catch(() => {});
  await browser.close();
  rmSync(pending, { force: true });
  if (completed || smoke) rmSync(work, { recursive: true, force: true });
  else console.error(`Unfinished capture preserved for recovery: ${work}`);
}
