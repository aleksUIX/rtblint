import { copyFileSync, existsSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chromium } from "playwright";
import { episodes, timeline, totalSeconds } from "./episodes.mjs";
import { build, output } from "./build.mjs";
import { renderCaptions } from "./captions.mjs";
import { encodePicture } from "./media.mjs";

export async function finishEpisode(episode, { source } = {}) {
  build();
  const stem = `episode-${episode.number}`;
  const reel = join(output, `${stem}.mp4`);
  const master = join(output, `${stem}.picture.mp4`);
  const pending = join(output, `${stem}.pending.mp4`);
  if (!source) {
    if (!existsSync(master)) copyFileSync(reel, master);
    source = master;
  }
  const browser = await chromium.launch({ headless: true });
  let captions;
  const cards = [];
  try {
    const page = await browser.newPage({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
    for (const kind of ["intro", "outro"]) {
      await page.goto(pathToFileURL(join(output, `${stem}.${kind}.html`)).href);
      await page.evaluate(() => document.fonts.ready);
      await page.waitForFunction(() => [...document.images].every(img => img.complete && img.naturalWidth > 0));
      const layout = await page.evaluate(() => {
        const card = document.querySelector(".card");
        const children = [...card.children].map(el => ({ tag: el.className || el.tagName, top: el.getBoundingClientRect().top, bottom: el.getBoundingClientRect().bottom }));
        return { fonts: document.fonts.check("16px Geist") && document.fonts.check('16px "Geist Mono"'), fits: card.scrollHeight === 1080, children };
      });
      if (!layout.fonts || !layout.fits || layout.children.some((el, i) => el.bottom > 1080 || (i && el.top < layout.children[i - 1].bottom - 1))) throw new Error(`${kind} card does not fit: ${JSON.stringify(layout)}`);
      await page.screenshot({ path: join(output, `${stem}.${kind}.png`) });
      cards.push({ kind, ...layout });
    }
    captions = await renderCaptions(browser, episode.number, timeline(episode), output, { speaker: episode.speaker });
  } finally {
    await browser.close();
  }
  console.log(`Episode ${episode.number}: episode 1 cards rendered; ${captions.cues} narration captions, ${captions.words} words. Encoding MP4.`);
  try {
    const framingFile = join(output, `${stem}.framing.json`);
    const framing = existsSync(framingFile) ? JSON.parse(readFileSync(framingFile, "utf8")) : null;
    if (framing && framing.seconds !== totalSeconds(episode)) throw new Error("Focus regions belong to a different picture timeline");
    const frame = { ...captions.frame, foci: framing?.foci || [] };
    const alignment = encodePicture({ source, destination: pending, materials: output, episode: episode.number, rows: timeline(episode), seconds: totalSeconds(episode), captions: captions.track, frame, crf: 18 });
    const probe = spawnSync("ffprobe", ["-v", "error", "-show_format", "-show_streams", "-of", "json", pending], { encoding: "utf8" });
    if (probe.status) throw new Error(probe.stderr);
    const media = JSON.parse(probe.stdout);
    if (Number(media.format.duration) !== totalSeconds(episode) || media.streams[0].width !== 1920 || media.streams[0].height !== 1080 || media.streams[0].avg_frame_rate !== "25/1") throw new Error("Finished picture does not match the episode clock or format.");
    if (source !== master) copyFileSync(source, master);
    renameSync(pending, reel);
    const digest = file => createHash("sha256").update(readFileSync(file)).digest("hex");
    const report = { episode: episode.number, finishedAt: new Date().toISOString(), cardStyle: "Episode 1: blue edge, paper, Geist, summary panels, author portrait", openingCard: "Fully revealed from the first frame", cards, captions, focusRegions: frame.foci, alignment, seconds: totalSeconds(episode), master, sha256: digest(reel), cueSha256: digest(join(output, `${stem}.cues.json`)), subtitleSha256: digest(join(output, `${stem}.srt`)), media };
    writeFileSync(join(output, `${stem}.presentation.json`), JSON.stringify(report, null, 2) + "\n");
    build();
    console.log(`Ready: ${reel}`);
    return report;
  } finally {
    rmSync(pending, { force: true });
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const episode = episodes[Number(process.argv[2])];
  if (!episode) throw new Error("Use node series/finish.mjs 2|3");
  await finishEpisode(episode);
}
