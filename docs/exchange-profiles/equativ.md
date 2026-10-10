# Equativ profiles preserve public field support and supplier routing limits.

`equativ-supplier` targets the public supplier OpenRTB API. `equativ` targets the public fields shared with bidder integrations. The [bidder field reference](https://help.equativ.com/connect-bidder-to-equativ-ssp-supported-fields), updated July 2026, refers to the supplier request and response field tables and excludes private bidder-specific fields. Shared field support does not transfer supplier authentication or processing limits to bidder input.

Five primary documents are pinned by SHA-256 and retrieval date in the source inventory. The field inventory accounts for 343 table entries, including canonical fields inherited from the selected OpenRTB schema and explicit source conflicts. The extension oracle covers 66 shape facts, with independently selected malformed, null, conditional and boundary cases. Full-payload valid controls are distinguished from individual field-shape controls.

## Supplier checks follow documented processing behavior.

The [supplier request table](https://help.equativ.com/open-rtb-api-integration-bid-request-specification) requires device and user, site domain and page, DOOH domain, supplied format dimensions and media MIME lists. It requires a mapped buyeruid, whose actual matching cannot be determined from JSON. Video placement may use modern plcmt or the listed legacy placement field. The guide explicitly supports plcmt 1 through 9, including current CTV values.

The [supplier setup guide](https://help.equativ.com/open-rtb-api-integration-get-started) documents two JSON methods for network identification: publisher.id or configured BidRequest.ext.network_id. Numeric network IDs and canonical string publisher IDs remain accepted. The same guide limits supplier requests to ten impression objects and responses to ten bids. Explicitly zero dimensions and mismatched or missing placement IDs in non-instream multi-impression requests receive processing-loss warnings. Multiple instream slots remain accepted. Unified-request activation and account routing cannot be verified from a capture.

## Extension validation retains documented alternatives.

Both directions check bidder placement extensions, optional format floors, root and impression bid-feedback arrays, device ATT values, site AMP flags, video orientation, DSA payloads and the four response deal-type strings. Publisher directpay must appear only once per supplier request. Missing per-format floors use the documented fallback; the validator does not impose floor acceptance that Equativ says it does not perform after bidding.

The guide explicitly accepts legacy consent, GDPR, EID and supply-chain paths alongside their mainline counterparts. Mainline values take precedence when both are sent. Supplied legacy supply chains receive nested type and mandatory-member checks. Supply-chain absence is not universally rejected, because owned inventory is exempt.

DSA request flags use the published 0 through 3 or 0 through 2 ranges. Each response bid must contain a DSA object when the request requires DSA with dsarequired 2 or 3. Requirements 0 and 1 preserve absent-object acceptance. DSA advertiser and payer names are checked at the documented maximum of 100 Unicode characters. Tracking URLs accept the documented string type and prose array-of-strings alternative. Optional feature tokens and billing callbacks are not made mandatory.

## Source conflicts are recorded instead of guessed.

The Publisher.id table says INTEGER while [official examples](https://help.equativ.com/openrtb-api-integration-samples) use strings. The response table lists integer response IDs and string no-bid reasons, in conflict with canonical OpenRTB. The tracking URL type column says STRING while its description says an array. Other columns label EIDs as objects despite describing arrays, playbackmethod as integer despite OpenRTB arrays, and pod IDs as strings despite canonical integers. Canonical types remain authoritative for unresolved conflicts; the publisher ID and tracking-URL compatibility envelopes are explicitly scoped.

Imp.bidfloor is labelled required but has default 0. TagID is labelled required while the setup guide shows accepted requests without it and describes processing only the first impression. Missing floors and TagIDs are therefore not fabricated wire errors. Video.ext.rewarded is described as obsolete and pre-2.6, without an explicit guarantee of current acceptance. The canonical moved-path error remains in current 2.6 validation. The currently maintained table explicitly lists user.ext.eids as its OpenRTB 2.5 representation. This exact legacy array path receives nested EID and UID type checks. Its OBJECT type column conflicts with the explicit array description. App.bundle is marked required in its child table while the parent description strongly recommends an App ID or Bundle ID. Missing bundle receives an advisory, including valid App ID alternatives, until that contradiction is resolved.

Native requests retain the canonical encoded-string form and native inner/pair validation. Transport headers, authentication against live account state, ownership classification, currency conversion, creative approval and HTTP callback behavior require external evidence.
