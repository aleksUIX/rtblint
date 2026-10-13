import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { presenterBadge, presenterCSS } from "./captions.mjs";

const demo = join(dirname(fileURLToPath(import.meta.url)), "..");
const referenceCSS = readFileSync(join(demo, "cards.html"), "utf8").match(/<style>([\s\S]*?)<\/style>/)[1];
const esc = text => text.replaceAll("&", "&amp;").replaceAll("<", "&lt;").replaceAll('"', "&quot;");
const scenes = {
  2: {
    lede: "ARTF gives hosted specialists a shared way to propose auction changes. Follow how the host checks those proposals and controls the OpenRTB sent to buyers.",
    steps: [
      ["The host assigns the call.", "Call ids and budgets tie the work to this auction.", "artf"],
      ["The agent proposes changes.", "Each proposal names an operation, target, and payload.", ""],
      ["The host chooses each change.", "Four proposals are accepted and an unoffered job is skipped.", "ok"],
      ["The result needs its own check.", "Invalid OpenRTB is caught before it reaches buyers.", "drop"],
      ["Shading runs after the bid.", "The price falls from $18.40 to $12.88 for the same creative.", "artf"],
    ],
    takeaways: [
      "OpenRTB carries auction data; ARTF defines how agents propose changes.",
      "Segments and deal floors can affect bidder decisions.",
      "The host chooses which changes to apply under its own rules.",
      "Checking the assembled request contains integration errors.",
      "Shading changes price; the exchange still decides bid eligibility.",
    ],
    next: "Next, see how the host coordinates several agents.",
    nextBody: "Several specialists share one auction clock and contribute to the object the host forwards.",
  },
  3: {
    lede: "Follow five request specialists and two buyers through one auction. The host combines their proposals, controls deadlines, and selects an eligible bid.",
    steps: [
      ["Specialists answer together.", "Five request agents share one starting envelope.", "artf"],
      ["The host combines their writes.", "Trace each accepted field to the agent that proposed it.", ""],
      ["Each buyer shades its own bid.", "Separate shaders work on separate responses.", "artf"],
      ["The host selects an eligible bid.", "A shaded price still has to meet the deal floor.", "ok"],
      ["Operations follow the same path.", "Observe achieved traffic, latency, and buyer call failures.", ""],
    ],
    takeaways: [
      "Parallel calls share the host's deadline.",
      "Accepted fields remain traceable to their source agents.",
      "Each buyer controls its own response and shader.",
      "Missing answers and conflicting writes need host policies.",
      "Trace one auction before increasing traffic.",
    ],
    next: "Trace one complete auction before increasing traffic.",
    nextBody: "Follow an accepted write to the buyer, then examine a missing answer and a conflicting proposal.",
  },
};

export function card(episode, kind) {
  const spec = scenes[episode.number];
  const title = kind === "intro" ? episode.title : "The host controls what reaches the next participant.";
  const content = kind === "intro"
    ? `<p class="lede">${esc(spec.lede)}</p><div class="hops">${spec.steps.map(([title, body, state]) => `<div class="hop ${state}"><strong>${esc(title)}</strong><span>${esc(body)}</span></div>`).join("")}</div><p class="band">${episode.number === 2 ? "The host checks the object it will send onward." : "The host checks each response before combining proposals."}</p>`
    : `<div class="lines">${spec.takeaways.map((line, i) => `<div class="line"><b>${String(i + 1).padStart(2, "0")}</b><p>${esc(line)}</p></div>`).join("")}</div><div class="closer"><h2>${esc(spec.next)}</h2><p>${esc(spec.nextBody)}</p></div>`;
  const css = referenceCSS.replaceAll('./linkedin/', '../../linkedin/');
  const byline = episode.speaker
    ? `<a class="presenter-link" href="https://www.linkedin.com/in/aleksandersekowski">linkedin.com/in/aleksandersekowski</a>`
    : `<div class="byline"><div class="who"><span class="role">Author</span><span class="name">Alex Sekowski</span><a href="https://www.linkedin.com/in/aleksandersekowski">linkedin.com/in/aleksandersekowski</a></div><img src="../../linkedin/author/alex.jpg" alt="Alex Sekowski" width="92" height="92"></div>`;
  let html = `<!doctype html><html lang="en"><head><meta charset="utf-8"><title>ARTF episode ${episode.number}</title><style>${css}
html,body{background:#111}.card h1{margin:0;max-width:1250px}.card.outro h1{font-size:60px;max-width:1500px}.hop strong{margin:0;font-size:27px}.hop{min-height:168px}.lines{margin-top:30px}.line p{font-size:29px}.episode-meta{font:400 15px/1.4 var(--mono);color:var(--artf);margin:0 0 5px}.foot .credit{max-width:1000px}.closer h2{font-size:36px}
${episode.speaker ? `${presenterCSS}.foot{min-height:111px}.presenter-link{font:400 16px/1.4 var(--mono);color:var(--artf);text-decoration:none}` : ""}
</style></head><body><main class="card ${kind}"><h1>${esc(title)}</h1>${content}<footer class="foot"><div class="credit"><p class="episode-meta">ARTF · Episode ${episode.number} of 3</p><span>${episode.number === 2 ? "Independent simulation. JSON fixture agent. <em>Real RTBlint gRPC.</em>" : "Independent simulation. Fixed business rules. <em>Real typed GetMutations calls.</em>"}</span></div>${byline}</footer></main>${episode.speaker ? presenterBadge(episode.speaker) : ""}</body></html>`;
  if (episode.number === 3) {
    html = html.replace(/url\(["']\.\.\/\.\.\/linkedin\/fonts\/([^"']+)["']\)/g, (_, name) => `url("data:font/woff2;base64,${readFileSync(join(demo, "linkedin/fonts", name)).toString("base64")}")`);
    const portrait = `data:image/jpeg;base64,${readFileSync(join(demo, "linkedin/author/alex.jpg")).toString("base64")}`;
    html = html.replaceAll("../../linkedin/author/alex.jpg", portrait);
  }
  return html;
}
