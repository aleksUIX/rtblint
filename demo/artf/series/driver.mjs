import { pathToFileURL } from "node:url";
import { join } from "node:path";
import { output } from "./build.mjs";

export async function action(page, row, episode) {
  const a = row.action;
  const url = process.env[episode.number === 2 ? "ARTF_DEMO_URL" : "ARTF_LAB_URL"] || episode.url;
  if (a.type === "card") {
    await page.goto(pathToFileURL(join(output, `episode-${episode.number}.${a.kind}.html`)).href);
    await page.evaluate(() => document.fonts.ready);
    return;
  }
  if (["console", "lab"].includes(a.type)) {
    await page.goto(url, { waitUntil: "domcontentloaded" });
    if (episode.number === 2) {
      await page.waitForFunction(() => document.getElementById("env-lifecycle")?.textContent.includes("PUBLISHER"));
      await page.waitForFunction(() => document.getElementById("t-deals")?.textContent.includes("deal-premium"));
    } else {
      await page.waitForFunction(() => typeof window.artfRun === "function");
    }
    return;
  }
  await page.evaluate(() => {
    document.querySelectorAll(".recording-focus").forEach(el => el.classList.remove("recording-focus"));
    document.querySelectorAll(".feed-pane").forEach(el => el.classList.remove("feed-pane"));
    document.body.classList.remove("feed-detail");
  });
  if (episode.number === 2) await page.evaluate(() => {
    document.querySelectorAll(".env-row.on, .mut.on").forEach(el => el.classList.remove("on"));
    document.querySelectorAll(".path-on").forEach(el => el.classList.remove("path-on"));
    document.getElementById("envelope")?.classList.remove("focus-intents");
  });
  if (a.type === "env") {
    await page.evaluate(key => window.artfEnvFocus(key), a.key);
  } else if (a.type === "mut") {
    await page.evaluate(i => window.artfMutFocus(i), a.index);
  } else if (a.type === "focus") {
    await page.locator(a.selector).evaluate(el => el.classList.add("recording-focus"));
    if (episode.number === 2) await page.evaluate(selector => {
      if (selector === "#t-metrics") window.artfInspectApplied();
      if (selector === "#n-rtblint") window.artfInspectValidation();
    }, a.selector);
  } else if (a.type === "scene") {
    const receiptBefore = await page.evaluate(async () => (await (await fetch("/inbox")).json()).total);
    const response = page.waitForResponse(r => new URL(r.url()).pathname === "/run" && r.request().method() === "POST");
    await page.locator(`[data-beat="${a.beat}"]`).click();
    const result = await (await response).json();
    if (result.error) throw new Error(result.error);
    await page.waitForFunction(() => ![...document.querySelectorAll(".beat")].some(b => b.disabled), null, { timeout: 25000 });
    await page.waitForTimeout(150);
    const expected = { policy: "ad", illegal: "unsold", breaking: "poison", shade: "ad" }[a.beat];
    await page.waitForFunction(mode => document.getElementById("tv")?.dataset.mode === mode, expected);
    const receiptAfter = await page.evaluate(async () => (await (await fetch("/inbox")).json()).total);
    const check = (condition, message) => { if (!condition) throw new Error(`${a.beat}: ${message}`); };
    if (a.beat === "policy") {
      check(result.forwarded && result.applied.length === 4 && result.skipped.length === 1, "expected four accepted writes and one skipped intent");
      check(receiptAfter === receiptBefore + 1, "accepted request did not reach the DSP");
      check(result.stage.after.deals.find(d => d.id === "deal-premium")?.floor === 14.5, "premium floor did not become 14.50");
      check(result.stage.after.segments.includes("seg-sports") && result.stage.after.segments.includes("seg-premium-viewer"), "segments did not preserve and add");
    } else if (a.beat === "illegal") {
      check(!result.forwarded && receiptAfter === receiptBefore, "invalid response reached the DSP");
      check(result.stage.responseId !== result.stage.envelope.epId, "wrong response id is missing");
      check(result.applied.length === 0 && JSON.stringify(result.stage.before) === JSON.stringify(result.stage.after), "wrong-id response changed the auction before rejection");
    } else if (a.beat === "breaking") {
      check(!result.forwarded && receiptAfter === receiptBefore, "invalid applied object reached the DSP");
      check(result.stage.after.metrics.some(m => m.value === "high") && result.stage.findings.some(f => f.id.startsWith("openrtb.") && f.severity === "error"), "missing string metric or resulting OpenRTB error");
    } else if (a.beat === "shade") {
      check(result.forwarded && result.stage.bidBefore.price === 18.4 && result.stage.bidAfter.price === 12.88, "shading did not change 18.40 to 12.88");
      check(result.stage.bidBefore.crid === result.stage.bidAfter.crid && receiptAfter === receiptBefore, "response stage changed the creative or called the request DSP");
    }
    return { scene: a.beat, receiptBefore, receiptAfter, result };
  } else if (a.type === "lab-run") {
    const response = page.waitForResponse(r => new URL(r.url()).pathname === "/ad" && new URL(r.url()).searchParams.get("format") === "json");
    await page.evaluate(b => window.artfRun(b), a.beat);
    const result = await (await response).json();
    if (result.error) throw new Error(result.error);
    await page.waitForFunction(() => document.getElementById("outcomes")?.textContent.includes("ACTIVATE_SEGMENTS"));
    const shadedRows = await page.locator("#outcomes tr").filter({ hasText: "BID_SHADE" }).count();
    if (a.beat === "clean" && shadedRows !== 2) throw new Error(`Expected two shader outcomes, received ${shadedRows}`);
    if (a.beat === "clean") {
      const auction = result.auction;
      if (auction?.winner?.seat !== "dsp-beta") throw new Error("Clean auction must select beta's eligible bid");
      if (!auction.dropped?.some(row => row.bid?.seat === "dsp-alpha" && /floor/i.test(row.reason))) throw new Error("Clean auction must reject alpha below the deal floor");
      const requestAgents = new Set(result.hops.filter(hop => hop.stage === "request-artf").map(hop => hop.name));
      if (requestAgents.size !== 5) throw new Error("Clean auction must call five request specialists");
      if (result.hops.some(hop => hop.error)) throw new Error("Clean auction contains an unexpected failed hop");
    }
    return { scene: a.beat, result };
  } else if (a.type === "lab-view") {
    await page.evaluate(a => window.artfView(a.view, a.nodes), a);
    if (a.view === "manifest") await page.waitForFunction(() => document.getElementById("manifest")?.textContent.includes("curator-agent"));
  } else if (a.type === "lab-play") {
    const response = page.waitForResponse(r => new URL(r.url()).pathname === "/lab/api/play");
    await page.evaluate(() => window.artfPlay());
    const result = await (await response).json();
    if (result.error) throw new Error(result.error);
    await page.waitForFunction(() => document.getElementById("summary")?.textContent.includes("vast played"));
    return { scene: "delivery", result };
  } else if (a.type === "lab-traffic") {
    await page.evaluate(qps => window.artfTrafficStart(qps), a.qps);
    await page.waitForFunction(() => /completed [1-9]/i.test(document.getElementById("traffic-status")?.textContent || ""));
    return { requestedAuctionsPerSecond: a.qps, status: await page.locator("#traffic-status").innerText() };
  } else if (a.type === "lab-stop") {
    await page.evaluate(() => { window.artfTrafficStop(); window.artfView("apply"); });
  } else {
    throw new Error(`Unknown picture action: ${a.type}`);
  }
}

export async function reset(episode) {
  if (episode.number === 3) {
    const base = process.env.ARTF_LAB_URL || episode.url;
    for (let attempt = 0; attempt < 3; attempt++) {
      const response = await fetch(new URL("/ad?beat=clean&format=json", base));
      const result = await response.json();
      if (response.ok && result.auction?.winner?.seat === "dsp-beta" && !result.hops?.some(hop => hop.error)) return;
    }
    throw new Error("Recording preflight could not obtain a clean auction within the unchanged lab budgets");
  }
  if (episode.number !== 2) return;
  const base = process.env.ARTF_DEMO_URL || episode.url;
  const r = await fetch(new URL("/reset", base), { method: "POST" });
  if (!r.ok) throw new Error(`Reset failed: HTTP ${r.status}`);
}
