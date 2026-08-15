// OpenRTB in-the-wild capture harness (POC).
// Loads each site in headless Chromium, intercepts network traffic at the
// CDP layer, and records every POST body that parses as an OpenRTB bid
// request plus every response body that parses as a bid response.
//
// Usage: node capture.mjs sites.txt captures/run1.jsonl [concurrency]

import { chromium } from "playwright";
import fs from "fs";
import path from "path";

const sitesFile = process.argv[2] ?? "sites.txt";
const outFile = process.argv[3] ?? `captures/run-${Date.now()}.jsonl`;
const CONCURRENCY = Number(process.argv[4] ?? 3);

const PAGE_TIMEOUT_MS = 30_000;
// Hard per-site budget. Without it a single hung page starves a worker slot
// for many minutes; enough of them and the remaining sites never get a fair
// load window, which silently collapses the capture rate rather than erroring.
const SITE_BUDGET_MS = 150_000;
const SETTLE_MS = 9_000;
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
const isOrtbResponse = (j) => j && ("seatbid" in j || "nbr" in j) && "id" in j;

async function crawlSite(browser, site) {
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
    maxTotalBufferSize: 100 * 1024 * 1024,
    maxResourceBufferSize: 20 * 1024 * 1024,
  });

  const pending = new Map(); // requestId -> {url, postData, headers}
  const counts = { ortbRequests: 0, ortbResponses: 0 };

  cdp.on("Network.requestWillBeSent", async (e) => {
    const { request, requestId } = e;
    if (request.method !== "POST") return;
    if (process.env.DEBUG_POSTS) console.log("POST", request.url.slice(0, 110), "hasPostData=", !!request.postData || !!request.hasPostData);
    let postData = request.postData;
    if (!postData && request.hasPostData) {
      try {
        const r = await cdp.send("Network.getRequestPostData", { requestId });
        postData = r.postData;
      } catch {
        return;
      }
    }
    if (!postData) return;
    const j = parseJson(postData);
    if (!isOrtbRequest(j)) return;
    const rec = {
      kind: "ortb-request",
      site,
      ts: new Date().toISOString(),
      endpoint: request.url,
      requestId,
      xOpenrtbVersion:
        request.headers["x-openrtb-version"] ?? request.headers["X-OpenRTB-Version"] ?? null,
      body: j,
    };
    pending.set(requestId, rec);
    counts.ortbRequests++;
    write(rec);
  });

  cdp.on("Network.loadingFinished", async (e) => {
    const req = pending.get(e.requestId);
    if (!req) return;
    pending.delete(e.requestId);
    try {
      const { body, base64Encoded } = await cdp.send("Network.getResponseBody", {
        requestId: e.requestId,
      });
      const text = base64Encoded ? Buffer.from(body, "base64").toString("utf8") : body;
      const j = parseJson(text);
      if (isOrtbResponse(j)) {
        counts.ortbResponses++;
        write({
          kind: "ortb-response",
          site,
          ts: new Date().toISOString(),
          endpoint: req.endpoint,
          requestId: e.requestId,
          body: j,
        });
      }
    } catch {
      /* body evicted or unavailable; request record already written */
    }
  });

  const budget = (promise, ms, label) =>
    Promise.race([
      promise,
      new Promise((_, rej) => setTimeout(() => rej(new Error(`budget exceeded: ${label}`)), ms)),
    ]);

  let status = "ok";
  try {
    await budget((async () => {
    await page.goto(`https://${site}`, {
      timeout: PAGE_TIMEOUT_MS,
      waitUntil: "domcontentloaded",
    });
    // best-effort consent acceptance (OneTrust, Quantcast, generic buttons)
    await page.waitForTimeout(2500);
    for (const sel of [
      "#onetrust-accept-btn-handler",
      "button[mode='primary']",
      ".qc-cmp2-summary-buttons button[mode='primary']",
      "button:has-text('Accept All')",
      "button:has-text('Accept all')",
      "button:has-text('I Accept')",
      "button:has-text('AGREE')",
      "button:has-text('Continue')",
    ]) {
      try {
        const b = page.locator(sel).first();
        if (await b.isVisible({ timeout: 400 })) {
          await b.click({ timeout: 1500 });
          break;
        }
      } catch {}
    }
    // scroll to trigger lazy-loaded ad slots
    for (let i = 0; i < 4; i++) {
      await page.mouse.wheel(0, 900);
      await page.waitForTimeout(1200);
    }
    await page.waitForTimeout(SETTLE_MS);
    // hop to one article page: ad units are denser there than on homepages
    try {
      const article = await page.evaluate(() => {
        const host = location.host;
        const links = [...document.querySelectorAll("a[href]")]
          .map((a) => a.href)
          .filter((h) => {
            try {
              const u = new URL(h);
              return u.host === host && u.pathname.length > 30 && !u.hash;
            } catch { return false; }
          });
        return links[0] ?? null;
      });
      if (article) {
        await page.goto(article, { timeout: PAGE_TIMEOUT_MS, waitUntil: "domcontentloaded" });
        for (let i = 0; i < 4; i++) {
          await page.mouse.wheel(0, 900);
          await page.waitForTimeout(1000);
        }
        await page.waitForTimeout(SETTLE_MS);
      }
    } catch {}
    })(), SITE_BUDGET_MS, site);
  } catch (err) {
    status = `error: ${String(err).slice(0, 120)}`;
  }

  let meta = {};
  try {
    meta = await budget(page.evaluate(() => {
      let p = window.pbjs;
      if (!p || !p.version) {
        // wrappers often rename the global; find anything that quacks like prebid
        for (const k of Object.getOwnPropertyNames(window)) {
          try {
            const v = window[k];
            if (v && typeof v === "object" && Array.isArray(v.installedModules) && typeof v.version === "string") {
              p = v;
              break;
            }
          } catch {}
        }
      }
      return {
        hasPbjs: !!p,
        pbjsGlobal: p ? (window.pbjs === p ? "pbjs" : "renamed") : null,
        pbjsVersion: p?.version ?? null,
        adUnits: Array.isArray(p?.adUnits) ? p.adUnits.length : null,
        events: (() => {
          try {
            const ev = p?.getEvents?.() ?? [];
            const c = {};
            for (const e of ev) c[e.eventType] = (c[e.eventType] ?? 0) + 1;
            return c;
          } catch { return null; }
        })(),
        bidResponses: (() => {
          try { return Object.keys(p?.getBidResponses?.() ?? {}).length; } catch { return null; }
        })(),
        bidders: p?.adUnits
          ? [...new Set(p.adUnits.flatMap((u) => (u.bids ?? []).map((b) => b.bidder)))].slice(0, 30)
          : null,
      };
    }), 15_000, "meta");
  } catch {
    meta = { hasPbjs: null };
  }

  write({ kind: "site-meta", site, ts: new Date().toISOString(), status, ...meta, ...counts });
  await context.close();
  return { site, status, ...meta, ...counts };
}

const browser = await chromium.launch({
  headless: true,
  args: ["--disable-blink-features=AutomationControlled"],
});
const queue = [...sites];
let done = 0;

async function worker() {
  while (queue.length) {
    const site = queue.shift();
    try {
      const r = await crawlSite(browser, site);
      done++;
      console.log(
        `[${done}/${sites.length}] ${site}: ${r.status} pbjs=${r.hasPbjs} v=${r.pbjsVersion ?? "-"} ortbReq=${r.ortbRequests} ortbResp=${r.ortbResponses}`
      );
    } catch (err) {
      done++;
      console.log(`[${done}/${sites.length}] ${site}: CRASH ${String(err).slice(0, 100)}`);
      write({ kind: "site-meta", site, status: `crash: ${String(err).slice(0, 120)}` });
    }
  }
}

await Promise.all(Array.from({ length: CONCURRENCY }, worker));
await browser.close();
out.end();
console.log(`\ncaptures written to ${outFile}`);
