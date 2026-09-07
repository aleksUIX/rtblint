# Episode 3: The envelope

How ARTF is built. Same living-room console as episodes 1 and 2. This cut is
generous to the spec: one RPC, a small addressing language, and a host that
stays the host.

Episode 1 taught where the hop sits. Episode 2 taught the check after apply.
This cut teaches the call itself.

Do not lead with failures. Do not say the spec is incomplete. Do not say
"production traffic." Say: synthetic mix, real gRPC, the envelope a host
already has to fill.

Reel: `artf-sim-three.mp4` (`node record-three.mjs` while compose is up).
Console: http://localhost:8080/?ep=3
Prompter: `teleprompter/three.html` (`npm run prompter:three`). Same beats as the
captions. Bookends are silent.

The envelope pane is always on in this mode. Beat 0 highlights members. Beat 1
freezes `applicable_intents`. Beat 2 runs the policy hop: four writes apply,
ADD_CIDS is skipped because the host did not offer it. Beat 3 is shade. Mixer
is the production mix.

Honesty, once, out loud:

> The TV is a dummy. The SSP, agent, and DSP are stubs. The ticket is the OpenRTB
> the host would forward. The apply path is real gRPC. ARTF stops at the auction.
> Nothing here is delivery.

---

## Idea, in one paragraph

ARTF is not a smart agent. ARTF is a constrained envelope around an agent the
host already trusts. The host names the stage (`lifecycle`), the budget
(`tmax`), the vocabulary it will even evaluate (`applicable_intents`), and the
OpenRTB object under mutation. The agent answers with independently acceptable
mutations: intent, operation, semantic path, typed payload. The orchestrator
accepts or rejects each one. The same RPC shape runs again after a bid, with a
different lifecycle, a different intent list, and a different path grammar.
That is the product.

---

## 0. Open · the call, not the dummy (0:00–0:35)

**Do not click yet.** Same CTV mid-roll at rest. Ticket filled. Perspective on
**Host / SSP**.

**Point at, in this order:**

1. The ticket. Still OpenRTB. Episode 1 already proved that.
2. The pipe: Host, Agent, Apply, DSP. Episode 1 already proved where it sits.
3. Hold on the agent node. We are about to look at what the host *sends*, not
   what the TV shows.

**Say:**

Episode 1 was the stack: a container inside the host, a patch on OpenRTB, a
DSP bidding on what you forwarded. This is the call. ARTF does not invent a
new bid object. It wraps the one you already speak in an envelope, hands that
envelope to an agent over gRPC, and takes back mutations. The host stays the
host. The envelope is how.

---

## 1. Envelope · what the host actually sends (0:35–1:20)

**Show the envelope pane** (lifecycle, id, tmax, originator, applicable_intents,
which OpenRTB member is attached). Leave the ticket visible. Do not rush the
member names.

**Point at, in this order:**

1. `lifecycle`: `LIFECYCLE_PUBLISHER_BID_REQUEST`. That is the stage, not a
   proof of contents.
2. `id`: the extension-point id. Correlation under concurrency. Not the bid
   request id.
3. `tmax`: 150 ms. This hop sits inside the auction clock.
4. `originator`: who is calling. TYPE_EXCHANGE on this seat.
5. `applicable_intents`: the vocabulary this orchestrator will evaluate on
   this hop.
6. `bid_request`: the OpenRTB object under mutation. Attached, not implied.

**Say:**

This is `RTBRequest`. Lifecycle names the stage you are in, so the agent
knows which intents are meaningful. It does not inject the matching OpenRTB
member. You still attach `bid_request` on a publisher hop, and you still
attach `bid_response` later when you actually have a bid. tmax is the SLA of
the hop. applicable_intents is the host's vocabulary for this call: what you
are willing to evaluate, not what the agent wishes you would. The agent
never gets a copy of the bidstream to take elsewhere. It gets this envelope,
inside your infra, on your clock.

**Punchline:** Lifecycle names the stage. You still have to attach the object.

---

## 2. Host policy · applicable_intents (1:20–2:00)

**Click curation, but freeze on the intent list before the cards land.**

Request-side offer on screen, as a row of tokens:

`ACTIVATE_SEGMENTS` · `ACTIVATE_DEALS` · `ADJUST_DEAL_FLOOR` · `ADD_METRICS`

No `BID_SHADE` on this list. That is the point.

**Say:**

The same container can sit beside an SSP and beside a DSP. What changes is
not the agent binary. What changes is this list. On a publisher hop you offer
deals, segments, floors, metrics: the jobs that have always been tight on
time. You do not offer BID_SHADE, because there is no bid yet. The spec is
built this way so you can package the agent once and keep policy on the
host. If the agent returns an intent you did not offer, that mutation is
dead on arrival. That is not a snub. That is how an exchange stays an
exchange.

**Punchline:** The agent is reusable. The intent list is not.

---

## 3. Semantic paths · how an agent points (2:00–2:40)

**Let the curation cards land.** Watch the ticket, not the dummy.

**Point at each path as it lights:**

1. `/user/data/segment` · `ACTIVATE_SEGMENTS`. `seg-sports` stays.
   `seg-cord-cutter` and `seg-premium-viewer` arrive.
2. `/imp/imp-1/pmp/deals/deal-premium` · `ADJUST_DEAL_FLOOR`. $12.00 → $14.50.
3. `/imp/imp-1/pmp/deals/deal-curated` · `ACTIVATE_DEALS`. A deal that was not
   on the wire.
4. `/imp/imp-1` · `ADD_METRICS`. `viewability 0.82`, a number.

**Say:**

A mutation is not a blob of JSON to merge. It is an intent, an operation, a
semantic path, and a typed payload. The path names a business entity by id:
this impression, this deal, this seat, this bid. It is not a JSON pointer
into a document layout the agent would have to know. That is how you patch
OpenRTB without shipping a new bid object, and without handing the agent the
keys to the whole ticket. If the path does not resolve on *this* auction, the
write has nowhere to go. The addressing scheme is the spec.

**Punchline:** Paths name ids in this auction. Not offsets in a blob.

---

## 4. Independent apply · the host still chooses (2:40–3:20)

**Same hop, one mutation skipped on purpose.** Four cards applied (green).
`ADD_CIDS` skipped: the host did not offer it on this hop. Ticket shows the
union of the ones that landed. DSP still called. Dummy still plays.

Do not play this as a failure. Play it as the design.

**Say:**

Mutations are independently acceptable. That sentence is the whole
orchestrator. You do not have to take the set. You accept or reject each
write, change by change, against the list you published and the paths that
actually exist. A skipped mutation is the host doing its job. The DSP then
bids on BidRequest-prime: whatever you actually forwarded, not whatever the
agent proposed in full. Control never left the exchange.

**Punchline:** Independently acceptable means the host still chooses.

---

## 5. Same RPC, later stage (3:20–4:05)

**Click shade.** Perspective flips to **DSP / bid response**. Envelope pane
updates.

**Point at the envelope first, then the path:**

1. `lifecycle` is now `LIFECYCLE_DSP_BID_RESPONSE`.
2. `applicable_intents` is now `BID_SHADE`. The request-side list is gone.
3. `bid_response` is attached. The bid is already in the envelope.
4. Path: `/seatbid/dsp-1/bid/bid-ctv`. Seat id, bid id. No impression index
   to fall back on.
5. Ticket: **$18.40** struck, **$12.88** lands. Same dummy.

**Say:**

The spec did not invent a second protocol for shading. Same gRPC, same
response shape, later in the auction. Lifecycle tells the agent the stage.
You attach bid_response because BID_SHADE addresses a seat and a bid id that
only exist once a buyer has answered. The path grammar changes with the
object: impressions and deals on the way out, seatbid on the way back. One
container model. Two moments. That is a small, complete design.

**Punchline:** Same RPC. Different stage, different list, different paths.

---

## 6. The clock · tmax at volume (4:05–4:55)

**Start mixer, production mix.** 5,000 /s. Grafana after the barcode is
green.

**Say (console):**

tmax is not a footnote. Identity, deals, segments, a shade on the way back:
those jobs have always been tight on time, which is why the working group
put them in a container beside the bidder instead of a second round trip.
Five thousand auctions a second is a laptop slice of that clock, not a colo.

**Say (Grafana):**

Mutations p99 has to live far under the 150 millisecond tmax, because this
hop sits inside the auction. Headroom is whatever is left for the rest of
the bid. Shed at zero means the limiter is not refusing work to protect that
clock. The stack is still the mix. The number that matters on this film is
not drop rate. It is whether the envelope budget still holds when the hop
is concurrent.

---

## 7. Close (4:55–5:15)

Back on the console.

**Say:**

That is how ARTF is built: an envelope the host fills, a vocabulary the host
publishes, paths that name ids in this auction, and mutations the
orchestrator may refuse one by one. Package the agent once. Keep policy and
timing where the bid already lives. Independent simulation. Not an IAB
product.

---

## What a production person should leave with

| Seat | The sentence |
|---|---|
| SSP / host | I fill the envelope. I publish the intent list. I apply each write. |
| Agent author | I answer this envelope. I point at ids. I do not own the ticket. |
| DSP / buyer | Request hop and response hop are the same RPC with a different stage. |
| On-call | Watch p99 against envelope tmax, not against a made-up SLO. |

---

## Honesty, out loud if asked

| They ask | You say |
|---|---|
| Is this the spec or RTBlint? | The envelope, intents, paths, and independent apply are ARTF. The OpenRTB check after apply is episode 2. |
| Why not JSON Pointer? | Semantic paths name business ids so the agent does not have to know the document layout. |
| Why echo the extension-point id? | Concurrent auctions. The bid request id is a different object. |
| Can one agent do request and response? | Yes. You change lifecycle and applicable_intents. That is the point. |
| Are you ARTF-compliant? | This is a simulation of the hop the spec describes. No badge. |
