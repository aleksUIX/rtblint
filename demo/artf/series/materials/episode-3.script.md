# Several agents can contribute to one auction.

Episode 3 of 3. Planned duration: 6:26, including pauses and cards.

Read at about 110 words per minute. Start with each caption, then pause while the shot holds. The prompter marks the speaking and pause windows.

## 0:08 · One auction can use several specialists.

Last time, we checked one agent's proposals. ARTF, the Agentic Real Time Framework, lets hosts call specialist services during an auction. Now let's follow several specialists working on the same OpenRTB request.

## 0:36 · Parallel calls share the request deadline.

The SSP, the seller's platform, calls five specialists together. Each gets the starting envelope, which wraps the auction data. They share one deadline. Parallel calls keep us from waiting for each service in sequence.

## 1:00 · The familiar curation job is split across agents.

Here's the request before and after. Audience adds segments; curator adds deals and raises the premium floor. Those changes can affect which campaigns consider the impression and which bids qualify. The host combines accepted proposals.

## 1:24 · Each accepted write can be traced to its agent.

Metrics and quality add measurements. Content ids describe the programme, while audience segments describe viewer groups. Each accepted write names its source agent. If a buyer receives unexpected data, we can trace who proposed it.

## 1:48 · Parallel execution still needs a conflict policy.

What if two agents propose different floors for the same deal? This lab applies answers in configured order, mostly to separate fields. A production host needs a conflict policy. Response speed shouldn't decide a price.

## 2:12 · Each buyer applies its own shading decision.

Now the request reaches two buyers, the DSPs. Each makes a bid and calls its own shader through GetMutations, ARTF's proposal interface. Shading adjusts the offered price before the bid returns to the exchange.

## 2:36 · The container's access follows its role.

Alpha's shader and beta's shader sit on separate internal networks. The request agents sit with the seller. In this lab, one shader has no direct route to the other buyer. The host controls that access.

## 3:00 · A changed price still has to qualify for the auction.

Alpha's shaded bid falls below the deal floor, so it's rejected. Beta remains eligible and wins this example. A shader can change the price, but the exchange still checks eligibility and selects the highest eligible bid.

## 3:24 · The host must decide what happens without an answer.

A deadline needs a fallback. Here, a missing request agent's answer is skipped; a failed shader leaves the original bid. These are lab policies. If a service performs a required check, continuing may be inappropriate.

## 3:48 · The descriptor helps an operator deploy the agent.

This sample descriptor declares the curator's image, capabilities, and operating needs. It helps explain deployment. The envelope still limits the jobs offered on each call. Declaring a capability doesn't grant unlimited auction access.

## 4:17 · Delivery begins after the ARTF auction work.

In this demo, ARTF's auction calls are finished. VAST now describes the ad and its tracking. We fetch dummy media and fire scripted events. Those requests don't prove a real viewer watched the ad.

## 4:41 · Measure the path that this traffic driver exercises.

Let's repeat this path under load. The driver requests twenty auctions a second; monitoring shows the rate actually achieved. One auction can call several agents, so auction rate and agent call rate measure different things.

## 5:05 · Read latency alongside the host's deadlines.

p95 estimates the time within which ninety-five percent of measured calls finish. Read it with the auction path and buyer call failures. Fast typical calls don't tell us whether every deadline was met.

## 5:30 · Coordination still needs the validation from episode two.

Typed calls still need host checks. Here we check call identity and offered intents before combining proposals. RTBlint checks our captured request separately. A deployed host still needs OpenRTB validation before forwarding.

## 5:54 · Trace one complete auction before increasing traffic.

I'd start with one auction: follow a proposal to the field the buyer receives. Then test a missing answer and a conflicting proposal. ARTF supplies the shared interface; each host remains responsible for its decisions.
