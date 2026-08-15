// Fast prevalence detection pass over a sampling frame.
// Loads each homepage once, accepts consent if a known CMP button is present,
// and records whether the site runs client-side header bidding and whether any
// OpenRTB payload crosses the wire. Bodies are NOT stored: this pass measures
// prevalence only. Deep capture is capture.mjs.
//
// Usage: node detect.mjs sites-tranco.txt detect/tranco.jsonl [concurrency]

import { chromium } from "playwright";
import fs from "fs";
import path from "path";

const sitesFile = process.argv[2] ?? "sites-tranco.txt";
const outFile = process.argv[3] ?? `detect/run-${Date.now()}.jsonl`;
const CONCURRENCY = Number(process.argv[4] ?? 8);

const NAV_TIMEOUT_MS = 20_000;
const SETTLE_MS = 8_000;
const MAX_BODY = 512 * 1024;

const sites = fs
  .readFileSync(sitesFile, "utf8")
  .split("\n")
  .map((s) => s.trim())
  .filter((s) => s && !s.startsWith("#"));

fs.mkdirSync(path.dirname(outFile), { recursive: true });
const out = fs.createWriteStream(outFile, { flags: "a" });
const write = (obj) => out.write(JSON.stringify(obj) + "\n");

function parseJson(text) {
  if (!text || text.length > MAX_BODY) return null;
  const t = text.trim();
  if (!t.startsWith("{")) return null;
  try {
    return JSON.parse(t);
  } catch {
    return null;
  }
}

const isOrtbRequest = (j) => j && Array.isArray(j.imp) && j.imp.length > 0 && "id" in j;

async function detect(browser, site) {
  const context = await browser.newContext({
    viewport: { width: 1366, height: 900 },
    locale: "en-US",
    userAgent:
      "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/151.0.0.0 Safari/537.36",
  });
  await context.addInitScript(() => {
    Object.defineProperty(navigator, "webdriver", { get: () => undefined });
  });
  const page = await context.newPage();
  const cdp = await context.newCDPSession(page);
  await cdp.send("Network.enable", {
    maxTotalBufferSize: 10 * 1024 * 1024,
    maxResourceBufferSize: 2 * 1024 * 1024,
  });

  const endpoints = new Set();
  let ortbRequests = 0;

  cdp.on("Network.requestWillBeSent", async (e) => {
    const { request, requestId } = e;
    if (request.method !== "POST") return;
    let postData = request.postData;
    if (!postData && request.hasPostData) {
      try {
        ({ postData } = await cdp.send("Network.getRequestPostData", { requestId }));
      } catch {
        return;
      }
    }
    if (!isOrtbRequest(parseJson(postData))) return;
    ortbRequests++;
    try {
      endpoints.add(new URL(request.url).host);
    } catch {}
  });

  let status = "ok";
  try {
    await page.goto(`https://${site}`, { timeout: NAV_TIMEOUT_MS, waitUntil: "domcontentloaded" });
    await page.waitForTimeout(2000);
    for (const sel of [
      "#onetrust-accept-btn-handler",
      ".qc-cmp2-summary-buttons button[mode='primary']",
      "button[mode='primary']",
      "button:has-text('Accept All')",
      "button:has-text('Accept all')",
      "button:has-text('I Accept')",
      "button:has-text('AGREE')",
    ]) {
      try {
        const b = page.locator(sel).first();
        if (await b.isVisible({ timeout: 300 })) {
          await b.click({ timeout: 1200 });
          break;
        }
      } catch {}
    }
    await page.mouse.wheel(0, 1200);
    await page.waitForTimeout(SETTLE_MS);
  } catch (err) {
    status = `error: ${String(err).split("\n")[0].slice(0, 90)}`;
  }

  let pb = { hasPbjs: false, pbjsVersion: null };
  try {
    pb = await page.evaluate(() => {
      let p = window.pbjs;
      let renamed = false;
      if (!p || !p.version) {
        for (const k of Object.getOwnPropertyNames(window)) {
          try {
            const v = window[k];
            if (v && typeof v === "object" && Array.isArray(v.installedModules) && typeof v.version === "string") {
              p = v;
              renamed = true;
              break;
            }
          } catch {}
        }
      }
      return { hasPbjs: !!p, pbjsVersion: p?.version ?? null, pbjsRenamed: renamed };
    });
  } catch {}

  const rec = {
    site,
    ts: new Date().toISOString(),
    status,
    ...pb,
    ortbRequests,
    endpoints: [...endpoints],
  };
  write(rec);
  await context.close();
  return rec;
}

const browser = await chromium.launch({
  headless: true,
  args: ["--disable-blink-features=AutomationControlled"],
});
const queue = [...sites];
let done = 0;
let hits = 0;

async function worker() {
  while (queue.length) {
    const site = queue.shift();
    try {
      const r = await detect(browser, site);
      if (r.ortbRequests > 0 || r.hasPbjs) hits++;
    } catch (err) {
      write({ site, status: `crash: ${String(err).slice(0, 90)}`, hasPbjs: false, ortbRequests: 0 });
    }
    done++;
    if (done % 25 === 0) {
      console.log(`[${done}/${sites.length}] hits so far: ${hits}`);
    }
  }
}

await Promise.all(Array.from({ length: CONCURRENCY }, worker));
await browser.close();
out.end();
console.log(`\ndetection complete: ${done} sites, ${hits} with pbjs or ORTB traffic -> ${outFile}`);
