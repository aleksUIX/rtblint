# The host decides which changes reach the auction.

Episode 2. Duration: 5:39. Simulator: http://localhost:8080/?ep=2

## 0:00 to 0:08 · Opening

Picture: Show the intro title card.

Title card. No voice.

## 0:08 to 0:28 · The host turns proposals into auction changes.

Picture: Open the episode 2 console at rest.

Voice: 0:09 to 0:25. Then pause until the next shot.

OpenRTB carries bid requests and responses. ARTF, the Agentic Real Time Framework, lets a host call specialist services during the auction. Let's see how their proposals become changes buyers see.

## 0:28 to 0:48 · The host gives a specialist access to the auction object.

Picture: Highlight envelope member lifecycle.

Voice: 0:29 to 0:44. Then pause until the next shot.

The exchange is our host. The agent is a specialist service inside its infrastructure. Here it returns fixed proposals. The envelope wraps OpenRTB so the host can request changes.

## 0:48 to 1:10 · The call has an identity and a time budget.

Picture: Highlight envelope member id.

Voice: 0:49 to 1:07. Then pause until the next shot.

This call id is separate from the auction id. Matching it ties the answer to the right call. The time budget is one hundred fifty milliseconds; a late answer can miss the auction.

## 1:10 to 1:29 · The host offers specific work for this stage.

Picture: Highlight envelope member intents.

Voice: 1:11 to 1:25. Then pause until the next shot.

Intents name the jobs this host offers. They limit what the agent can change. Here that's segments, deals, floors, and metrics. Shading waits until there's a bid.

## 1:29 to 1:35 · The host evaluates the proposals.

Picture: Run scene policy and wait for the verdict.

Run Host policy. Let the cards and DSP receipt finish.

## 1:35 to 1:56 · Audience segments can affect which campaigns consider the impression.

Picture: Highlight proposal 1.

Voice: 1:36 to 1:54. Then pause until the next shot.

A mutation is a proposed change, with an operation, target, and payload. Here it adds audience segments. Those labels can affect which campaigns consider this impression. The host chooses whether to apply it.

## 1:56 to 2:20 · A higher deal floor can change which bids qualify.

Picture: Highlight proposal 3.

Voice: 1:57 to 2:16. Then pause until the next shot.

This floor change raises the premium deal's minimum price from twelve dollars to fourteen fifty per thousand impressions. That changes which bids qualify. A higher floor can also leave the impression without an eligible bid.

## 2:20 to 2:44 · Independent acceptance lets the host keep useful proposals.

Picture: Highlight proposal 5.

Voice: 2:21 to 2:37. Then pause until the next shot.

Content id wasn't offered here, so it's skipped. The DSP, the buyer's platform, still receives four accepted changes. Independent acceptance lets the host keep useful work while enforcing its own rules.

## 2:44 to 2:50 · A response can name the wrong call.

Picture: Run scene illegal and wait for the verdict.

Run Wrong response. Wait for the rejection and the unchanged DSP receipt count.

## 2:50 to 3:13 · A returned response still needs to address this call.

Picture: Highlight proposal 1.

Voice: 2:51 to 3:09. Then pause until the next shot.

This answer names the wrong call. The host rejects the response before applying any changes. The original auction stays unchanged, and nothing new reaches the buyer. A usable proposal can't bypass that identity check.

## 3:13 to 3:33 · A supported intent needs a defined apply rule.

Picture: Highlight proposal 6.

Voice: 3:14 to 3:29. Then pause until the next shot.

Deal margin shows an integration gap. This host has no standard OpenRTB field for it. An intent needs an agreed mapping, or the host cannot apply the proposal.

## 3:33 to 3:39 · The resulting object needs its own check.

Picture: Run scene breaking and wait for the verdict.

Run Applied object. Hold on the red viewability metric and the OpenRTB finding.

## 3:39 to 4:01 · The JSON adapter produces an invalid metric.

Picture: Highlight #t-metrics.

Voice: 3:40 to 3:56. Then pause until the next shot.

Here, viewability is the word high. ARTF's typed metric requires a number. Our HTTP JSON fixture introduces this fault. The resulting request gives the buyer a value it cannot reliably use.

## 4:01 to 4:24 · Checking the assembled request contains an integration error.

Picture: Highlight #t-metrics.

Voice: 4:02 to 4:19. Then pause until the next shot.

The mutation was applied, but the request it produced is invalid. That's why we check the assembled OpenRTB too. The host stops this error here, before a buyer has to handle it.

## 4:24 to 4:44 · A shared interface still needs the host's business rules.

Picture: Highlight #n-rtblint.

Voice: 4:25 to 4:43. Then pause until the next shot.

RTBlint performs these checks in our demo. ARTF provides the shared interface. The host supplies the business rules. That allows specialists to integrate through a common contract while the platform controls the auction.

## 4:44 to 4:50 · The response stage carries a different object.

Picture: Run scene shade and wait for the verdict.

Run Bid response. Let the envelope and the bid price settle.

## 4:50 to 5:12 · Shading changes the price competing in the auction.

Picture: Highlight envelope member attached.

Voice: 4:51 to 5:08. Then pause until the next shot.

In this separate bid response example, eighteen forty becomes twelve eighty eight for the same creative. Shading changes the price competing in the auction. The exchange still decides whether the bid qualifies.

## 5:12 to 5:31 · Several agents will still need one host decision.

Picture: Highlight envelope member intents.

Voice: 5:13 to 5:27. Then pause until the next shot.

OpenRTB carries the auction data. ARTF gives specialists a common way to propose changes, with the host in control. Next, we'll coordinate several agents around one auction.

## 5:31 to 5:39 · Closing

Picture: Show the outro title card.

Closing card. No voice.
