import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

const seconds = stamp => stamp.split(":").reduce((sum, part) => sum * 60 + Number(part.replace(",", ".")), 0);
const esc = value => value.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll('"', "&quot;");

export const presenterCSS = `
#speaker{position:fixed;right:32px;bottom:8px;height:56px;display:flex;align-items:center;gap:12px;color:#161616;font:400 22px/1.2 Geist,system-ui,sans-serif;background:#f4f3ee}
#speaker .identity{text-align:right}#speaker .label{display:block;color:#1d4ed8;font-size:16px;line-height:1.4;margin-bottom:2px}#speaker .name{display:block;font-weight:500}#speaker img{width:56px;height:56px;border-radius:50%;border:2px solid #1d4ed8;object-fit:cover}
`;

export function presenterBadge(speaker) {
  return `<aside id="speaker" aria-label="${esc(speaker.label)} ${esc(speaker.name)}"><div class="identity"><span class="label">${esc(speaker.label)}</span><span class="name">${esc(speaker.name)}</span></div><img src="${esc(speaker.portrait)}" alt="${esc(speaker.name)}" width="56" height="56"></aside>`;
}

export function captionCues(file) {
  return readFileSync(file, "utf8").trim().split(/\r?\n\r?\n/).map(block => {
    const [, timing, ...lines] = block.split(/\r?\n/);
    const [start, end] = timing.split(" --> ").map(seconds);
    if (!(end > start) || lines.length > 2) throw new Error(`Invalid subtitle cue: ${block}`);
    return { start, end, text: lines.join("\n") };
  });
}

export async function renderCaptions(browser, episode, rows, materials, { speaker } = {}) {
  const cues = captionCues(join(materials, `episode-${episode}.srt`));
  const flatten = text => text.trim().replace(/\s+/g, " ");
  if (flatten(cues.map(c => c.text).join(" ")) !== flatten(rows.map(r => r.text).filter(Boolean).join(" "))) throw new Error("Burned captions must contain every word of the narration.");
  const dir = join(materials, `episode-${episode}.captions`);
  mkdirSync(dir, { recursive: true });
  const context = await browser.newContext({ viewport: { width: 1920, height: 1080 }, deviceScaleFactor: 1 });
  const page = await context.newPage();
  const fontSize = 64;
  const maxWidth = 1680;
  const frame = speaker ? { width: 1920, height: 768, x: 0, y: 0, footerHeight: 312 } : undefined;
  const singleLineBoxHeight = fontSize * 1.32 + 10 + 12;
  const bottom = speaker ? Math.ceil(singleLineBoxHeight) : 292;
  const idleFile = speaker ? "presenter.png" : "blank.png";
  let speakerBounds;
  const events = [];
  const event = (at, file) => {
    const frame = Math.round(at * 25);
    if (events.at(-1)?.frame === frame) events.pop();
    events.push({ frame, file });
  };
  try {
    const html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Episode ${episode} presenter badge</title><style>
@font-face{font-family:Geist;font-style:normal;font-weight:400;src:url('../../linkedin/fonts/geist-latin-400.woff2') format('woff2')}
@font-face{font-family:Geist;font-style:normal;font-weight:500;src:url('../../linkedin/fonts/geist-latin-500.woff2') format('woff2')}
*{box-sizing:border-box}html,body{margin:0;background:transparent;width:1920px;height:1080px}
#caption{position:fixed;left:50%;bottom:${bottom}px;transform:translateX(-50%);max-width:${maxWidth}px;width:max-content;padding:10px 24px 12px;background:rgba(0,0,0,0.82);color:#fff;text-align:center;font:500 ${fontSize}px/1.32 Helvetica,Arial,sans-serif;white-space:pre-line;display:none}
${presenterCSS}
</style></head><body><div id="caption"></div>${speaker ? presenterBadge(speaker) : ""}</body></html>`;
    const overlayFile = join(materials, `episode-${episode}.speaker.html`);
    writeFileSync(overlayFile, html);
    await page.goto(pathToFileURL(overlayFile).href);
    await page.evaluate(() => document.fonts.ready);
    if (speaker) {
      await page.waitForFunction(() => [...document.images].every(img => img.complete && img.naturalWidth > 0));
      if (!await page.evaluate(() => document.fonts.check("22px Geist"))) throw new Error("Presenter badge font did not load");
      speakerBounds = await page.locator("#speaker").boundingBox();
      if (speakerBounds.y < frame.height || speakerBounds.y + speakerBounds.height > 1080 || speakerBounds.x + speakerBounds.width > 1920) throw new Error("Presenter badge must fit entirely below the simulator");
      await page.locator("#speaker").screenshot({ path: join(materials, `episode-${episode}.speaker.png`), omitBackground: true });
    }
    await page.screenshot({ path: join(dir, idleFile), omitBackground: true });
    event(0, idleFile);
    for (const [i, cue] of cues.entries()) {
      await page.evaluate(text => {
        const el = document.getElementById("caption");
        el.style.display = "block";
        el.textContent = text;
      }, cue.text);
      const bounds = await page.locator("#caption").boundingBox();
      const minimumTop = speaker ? frame.height + 12 : 690;
      const maximumBottom = speaker ? 1080 - bottom : 800;
      if (bounds.x < 0 || bounds.width > maxWidth || bounds.y < minimumTop || bounds.y + bounds.height > maximumBottom) throw new Error(`Caption ${i + 1} covers the ticket or inspector`);
      if (speaker && bounds.x < speakerBounds.x + speakerBounds.width && bounds.x + bounds.width > speakerBounds.x && bounds.y < speakerBounds.y + speakerBounds.height && bounds.y + bounds.height > speakerBounds.y) throw new Error(`Caption ${i + 1} covers the presenter badge`);
      const file = `${String(i + 1).padStart(3, "0")}.png`;
      await page.screenshot({ path: join(dir, file), omitBackground: true });
      event(cue.start, file);
      event(cue.end, idleFile);
    }
  } finally {
    await context.close();
  }
  const totalFrames = Math.round(rows.at(-1).end * 25);
  let concat = "ffconcat version 1.0\n";
  for (let i = 0; i < events.length; i++) {
    const duration = ((events[i + 1]?.frame ?? totalFrames) - events[i].frame) / 25;
    if (duration <= 0) continue;
    concat += `file '${events[i].file}'\nduration ${duration.toFixed(6)}\n`;
  }
  concat += `file '${idleFile}'\n`;
  const track = join(dir, "captions.ffconcat");
  writeFileSync(track, concat);
  const report = { cues: cues.length, words: flatten(cues.map(c => c.text).join(" ")).split(" ").length, style: `Helvetica or Arial, ${fontSize}px, white on 82% opaque black, bottom ${bottom}px`, fontSize, backgroundOpacity: 0.82, singleLineBoxHeight, bottom, mobileFontAt360CssPixels: fontSize * 360 / 1920, layout: "No overlap with the picture or presenter", first: cues[0].start, last: cues.at(-1).end, track, ...(speaker ? { frame, speaker: { ...speaker, bounds: speakerBounds, portraitSize: 56, visibility: "Throughout the video, including pauses and title cards." } } : {}) };
  writeFileSync(join(dir, "captions.json"), JSON.stringify({ ...report, cues }, null, 2) + "\n");
  return report;
}
