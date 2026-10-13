import assert from "node:assert/strict";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { pathToFileURL } from "node:url";
import { chromium } from "playwright";
import { output } from "./build.mjs";
import { episodes } from "./episodes.mjs";
import { createHash } from "node:crypto";

const fullTake = process.argv.includes("--full-take");
const number = Number(process.argv.slice(2).find(arg => /^\d+$/.test(arg)) ?? 2);
const episode = episodes[number];
if (!episode) throw new Error("Use node series/check-prompter.mjs 2|3 [--full-take]");
const stem = `episode-${number}`;
const url = `http://localhost:8070/${stem}.prompter.html`;
const rows = JSON.parse(readFileSync(join(output, `${stem}.cues.json`), "utf8")).rows;
const captions = JSON.parse(readFileSync(join(output, `${stem}.captions/captions.json`), "utf8")).cues;
const frameCues = captions.map((cue, index) => ({ ...cue, index, startFrame: Math.round(cue.start * 25), endFrame: Math.round(cue.end * 25) }));
const flat = text => text.trim().replace(/\s+/g, " ");
const spoken = rows.filter(row => row.text);
const total = rows.at(-1).end;
const words = spoken.reduce((sum, row) => sum + row.text.split(/\s+/).length, 0);
const presentation = JSON.parse(readFileSync(join(output, `${stem}.presentation.json`), "utf8"));
const versions = { videoSha256: presentation.sha256, cueSha256: presentation.cueSha256, subtitleSha256: presentation.subtitleSha256, standaloneHtmlSha256: createHash("sha256").update(readFileSync(join(output, `${stem}.prompter.html`))).digest("hex") };
const pacing = spoken.map((row, index) => ({ at: row.at, words: row.text.split(/\s+/).length, speakAt: row.voiceAt, speakUntil: row.voiceEnd, wordsPerMinute: row.text.split(/\s+/).length * 60 / (row.voiceEnd - row.voiceAt), nextSpeakingGapSeconds: spoken[index + 1] ? spoken[index + 1].voiceAt - row.voiceEnd : undefined }));
for (const row of spoken) assert(row.voiceEnd <= row.end - 1 + 0.001);
assert(pacing.every(row => Math.abs(row.wordsPerMinute - episode.narrationWpm) < 0.001));
const minimumGap = Math.min(...pacing.flatMap(row => row.nextSpeakingGapSeconds === undefined ? [] : [row.nextSpeakingGapSeconds]));
assert(minimumGap >= 3 - 0.001);

const browser = await chromium.launch({ headless: true });
const context = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
const page = await context.newPage();
const errors = [];
page.on("pageerror", error => errors.push(error.message));
await page.goto(url);
await page.waitForFunction(() => window.artfPrompter?.state.ready, null, { timeout: 45000 });
assert(await page.locator("#picture").evaluate(video => video.src.startsWith("blob:")));
assert.equal(await page.locator("#mode").inputValue(), "recording");
assert(await page.locator("#pace").isDisabled());
assert.equal(await page.locator("#picture").evaluate(video => video.duration), total);

async function seek(time) {
  await page.evaluate(time => { document.getElementById("picture").currentTime = time; }, time);
  await page.waitForFunction(time => !document.getElementById("picture").seeking && Math.abs(window.artfPrompter.state.at - time) < 0.001, time);
}

try {
  if (fullTake) {
    await context.setOffline(true);
    await page.evaluate(() => {
      const video = document.getElementById("picture");
      const plan = JSON.parse(document.getElementById("plan").textContent);
      const result = window.takeResult = { frames: 0, maximumClockErrorSeconds: 0, mismatches: [], sections: [], cues: [], interruptions: [] };
      const sections = new Set(), cues = new Set();
      video.addEventListener("pause", () => { if (!video.ended) result.interruptions.push(video.currentTime); });
      const observer = (now, metadata) => {
        const state = window.artfPrompter.state;
        if (!video.paused && state.ready) {
          result.frames++;
          result.maximumClockErrorSeconds = Math.max(result.maximumClockErrorSeconds, Math.abs(state.at - metadata.mediaTime));
          sections.add(state.section);
          if (state.cue >= 0) cues.add(state.cue);
          const expected = time => plan.cues.find(cue => cue.startFrame <= Math.round(time * 25) && Math.round(time * 25) < cue.endFrame)?.index ?? -1;
          if (![expected(metadata.mediaTime), expected(metadata.mediaTime - .04), expected(metadata.mediaTime + .04)].includes(state.cue)) result.mismatches.push({ at: metadata.mediaTime, cue: state.cue });
          result.sections = [...sections]; result.cues = [...cues];
        }
        video.requestVideoFrameCallback(observer);
      };
      video.requestVideoFrameCallback(observer);
    });
    await page.locator("#toggle").click();
    console.log("Full take started: 100% speed, offline after complete preload.");
    for (let minute = 1; minute * 60 < total; minute++) {
      await page.waitForFunction(at => window.artfPrompter.state.at >= at, minute * 60, { timeout: 70000 });
      console.log(`Full take: ${minute}:00, picture and prompt still aligned.`);
    }
    await page.waitForFunction(() => document.getElementById("picture").ended, null, { timeout: 60000 });
    const result = await page.evaluate(() => ({ ...window.takeResult, endedAt: window.artfPrompter.state.at, finalRunning: window.artfPrompter.state.running }));
    assert.equal(result.endedAt, total);
    assert.equal(result.finalRunning, false);
    assert.deepEqual(result.interruptions, []);
    assert.deepEqual(result.mismatches, []);
    assert.equal(result.sections.length, rows.length);
    assert.equal(result.cues.length, captions.length);
    assert(result.frames > total * 25 * .95);
    assert(result.maximumClockErrorSeconds <= .040001);
    assert.deepEqual(errors, []);
    writeFileSync(join(output, `${stem}.prompter.full-take.json`), JSON.stringify({ ...result, ...versions, speed: 1, networkRequiredAfterLoading: false, seconds: total, checkedAt: new Date().toISOString() }, null, 2) + "\n");
    console.log(JSON.stringify(result));
  } else {
    for (const [index, row] of rows.entries()) {
      await page.selectOption("#seek", String(index));
      await page.waitForFunction(at => !document.getElementById("picture").seeking && Math.abs(window.artfPrompter.state.at - at) < .001, row.at);
      if (row.text) assert.equal(flat(await page.locator("#text").innerText()), flat(row.text));
    }
    let boundaries = 0;
    for (const cue of frameCues) {
      for (const frame of [cue.startFrame - 1, cue.startFrame, cue.endFrame - 1, cue.endFrame]) {
        await seek(frame / 25);
        const expected = frameCues.find(c => c.startFrame <= frame && frame < c.endFrame)?.index ?? -1;
        assert.equal(await page.evaluate(() => window.artfPrompter.state.cue), expected, `Subtitle frame ${frame}`);
        if (expected >= 0) assert.equal(flat(await page.locator("#text .current").innerText()), flat(frameCues[expected].text));
        boundaries++;
      }
    }
    const layouts = [];
    for (const viewport of [{ width: 1920, height: 1080 }, { width: 1440, height: 900 }, { width: 1366, height: 768 }]) {
      await page.setViewportSize(viewport);
      for (const picture of [true, false]) {
        await page.locator("#show-picture").setChecked(picture);
        for (const row of spoken) {
          await page.selectOption("#seek", String(row.index));
          await page.waitForFunction(index => window.artfPrompter.state.section === index && !document.getElementById("picture").seeking, row.index);
          const fit = await page.locator("#narration").evaluate(el => ({ top: el.getBoundingClientRect().top, bottom: el.getBoundingClientRect().bottom, height: innerHeight, pageFits: document.documentElement.scrollHeight === innerHeight, fontSize: parseFloat(getComputedStyle(document.getElementById("text")).fontSize) }));
          assert(fit.top >= 0 && fit.bottom <= fit.height && fit.pageFits, JSON.stringify({ viewport, picture, row: row.index, fit }));
          assert(fit.fontSize >= 30, `Section ${row.index} needs larger reading text`);
          layouts.push({ viewport, picture, section: row.index, fontSize: fit.fontSize });
        }
      }
    }
    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.locator("#show-picture").setChecked(true);
    await seek(11);
    await page.screenshot({ path: join(output, `qa/${stem}-recording-prompter.png`) });
    await context.setOffline(true);
    await page.locator("#toggle").click();
    await page.waitForFunction(() => !document.getElementById("picture").paused);
    await page.locator("#show-picture").setChecked(false);
    const start = await page.evaluate(() => document.getElementById("picture").currentTime);
    await page.evaluate(() => { const start = performance.now(); while (performance.now() - start < 700) {} });
    await page.waitForFunction(start => document.getElementById("picture").currentTime > start + .7, start);
    const hidden = await page.evaluate(() => ({ ...window.artfPrompter.state, videoAt: document.getElementById("picture").currentTime, rate: document.getElementById("picture").playbackRate }));
    assert.equal(hidden.running, true);
    assert.equal(hidden.rate, 1);
    assert(Math.abs(hidden.videoAt - hidden.at) < .08);
    await page.locator("#toggle").click();
    const paused = await page.evaluate(() => ({ videoAt: document.getElementById("picture").currentTime, promptAt: window.artfPrompter.state.at }));
    await page.waitForTimeout(400);
    const still = await page.evaluate(() => ({ videoAt: document.getElementById("picture").currentTime, promptAt: window.artfPrompter.state.at }));
    assert.equal(still.videoAt, paused.videoAt);
    assert.equal(still.promptAt, paused.promptAt);
    for (let i = 0; i < 6; i++) {
      await page.locator("#reset").click();
      await page.locator("#toggle").click();
      await page.locator("#reset").click();
      await page.waitForFunction(() => !document.getElementById("picture").seeking);
      assert.equal(await page.evaluate(() => window.artfPrompter.state.running), false);
    }
    await page.selectOption("#mode", "rehearsal");
    await page.selectOption("#pace", "0.9");
    await page.locator("#toggle").click();
    await page.waitForFunction(() => document.getElementById("picture").currentTime > .3);
    assert.equal(await page.locator("#picture").evaluate(video => video.playbackRate), .9);
    await page.locator("#toggle").click();
    await page.selectOption("#mode", "recording");
    assert.equal(await page.locator("#picture").evaluate(video => video.playbackRate), 1);
    await page.locator("#picture").evaluate(video => { video.playbackRate = 1.1; });
    await page.waitForFunction(() => document.getElementById("picture").playbackRate === 1);
    for (let take = 0; take < 3; take++) {
      await seek(total - 2);
      await page.locator("#toggle").click();
      await page.waitForFunction(() => document.getElementById("picture").ended, null, { timeout: 5000 });
      assert.equal(await page.evaluate(() => window.artfPrompter.state.at), total);
      assert.equal(await page.evaluate(() => window.artfPrompter.state.running), false);
    }
    await context.setOffline(false);
    const mismatch = await browser.newPage();
    await mismatch.route(`**/${stem}.prompter.html`, async route => {
      const response = await route.fetch();
      const html = await response.text();
      const body = html.replace(/(<script id="plan" type="application\/json">)([\s\S]*?)(<\/script>)/, (_, start, json, end) => {
        const data = JSON.parse(json); data.presentation.sha256 = "0".repeat(64);
        return start + JSON.stringify(data).replaceAll("<", "\\u003c") + end;
      });
      await route.fulfill({ body, contentType: "text/html" });
    });
    await mismatch.goto(url);
    await mismatch.waitForFunction(() => document.getElementById("status").textContent.includes("video changed"));
    assert(await mismatch.locator("#toggle").isDisabled());
    await mismatch.close();
    const stale = await browser.newPage();
    await stale.route(`**/${stem}.prompter.html`, async route => {
      const response = await route.fetch();
      const html = await response.text();
      const body = html.replace(/(<script id="plan" type="application\/json">)([\s\S]*?)(<\/script>)/, (_, start, json, end) => {
        const data = JSON.parse(json); data.presentation.cueSha256 = "0".repeat(64);
        return start + JSON.stringify(data).replaceAll("<", "\\u003c") + end;
      });
      await route.fulfill({ body, contentType: "text/html" });
    });
    await stale.goto(url);
    await stale.waitForFunction(() => document.getElementById("status").textContent.includes("matching rebuild"));
    assert(await stale.locator("#toggle").isDisabled());
    await stale.close();
    const fileContext = await browser.newContext();
    await fileContext.setOffline(true);
    const filePage = await fileContext.newPage();
    await filePage.goto(pathToFileURL(join(output, `${stem}.prompter.html`)).href);
    await filePage.waitForFunction(() => window.artfPrompter?.state.ready);
    await filePage.locator("#toggle").click();
    await filePage.waitForFunction(() => window.artfPrompter.state.at > .5);
    await fileContext.close();
    const noScriptContext = await browser.newContext({ javaScriptEnabled: false });
    const noScriptPage = await noScriptContext.newPage();
    await noScriptPage.goto(pathToFileURL(join(output, `${stem}.prompter.html`)).href);
    assert(await noScriptPage.locator(".fallback").isVisible());
    assert.equal(await noScriptPage.locator(".fallback section").count(), spoken.length);
    assert.equal(flat(await noScriptPage.locator(".fallback section p").allTextContents().then(text => text.join(" "))), flat(spoken.map(row => row.text).join(" ")));
    await noScriptContext.close();
    assert.deepEqual(errors, []);
    const result = { checkedAt: new Date().toISOString(), ...versions, episode: number, videoSeconds: total, narrationWords: words, wordsPerMinute: episode.narrationWpm, minimumGapBetweenParagraphsSeconds: minimumGap, sectionsMatched: rows.length, subtitleBoundaryChecks: boundaries, layouts, fullPreload: "passed", hiddenPictureClock: "passed", offlinePlayback: "passed", directFilePlayback: "passed", scriptWithoutJavaScript: "passed", pauseResumeResetAndRapidClicks: "passed", recordingSpeedLock: "passed", finalFadePlayback: "passed three times", changedVideoRejected: "passed", staleScriptRejected: "passed", pacing };
    writeFileSync(join(output, `${stem}.prompter.verification.json`), JSON.stringify(result, null, 2) + "\n");
    console.log(`Prompter checks passed: ${boundaries} subtitle boundaries, ${rows.length} sections, all reading layouts, hidden picture, offline playback, controls, speed lock, and version checks.`);
  }
} catch (error) {
  const diagnosis = await page.evaluate(() => ({ state: window.artfPrompter?.state, status: document.getElementById("status").textContent, videoAt: document.getElementById("picture").currentTime, paused: document.getElementById("picture").paused, ended: document.getElementById("picture").ended, readyState: document.getElementById("picture").readyState, error: document.getElementById("picture").error?.message, fullTake: window.takeResult }));
  console.error(JSON.stringify({ failure: error.message, diagnosis }));
  throw error;
} finally { await browser.close(); }
