# The host decides which changes reach the auction.

Episode 2 of 3. Planned duration: 5:39, including pauses and cards.

Read at about 110 words per minute. Start with each caption, then pause while the shot holds. The prompter marks the speaking and pause windows.

## 0:08 · The host turns proposals into auction changes.

OpenRTB carries bid requests and responses. ARTF, the Agentic Real Time Framework, lets a host call specialist services during the auction. Let's see how their proposals become changes buyers see.

## 0:28 · The host gives a specialist access to the auction object.

The exchange is our host. The agent is a specialist service inside its infrastructure. Here it returns fixed proposals. The envelope wraps OpenRTB so the host can request changes.

## 0:48 · The call has an identity and a time budget.

This call id is separate from the auction id. Matching it ties the answer to the right call. The time budget is one hundred fifty milliseconds; a late answer can miss the auction.

## 1:10 · The host offers specific work for this stage.

Intents name the jobs this host offers. They limit what the agent can change. Here that's segments, deals, floors, and metrics. Shading waits until there's a bid.

## 1:35 · Audience segments can affect which campaigns consider the impression.

A mutation is a proposed change, with an operation, target, and payload. Here it adds audience segments. Those labels can affect which campaigns consider this impression. The host chooses whether to apply it.

## 1:56 · A higher deal floor can change which bids qualify.

This floor change raises the premium deal's minimum price from twelve dollars to fourteen fifty per thousand impressions. That changes which bids qualify. A higher floor can also leave the impression without an eligible bid.

## 2:20 · Independent acceptance lets the host keep useful proposals.

Content id wasn't offered here, so it's skipped. The DSP, the buyer's platform, still receives four accepted changes. Independent acceptance lets the host keep useful work while enforcing its own rules.

## 2:50 · A returned response still needs to address this call.

This answer names the wrong call. The host rejects the response before applying any changes. The original auction stays unchanged, and nothing new reaches the buyer. A usable proposal can't bypass that identity check.

## 3:13 · A supported intent needs a defined apply rule.

Deal margin shows an integration gap. This host has no standard OpenRTB field for it. An intent needs an agreed mapping, or the host cannot apply the proposal.

## 3:39 · The JSON adapter produces an invalid metric.

Here, viewability is the word high. ARTF's typed metric requires a number. Our HTTP JSON fixture introduces this fault. The resulting request gives the buyer a value it cannot reliably use.

## 4:01 · Checking the assembled request contains an integration error.

The mutation was applied, but the request it produced is invalid. That's why we check the assembled OpenRTB too. The host stops this error here, before a buyer has to handle it.

## 4:24 · A shared interface still needs the host's business rules.

RTBlint performs these checks in our demo. ARTF provides the shared interface. The host supplies the business rules. That allows specialists to integrate through a common contract while the platform controls the auction.

## 4:50 · Shading changes the price competing in the auction.

In this separate bid response example, eighteen forty becomes twelve eighty eight for the same creative. Shading changes the price competing in the auction. The exchange still decides whether the bid qualifies.

## 5:12 · Several agents will still need one host decision.

OpenRTB carries the auction data. ARTF gives specialists a common way to propose changes, with the host in control. Next, we'll coordinate several agents around one auction.
