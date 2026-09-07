# Episode 1 voiceover

Reel: `artf-sim-one.mp4`. Prompter: `npm run prompter:one`.

Start the reel and the prompter on the same 3s countdown, 1.00x. Grey
lines are holds. Do not speak them. Who beat has picture (console at
rest) and no burned caption. Picture is locked to this clock (323s
body + 13s intro + 15s outro).

Clocks are the prompter clock, including intro and outro cards.

---

## 0:00 · Intro card · hold 13s

No voice.

## 0:13 · Who · live, no caption

This is episode one of a series on ARTF. This demo is the hop: an agent
inside the host, patching this auction.

## 0:24 · Open · living room

This is the auction a CTV app is about to run: a mid-roll, two private
deals, a floor already on the impression.

OpenRTB is the object every SSP and DSP already speaks. ARTF does not
replace it.

IAB Tech Lab published ARTF so an agent can take part in this auction
from inside the host that is already running it.

## 0:54 · Where the hop sits

That host is an SSP, a DSP, or an exchange. You deploy the agent as a
container in your own infrastructure, and you call it while the bid is
still moving.

The bidstream stays in your infrastructure. The agent runs next to this
ticket.

You keep the timing, and you keep the decision of which changes actually
land.

The agent returns mutations: patches against this OpenRTB object. The
orchestrator accepts or rejects each one. Buyers then bid on whatever you
actually forwarded.

The same hop can run again after a bid comes back, this time on the
BidResponse. What ARTF does not cover is everything after a winner is
chosen: VAST, the player, measurement.

## 1:55 · DSP · curation

This is the request-side hop a buyer actually feels. Watch the ticket,
not the dummy, while the patch lands.

Hold 7s. Ticket writes, then dummy.

The DSP did not bid on the publisher's original ticket. It bid on the
one the agent wrote: extra segments, a new deal, a higher floor, a
numeric viewability.

The dummy on the TV is what that bid is holding. It is not VAST, and
ARTF never gets as far as delivery.

## 2:35 · Publisher · suppress

Same starting ticket, a second job. From the publisher seat, taking a
deal off can be the product.

Hold 7s. deal-standard strikes through.

deal-standard just left the wire, and the DSP never saw it. A private
marketplace can look like this: only the premium deal remains.

## 3:02 · DSP · shade

Third job, not the bid you just watched. After a DSP answers, a
container can shade the price.

Hold 7s. $18.40 struck, $12.88 lands.

$18.40 comes in, $12.88 goes out. The creative did not change. The bid
object did.

The request hop was PUBLISHER_BID_REQUEST. This is DSP_BID_RESPONSE. Same
container model, later in the auction.

## 3:35 · Volume

One slot is the story. A bidstream is the job. This is a laptop running
the same hop, not a colo POP.

Hold 5s. Mixer starts.

The mixer is set to five thousand a second. Grafana is the count.

Mostly clean curation, with some suppress and some shade: the jobs that
have always been tight on time.

Green on the barcode is a forward. Drop should sit at zero on this mix.

Hold 6s. Hose fills.

Grafana is the unsampled view of the same hop.

## 4:32 · Grafana

Hold 6s. Dashboard loads.

Auctions per second is the mixer, as a raw count, not a claim about
anyone's production.

Forward rate should sit near one hundred percent here. Suppress and
shade still forward; those are legal jobs.

Hop p99 has to live far under the 150 millisecond tmax, because this hop
sits inside the auction clock.

Headroom is whatever is left for the rest of the bid. The mix is clean,
suppress, and shade. Synthetic. Same hop, many times a second.

## 5:15 · Close

Hold 4s. Back on the console.

That is ARTF in the stack you already run: a container beside the
exchange, a patch on OpenRTB, a DSP bidding on what you forwarded, and
the same hop after the bid comes back. Independent simulation. Not an
IAB product.

## 5:36 · Outro card · hold 15s

No voice. End 5:51.

---

If the prompter clock goes red, you are past the beat. Finish the
sentence, then skip ahead with right arrow, or pause (space) and wait
for picture.
