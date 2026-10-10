# BidSwitch buyer and supplier profiles follow separate public contracts.

`bidswitch` checks BidSwitch buyer protocol 5.7. `bidswitch-supplier` checks supplier protocol 1.1. Both versions support OpenRTB 2.6. The source inventory pins 81 public primary pages retrieved on 2026-10-09, with SHA-256 receipts. The descriptor inventory contains 331 field-name and JSON-type facts. Proprietary page text is not bundled.

The profiles add request minima, inventory context, extension types, explicit extension enumerations, identifier lengths, advertiser-domain encoding, native object forms and DSA name boundaries. Pair validation checks banner preview and markup, video protocol and URL requirements, audio URL selection, native assets and Ad Choices requirements, server-to-server macro restrictions, secure browser notices, supplier-conditioned response fields and supplier seat requirements. Buyer responses also check seat grouping, at most two bids per ad slot, billing macros, required supplier click macros and repeated impression macros. Optional fields and unknown extension members remain open.

## The documented directions have different requirements.

The [buyer request table](https://protocol.bidswitch.com/standards-v57/bidrequest.html) requires device, user, tmax, cur and ext. An empty user object is expressly valid for privacy opt-outs. The [supplier request table](https://protocol.bidswitch.com/ssp-protocol-v11/ssp-request.html) makes user, tmax, cur and ext optional. Device geo is recommended for buyers and required by the supplier table. IP availability depends on CTV and privacy context; absent IP is not universally rejected.

Audio duration and protocol rows describe recommendations even when their field labels lack the usual optional marker. These are not converted into mandatory fields. Explicit video durations may use rqddurs in place of duration bounds. Modern placement fields are accepted without demanding a deprecated counterpart.

Buyer Native.request and supplier Native.request_native are objects. Supplier responses use Bid.adm_native. Buyer responses use Bid.ext.native. Their native payloads receive native request, response and paired asset validation. Supplier requests require native version 1.2. Interest-group responses can omit classical seatbid when ext.igbid supplies an actual interest group; buyer-specific JSON signals remain unconstrained.

## Billing and compatibility exceptions are explicit.

The [billing overview](https://protocol.bidswitch.com/standards-v57/burl.html) says nurl is optional when burl is used, overriding the older bid table's missing optional marker. Buyer billing URLs require the win-price macro. The same macro cannot appear in both adm and nurl, or more than once in adm. It is forbidden in VAST and DAAST document URLs. HTTPS applies to secure browser notifications; HTTP remains valid for server-to-server billing and notifications.

Both directions let wseat take precedence over bseat. Buyer 5.7 explicitly retains legacy consent, EID, supply-chain, inventory-partner and several impression-extension paths. Those exceptions stay scoped to the buyer profile. Supplier 1.1 receives no inferred legacy-path exemptions.

## Coverage has bounded limits.

Account-specific seat defaults, custom bidder extensions, provider ID registries, MicroAd premium account classification, upstream creative approval, currency conversion, floor acceptance and HTTP callback status are unavailable from a payload. Macro expansion and document contents behind VAST or DAAST URLs require runtime evidence. Supplier IDs are enforced only for exact documented identification strings. If an impression supplies several media types or any malformed media container, media-specific mandatory-field checks defer. Standalone request validation still reports malformed media types. Transport headers, response compression and live processing time are outside the JSON contract.

The fixture oracle distinguishes full-payload valid controls from descriptor type controls. Type controls check only the independently sourced JSON shape; they do not claim that an isolated extension object satisfies every surrounding contract. Positive, malformed, null, conditional, privacy, boundary and paired controls run under each directional profile, with default Spec isolation checked for every case.
