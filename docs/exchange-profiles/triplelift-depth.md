# TripleLift supplier validation follows the current documentation

Reviewed 2026-10-09 against the public knowledge base pages updated September 16, 2026. `triplelift-supplier` covers S2S supply requests sent to TripleLift and their corresponding responses. It does not impose supplier requiredness on requests TripleLift sends to DSPs.

Each supplier impression requires a placement tag. Banner formats must contain a size. Exact width and height remain a documented fallback, with an advisory because the format table marks the array required. App bundle is also advisory when absent: its table marks it required but the adjacent prose explicitly says it is not strictly required. Device, user identifiers and consent remain optional. The current object tables support multiple formats and audio, so the historical native-only contract does not restrict current requests. Documented privacy declarations in `regs.ext` remain accepted as well as modern mainline fields.

Source: [current bid request objects](https://support.triplelift.com/en_US/openrtb/openrtb-2x-bid-request-objects).

Native 1.2 encoded request contents receive recursive checks for every unambiguous published field shape, including asset containers, IDs, integer flags, title/image/video/data attributes and event methods. Image type is required. Missing image dimensions produce the documented recommendation as an advisory. Ordinary Native Ads checks retain unique assets, required fields, one-of content and supported enums.

The OpenRTB media envelope keeps a string `native.request`, as explicitly specified by the current envelope table. The Native page's reference to direct objects describes the inner Native representation and does not prove that every supplier endpoint accepts object-valued `native.request`.

Source: [current Native request objects](https://support.triplelift.com/en_US/openrtb/native-request-object-v12).

Native responses validate nested field types, encoded markup, tracking event/method IDs and wrapped HTML for JavaScript trackers. Paired remote assets require declared `aurlsupport=1`; omission means unsupported. Returned event/method combinations must match the declared request options. Malformed or ambiguous request references defer these comparisons. Wrapped payloads retain the `adm.native` path in new diagnostics. Standard Native validation continues to check asset content, links and paired required assets.

Sources: [current bid response objects](https://support.triplelift.com/en_US/openrtb/openrtb-2x-bid-response-objects), [current Native response objects](https://support.triplelift.com/en_US/openrtb/native-response-object-v12).

The Native response Data table repeats `vasttag` from the Video table. The request page also mislabels several standard types and names. Those transcription conflicts do not justify changing standard wire types or aliases. Remote creative contents, transport status, cookie synchronization and account registration remain unverified. The old 2.3 guide's ten-bid cap, integer Native version, fixed impression ID and required advertiser domain are not applied to these current tables.

Pinned source hashes and independent fixtures accompany this profile. They establish the implemented boundaries, not complete behavioral certification.

The independent corpus contains 145 cases, including 73 positive controls. Request, response, pair and default-spec groups pass. `triplelift-case-generate.py` reproduces the JSON exactly; `triplelift-verification.json` records the corpus hash and focused test commands.
