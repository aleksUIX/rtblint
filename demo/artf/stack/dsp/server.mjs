#!/usr/bin/env node
// Downstream DSP. Receives a bid request only if the SSP forwarded it.
// Returns a dummy 15s CTV bid so the console can show "what would have
// played." Not a bidder. Not delivery. Not VAST playback.

import { createServer } from "node:http";

const PORT = Number(process.env.PORT || 8092);
const inbox = [];
let total = 0;

const DUMMIES = {
  "dummy-northwind": {
    id: "dummy-northwind",
    title: "Northwind Spark",
    kicker: "Sports drink · 15s mid-roll",
    line: "Stay in the game.",
    advertiser: "northwind.example",
    tone: "green",
  },
  "dummy-acme": {
    id: "dummy-acme",
    title: "Acme Cover",
    kicker: "Insurance · 15s mid-roll",
    line: "What if the next play costs more.",
    advertiser: "acme-cover.example",
    tone: "amber",
  },
  "dummy-river": {
    id: "dummy-river",
    title: "Riverstream Originals",
    kicker: "House promo · 15s mid-roll",
    line: "Tonight, only here.",
    advertiser: "streaming.example",
    tone: "blue",
  },
};

function pickDummy(request) {
  const deals = request.imp?.[0]?.pmp?.deals || [];
  const ids = new Set(deals.map((d) => d.id));
  const segs = new Set();
  for (const data of request.user?.data || []) {
    for (const seg of data.segment || []) if (seg?.id) segs.add(seg.id);
  }
  if (ids.has("deal-curated") || segs.has("seg-premium-viewer")) {
    return DUMMIES["dummy-northwind"];
  }
  if (ids.has("deal-premium") && !ids.has("deal-standard")) {
    return DUMMIES["dummy-acme"];
  }
  return DUMMIES["dummy-river"];
}

function bidPrice(request) {
  const deals = request.imp?.[0]?.pmp?.deals || [];
  const floors = deals.map((d) => d.bidfloor).filter((n) => typeof n === "number");
  const floor = floors.length ? Math.max(...floors) : Number(request.imp?.[0]?.bidfloor) || 8;
  return Math.round((floor + 2.4) * 100) / 100;
}

const server = createServer((req, res) => {
  const url = new URL(req.url || "/", `http://${req.headers.host}`);
  res.setHeader("Access-Control-Allow-Origin", "*");

  if (req.method === "GET" && url.pathname === "/health") {
    json(res, 200, { ok: true, role: "dsp", received: total });
    return;
  }

  if (req.method === "GET" && url.pathname === "/inbox") {
    json(res, 200, { received: total, total, last: inbox[inbox.length - 1] ?? null });
    return;
  }

  if (req.method === "POST" && url.pathname === "/bid") {
    readBody(req).then((raw) => {
      let request;
      try {
        request = JSON.parse(raw);
      } catch {
        json(res, 400, { error: "bid request is not JSON" });
        return;
      }
      const dummy = pickDummy(request);
      const price = bidPrice(request);
      const rec = {
        at: new Date().toISOString(),
        auction: request.id ?? null,
        imps: Array.isArray(request.imp) ? request.imp.map((i) => i.id) : [],
        deals: request.imp?.[0]?.pmp?.deals?.map((d) => ({ id: d.id, bidfloor: d.bidfloor })) ?? [],
        metrics: request.imp?.[0]?.metric ?? [],
        segments: request.user?.data?.[0]?.segment ?? [],
        creative: dummy.id,
        price,
      };
      inbox.push(rec);
      if (inbox.length > 8) inbox.shift();
      total += 1;
      json(res, 200, {
        id: request.id,
        cur: "USD",
        seatbid: [
          {
            seat: "dsp-1",
            bid: [
              {
                id: "bid-1",
                impid: rec.imps[0] || "1",
                price,
                crid: dummy.id,
                adomain: [dummy.advertiser],
                w: 1920,
                h: 1080,
                dur: 15,
                mtype: 2,
              },
            ],
          },
        ],
        dummy,
      });
    });
    return;
  }

  json(res, 404, { error: "not found" });
});

function readBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on("data", (c) => chunks.push(c));
    req.on("end", () => resolve(Buffer.concat(chunks).toString("utf8")));
    req.on("error", reject);
  });
}

function json(res, status, body) {
  res.writeHead(status, { "content-type": "application/json" });
  res.end(JSON.stringify(body));
}

server.listen(PORT, "0.0.0.0", () => {
  console.log(`dsp listening on ${PORT}`);
});
