# Exchange profiles follow documented integration directions.

The 0.14.0 release deepens the four existing vendor profiles and adds 22 profiles across 16 additional products or integration families. There are 26 vendor and producer profiles plus `spec`. Directional profiles are separate contracts, not additional exchange companies.

All profiles retain versioned canonical OpenRTB validation. Vendor checks cover retrieved public field declarations and documented local conditions. Source manifests pin official repository commits or fetched byte hashes. Independent fixtures exercise valid values, wrong types, bounds, optional absence, nulls, unknown fields and pairing context. A field-shape inventory does not certify every behavioral rule or complete exchange acceptance.

## Choose the direction represented by the capture.

- `google-ab`, `prebid-server`, `xandr`, `magnite`, `dv360`, `index-exchange`, `unity`, `vungle`, `bidswitch`, `bidswitch-supplier`, `inmobi`, `inmobi-supplier`, `mobilefuse`, `mobilefuse-sdk`, `digital-turbine`, `equativ`, `equativ-supplier`, `triplelift-supplier`, `adform-handler`, `yandex-sdk-bidding`, `pubmatic-openwrap` and `pubmatic-openwrap-ctv` have documented request and response scopes. Buyer, supplier and SDK names identify the applicable integration.
- `index-exchange-seller`, `commerce-grid` and `sovrn` have request scopes.
- `applovin-alx` has a response scope.

Unsupported directions and OpenRTB 3.0 receive canonical validation and `openrtb.profile.scope.unsupported`. An unsupported direction does not silently claim vendor coverage. The two OpenWrap profiles describe PubMatic's pinned public middleware implementation; the full private PubMatic exchange contract remains deferred.

Paired validation checks a response and uses its originating request as reference context. Run standalone request validation separately for a complete request report. Pair comparisons defer when the relevant captured reference is malformed, ambiguous or unavailable through stored-request expansion. Transport status, account configuration, creative rendering and remote approval require other evidence.

## Review the scope and evidence for each family.

- [Google Authorized Buyers and Prebid Server](google-prebid-depth.md): recursive protocol extensions, default and normalization behavior, billing and media pairing.
- [Xandr and Magnite DV+ xAPI](xandr-magnite.md): bidder-side constraints, nested extension descriptors and pair boundaries.
- [Index Exchange and DV360](index-dv360.md): seller versus buyer fields, privacy aliases, video and banner constraints.
- [Unity and Vungle](unity-vungle.md): response fields, notification alternatives and SKAdNetwork controls.
- [BidSwitch buyer and supplier](bidswitch.md): independent protocol directions and nested field inventories.
- [InMobi and MobileFuse](inmobi-mobilefuse-depth.md): outgoing versus supplier traffic, SDK placement and identity extensions.
- [ALX, Commerce Grid, Digital Turbine and Sovrn](expansion-batch.md): selected public directions, Native encodings and response boundaries.
- [Digital Turbine's current public guide](digital-turbine-depth.md): request tables, response extension fields and SKAdNetwork conditions recovered from the migrated documentation.
- [Equativ bidder and supplier](equativ.md): recursive table fields, legacy aliases, DSA and pairing.
- [TripleLift supplier](triplelift-depth.md): request objects, Native 1.2, privacy aliases and response pairing.
- [Adform handler and Yandex SDK bidding](handler-sdk.md): identity containers, privacy fields, signal carriers and integration-specific conditions.
- [PubMatic OpenWrap](pubmatic-openwrap-depth.md): custom wire types and the selected CTV middleware stage.

[Coverage results](coverage-results.json) records final corpus counts and regression aggregates. [Discovery inventory](discovery-inventory.json) records all 37 reviewed additional protocol surfaces, including explicit deferrals. [Remaining source review](remaining-exchange-source-review.md) and [FreeWheel source review](freewheel-source-review.md) explain current documentation access gaps.

## Reproduce the consumer contracts.

```sh
cargo test --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
python3 scripts/sync-profile-consumers.py --infra-dir ../rtblint-infra
python3 scripts/sync-profile-findings.py --infra-dir ../rtblint-infra
```

The CLI, native MCP, npm CJS and ESM, browser WASM and Worker WASM share the same profile IDs and core. Companion web checks replay independently asserted reference examples, verify support directions and preserve raw response bytes for ALX size limits. `scripts/check-profile-distributions.mjs` and `scripts/check-profile-mcp.py` compare complete finding multisets with a supplied native corpus oracle.

Corpus comparisons for the four existing profiles select the same profile in the old and new versions. Added profiles use an explicitly labeled old `spec` fallback because the old release cannot select them. These constructed boundary cases measure check behavior, not a production detection rate. Private D1 payloads are excluded from this repository.
