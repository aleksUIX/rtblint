#!/usr/bin/env node
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { mkdtempSync, rmSync, writeFileSync, copyFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import { chromium } from "playwright";

const dir = dirname(fileURLToPath(import.meta.url));
const root = join(dir, "..");
const html = pathToFileURL(join(dir, "clip.html")).href;
const out = join(dir, "artf-series-clip.mp4");
const W = 1080;
const H = 1350;
const VW = 1080;
const VH = 760;
const FPS = 25;

function ff(args) {
  const r = spawnSync("ffmpeg", args, { stdio: "inherit" });
  if (r.status !== 0) throw new Error("ffmpeg failed");
}

const shots = [
  {
    id: "break",
    src: join(root, "artf-sim-five.mp4"),
    ss: 99.4,
    t: 3.3,
    crop: "1920:660:0:0",
    fill: false,
    cap: "cap-break",
  },
  {
    id: "env",
    src: join(root, "artf-sim-three.mp4"),
    ss: 183.2,
    t: 3.4,
    crop: "1920:660:0:0",
    fill: false,
    cap: "cap-env",
  },
  {
    id: "fan",
    src: join(root, "artf-sim-four.mp4"),
    ss: 44.0,
    t: 3.6,
    crop: "1920:980:0:88",
    fill: true,
    cap: "cap-fan",
  },
  {
    id: "vol",
    src: join(root, "artf-sim-five.mp4"),
    ss: 268.0,
    t: 4.6,
    speed: 1.32,
    crop: "1920:980:0:36",
    fill: true,
    cap: "cap-vol",
  },
];

const tmp = mkdtempSync(join(tmpdir(), "artf-clip-"));
const browser = await chromium.launch({
  headless: true,
  channel: "chrome",
});
const page = await browser.newPage({
  viewport: { width: 1080, height: 1350 },
  deviceScaleFactor: 2,
});
await page.goto(html, { waitUntil: "load" });
await page.evaluate(async () => {
  await document.fonts.ready;
  const ok =
    document.fonts.check("16px Geist") &&
    document.fonts.check('12px "Geist Mono"');
  if (!ok) throw new Error("Geist did not load");
});

const png = (id) => join(tmp, `${id}.png`);
for (const id of ["intro", "outro", ...shots.map((s) => s.cap)]) {
  const loc = page.locator("#" + id);
  await loc.screenshot({ path: png(id) });
}
await browser.close();

function still(id, secs, fadeOutAt) {
  const mp4 = join(tmp, `${id}.mp4`);
  ff([
    "-y",
    "-loop",
    "1",
    "-framerate",
    String(FPS),
    "-t",
    String(secs),
    "-i",
    png(id),
    "-vf",
    `scale=${W}:${H}:flags=lanczos,setsar=1,format=yuv420p,fade=t=in:st=0:d=0.25,fade=t=out:st=${fadeOutAt}:d=0.35`,
    "-c:v",
    "libx264",
    "-profile:v",
    "high",
    "-crf",
    "17",
    "-preset",
    "fast",
    "-pix_fmt",
    "yuv420p",
    "-r",
    String(FPS),
    "-video_track_timescale",
    "12800",
    "-an",
    mp4,
  ]);
  return mp4;
}

const introMp4 = still("intro", 2.2, 1.85);
const outroMp4 = still("outro", 4.0, 3.65);

const body = [];
for (const shot of shots) {
  const vid = join(tmp, `${shot.id}-v.mp4`);
  const stacked = join(tmp, `${shot.id}.mp4`);
  const speed = shot.speed || 1;
  const fit = shot.fill
    ? `scale=${VW}:${VH}:force_original_aspect_ratio=increase:flags=lanczos,crop=${VW}:${VH}:0:0`
    : `scale=${VW}:${VH}:force_original_aspect_ratio=decrease:flags=lanczos,pad=${VW}:${VH}:(ow-iw)/2:(oh-ih)/2:color=0x111111`;
  const setpts = speed === 1 ? "PTS-STARTPTS" : `PTS/${speed}`;
  ff([
    "-y",
    "-ss",
    String(shot.ss),
    "-t",
    String(shot.t),
    "-i",
    shot.src,
    "-vf",
    `crop=${shot.crop},setsar=1,${fit},setpts=${setpts},fps=${FPS},format=yuv420p`,
    "-an",
    "-c:v",
    "libx264",
    "-profile:v",
    "high",
    "-crf",
    "17",
    "-preset",
    "fast",
    "-pix_fmt",
    "yuv420p",
    "-r",
    String(FPS),
    "-video_track_timescale",
    "12800",
    vid,
  ]);
  ff([
    "-y",
    "-i",
    vid,
    "-i",
    png(shot.cap),
    "-filter_complex",
    `[0:v]setsar=1[v];[1:v]scale=${W}:590:flags=lanczos,setsar=1[c];[v][c]vstack=inputs=2,format=yuv420p,setsar=1`,
    "-c:v",
    "libx264",
    "-profile:v",
    "high",
    "-crf",
    "17",
    "-preset",
    "fast",
    "-pix_fmt",
    "yuv420p",
    "-r",
    String(FPS),
    "-video_track_timescale",
    "12800",
    "-an",
    stacked,
  ]);
  body.push(stacked);
}

const list = join(tmp, "concat.txt");
writeFileSync(
  list,
  [introMp4, ...body, outroMp4].map((f) => `file '${f}'`).join("\n") + "\n",
);
const wrapped = join(tmp, "wrapped.mp4");
ff([
  "-y",
  "-f",
  "concat",
  "-safe",
  "0",
  "-i",
  list,
  "-c",
  "copy",
  "-movflags",
  "+faststart",
  wrapped,
]);
copyFileSync(wrapped, out);
rmSync(tmp, { recursive: true, force: true });
console.log(out);
