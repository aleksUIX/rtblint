#!/usr/bin/env node
// Curation / shading agent. Simulated ARTF service: given a beat, return the
// matching mutation set. HTTP JSON on purpose; we do not vendor the ARTF proto.
// Payloads are the fixtures the linter already tests, plus two demo scenes.

import { readFileSync } from "node:fs";
import { createServer } from "node:http";
import { join } from "node:path";

const PORT = Number(process.env.PORT || 8091);
const FIX = process.env.FIXTURES || "/fixtures";

const BEATS = {
  illegal: "invalid-mutations.json",
  breaking: "breaking-mutations.json",
  clean: "valid-mutations.json",
  policy: "policy-mutations.json",
  suppress: "suppress-mutations.json",
  shade: "shade-mutations.json",
};

const server = createServer((req, res) => {
  const url = new URL(req.url || "/", `http://${req.headers.host}`);
  res.setHeader("Access-Control-Allow-Origin", "*");

  if (req.method === "GET" && url.pathname === "/health") {
    json(res, 200, { ok: true, role: "agent", beats: Object.keys(BEATS) });
    return;
  }

  if (req.method === "GET" && url.pathname === "/mutate") {
    const beat = url.searchParams.get("beat") || "clean";
    const file = BEATS[beat];
    if (!file) {
      json(res, 400, { error: `unknown beat: ${beat}`, beats: Object.keys(BEATS) });
      return;
    }
    const mutations = JSON.parse(readFileSync(join(FIX, file), "utf8"));
    json(res, 200, {
      beat,
      agent: beat === "shade" ? "shader-1" : "curator-1",
      intent_count: Array.isArray(mutations.mutations) ? mutations.mutations.length : 0,
      mutations,
    });
    return;
  }

  json(res, 404, { error: "not found" });
});

function json(res, status, body) {
  res.writeHead(status, { "content-type": "application/json" });
  res.end(JSON.stringify(body));
}

server.listen(PORT, "0.0.0.0", () => {
  console.log(`agent listening on ${PORT}`);
});
