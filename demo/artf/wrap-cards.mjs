#!/usr/bin/env node
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { copyFileSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import { chromium } from "playwright";

const root = dirname(fileURLToPath(import.meta.url));
const html = pathToFileURL(join(root, "cards.html")).href;
const INTRO_S = 13;
const OUTRO_S = 15;
const EPISODES = {
  five: {
    intro: "#intro",
    outro: "#outro",
    introPng: "intro.png",
    outroPng: "outro.png",
    defaultMp4: "artf-sim-five.mp4",
  },
  one: {
    intro: "#intro-one",
    outro: "#outro-one",
    introPng: "intro-one.png",
    outroPng: "outro-one.png",
    defaultMp4: "artf-sim-one.mp4",
  },
  three: {
    intro: "#intro-three",
    outro: "#outro-three",
    introPng: "intro-three.png",
    outroPng: "outro-three.png",
    defaultMp4: "artf-sim-three.mp4",
  },
  four: {
    intro: "#intro-four",
    outro: "#outro-four",
    introPng: "intro-four.png",
    outroPng: "outro-four.png",
    defaultMp4: "artf-sim-four.mp4",
  },
};

function ff(args) {
  const r = spawnSync("ffmpeg", args, { stdio: "inherit" });
  if (r.status !== 0) throw new Error("ffmpeg failed");
}

export async function wrapReel(mp4 = join(root, "artf-sim-five.mp4"), { replace = false, episode = "five" } = {}) {
  const spec = EPISODES[episode] || EPISODES.five;
  const tmp = mkdtempSync(join(tmpdir(), "artf-cards-"));
  const introPng = join(root, spec.introPng);
  const outroPng = join(root, spec.outroPng);
  const introMp4 = join(tmp, "intro.mp4");
  const outroMp4 = join(tmp, "outro.mp4");
  const wrapped = join(tmp, "wrapped.mp4");
  let body = mp4;

  if (replace) {
    const probe = spawnSync(
      "ffprobe",
      ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", mp4],
      { encoding: "utf8" },
    );
    if (probe.status !== 0) throw new Error(probe.stderr || "ffprobe failed");
    const dur = Number(probe.stdout.trim());
    const bodyT = dur - INTRO_S - OUTRO_S;
    if (!(bodyT > 30)) throw new Error(`body too short to replace bookends (${dur}s)`);
    body = join(tmp, "body.mp4");
    ff(["-y", "-ss", String(INTRO_S), "-i", mp4, "-t", String(bodyT), "-c", "copy", body]);
  }

  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({
    viewport: { width: 1920, height: 1080 },
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
  await page.locator(spec.intro).screenshot({ path: introPng });
  await page.locator(spec.outro).screenshot({ path: outroPng });
  await browser.close();

  const still = (png, out, secs, fadeOutAt) =>
    ff([
      "-y",
      "-loop",
      "1",
      "-framerate",
      "25",
      "-t",
      String(secs),
      "-i",
      png,
      "-vf",
      `scale=1920:1080,setsar=1,format=yuv420p,fade=t=in:st=0:d=0.5,fade=t=out:st=${fadeOutAt}:d=0.6`,
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
      "-an",
      out,
    ]);
  still(introPng, introMp4, INTRO_S, INTRO_S - 0.6);
  still(outroPng, outroMp4, OUTRO_S, OUTRO_S - 0.6);

  const list = join(tmp, "concat.txt");
  writeFileSync(
    list,
    `file '${introMp4}'\nfile '${body}'\nfile '${outroMp4}'\n`,
  );
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

  copyFileSync(wrapped, mp4);
  rmSync(tmp, { recursive: true, force: true });
  return mp4;
}

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
  const args = process.argv.slice(2);
  const episode = args.includes("--four")
    ? "four"
    : args.includes("--three")
      ? "three"
      : args.includes("--one")
        ? "one"
        : "five";
  const spec = EPISODES[episode];
  const mp4 =
    args.find((a) => !a.startsWith("-")) || join(root, spec.defaultMp4);
  await wrapReel(mp4, { replace: true, episode });
  console.log(mp4);
}
