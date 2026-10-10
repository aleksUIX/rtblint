# Xandr bidder traffic and Magnite DV+ xAPI have distinct contracts.

Reviewed on October 9, 2026. The [pinned source inventory](xandr-magnite-sources.json)
records repository commits and SHA-256 digests. These profiles add locally
checkable vendor constraints to the selected OpenRTB snapshot. They do not
certify complete exchange acceptance.

## Xandr uses the bidder-facing direction.

The `xandr` profile covers Monetize requests sent to bidders and bidder
responses. Supplier-ingest `markup_delivery` is outside that scope. Neither
seller-member identity nor video context is marked mandatory in the outgoing
request tables. Optional absence therefore passes. Provided extensions receive
type checks, payment enums, transaction-ID type checks, deal flag checks and
the 64-domain request bound. Visibility-limited identities remain optional.
[Outgoing request source](https://github.com/MicrosoftDocs/xandr-docs/blob/ee7a9adff52bd888ef75f6d9b5b2bb0731bb1222/xandr-docs/bidders/outgoing-bid-request-to-bidders.md).

Response checks require the seat, accept registered nonnumeric seat codes,
validate custom-macro strings and their 550-character bound, check notification
macro placement, and enforce static notify URL and Unicode DSA-name limits.
Non-impression payment bids need `burl` and USD. Account feature enablement,
creative registration and post-expansion URL lengths remain unavailable.
Existing generic DSA validation supplies its required-name checks.
[Incoming response source](https://github.com/MicrosoftDocs/xandr-docs/blob/ee7a9adff52bd888ef75f6d9b5b2bb0731bb1222/xandr-docs/bidders/incoming-bid-response-from-bidders.md).

Contexts 8, 9 and 10 extend the earlier video range for accompanying content.
Pairs check payment offers, individually oversized pod creatives and guaranteed
slot declarations. Candidate bids may exceed a pod's total capacity because
the platform selects a subset. The validator does not sum every offer as a
winning sequence. Missing or malformed captured offer data is handled
conservatively.
[Bidder 2.6 source](https://github.com/MicrosoftDocs/xandr-docs/blob/ee7a9adff52bd888ef75f6d9b5b2bb0731bb1222/xandr-docs/bidders/integration-with-openrtb-2-6.md).

## Magnite uses the public DV+ xAPI model.

The `magnite` profile checks all 111 field descriptors in the pinned public
proto2 extension model across 45 messages. This includes nested window, floor,
proxy-demand, targeting, request and response extensions, and extensions within
encoded Native payloads. The descriptor inventory is
[replayable](xandr-magnite-descriptors.json). Optional fields, including account,
site and zone identities, remain optional. Unknown extensions stay open.
[Public model source](https://github.com/MagniteEngineering/xapi-proto/blob/69975f4206b85dd8db159231d7b3bc8f33232f15/src/proto/com/magnite/openrtb/v2/openrtb-xapi.proto).

Checks cover scalar and repeated types, integer widths, documented closed
values, list bounds and response-time rounding. Deprecated fields produce
warnings. Pairs check HTTPS impression trackers for secure impressions,
declared media, API frameworks, explicit banner sizes and requested markup
MIME types. Slot-name configuration prevents an unwarranted size rejection.
Malformed supplied media containers defer media, API, MIME and size comparisons.
Standalone request type errors and independent secure-tracker checks remain active.

The public JSON integration guide redirects to login. Boolean flags accept
both protobuf booleans and integer OpenRTB flags. Key/value collections accept
the documented map shape and repeated protobuf messages. Optional null values
remain unset. This compatibility envelope is deliberate, not proof that every
accepted representation is deployed on the JSON endpoint. Negotiated fees,
floors, status catalogs, size/vendor catalogs, account configuration and actual
rendered creative behavior remain gaps. Streaming and SpringServe are separate
protocols.

## Source fixtures preserve positive and negative boundaries.

The synthetic [fixture corpus](../../crates/rtblint-core/tests/fixtures/exchange-depth/xandr-magnite/cases.jsonl)
contains 465 cases: 361 Magnite and 104 Xandr. Type cases come from the pinned
descriptor inventory without reading validator implementation. Separately
authored prose controls cover boundaries, direction, paired observations and
unknown data. The fixture builder is committed beside the corpus. Each case
records its source basis and expected profile errors. The integration harness
also checks that vendor constraints do not leak into the default Spec profile.
