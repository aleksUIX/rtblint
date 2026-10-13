# Several agents can contribute to one auction.

Episode 3. Duration: 6:26. Simulator: http://localhost:8081/lab/?ep=3

## 0:00 to 0:08 · Opening

Picture: Show the intro title card.

Title card. No voice.

## 0:08 to 0:31 · One auction can use several specialists.

Picture: Open the episode 3 lab at rest.

Voice: 0:09 to 0:26. Then pause until the next shot.

Last time, we checked one agent's proposals. ARTF, the Agentic Real Time Framework, lets hosts call specialist services during an auction. Now let's follow several specialists working on the same OpenRTB request.

## 0:31 to 0:36 · The host calls the request agents together.

Picture: Run lab scene clean.

Run the clean auction. Keep the graph and the request apply table visible.

## 0:36 to 1:00 · Parallel calls share the request deadline.

Picture: Show apply. Highlight ssp, audience, curator, metrics, quality, cids.

Voice: 0:37 to 0:55. Then pause until the next shot.

The SSP, the seller's platform, calls five specialists together. Each gets the starting envelope, which wraps the auction data. They share one deadline. Parallel calls keep us from waiting for each service in sequence.

## 1:00 to 1:24 · The familiar curation job is split across agents.

Picture: Show ticket. Highlight ssp, curator.

Voice: 1:01 to 1:20. Then pause until the next shot.

Here's the request before and after. Audience adds segments; curator adds deals and raises the premium floor. Those changes can affect which campaigns consider the impression and which bids qualify. The host combines accepted proposals.

## 1:24 to 1:48 · Each accepted write can be traced to its agent.

Picture: Show apply. Highlight audience, metrics, quality, cids.

Voice: 1:25 to 1:44. Then pause until the next shot.

Metrics and quality add measurements. Content ids describe the programme, while audience segments describe viewer groups. Each accepted write names its source agent. If a buyer receives unexpected data, we can trace who proposed it.

## 1:48 to 2:12 · Parallel execution still needs a conflict policy.

Picture: Show apply. Highlight ssp.

Voice: 1:49 to 2:08. Then pause until the next shot.

What if two agents propose different floors for the same deal? This lab applies answers in configured order, mostly to separate fields. A production host needs a conflict policy. Response speed shouldn't decide a price.

## 2:12 to 2:36 · Each buyer applies its own shading decision.

Picture: Show bids. Highlight dsp-alpha, dsp-beta.

Voice: 2:13 to 2:31. Then pause until the next shot.

Now the request reaches two buyers, the DSPs. Each makes a bid and calls its own shader through GetMutations, ARTF's proposal interface. Shading adjusts the offered price before the bid returns to the exchange.

## 2:36 to 3:00 · The container's access follows its role.

Picture: Show hops. Highlight shader-alpha, shader-beta.

Voice: 2:37 to 2:56. Then pause until the next shot.

Alpha's shader and beta's shader sit on separate internal networks. The request agents sit with the seller. In this lab, one shader has no direct route to the other buyer. The host controls that access.

## 3:00 to 3:24 · A changed price still has to qualify for the auction.

Picture: Show bids. Highlight ssp, dsp-alpha, dsp-beta.

Voice: 3:01 to 3:20. Then pause until the next shot.

Alpha's shaded bid falls below the deal floor, so it's rejected. Beta remains eligible and wins this example. A shader can change the price, but the exchange still checks eligibility and selects the highest eligible bid.

## 3:24 to 3:48 · The host must decide what happens without an answer.

Picture: Show hops. Highlight ssp, shader-alpha, shader-beta.

Voice: 3:25 to 3:44. Then pause until the next shot.

A deadline needs a fallback. Here, a missing request agent's answer is skipped; a failed shader leaves the original bid. These are lab policies. If a service performs a required check, continuing may be inappropriate.

## 3:48 to 4:12 · The descriptor helps an operator deploy the agent.

Picture: Show manifest. Highlight curator.

Voice: 3:49 to 4:07. Then pause until the next shot.

This sample descriptor declares the curator's image, capabilities, and operating needs. It helps explain deployment. The envelope still limits the jobs offered on each call. Declaring a capability doesn't grant unlimited auction access.

## 4:12 to 4:17 · Delivery starts after the auction decision.

Picture: Run Play VAST and wait for the media and tracker hops.

Play VAST once. Hold on the graph and the tracker events.

## 4:17 to 4:41 · Delivery begins after the ARTF auction work.

Picture: Show events. Highlight cdn, tracker.

Voice: 4:18 to 4:36. Then pause until the next shot.

In this demo, ARTF's auction calls are finished. VAST now describes the ad and its tracking. We fetch dummy media and fire scripted events. Those requests don't prove a real viewer watched the ad.

## 4:41 to 5:05 · Measure the path that this traffic driver exercises.

Picture: Start synthetic traffic at 20 auctions/s. Show live operations.

Voice: 4:42 to 5:01. Then pause until the next shot.

Let's repeat this path under load. The driver requests twenty auctions a second; monitoring shows the rate actually achieved. One auction can call several agents, so auction rate and agent call rate measure different things.

## 5:05 to 5:30 · Read latency alongside the host's deadlines.

Picture: Show ops. Highlight audience, curator, shader-alpha, shader-beta.

Voice: 5:06 to 5:24. Then pause until the next shot.

p95 estimates the time within which ninety-five percent of measured calls finish. Read it with the auction path and buyer call failures. Fast typical calls don't tell us whether every deadline was met.

## 5:30 to 5:54 · Coordination still needs the validation from episode two.

Picture: Show ops. Highlight ssp.

Voice: 5:31 to 5:48. Then pause until the next shot.

Typed calls still need host checks. Here we check call identity and offered intents before combining proposals. RTBlint checks our captured request separately. A deployed host still needs OpenRTB validation before forwarding.

## 5:54 to 6:18 · Trace one complete auction before increasing traffic.

Picture: Stop synthetic traffic and return to the graph and apply table.

Voice: 5:55 to 6:14. Then pause until the next shot.

I'd start with one auction: follow a proposal to the field the buyer receives. Then test a missing answer and a conflicting proposal. ARTF supplies the shared interface; each host remains responsible for its decisions.

## 6:18 to 6:26 · Closing

Picture: Show the outro title card.

Closing card. No voice.
