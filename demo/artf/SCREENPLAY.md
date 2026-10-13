# Pitch screenplay: Episode 2, five views of one CTV slot

Series: 1 `SCREENPLAY-ONE.md` · 2 this cut · 3 `SCREENPLAY-THREE.md` ·
4 `SCREENPLAY-FOUR.md`.

Episode 1 (`SCREENPLAY-ONE.md`, `artf-sim-one.mp4`) is the intro to the hop.
This cut is the missing check. Same compose. Same binary.

Use this live (workshop / table / 10-minute block) and as captions on the five-minute reel.
Independent implementation. Not an IAB product. Do not say "production traffic."
Say: synthetic mix, real guardrail, real gRPC.

The 90s send-ahead still exists (`artf-sim-pitch.mp4`, `record-pitch.mjs`). This cut is
the one that teaches the check.

Reel: `artf-sim-five.mp4` (`node record-five.mjs` while compose is up).
Prompter: `teleprompter/index.html` (`npm run prompter`). Same beats as the captions. Bookends are silent.

Honesty, once, out loud:

> The TV is a dummy. The SSP, agent, and DSP are stubs. The ticket is the OpenRTB
> the host would forward. RTBlint is the real process. ARTF stops at the auction.
> Nothing here is delivery.

---

## 0. Open · publisher / living room (0:00–0:40)

**Do not click yet.** Console at rest. Perspective rail on **Publisher / CTV**.
TV: *Night Shift S3 E4*, progress bar into a mid-roll. Ticket already filled.

**Point at the ticket, in this order:**

1. 1920×1080, 15–30s, `streaming.example`. That is a CTV mid-roll.
2. Private marketplace. Two deals: `deal-premium` at $12, `deal-standard` at $6.
3. User already has `seg-sports`. Floor on the imp is $8.
4. No bid yet. The show is still playing.

**Say:**

This is the auction a CTV app is about to run: a mid-roll, two private deals, a
floor already on the impression. OpenRTB is the object every SSP and DSP already
speaks. ARTF lets a container beside the exchange rewrite that object before any
buyer sees it. What v1.0 never asks is whether the auction is still valid
OpenRTB afterwards. That hop, slowed down, is what we are about to walk.

---

## 1. SSP view · looks legal, is not (0:40–1:25)

**Click scene 1.** Let mutations land as cards. Do not rush the rule IDs.

TV stays on the show, then **NO BID**. DSP inbox stays 0. The ticket may still
show a ghost write: that is the applied pass reporting what would have landed.

**Point at the mutation cards:**

1. `BID_SHADE` on a publisher-request lifecycle. `artf.mutation.intent_not_applicable`.
   Shading is a bid-response intent. Wrong stage. Scene 5 is that stage.
2. `artf.response.id_mismatch`. The agent echoed the bid request id, not the
   extension-point id. The reference implementation made this mistake.
3. Unknown impression `imp-404`, unknown deal `deal-unlisted`. The auction does
   not contain them. A host that forwards this writes into nothing, or into the
   wrong object.
4. `artf.mutation.legacy_spec_encoding`. Document vocabulary, not proto. This
   one is mapped, not discarded: `deal-standard` would have moved $6 → $7.25.
   We still drop the whole set. Independent acceptability is not "forward the
   ones that worked."
5. `ADJUST_DEAL_MARGIN` has no OpenRTB field. Intent exists. Nowhere to write.

**Say:**

If you sit at an SSP, this is the nightmare in slow motion: an agent that speaks
fluent gRPC and still cannot point at the impression you handed it. Wrong
lifecycle, wrong ids, paths that are not even on this ticket. One of the writes
would have landed. We still drop the whole set. The framework accepted the RPC.
The auction was never asked, so we never called a DSP, and the living room never
left the show.

**Punchline:** The framework accepted the RPC. The auction was never asked.

---

## 2. Engineer view · mutation fine, auction not (1:25–2:00)

**Click scene 2.**

Mutation card: `ADD_METRICS` / applied. Ticket metric chip turns red:
`viewability high`. TV: **INVALID AUCTION**. DSP still empty.

**Say:**

From the exchange engineer seat, the next failure is quieter. The mutation can
be well-formed ARTF and still wreck the auction. ADD_METRICS is allowed, the
path resolves, the payload member matches. After apply, OpenRTB wanted a number
and it got a string. Well-formed mutation and still-valid OpenRTB are different
questions. If you forward because the RPC succeeded, you just shipped a broken
bid request.

**Punchline:** Well-formed mutation and still-valid OpenRTB are different questions.

---

## 3. DSP view · curation lands, dummy mid-roll (2:00–2:50)

**Click scene 3.** Watch the ticket, not the log.

**Point at, as they light up:**

1. Segments: `seg-sports` stays. `seg-cord-cutter` and `seg-premium-viewer` arrive.
2. Deals: `deal-curated` appears. `deal-premium` floor $12.00 → $14.50.
3. Metrics: `viewability 0.82`, numeric, vendor tagged.
4. Empty finding list. Host forwards. DSP inbox gets a receipt.
5. TV cuts to **Northwind Spark**. Dummy. CSS. Labeled on the glass.

**Say:**

This is the hop a buyer actually feels. Watch the ticket, not the dummy, while
the patch lands. The DSP did not bid on the publisher's original ticket. It bid
on the one the agent wrote: extra segments, a new deal, a higher floor, a
numeric viewability. The dummy on the glass is what that bid is holding. It is
not VAST, and ARTF never gets as far as delivery.

**Punchline:** Empty findings. The DSP is bidding on the auction the agent actually wrote.

---

## 4. Publisher view · a deal leaves the wire (2:50–3:30)

**Click scene 4.**

Ticket: `deal-standard $6.00` strikes through. `deal-premium` remains. DSP still
gets a bid (Acme Cover dummy: only the premium deal is left). Inbox increments.

**Say:**

From the publisher seat, curation is allowed to take a deal off the auction.
That can be the product. It can also be the wrong id. deal-standard just left
the wire, and the DSP never saw it. The path still has to resolve against this
auction, and the result still has to be OpenRTB, or you find out at reporting.

**Punchline:** deal-standard is off the wire. The DSP never saw it.

---

## 5. DSP view · the bid is shaded (3:30–4:15)

**Click scene 5.** Perspective flips to **DSP / bid response**. Lifecycle on the
envelope is `LIFECYCLE_DSP_BID_RESPONSE`. The bid is already in the envelope.
We do not call the stub DSP again.

Ticket bid row: **$18.40** struck, **$12.88** lands. Same dummy, new clearing
price. Mutation card: `BID_SHADE` applied on `/seatbid/dsp-1/bid/bid-ctv`.

**Say:**

ARTF is not only a request-side curator. After the DSP answers, a container can
shade the price. $18.40 comes in, $12.88 goes out. The
creative did not change. The bid object did. The first scene tried to shade at
the wrong lifecycle and bounced. This is the legal stage, and the number still
has to remain a bid.

**Punchline:** The bid is still a bid. The price is not.

---

## 6. Volume · same hops, many living rooms (3:50–5:00)

**Start mixer at 5,000 /s for 180s.** Mix is 85% clean, 8% illegal, 2% breaking,
4% suppress, 1% shade. Same fixtures, concurrent, real gRPC. Mixer lints only;
named hops still call the DSP. Barcode: green forward, red drop.

**Say (console):**

One slot is the story. A bidstream is the job. This is a laptop running the
same binary, not a colo POP. Five thousand auctions a second through real gRPC,
mostly clean, with some planted dirt and a little legal curation. Green on the
barcode is a forward. Red is a drop. The log is sampled so you can still read
it. Grafana is the unsampled view of the same hop. That is where you actually
operate this check.

Then **open Grafana** (kiosk, 1s refresh, last 1 minute). Walk the board in
full sentences. Do not park one line on the dashboard, and do not label panels
like a product tour.

**Say (Grafana):**

This is the same hop without sampling. The auctions-per-second figure is the
mixer, as a raw count, not a claim about anyone's production. Drop rate should
sit near ten percent, because that is the dirt we planted. Suppress and shade
still forward; those are legal curation. Mutations p99 should sit far under the
sit far under the 150 millisecond tmax. Headroom is whatever is left for the rest
of the hop. Shed has to stay at zero. If it ticks up, the limiter is refusing
work rather than lying about latency. The stack is the mix by beat. The heatmap
should stay in the first milliseconds; a ridge walking right is queueing. On
the drop gate, mutations means the agent never pointed at this auction. Applied
means the patch wrote illegal OpenRTB, so it must not fan out. The rule list is
what on-call reads when the rate turns. You call ValidateArtfMutations with
apply true before you forward, and before you accept a shaded bid.

---

## 7. Close (5:00–5:15)

Back on the console. Mixer still running is fine. Perspective rail can sit on DSP.

**Say:**

Those are the three passes: envelope, mutation, and applied. VASTlint was the
creative hop. This is the bid hop, an independent implementation, not an IAB
product.

**Ask:** Ten minutes in the agentic block, or a table.

---

## What each seat is supposed to feel

| Seat | Scene | The sentence they should leave with |
|---|---|---|
| Publisher / CTV | 0, 4 | An agent can add or pull a deal on my mid-roll. I want that applied object checked. |
| SSP / host | 1, 6 | Fluent gRPC is not a valid auction. Drop before the DSP. |
| Exchange engineer | 2 | Static ARTF pass can still emit illegal OpenRTB. Apply, then revalidate. |
| DSP / buyer | 3, 5 | I bid on the rewritten ticket, and my own bid can be shaded afterwards. |

---

## Honesty, out loud if asked

| They ask | You say |
|---|---|
| Is this a real SSP? | No. Stub host, stub agent, stub DSP. Real RTBlint. |
| Is that a real ad? | No. CSS dummy. Labeled. Not VAST, not delivery. ARTF stops at the auction. |
| Is 5,000/s production? | No. A colo POP is larger. This is the same hop, on a laptop, hard enough that p99 and drops are visible. |
| Do you speak ARTF gRPC to the agent? | The linter does. The stub agent is HTTP fixture playback. |
| Are you ARTF-compliant? | We implement the check the spec does not define. No badge. |
| Latency number on a slide? | Only what Grafana is showing in the room. |
| Why a TV if ARTF is not delivery? | So a CTV person can see the object they already know. The ticket is the demo. The TV is the caption. |

---

## Build notes (what the five minutes added)

Not in the original three-beat harness:

- Auction ticket: before/after deals, segments, floors, metrics, bid price.
- CTV stage: content / unsold / poisoned / dummy mid-roll.
- Scene 4: `SUPPRESS_DEALS` on `deal-standard` (`fixtures/suppress-mutations.json`).
- Scene 5: `LIFECYCLE_DSP_BID_RESPONSE` + `BID_SHADE` on a CTV video bid
  (`fixtures/ctv-dsp-request.json`, `fixtures/shade-mutations.json`).
- DSP stub returns a dummy creative id so scene 3 and 4 have something to put
  in the glass.

Still real: `rtblint-grpc`, `ValidateArtfEnvelope`, `ValidateArtfMutations` with
`apply: true`, Prometheus scrape, Grafana dashboard.

---

## Live backup

If the projector hates the laptop: `artf-sim-five.mp4`, then `artf-sim-pitch.mp4`.
If Docker is dead: `./run.sh` in the terminal. Three beats, no TV, no Grafana.
