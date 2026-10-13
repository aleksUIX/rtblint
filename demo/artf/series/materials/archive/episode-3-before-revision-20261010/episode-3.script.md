# Several agents can contribute to one auction.

Episode 3 of 3. Planned duration: 7:36, including pauses and cards.

Read the paragraphs only. Directions are in the screenplay.

## 0:08 · One auction can use several specialists.

Episode one showed the jobs an agent can do. Episode two followed a proposal through the host's checks. Now we split the work among five request agents and two buyers, each with a shader. These agents use fixed business rules and real GetMutations calls. The graph lets us follow their coordination around one auction.

## 0:40 · Parallel calls share the request deadline.

The SSP, the seller's platform, calls audience, curator, metrics, quality, and content ids together. Each receives a copy of the starting envelope. Audience does not wait for curator. In this lab, the group shares one deadline. Parallel calls avoid adding every agent's waiting time in sequence. The host still needs to decide which answers arrived in time.

## 1:09 · The familiar curation job is split across agents.

The ticket makes that coordination visible. Audience adds the segments we saw in episode one. Curator adds a deal and raises the premium floor. Metrics and quality add their measurements. Content ids add data about the programme. The host combines accepted proposals into one request for buyers. The before and after views show what the host actually wrote.

## 1:40 · Each accepted write can be traced to its agent.

The apply table connects a proposed intent to its agent and OpenRTB field. Viewability and invalid traffic are separate measurements. Content ids describe the programme; audience segments describe the viewer's grouping. When a field changes, this table helps you trace that change back to its source.

## 2:04 · Parallel execution still needs a conflict policy.

Two agents could propose different floors for the same deal. Finishing first is an unreliable business rule. These clean examples mostly write separate fields. The lab collects and applies responses in configured agent order. A real host needs a deliberate policy for conflicting writes. Independent acceptance lets the host consider proposals separately.

## 2:31 · Each buyer applies its own shading decision.

Alpha and beta receive the request after the host applies the accepted changes. Each buyer makes a bid, attaches its bid response to a new envelope, and offers bid shade. This is the later stage we inspected in episode two. GetMutations is the call for proposed changes. Each buyer applies its own accepted price change before returning its bid to the exchange.

## 3:02 · The container's access follows its role.

The graph also shows access boundaries. Alpha's shader sits with alpha on an internal Docker network. Beta's shader sits with beta on another. Request agents sit with the SSP on a separate network. Monitoring can scrape their counters, but the agents have no direct route into the other buyer's network. This lab shows how the host can limit each container's access.

## 3:34 · A changed price still has to qualify for the auction.

Shading changes a price; the exchange still decides whether that bid qualifies. The table shows beta's eligible bid and alpha's bid rejected below the deal floor. That connects two jobs we have already seen: raising a floor on the request and changing a price on the response. This first price lab selects the highest eligible returned bid. Eligibility and winner selection remain host decisions.

## 4:06 · The host must decide what happens without an answer.

The deadline also needs a fallback. If a request agent misses it, this host records the error and continues with available responses. If a shader call fails, the buyer keeps its original bid. Those choices fit this lab. A required control might need another policy. Decide what the host may forward when an agent cannot answer.

## 4:35 · The descriptor helps an operator deploy the agent.

The curator descriptor tells an operator what the container provides and needs: its version, image, intents, resources, and health endpoints. The descriptor supports deployment. The envelope still limits the work offered on each auction call. Compose runs our local examples from those configured roles.

## 5:04 · Delivery begins after the ARTF auction work.

Episode one stopped at the auction. Here we show what follows it: a VAST response, a dummy media fetch, and scripted impression and quartile requests to the tracker. The lit CDN and tracker are downstream HTTP hops. ARTF's mutation calls have finished. These scripted events illustrate the boundary; they are not measurements of a real viewer watching an ad.

## 5:34 · Measure the path that this traffic driver exercises.

Episode one's mixer stressed the validation hop. This driver repeats the broader simulated auction path. We request twenty auctions a second; Prometheus reports the rate actually observed. Each auction can call five request agents and two shaders. Read auction rate and agent call rate separately. These local workloads exercise different paths.

## 6:00 · Read latency alongside the host's deadlines.

The budgets above the table are demo settings. Request agents share a group deadline, and shading must finish inside the buyer call. A per agent p95 estimates the time within which ninety five percent of measured calls finish. Also watch the SSP hop and the timeout count. Fast typical calls can coexist with late answers that the host must handle.

## 6:32 · Coordination still needs the validation from episode two.

The mutation counters show accepted work, while the apply table explains individual writes. They help operate this path. This lab demonstrates coordination and typed calls; its auction path does not include RTBlint. Episode two showed that additional validation separately. Bringing the lessons together means checking the object the host will forward, after all the accepted proposals have been combined.

## 7:02 · Trace one complete auction before increasing traffic.

Start with one auction when evaluating an integration. Trace an accepted proposal to the field the host wrote, then to the object a buyer receives. Repeat with a missing answer and a conflicting proposal. Once those decisions are understood, measure the same path under load. That is a way to examine ARTF.
