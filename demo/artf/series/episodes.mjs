// One timeline drives narration, prompters, subtitles, cues, and picture.
// Short narration leaves time to inspect each result.
const beat = (chapter, action, text, seconds) => ({ chapter, action, text, seconds });
const hold = (chapter, action, seconds, cue) => ({ chapter, action, seconds, cue, text: "" });

export const episodes = {
  2: {
    number: 2,
    title: "The host decides which changes reach the auction.",
    description: "Follow an agent's proposal to the OpenRTB object a buyer receives.",
    narrationWpm: 110,
    speaker: { name: "Alex Sekowski", label: "Now presenting:", portrait: "../../linkedin/author/alex.jpg" },
    url: "http://localhost:8080/?ep=2",
    beats: [
      hold("Opening", {"type":"card","kind":"intro"}, 8,
        "Title card. No voice."),
      beat("The host turns proposals into auction changes.", {"type":"console"},
        "OpenRTB carries bid requests and responses. ARTF, the Agentic Real Time Framework, lets a host call specialist services during the auction. Let's see how their proposals become changes buyers see.", 20),
      beat("The host gives a specialist access to the auction object.", {"type":"env","key":"lifecycle"},
        "The exchange is our host. The agent is a specialist service inside its infrastructure. Here it returns fixed proposals. The envelope wraps OpenRTB so the host can request changes.", 20),
      beat("The call has an identity and a time budget.", {"type":"env","key":"id"},
        "This call id is separate from the auction id. Matching it ties the answer to the right call. The time budget is one hundred fifty milliseconds; a late answer can miss the auction.", 22),
      beat("The host offers specific work for this stage.", {"type":"env","key":"intents"},
        "Intents name the jobs this host offers. They limit what the agent can change. Here that's segments, deals, floors, and metrics. Shading waits until there's a bid.", 19),
      hold("The host evaluates the proposals.", {"type":"scene","beat":"policy"}, 6,
        "Run Host policy. Let the cards and DSP receipt finish."),
      beat("Audience segments can affect which campaigns consider the impression.", {"type":"mut","index":0},
        "A mutation is a proposed change, with an operation, target, and payload. Here it adds audience segments. Those labels can affect which campaigns consider this impression. The host chooses whether to apply it.", 21),
      beat("A higher deal floor can change which bids qualify.", {"type":"mut","index":2},
        "This floor change raises the premium deal's minimum price from twelve dollars to fourteen fifty per thousand impressions. That changes which bids qualify. A higher floor can also leave the impression without an eligible bid.", 24),
      beat("Independent acceptance lets the host keep useful proposals.", {"type":"mut","index":4},
        "Content id wasn't offered here, so it's skipped. The DSP, the buyer's platform, still receives four accepted changes. Independent acceptance lets the host keep useful work while enforcing its own rules.", 24),
      hold("A response can name the wrong call.", {"type":"scene","beat":"illegal"}, 6,
        "Run Wrong response. Wait for the rejection and the unchanged DSP receipt count."),
      beat("A returned response still needs to address this call.", {"type":"mut","index":0},
        "This answer names the wrong call. The host rejects the response before applying any changes. The original auction stays unchanged, and nothing new reaches the buyer. A usable proposal can't bypass that identity check.", 23),
      beat("A supported intent needs a defined apply rule.", {"type":"mut","index":5},
        "Deal margin shows an integration gap. This host has no standard OpenRTB field for it. An intent needs an agreed mapping, or the host cannot apply the proposal.", 20),
      hold("The resulting object needs its own check.", {"type":"scene","beat":"breaking"}, 6,
        "Run Applied object. Hold on the red viewability metric and the OpenRTB finding."),
      beat("The JSON adapter produces an invalid metric.", {"type":"focus","selector":"#t-metrics"},
        "Here, viewability is the word high. ARTF's typed metric requires a number. Our HTTP JSON fixture introduces this fault. The resulting request gives the buyer a value it cannot reliably use.", 22),
      beat("Checking the assembled request contains an integration error.", {"type":"focus","selector":"#t-metrics"},
        "The mutation was applied, but the request it produced is invalid. That's why we check the assembled OpenRTB too. The host stops this error here, before a buyer has to handle it.", 23),
      beat("A shared interface still needs the host's business rules.", {"type":"focus","selector":"#n-rtblint"},
        "RTBlint performs these checks in our demo. ARTF provides the shared interface. The host supplies the business rules. That allows specialists to integrate through a common contract while the platform controls the auction.", 20),
      hold("The response stage carries a different object.", {"type":"scene","beat":"shade"}, 6,
        "Run Bid response. Let the envelope and the bid price settle."),
      beat("Shading changes the price competing in the auction.", {"type":"env","key":"attached"},
        "In this separate bid response example, eighteen forty becomes twelve eighty eight for the same creative. Shading changes the price competing in the auction. The exchange still decides whether the bid qualifies.", 22),
      beat("Several agents will still need one host decision.", {"type":"env","key":"intents"},
        "OpenRTB carries the auction data. ARTF gives specialists a common way to propose changes, with the host in control. Next, we'll coordinate several agents around one auction.", 19),
      hold("Closing", {"type":"card","kind":"outro"}, 8,
        "Closing card. No voice."),
    ],
  },
  3: {
    number: 3,
    title: "Several agents can contribute to one auction.",
    description: "See how specialists share an auction clock and keep separate responsibilities.",
    narrationWpm: 110,
    speaker: { name: "Alex Sekowski", label: "Now presenting:", portrait: "../../linkedin/author/alex.jpg" },
    url: "http://localhost:8081/lab/?ep=3",
    beats: [
      hold("Opening", {"type":"card","kind":"intro"}, 8,
        "Title card. No voice."),
      beat("One auction can use several specialists.", {"type":"lab"},
        "Last time, we checked one agent's proposals. ARTF, the Agentic Real Time Framework, lets hosts call specialist services during an auction. Now let's follow several specialists working on the same OpenRTB request.", 23),
      hold("The host calls the request agents together.", {"type":"lab-run","beat":"clean"}, 5,
        "Run the clean auction. Keep the graph and the request apply table visible."),
      beat("Parallel calls share the request deadline.", {"type":"lab-view","view":"apply","nodes":["ssp","audience","curator","metrics","quality","cids"]},
        "The SSP, the seller's platform, calls five specialists together. Each gets the starting envelope, which wraps the auction data. They share one deadline. Parallel calls keep us from waiting for each service in sequence.", 24),
      beat("The familiar curation job is split across agents.", {"type":"lab-view","view":"ticket","nodes":["ssp","curator"]},
        "Here's the request before and after. Audience adds segments; curator adds deals and raises the premium floor. Those changes can affect which campaigns consider the impression and which bids qualify. The host combines accepted proposals.", 24),
      beat("Each accepted write can be traced to its agent.", {"type":"lab-view","view":"apply","nodes":["audience","metrics","quality","cids"]},
        "Metrics and quality add measurements. Content ids describe the programme, while audience segments describe viewer groups. Each accepted write names its source agent. If a buyer receives unexpected data, we can trace who proposed it.", 24),
      beat("Parallel execution still needs a conflict policy.", {"type":"lab-view","view":"apply","nodes":["ssp"]},
        "What if two agents propose different floors for the same deal? This lab applies answers in configured order, mostly to separate fields. A production host needs a conflict policy. Response speed shouldn't decide a price.", 24),
      beat("Each buyer applies its own shading decision.", {"type":"lab-view","view":"bids","nodes":["dsp-alpha","dsp-beta"]},
        "Now the request reaches two buyers, the DSPs. Each makes a bid and calls its own shader through GetMutations, ARTF's proposal interface. Shading adjusts the offered price before the bid returns to the exchange.", 24),
      beat("The container's access follows its role.", {"type":"lab-view","view":"hops","nodes":["shader-alpha","shader-beta"]},
        "Alpha's shader and beta's shader sit on separate internal networks. The request agents sit with the seller. In this lab, one shader has no direct route to the other buyer. The host controls that access.", 24),
      beat("A changed price still has to qualify for the auction.", {"type":"lab-view","view":"bids","nodes":["ssp","dsp-alpha","dsp-beta"]},
        "Alpha's shaded bid falls below the deal floor, so it's rejected. Beta remains eligible and wins this example. A shader can change the price, but the exchange still checks eligibility and selects the highest eligible bid.", 24),
      beat("The host must decide what happens without an answer.", {"type":"lab-view","view":"hops","nodes":["ssp","shader-alpha","shader-beta"]},
        "A deadline needs a fallback. Here, a missing request agent's answer is skipped; a failed shader leaves the original bid. These are lab policies. If a service performs a required check, continuing may be inappropriate.", 24),
      beat("The descriptor helps an operator deploy the agent.", {"type":"lab-view","view":"manifest","nodes":["curator"]},
        "This sample descriptor declares the curator's image, capabilities, and operating needs. It helps explain deployment. The envelope still limits the jobs offered on each call. Declaring a capability doesn't grant unlimited auction access.", 24),
      hold("Delivery starts after the auction decision.", {"type":"lab-play"}, 5,
        "Play VAST once. Hold on the graph and the tracker events."),
      beat("Delivery begins after the ARTF auction work.", {"type":"lab-view","view":"events","nodes":["cdn","tracker"]},
        "In this demo, ARTF's auction calls are finished. VAST now describes the ad and its tracking. We fetch dummy media and fire scripted events. Those requests don't prove a real viewer watched the ad.", 24),
      beat("Measure the path that this traffic driver exercises.", {"type":"lab-traffic","qps":20},
        "Let's repeat this path under load. The driver requests twenty auctions a second; monitoring shows the rate actually achieved. One auction can call several agents, so auction rate and agent call rate measure different things.", 24),
      beat("Read latency alongside the host's deadlines.", {"type":"lab-view","view":"ops","nodes":["audience","curator","shader-alpha","shader-beta"]},
        "p95 estimates the time within which ninety-five percent of measured calls finish. Read it with the auction path and buyer call failures. Fast typical calls don't tell us whether every deadline was met.", 25),
      beat("Coordination still needs the validation from episode two.", {"type":"lab-view","view":"ops","nodes":["ssp"]},
        "Typed calls still need host checks. Here we check call identity and offered intents before combining proposals. RTBlint checks our captured request separately. A deployed host still needs OpenRTB validation before forwarding.", 24),
      beat("Trace one complete auction before increasing traffic.", {"type":"lab-stop"},
        "I'd start with one auction: follow a proposal to the field the buyer receives. Then test a missing answer and a conflicting proposal. ARTF supplies the shared interface; each host remains responsible for its decisions.", 24),
      hold("Closing", {"type":"card","kind":"outro"}, 8,
        "Closing card. No voice."),
    ],
  },
};

export function timeline(episode) {
  let at = 0;
  return episode.beats.map((b, index) => {
    const row = { ...b, index, at, end: at + b.seconds };
    if (b.text && episode.narrationWpm) {
      row.voiceAt = at + 1;
      row.voiceEnd = row.voiceAt + b.text.trim().split(/\s+/).length * 60 / episode.narrationWpm;
    }
    at = row.end;
    return row;
  });
}
export const totalSeconds = (episode) => episode.beats.reduce((s, b) => s + b.seconds, 0);
