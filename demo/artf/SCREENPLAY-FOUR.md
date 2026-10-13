# Episode 4: Fan-in

Many agents, one ticket, two stages. This is the production shape of ARTF:
specialists in parallel on the way out, a shader isolated per DSP on the way
back. The host still applies. Agents never write the bid.

Episode 3 taught the envelope of one call. This cut teaches the hop graph
around that call. Generous to the spec. Do not lead with dirt. Do not say
"production traffic." Say: synthetic mix, real gRPC, the graph a host already
has to run.

Picture: the artf-simulator lab hop graph (`http://localhost:8080/lab/`).
Gold nodes are GetMutations. Green nodes fired. Apply table lists intent,
agent, OpenRTB field. Ticket before / after is the proof.

Reel: `artf-sim-four.mp4` (`node record-four.mjs` while the lab is up on
:8080 and Grafana on :3000). Prompter: `teleprompter/four.html`
(`npm run prompter:four`). Bookends are silent.

If you shoot this on the teaching console instead, you are lying about the
shape. One stub agent is episode 1 through 3. Fan-in is this lab.

Honesty, once, out loud:

> Dummy creative. Stub SSP, stub DSPs, stub agents. Compose is the
> orchestrator. The apply path is real gRPC. ARTF stops at the auction.
> VAST, CDN, and tracker light after the win so you can see the boundary.
> They are not ARTF.

---

## Idea, in one paragraph

ARTF is not one curator with a god-object. A host fans the same envelope out
to several containers on the publisher hop (audience, curator, metrics,
quality, cids), waits inside tmax, then independently applies each mutation
onto one OpenRTB ticket. DSPs bid on that merged BidRequest-prime. Each DSP
then calls its own shader with a response-stage envelope; alpha cannot see
beta. The hop graph is the product: specialists, isolation, one apply. That
is how you put identity, deals, measurement, and shade beside an auction
without giving any one agent the keys.

---

## 0. Open · one auction id (0:00–0:35)

**Do not click yet.** Hop graph at rest. Same CTV facts as the teaching
console, named once so the room knows the object:

- Mid-roll, 1920×1080, 15–30s, `streaming.example`
- PMP: `deal-premium` $12, `deal-standard` $6
- User already has `seg-sports`
- Floor $8. No bid yet.

Hold on the gold GetMutations nodes. They are not lit.

**Say:**

Same mid-roll as the last three films. Different picture. Episode 3 was one
envelope, one agent. Production is not that. Production is several
containers answering the same envelope in parallel, then one apply onto one
ticket, then a second envelope after each DSP bids. The gold nodes are
GetMutations. We are about to light them in the order a host actually runs
them.

---

## 1. Request fan-in · five specialists, one tmax (0:35–1:25)

**Run the clean request beat.** Gold request agents go green together:
audience, curator, metrics, quality, cids. Do not serialise them in the
voiceover. They are concurrent.

**Point at the graph:**

1. SSP node. That is the host. It dials. Agents do not dial each other.
2. Five gold nodes on the request path, one wait.
3. Networks: request agents sit on an internal net with the SSP. They cannot
   see DSPs. That is isolation, not a demo flourish.

**Say:**

The host fills one envelope: publisher-bid-request lifecycle, one tmax, one
applicable_intents list that covers the jobs these containers are for. Then
it fans that envelope out. Audience does not wait for curator. Metrics does
not wait for quality. They answer independently because mutations are
independently acceptable. The auction clock is one budget for the set, not
a budget per vendor. If you serialise these calls you have already lost the
reason the working group put them in-process.

**Punchline:** One envelope. Many answers. One clock.

---

## 2. Apply table · who wrote what (1:25–2:15)

**Hold the apply table.** Ticket before / after beside it. Do not watch the
player yet.

**Point at each row (intent · agent · field):**

1. `ACTIVATE_SEGMENTS` · audience · `user.data[].segment`
2. `ACTIVATE_DEALS` · curator · `imp[].pmp.deals`
3. `ADJUST_DEAL_FLOOR` · curator · deal `bidfloor`
4. `ADD_METRICS` · metrics / quality · `imp[].metric` (viewability, ivt)
5. `ADD_CIDS` · cids · `app.content.data`

If a row is skipped, say it as host policy (`known` apply: an intent this
orchestrator will not write), not as a gotcha.

**Say:**

This table is the whole contract. Each row is an intent, the agent that
proposed it, and the OpenRTB field the *host* wrote. The ticket on the right
is BidRequest-prime. The ticket on the left is what the publisher sent.
Agents proposed. They never held the bid. If two specialists touch different
paths, the merge is the union. If they collide, the orchestrator still
chooses. Fan-in is not a mash-up. It is apply.

**Punchline:** The apply table is the proof. Agents never wrote the bid.

---

## 3. Fanout · two DSPs, then shade (2:15–3:05)

**Let the graph continue:** HTTP POST /openrtb/bid to dsp-alpha and dsp-beta.
Then gold shader nodes, isolated.

**Point at:**

1. Two DSP nodes. Same BidRequest-prime. Separate bids.
2. Shader-alpha on an internal net with dsp-alpha only.
3. Shader-beta on an internal net with dsp-beta only.
4. Envelope on this hop: `LIFECYCLE_DSP_BID_RESPONSE`, `BID_SHADE` only,
   `bid_response` attached, path `/seatbid/{seat}/bid/{id}`.
5. Price after shade. First-price still happens on the host.

**Say:**

Buyers bid on what you forwarded. Then each DSP runs the same RPC at the
later stage, with a different intent list, against its own bid. Isolation
is the design: a shader beside alpha must not see beta's price, and it must
not see the request-side agents. You get shade without a second bid
protocol and without a shared god-agent that can observe every seat. The
host still applies BID_SHADE. The auction still clears on OpenRTB.

**Punchline:** Request agents share an SSP. Shaders never share a DSP.

---

## 4. The boundary · after ARTF (3:05–3:40)

**Play VAST / win path once.** CDN and tracker light. Impression, billing,
quartiles. One pass.

**Say:**

ARTF stopped at apply. What lights now is the rest of CTV: VAST InLine, a
MediaFile from a CDN stub, win and billing notices, quartile pixels. Useful
so you can see where the framework ends. Not a claim that the lab is a
player, an encoder, or a measurement company. If a room asks where delivery
lives, this is the answer: after the hop, on purpose.

**Punchline:** The gold nodes were the framework. The rest is the stack.

---

## 5. Volume · rate by agent, not by dummy (3:40–4:35)

**Start traffic profile if the lab has it.** Grafana: GetMutations rate and
p95 by agent, intents applied vs skipped, DSP timeouts.

**Say:**

At volume you watch specialists, not a blended hop. Audience p95 is not
curator p95. A shader that blows tmax on alpha does not excuse a stall on
the request fan-in. Applied vs skipped is host policy under load: the
orchestrator still choosing, five thousand times a second on a laptop,
which is a slice, not a colo. Shed and timeouts are the same clock as
episode 3. The new number is fan-in: five request RPCs and two shade RPCs
inside one auction id.

---

## 6. Close (4:35–5:00)

Back on the hop graph. Gold nodes dark again.

**Say:**

That is ARTF as a host actually runs it: several containers on the way out,
an isolated shader per buyer on the way back, one apply onto OpenRTB, and a
hard stop before VAST. Package specialists. Keep isolation. Keep the ticket.
Independent simulation. Not an IAB product.

---

## What a production person should leave with

| Seat | The sentence |
|---|---|
| SSP / host | I fan one envelope to request agents, merge with apply, then fan bids. |
| Agent author | I am a specialist. I do not need the other containers' data. |
| DSP / buyer | My shader sits beside me. It cannot see the other seat. |
| On-call | Alert per agent p95 against tmax, plus applied vs skipped. |

---

## Honesty, out loud if asked

| They ask | You say |
|---|---|
| Is this the teaching console? | No. Episodes 1–3 are the living-room hop. This is the lab hop graph. |
| Are the agents real vendors? | No. Role stubs: audience, curator, metrics, quality, cids, shader. |
| Can they see each other? | Request agents see the SSP. Each shader sees only its DSP. |
| Why skip some intents? | Host apply policy. Independently acceptable. Same rule as episode 3. |
| Is Play VAST the product? | No. It bounds the hop. ARTF does not deliver the stream. |
| Are you ARTF-compliant? | This is a simulation of the hop graph the spec enables. No badge. |
