// OpenRTB as a trader reads it. The CTV stage paints this, not the raw JSON.

export const DUMMIES = {
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

export function fromBidRequest(req) {
  const imp = req?.imp?.[0] || {};
  const video = imp.video || {};
  const deals = Array.isArray(imp.pmp?.deals)
    ? imp.pmp.deals.map((d) => ({
        id: d.id,
        floor: d.bidfloor,
        at: d.at,
      }))
    : [];
  const segments = [];
  for (const data of req?.user?.data || []) {
    for (const seg of data.segment || []) {
      if (seg?.id) segments.push(seg.id);
    }
  }
  return {
    id: req?.id || "",
    domain: req?.site?.domain || "",
    page: req?.site?.page || "",
    content: req?.site?.content?.id || "",
    w: video.w || 0,
    h: video.h || 0,
    min: video.minduration || 0,
    max: video.maxduration || 0,
    floor: imp.bidfloor ?? null,
    privateAuction: Boolean(imp.pmp?.private_auction),
    deals,
    segments,
    metrics: Array.isArray(imp.metric)
      ? imp.metric.map((m) => ({
          type: m.type,
          value: m.value,
          vendor: m.vendor,
        }))
      : [],
  };
}

export function fromBidResponse(res) {
  const seat = res?.seatbid?.[0];
  const bid = seat?.bid?.[0];
  if (!bid) return null;
  const crid = bid.crid || "";
  return {
    seat: seat.seat || "",
    bidId: bid.id || "",
    price: bid.price,
    crid,
    dur: bid.dur || 15,
    w: bid.w || 1920,
    h: bid.h || 1080,
    advertiser: bid.adomain?.[0] || "",
    dummy: DUMMIES[crid] || null,
  };
}

export function parseJson(raw) {
  if (!raw || typeof raw !== "string") return null;
  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

export function pickDummy(ticket) {
  const deals = ticket?.deals || [];
  const ids = new Set(deals.map((d) => d.id));
  const segs = new Set(ticket?.segments || []);
  if (ids.has("deal-curated") || segs.has("seg-premium-viewer")) {
    return DUMMIES["dummy-northwind"];
  }
  if (ids.has("deal-premium") && !ids.has("deal-standard")) {
    return DUMMIES["dummy-acme"];
  }
  return DUMMIES["dummy-river"];
}

export function annotateMutations(mutations, issues, applied, skipped) {
  const list = Array.isArray(mutations) ? mutations : [];
  const findings = Array.isArray(issues) ? issues : [];
  const appliedSet = new Set((applied || []).map((n) => Number(n)));
  const skippedSet = new Set((skipped || []).map((n) => Number(n)));
  return list.map((m, i) => {
    const mine = findings.filter((iss) => {
      const path = iss.path || "";
      return path === `mutations[${i}]` || path.startsWith(`mutations[${i}].`);
    });
    const errors = mine.filter((iss) => iss.severity === "error");
    let status = "proposed";
    if (errors.length) status = "bounced";
    else if (skippedSet.has(i)) status = "skipped";
    else if (appliedSet.has(i)) status = "applied";
    return {
      index: i,
      intent: m.intent || "",
      op: m.op || "",
      path: m.path || "",
      status,
      findings: mine,
    };
  });
}
