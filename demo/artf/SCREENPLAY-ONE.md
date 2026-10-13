# Episode 1: ARTF inside the host

Series: 1 this cut · 2 `SCREENPLAY.md` the check · 3 `SCREENPLAY-THREE.md`
the envelope · 4 `SCREENPLAY-FOUR.md` fan-in.

Intro for practitioners who run production systems and have not lived inside
the spec. Same compose as episode 2. Same binary. This cut teaches the hop:
three legal jobs on the same starting ticket, then volume. Episode 2
(`SCREENPLAY.md`) is the missing check.

Do not lead with shortcomings. Do not say "production traffic."
Say: synthetic mix, real gRPC apply, the hop a host already has to run.

Reel: `artf-sim-one.mp4` (`node record-one.mjs` while compose is up).
Booth script: `VO-ONE.md`.
Console: http://localhost:8080/?ep=1
Prompter: `teleprompter/one.html` (`npm run prompter:one`).
Who beat is live over 10.6s of console at rest. Do not burn it into the
captions.

Honesty, once, out loud:

> The TV is a dummy. The SSP, agent, and DSP are stubs. The ticket is the OpenRTB
> the host would forward. The apply path is real gRPC. The stub agent is HTTP
> fixture playback. ARTF stops at the auction. Nothing here is delivery.

---

## 0. Open · publisher / living room

**Do not click yet.** Console at rest. Perspective rail on **Publisher / CTV**.
TV: *Night Shift S3 E4*, progress bar into a mid-roll. Ticket already filled.

**After the intro card, 10.6s of console at rest (no caption). Say this
over that bed. Prompter Who beat. Not burned in:**

This is episode one of a series on ARTF. This demo is the hop: an agent
inside the host, patching this auction.

**Point at the ticket, in this order:**

1. 1920×1080, 15–30s, `streaming.example`. That is a CTV mid-roll.
2. Private marketplace. Two deals: `deal-premium` at $12, `deal-standard` at $6.
3. User already has `seg-sports`. Floor on the imp is $8.
4. No bid yet. The show is still playing.

**Say:**

This is the auction a CTV app is about to run: a mid-roll, two private deals, a
floor already on the impression. OpenRTB is the object every SSP and DSP already
speaks. ARTF does not replace it. IAB Tech Lab published ARTF so an agent can
take part in this auction from inside the host that is already running it.

---

## 1. Where the hop sits

**Click 0 Hop.** Full-screen map. Point host box, then agent, then orchestrator,
then DSP. Then the dashed row: VAST / player / beacons.

**Say:**

That host is an SSP, a DSP, or an exchange. You deploy the agent as a container
in your own infrastructure, and you call it while the bid is still moving. The
bidstream stays in your infrastructure. The agent runs next to this ticket.
You keep the timing, and you keep the decision of which
changes actually land. The agent returns mutations: patches against this OpenRTB
object. The orchestrator accepts or rejects each one. Buyers then bid on
whatever you actually forwarded. The same hop can run again after a bid comes
back, this time on the BidResponse. What ARTF does not cover is everything after
a winner is chosen: VAST, the player, measurement.

**Click the map or Esc** to close it.

---

## 2. DSP view · curation lands (job 1)

**Click scene 1 (Curation).** Watch the ticket, not the log. This is job one of
three on the same starting ticket. The next two clicks reset to that ticket.
They are not this auction continuing.

**Point at, as they light up:**

1. Segments: `seg-sports` stays. `seg-cord-cutter` and `seg-premium-viewer` arrive.
2. Deals: `deal-curated` appears. `deal-premium` floor $12.00 → $14.50.
3. Metrics: `viewability 0.82`, numeric.
4. Host forwards. DSP inbox gets a receipt. Bid lands near $16.90.
5. TV cuts to **Northwind Spark**. Dummy. CSS. Labeled on the TV.

**Say:**

This is the request-side hop a buyer actually feels. Watch the ticket, not the
dummy, while the patch lands. The DSP did not bid on the publisher's original
ticket. It bid on the one the agent wrote: extra segments, a new deal, a higher
floor, a numeric viewability. The dummy on the TV is what that bid is holding.
It is not VAST, and ARTF never gets as far as delivery.

---

## 3. Publisher view · a deal leaves the wire (job 2)

**Click scene 2 (Suppress).** Same starting ticket, a second job. Curation from
scene 1 is gone on purpose: this is not a sequel.

Ticket: `deal-standard $6.00` strikes through. `deal-premium` remains. DSP still
gets a bid. Inbox increments. Dummy switches to **Acme Cover**.

**Say:**

Same starting ticket, a second job. From the publisher seat, taking a deal off
can be the product. deal-standard just left the wire, and the DSP never saw it.
A private marketplace can look like this: only the premium deal remains.

---

## 4. DSP view · the bid is shaded (job 3)

**Click scene 3 (Shade).** Same starting ticket, a later stage. Not the $16.90
from scene 1. Perspective flips to **DSP / bid response**. Lifecycle is
`LIFECYCLE_DSP_BID_RESPONSE`. Agent node reads `shader-1`.

Ticket bid row: **$18.40** struck, **$12.88** lands. Dummy is Northwind again.
Mutation card: `BID_SHADE` applied. DSP inbox does not increment: the bid is
already in the envelope.

**Say:**

Third job, not the bid you just watched. After a DSP answers, a container can
shade the price. $18.40 comes in, $12.88 goes out. The creative did not change.
The bid object did. The request hop was PUBLISHER_BID_REQUEST. This is
DSP_BID_RESPONSE. Same container model, later in the auction.

---

## 5. Volume · same hop, many living rooms

**Start mixer at 5,000 /s for 180s.** Mix is production: 88% clean, 8% suppress,
4% shade. No planted dirt. Concurrent auctions, real gRPC apply. Mixer does not
call the DSP; named scenes do. The dial is 5,000. Grafana is the count that
landed on this laptop.

**Say (console):**

One slot is the story. A bidstream is the job. This is a laptop running the
same hop, not a colo POP. The mixer is set to five thousand a second. Grafana
is the count. Mostly clean curation, with some suppress and some shade: the
jobs that have always been tight on time. Green on the barcode is a forward.
Drop should sit at zero on this mix. Grafana is the unsampled view of the same
hop.

Then **open Grafana** (kiosk, `artf-sim-hop`, 1s refresh, last 1 minute).

**Say (Grafana):**

Auctions per second is the mixer, as a raw count, not a claim about anyone's
production. Forward rate should sit near one hundred percent here. Suppress and
shade still forward; those are legal jobs. Hop p99 has to live far under the
150 millisecond tmax, because this hop sits inside the auction clock. Headroom
is whatever is left for the rest of the bid. The mix is clean, suppress, and
shade. Synthetic. Same hop, many times a second.

---

## 6. Close

Back on the console. Mixer still running is fine. Auction counters should still
show the hose.

**Say:**

That is ARTF in the stack you already run: a container beside the exchange, a
patch on OpenRTB, a DSP bidding on what you forwarded, and the same hop after
the bid comes back. Independent simulation. Not an IAB product.

---

## What a production person should leave with

| Seat | The sentence |
|---|---|
| SSP / host | The agent is my container. I call it inside tmax. I apply the patch. |
| Publisher | An agent can add or pull a deal on my mid-roll before any buyer sees it. |
| DSP / buyer | I bid on BidRequest-prime, and my own bid can be shaded afterwards. |
| On-call | Watch hop p99 against tmax. Drop at zero on a clean mix. |

---

## Honesty, out loud if asked

| They ask | You say |
|---|---|
| Is this a real SSP? | No. Stub host, stub agent, stub DSP. Real gRPC apply. |
| Is that a real ad? | No. CSS dummy. Labeled. Not VAST, not delivery. ARTF stops at the auction. |
| Is 5,000/s production? | No. A colo POP is larger. This is the same hop, on a laptop. |
| Do you speak ARTF gRPC to the agent? | The apply path does. The stub agent is HTTP fixture playback. |
| Does the mixer run both RPCs? | Named hops do envelope then apply. The mixer apply is mutations-only so the laptop can keep up. |
| Are you ARTF-compliant? | This is a simulation of the hop the spec describes. No badge. |
| Why did suppress undo curation? | Three jobs, one starting ticket. Not one auction in sequence. |
| Where is the check? | Episode 2. |
