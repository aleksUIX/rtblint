#!/usr/bin/env node
import { dirname, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { chromium } from "playwright";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";

const dir = dirname(fileURLToPath(import.meta.url));
const html = pathToFileURL(join(dir, "slides.html")).href + "?export=1";

const slides = [
  ["#slide-1", "01-artf.png"],
  ["#slide-2", "02-break.png"],
  ["#slide-3", "03-check.png"],
  ["#slide-4", "04-findings.png"],
];

const browser = await chromium.launch({
  headless: true,
  channel: "chrome",
});
const page = await browser.newPage({
  viewport: { width: 720, height: 1600 },
  deviceScaleFactor: 2,
});
await page.goto(html, { waitUntil: "load" });
await page.evaluate(async () => {
  await document.fonts.ready;
  const loaded =
    document.fonts.check("16px Geist") &&
    document.fonts.check('12px "Geist Mono"');
  if (!loaded) throw new Error("Geist did not load");
});

const heights = [];
for (const [sel, file] of slides) {
  const loc = page.locator(sel);
  await loc.screenshot({ path: join(dir, file) });
  heights.push(
    Math.round(await loc.evaluate((el) => el.getBoundingClientRect().height)),
  );
}
await browser.close();

const tmp = mkdtempSync(join(tmpdir(), "artf-slides-"));
const pdfs = slides.map(([, file], i) => {
  const pdf = join(tmp, `${String(i + 1).padStart(2, "0")}.pdf`);
  const png = join(dir, file);
  const r = spawnSync("sips", ["-s", "format", "pdf", png, "--out", pdf], {
    encoding: "utf8",
  });
  if (r.status !== 0) throw new Error(r.stderr || "sips failed");
  return pdf;
});
const joinBin =
  "/System/Library/Automator/Combine PDF Pages.action/Contents/MacOS/join";
const joined = spawnSync(joinBin, ["-o", join(dir, "artf-slides.pdf"), ...pdfs], {
  encoding: "utf8",
});
rmSync(tmp, { recursive: true, force: true });
if (joined.status !== 0) throw new Error(joined.stderr || "pdf join failed");

console.log(
  slides
    .map(([, file], i) => `${file} (${heights[i]}px)`)
    .join(", ") + ", artf-slides.pdf",
);
