# Unity and Vungle profiles validate the exchange's published DSP contract.

The `unity` and `vungle` profiles check requests emitted by the exchange and responses returned by a DSP. Generic OpenRTB validation remains active. Unknown extension members remain open. Select the OpenRTB version independently: Vungle publishes a 2.5 contract, while Unity's current documentation mixes 2.6 integration guidance with older field descriptions.

## Unity checks its placement and bidding constraints.

Unity checks one impression with id `"1"`, first-price auctions, secure impressions, unambiguous banner/interstitial flags, and its published video duration and position values. Documented SKAdNetwork, ATT and ad-experience extension types and ranges are checked. Missing privacy identifiers are permitted. [Request contract](https://docs.unity.com/en-us/grow/programmatic/unity-exchange/bid-requests), [privacy contract](https://docs.unity.com/en-us/grow/programmatic/unity-exchange/data-privacy).

Bidding responses require currency, seat, markup, categories and creative type. Prices must be positive with at most three decimal places. Advertiser domains are limited to one domain without protocol, paths or `www`. A billing URL satisfies tracking requirements; its alternative requires a win URL and impression trackers. Observable advertised-app fields require bundle and store URL. Banner bids require dimensions. Other configured currencies remain valid. [Response contract](https://docs.unity.com/en-us/grow/programmatic/unity-exchange/bid-responses).

Pair validation matches source app and network identifiers against the uniquely resolved impression. A selected deal ID must belong to the explicit, complete and unambiguous captured deal list. App-store attribution identifiers are compared with advertised bundle. Version 4 attribution uses the four-digit source identifier. Signatures are checked structurally, without cryptographic verification. [Attribution contract](https://docs.unity.com/en-us/grow/programmatic/unity-exchange/ios14-support).

## Vungle checks its producer fields and attribution extensions.

Vungle checks published required request fields, first-price and USD request offers, secure impressions, required SDK metadata, non-null persistent-CTA eligibility, supported banner MIME types, video delivery/protocol/playback subsets, bitrate limits and rewarded flags. It checks conditional IPv4/IPv6 alternatives, ATT and App Set scope, consistent IDFV aliases, and SKAdNetwork identifiers and versions. Supplied legacy supply-chain containers and members receive nested type and flag checks. Optional Regs requires its documented extension object when supplied, without requiring optional privacy children. [OpenRTB 2.5 contract](https://support.vungle.com/hc/en-us/articles/360045953431-Vungle-Exchange-OpenRTB-2-5-Integration-Guide).

Response checks cover required markup/domains, integer feature flags, nullable documented defaults, version-specific attribution campaign ranges, four-digit source identifiers, fidelity objects, UUID syntax, timestamp strings, overlay dependencies and the VAST prefix byte limit. Pair checks cover source app, network, advertised attribution versions and persistent-CTA bundle requirements. Non-USD response currency produces a warning because Vungle documents coercing it to USD. Recommended seat, creative and notification fields remain optional.

Native checks cover required main-image asset id, image URL extensions and supplied icon dimensions. Unsupported JavaScript tracking, fallback links and viewability trackers produce warnings. GIF input remains valid because the feature table explicitly describes rendering it as a static image. [Native integration contract](https://support.vungle.com/hc/en-us/articles/8582189840923-Vungle-Exchange-OpenRTB-2-5-Native-Ad-Integration).

## Document contradictions and external state limit these checks.

Unity's response example includes `PLAYABLE`, absent from its creative-type table. Both are accepted. Its LGPD type column conflicts with the example, so no closed LGPD type is imposed. Vungle's examples use request `sko: 0`; both integer flags are accepted. Its companion banners omit MIME types, so those do not inherit the main-banner presence requirement. Vungle 4.0 sourceidentifier can replace campaign, as its field description states.

Account currency configuration, SDK/OS capability, publisher registrations, Apple signatures, asset bytes, runtime MRAID behavior and transport headers require evidence outside the supplied JSON. HTML/VAST policy requirements requiring parsed creative internals are not inferred from creative-type labels. Pair checks defer missing or ambiguous impression identities. Neither profile certifies full exchange acceptance.

The fixture corpus contains positive alternatives and targeted adversarial mutations. Its manifest records profile, protocol version, direction and expected finding paths. Sources were reviewed on 2026-10-09.
