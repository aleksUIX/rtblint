# rtblint-grpc

gRPC server for [rtblint](https://rtblint.org). Serves `openadtech.rtblint.v1`
over HTTP/2, with server reflection and `grpc.health.v1`.

Sibling of `vastlint-grpc`. Building it was partly a test of whether that design
generalises, and the answer is worth stating precisely, because it is not "yes".

## What did not transfer: the contract

Three of the VAST contract's decisions inverted under OpenRTB's facts. Each is
argued at the point of divergence in
[`rtblint.proto`](../../proto/openadtech/rtblint/v1/rtblint.proto).

**The payload kind is supplied, not inferred.** vastlint dispatches on the XML
root element and never asks what it is looking at. A bid request and a bid
response are both JSON objects with an `id`, and any sniffing heuristic would
fail hardest on exactly the malformed payloads this service exists to diagnose.
So `kind` is required, and a request that omits it is rejected rather than
guessed at.

**The version is a string, not an enum.** This is the opposite of the call made
for VAST versions, from the same reasoning applied to different facts. VAST
published seven versions in fifteen years, so an enum costs nothing. OpenRTB 2.6
alone has had ten dated revisions since 2022, roughly one a quarter, out of
eighteen tracked in total. An enum would mean cutting a wire contract release
every time the IAB published a dated errata. The honest limitation: a string does
not let a caller target a revision the server has not shipped support for, it
only removes the need to regenerate a client to name one the server already has.

**Severity has two levels, not three.** rtblint emits no advisory level.
Declaring one the server never sends would be a field consumers could branch on
and never reach. Proto enums are append-only, so adding it later is not a
breaking change, which is exactly why it does not need pre-declaring.

## What did transfer, and why that is a problem

`limit`, `ratelimit`, `metrics`, `config`, and `deadline` are near-copies of
their vastlint counterparts. The adaptive AIMD limiter, the shedding policy, the
per-caller token bucket, the metric shape, the environment-driven configuration:
all of it ported essentially unchanged, which is the good news.

The bad news is that they now exist twice, in two repositories, with nothing
keeping them in sync. That is precisely the drift the "one core, many surfaces"
architecture exists to prevent, reintroduced one layer up. A fix to the limiter
here does not reach vastlint, and the next validator would make it three copies.

The honest read is that the abstraction boundary is in the wrong place. The
right shape is a published `openadtech-ingress` crate that both servers depend
on, the same way both depend on their own core. That has a real cost, a
versioned dependency between two repositories that are deliberately independent,
which is why it is written down here rather than done in passing.

One thing did carry across correctly, and it is a small argument for the mirror
being worth building: `UNGOVERNED_PATHS` exists in this crate while empty,
because the sibling server learned that a long-lived RPC through a per-request
limiter reports its whole lifetime as one latency sample and ratchets the limit
to the floor. Every RPC here is unary, so it cannot happen yet. The mechanism and
the test are in place anyway so the next person does not rediscover it.

## Running

```sh
cargo run -p rtblint-grpc
# or
RTBLINT_GRPC_ADDR=0.0.0.0:50061 rtblint-grpc
```

```sh
grpcurl -plaintext localhost:50061 list
grpcurl -plaintext localhost:50061 describe openadtech.rtblint.v1.RtblintService
grpcurl -plaintext -d '{"document":"{}","kind":"PAYLOAD_KIND_BID_REQUEST"}' \
  localhost:50061 openadtech.rtblint.v1.RtblintService/Validate
grpcurl -plaintext -d '{"document":"{}","kind":"PAYLOAD_KIND_BID_REQUEST","context":{"dialect":"JSON_DIALECT_PROTO"}}' \
  localhost:50061 openadtech.rtblint.v1.RtblintService/Validate
grpcurl -plaintext -d '{}' localhost:50061 grpc.health.v1.Health/Check
```

## RPCs

| RPC | Purpose |
| --- | --- |
| `Validate` | Validate one bid request or bid response. |
| `ValidatePair` | Validate a bid response against the request it answers. |
| `ValidateArtfEnvelope` | Validate an ARTF `RTBRequest` and the OpenRTB payloads it carries. |
| `ValidateArtfMutations` | Validate an ARTF mutation set against the envelope it answers, optionally applying it. |
| `ListVersions` | The OpenRTB versions this server knows, and its default. |

`ValidatePair` is its own RPC rather than a flag on `Validate` because it asks a
different question. A bid response can be perfectly well formed and still bid on
an impression the request never offered. Those are different failures with
different owners, and the tests assert that the single-payload check genuinely
cannot see the cross-payload one.

## ARTF

ARTF, the IAB Tech Lab Agentic Real Time Framework, mandates gRPC for the
extension point itself. That is why the two ARTF RPCs live here and not only on
the CLI: an orchestrator checking what an agent proposed does it in band, on the
same transport the agent was called over, before forwarding anything.

```sh
grpcurl -plaintext -d @ localhost:50061 \
  openadtech.rtblint.v1.RtblintService/ValidateArtfMutations <<'EOF'
{"rtb_request": "...RTBRequest JSON...", "rtb_response": "...RTBResponse JSON...", "apply": true}
EOF
```

`ValidateArtfMutations` splits from `ValidateArtfEnvelope` on the same reasoning
as `ValidatePair`: whether a mutation is well formed and whether it targets
something the auction actually carries are different questions. With `apply`
set, the response also returns the rewritten payloads and the indexes of the
mutations that went in, along with the OpenRTB findings the mutations
introduced. A mutation the server could not apply is reported in `skipped`
rather than folded into `applied`, because "could not apply" is not "accepted".

`ValidationContext.dialect` is refused on both ARTF RPCs, including when it
names the correct dialect. ARTF carries its OpenRTB messages as protobuf, so
their JSON encoding is a fact about the framework rather than a choice, and
accepting the field would teach callers otherwise.

## Provenance and versions

Every response carries `Provenance` (catalog version, catalog content digest,
engine version) and the `effective_version` the verdict was produced under. The
version echo matters more here than in the VAST server: the same payload is
legitimately valid under 2.5 and invalid under 2.6, so a verdict without its
version is not merely hard to reproduce, it is ambiguous.

An unknown version is rejected with `INVALID_ARGUMENT` listing what is
available, rather than falling back to the default. Validating against the wrong
specification revision and reporting success is the worst outcome available: the
caller gets a verdict that looks authoritative and answers a question it did not
ask.

## Ingress control

Same stack as the sibling server: adaptive concurrency limiting with shedding to
`RESOURCE_EXHAUSTED`, per-caller token bucket, request size caps at the decoder,
and Prometheus on a separate port. Health and reflection are exempt from
shedding. See [`src/config.rs`](src/config.rs) for the environment variables;
every value is printed at startup.

The load measurements and the published SLO live with the vastlint server, which
is where the experiment was run. They have not been reproduced here, and the
numbers should not be assumed to carry over: the workload is JSON parsing rather
than XML tree building, and the corpus is different.

## Building

No `protoc` required. Code generation goes through `protox`.

## License

Apache-2.0.
