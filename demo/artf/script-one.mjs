// Source of truth for episode 1 picture + prompter + VO-ONE.md.
// Prompter SCRIPT in teleprompter/one.html must use these seconds and texts.
// record-one.mjs holds each body beat to exactly `seconds`.
// wrap-cards adds intro 13s + outro 15s around the body.

export const INTRO_S = 13;
export const OUTRO_S = 15;

export const SCRIPT = [
  {
    id: "intro",
    title: "Intro card",
    beats: [
      { seconds: 13, silent: true, text: "(intro card. no voice. start the camera, then wait for the console.)" },
    ],
  },
  {
    id: "who",
    title: "Who",
    beats: [
      {
        seconds: 10.6,
        cue: "who",
        text: "This is episode one of a series on ARTF. This demo is the hop: an agent inside the host, patching this auction.",
      },
    ],
  },
  {
    id: "open",
    title: "Open · living room",
    beats: [
      { seconds: 10.6, cue: "say", text: "This is the auction a CTV app is about to run: a mid-roll, two private deals, a floor already on the impression." },
      { seconds: 8.5, cue: "say", text: "OpenRTB is the object every SSP and DSP already speaks. ARTF does not replace it." },
      { seconds: 11.0, cue: "say", text: "IAB Tech Lab published ARTF so an agent can take part in this auction from inside the host that is already running it." },
    ],
  },
  {
    id: "map",
    title: "Where the hop sits",
    beats: [
      { seconds: 14.2, cue: "map-open", where: "grafana-top", text: "That host is an SSP, a DSP, or an exchange. You deploy the agent as a container in your own infrastructure, and you call it while the bid is still moving." },
      { seconds: 8.5, cue: "say", where: "grafana-top", text: "The bidstream stays in your infrastructure. The agent runs next to this ticket." },
      { seconds: 12.2, cue: "say", where: "grafana-top", text: "You keep the timing, and you keep the decision of which changes actually land." },
      { seconds: 11.4, cue: "say", where: "grafana-top", text: "The agent returns mutations: patches against this OpenRTB object. The orchestrator accepts or rejects each one. Buyers then bid on whatever you actually forwarded." },
      { seconds: 14.6, cue: "map-last", where: "grafana-top", text: "The same hop can run again after a bid comes back, this time on the BidResponse. What ARTF does not cover is everything after a winner is chosen: VAST, the player, measurement." },
    ],
  },
  {
    id: "curate",
    title: "DSP · curation lands",
    beats: [
      { seconds: 9.4, cue: "say", text: "This is the request-side hop a buyer actually feels. Watch the ticket, not the dummy, while the patch lands." },
      { seconds: 7, silent: true, cue: "hop-clean", text: "(scene 1 lands. Watch deals, segments, floor, then the dummy.)" },
      { seconds: 13.4, cue: "say", text: "The DSP did not bid on the publisher's original ticket. It bid on the one the agent wrote: extra segments, a new deal, a higher floor, a numeric viewability." },
      { seconds: 11.0, cue: "say", text: "The dummy on the TV is what that bid is holding. It is not VAST, and ARTF never gets as far as delivery." },
    ],
  },
  {
    id: "suppress",
    title: "Publisher · a deal leaves",
    beats: [
      { seconds: 9.0, cue: "say", text: "Same starting ticket, a second job. From the publisher seat, taking a deal off can be the product." },
      { seconds: 7, silent: true, cue: "hop-suppress", text: "(scene 2 lands. deal-standard strikes through.)" },
      { seconds: 11.0, cue: "say", text: "deal-standard just left the wire, and the DSP never saw it. A private marketplace can look like this: only the premium deal remains." },
    ],
  },
  {
    id: "shade",
    title: "DSP · the bid is shaded",
    beats: [
      { seconds: 9.0, cue: "say", text: "Third job, not the bid you just watched. After a DSP answers, a container can shade the price." },
      { seconds: 7, silent: true, cue: "hop-shade", text: "(scene 3 lands. $18.40 struck, $12.88 lands.)" },
      { seconds: 8.5, cue: "say", text: "$18.40 comes in, $12.88 goes out. The creative did not change. The bid object did." },
      { seconds: 8.5, cue: "say", text: "The request hop was PUBLISHER_BID_REQUEST. This is DSP_BID_RESPONSE. Same container model, later in the auction." },
    ],
  },
  {
    id: "volume",
    title: "Volume · mixer",
    beats: [
      { seconds: 10.6, cue: "say", text: "One slot is the story. A bidstream is the job. This is a laptop running the same hop, not a colo POP." },
      { seconds: 5, silent: true, cue: "mixer-start", text: "(mixer starts. production mix. wait for the barcode.)" },
      { seconds: 8.5, cue: "say", text: "The mixer is set to five thousand a second. Grafana is the count." },
      { seconds: 9.0, cue: "say", text: "Mostly clean curation, with some suppress and some shade: the jobs that have always been tight on time." },
      { seconds: 8.5, cue: "say", text: "Green on the barcode is a forward. Drop should sit at zero on this mix." },
      { seconds: 6, silent: true, cue: "hose", text: "(hose fills. do not talk. Grafana is next.)" },
      { seconds: 8.5, cue: "say", text: "Grafana is the unsampled view of the same hop." },
    ],
  },
  {
    id: "grafana",
    title: "Grafana",
    beats: [
      { seconds: 6, silent: true, cue: "grafana", text: "(dashboard loads. kiosk, last one minute.)" },
      { seconds: 8.5, cue: "say", where: "grafana", text: "Auctions per second is the mixer, as a raw count, not a claim about anyone's production." },
      { seconds: 9.0, cue: "say", where: "grafana", text: "Forward rate should sit near one hundred percent here. Suppress and shade still forward; those are legal jobs." },
      { seconds: 9.4, cue: "say", where: "grafana", text: "Hop p99 has to live far under the 150 millisecond tmax, because this hop sits inside the auction clock." },
      { seconds: 10.6, cue: "say", where: "grafana", text: "Headroom is whatever is left for the rest of the bid. The mix is clean, suppress, and shade. Synthetic. Same hop, many times a second." },
    ],
  },
  {
    id: "close",
    title: "Close",
    beats: [
      { seconds: 4, silent: true, cue: "back", text: "(back on the console.)" },
      { seconds: 17.0, cue: "say", text: "That is ARTF in the stack you already run: a container beside the exchange, a patch on OpenRTB, a DSP bidding on what you forwarded, and the same hop after the bid comes back. Independent simulation. Not an IAB product." },
    ],
  },
  {
    id: "outro",
    title: "Outro card",
    beats: [
      { seconds: 15, silent: true, text: "(outro card. no voice. let it hold, then stop.)" },
    ],
  },
];

export function bodyBeats() {
  return SCRIPT.filter((c) => c.id !== "intro" && c.id !== "outro").flatMap((c) =>
    c.beats.map((b) => ({ ...b, clip: c.id })),
  );
}

export function totalSeconds() {
  return SCRIPT.reduce((sum, c) => sum + c.beats.reduce((s, b) => s + b.seconds, 0), 0);
}
