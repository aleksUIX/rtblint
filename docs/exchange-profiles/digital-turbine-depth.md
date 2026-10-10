# Digital Turbine validates its public request and response contract.

`digital-turbine` validates Digital Turbine Exchange requests sent to DSPs, DSP bid responses, and request-response pairs. Its primary source is the current [OpenRTB 2.5 guide](https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs). Request checks describe the exchange's documented producer payload. They do not imply that Digital Turbine rejects a DSP response because an unused request field is absent.

The guide lists support for OpenRTB 2.2, 2.3 and 2.5. Its current tables also include newer fields such as `imp.rwdd` and `video.plcmt`. The regression corpus exercises 2.5 and the latest 2.6 catalog, with narrowly scoped compatibility for the documented fields. This does not certify the complete vendor protocol for every OpenRTB version.

## Requests receive nested field and producer checks.

The profile checks required request context, app and publisher metadata, impressions, banner and video fields, metrics, privacy fields, and supply-chain members. It checks the published video constants and supported protocol values, framework labels, the store-subcategory limit, device ranges, user percentages, and typed app, device, user, impression and regulatory extensions.

Extended identifiers receive recursive checks for sources, UID arrays, IDs, agent types and extension objects. Supply-chain validation checks the chain version, completeness flag, nonempty node array, and each node's advertising-system domain, seller ID and payment flag. Unknown extension members remain open. Optional identifier omissions do not create IDFA or consent errors inferred from geography.

Documented legacy paths remain accepted only under this profile: `source.ext.schain`, `user.ext.eids`, `user.ext.consent`, `regs.ext.gdpr` and `regs.ext.us_privacy`. The 2.5 catalog also accepts the guide's GPP fields, `imp.rwdd` and `video.plcmt`. `geo.dma` is accepted because it is documented, but its JSON type is unspecified and is not guessed.

## Native requests use the documented representations.

Both the guide's direct Native object and a canonical `native.request` JSON string reach decoded Native 1.2 checks. The profile recursively checks Native metadata, assets, title, image, video, data and event trackers. Published producer tables define the asset IDs and their corresponding asset types, context values and placement constants. These are request producer constraints, rather than a closed list of accepted future creative formats.

Direct Native strings and `request_native` are not documented alternatives for this profile. The corpus checks that the compatibility hook does not grant those carriers an exemption. The guide misspells placement keys in a table; canonical `plcmttype` and `plcmtcnt` follow its examples and the linked Native specification.

## Responses receive metadata and SDK feature checks.

The profile requires seat, campaign ID, creative ID and advertiser domains, and enforces USD. The [auction macro guide](https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs/supported-auction-macros) requires an auction-price macro in a documented tracking carrier and limits minimum-to-win substitution to the win notice. Markup execution and the actual position of a tracking pixel cannot be established from these JSON checks.

Typed response extensions cover creative type, SKAdNetwork, SKOverlay, AutoStore, dual end cards and click trackers. An unknown creative-type string remains accepted because the current guide says unsupported attributes are ignored. AutoStore requires an advertised store ID when enabled. Dual end cards require the advertised app bundle. SDK click trackers are required for display bids only when the payload explicitly enables both the corresponding SDK feature and its autoclick setting. Disabled settings and ordinary video creatives retain their documented alternatives.

The [SKAdNetwork guide](https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs/skadnetwork) supplies version, campaign, source-identifier, nonce, timestamp and fidelity constraints. The profile validates UUID syntax, decimal timestamps, campaign bounds, four-digit version 4 source identifiers, and required nested members. The [SKOverlay guide](https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs/skoverlay) supplies flag and delay ranges. Cryptographic signature validity remains outside the JSON contract.

## Pairs check the fields supplied in both payloads.

Pairs check SKAdNetwork source app, network and supported-version agreement, and require attribution only when the request offers a usable network list. A request with `skadnetids: null` retains the documented behavior for a network absent from the publisher's list. This exception does not remove other SKAdNetwork request checks.

Deal IDs are compared against deals in the matching impression. [Rewarded playable](https://docs.digitalturbine.com/dt-ads-demand/ad-creative-types/rewarded-playables) responses receive their conditional interactive-attribute check when the request explicitly offers that format. Native response assets are matched against the decoded request, including required assets, for both supported request representations.

## Conflicting source labels remain advisory.

The current full guide and older [onboarding](https://dsp-form.prod.fyber.com/) and [content guidelines](https://www.digitalturbine.com/legal/fyber-demand-content-guidelines) disagree about mandatory response attributes, categories and creative type. Their omission produces `openrtb.profile.digital_turbine.source_conflict` warnings. Present fields still receive type checks. The current guide also conflicts with its own prose or examples about request user context, currency, blocklists, transaction ID, publisher name, security, browser clicks, dimensions, rewarded support, deal auction type, deal seats, response markup and billing URLs. Those omissions remain advisory.

The guide describes some boolean extensions with numeric examples. Those specific fields accept JSON booleans or integer flags; unambiguous integer flags remain restricted to 0 and 1. `dspdualendcard` retains the explicitly documented JSON boolean type. Conditional SDK availability, account configuration, privacy jurisdiction, delivery, compression and creative content policy require external evidence.

The separate depth corpus contains 343 cases: 264 requests, 64 responses and 15 pairs. Forty-five controls have no profile findings. Adversarial controls assert finding ID, path, severity and validity. They cover nested types, boundaries, source conflicts, disabled feature settings, unknown extension members, Native carriers, SKAdNetwork null handling and Spec profile isolation. The earlier expansion corpus retains 19 Digital Turbine response cases with its contradictory presence requirements updated to warnings.

The [source and verification receipt](digital-turbine-depth-evidence.json) records official URLs, retrieval hashes, scope limits and checked corpus counts. Public reproduction replays the committed fixture JSON through `cargo test -p rtblint-core --test exchange_depth_digital_turbine`. Full mutation regeneration requires the local audit generator.
