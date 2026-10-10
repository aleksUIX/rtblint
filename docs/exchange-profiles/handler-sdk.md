# Handler and SDK profiles validate their documented integration paths.

## Adform Handler has additional extension contracts.

`adform-handler` applies to supplier requests into Adform's OpenRTB Handler and its responses. The Handler publishes OpenRTB 2.5 compatibility.

Request checks cover supplied price-type and privacy extension types, recursive EID/UID types, supply-chain types and mandatory chain fields, DSA enums and targeting parameters. Legacy identity and supply-chain extension locations remain valid under this profile. Its current table also documents canonical `user.eids`, `regs.gpp` and `regs.gpp_sid` despite its OpenRTB 2.5 declaration. These paths receive source-scoped type checks under that version; the default specification keeps its ordinary version boundaries. Adform's request examples use `required` and `params` where the field table uses `dsarequired` and `dsaparams`. Both spellings are accepted. The profile does not treat supported fields as mandatory or invent a closed price-type list.

Responses validate DSA types, targeting parameters, the always-included payer field and the 100 Unicode-character limit. Paired validation requires response DSA when the request requires it. Native checks reject image assets with no usable dimensions. This necessary check does not resolve every SDK ratio representation.

Dynamic placement approval, inventory mapping, HTTP fallback, header-derived values and account settings require external context. Examples contain incomplete media fields and malformed response JSON, so they are evidence of explicitly described alternatives, not universal validation oracles.

Sources: [setup](https://www.adformhelp.com/hc/en-us/articles/9739105036177-Set-Up-Custom-Integrations-With-Adform-OpenRTB-Handler), [request table](https://www.adformhelp.com/hc/en-us/articles/10431570694033), [request examples and image dimensions](https://www.adformhelp.com/hc/en-us/articles/9739088948113-Adform-OpenRTB-Handler-Sample-Bid-Requests), [response table](https://www.adformhelp.com/hc/en-us/articles/9739105213841-Adform-OpenRTB-Handler-Bid-Response-Specifications).

## Yandex SDK Open Bidding uses token-based requests and responses.

`yandex-sdk-bidding` covers the published Android and iOS Open Bidding integrations, which specify OpenRTB 2.5. It checks required app, device, timeout, currency, placement and SDK token data; ad-type enums; privacy extension types; response signal data; one-seat and one-bid shape; and notification macro placement.

Fullscreen and native examples omit ordinary media objects. This profile accepts the documented SDK envelope. Response signal data stays opaque because the client SDK consumes it. Ordinary DSP markup rules do not apply to that envelope. Additional impressions produce a warning because Yandex considers the first; paired responses must reference that impression.

The server endpoint, unit ownership, token freshness and client rendering require SDK or account context. Copy errors in the Native version and video descriptions do not change canonical field types. The profile does not claim the complete Yandex DSP protocol.

Sources: [Android integration](https://ads.yandex.com/helpcenter/en/support/open-bidding/open-bidding-integration-android), [iOS integration](https://ads.yandex.com/helpcenter/en/support/open-bidding/open-bidding-integration-ios).

The fixture corpus has 126 cases: 86 Adform, 39 Yandex and one default-specification isolation case, including 35 positive alternatives. Legacy identity and supply-chain controls cover both OpenRTB 2.5 and the current 2.6 catalog. A warning fixture explicitly verifies that ignored additional impressions retain a valid result.
