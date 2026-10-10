# Four exchange contracts add specific validation.

These profiles extend canonical OpenRTB validation. Each has an explicit direction and public evidence. The fixture manifest records the version, direction and expected finding paths. Positive fixtures exercise defaults, alternatives and account-dependent omissions.

## AppLovin ALX validates standard DSP responses.

`applovin-alx` checks required markup or the documented `adm_native` alternative, advertiser metadata, USD, supplied seat syntax, creative types, unsupported APIs, loss macros and typed extensions. Paired checks require duration for CTV and prevent the publisher bundle being used as the advertised app.

SKAdNetwork checks include version, campaign ranges, advertised app matching, required fields, UUID nonces, decimal timestamps, fidelity entries, HTML click tracking and SKOverlay delays. Paired checks compare source app and network IDs. Version 4 accepts ALX's documented campaign alias.

The raw JSON response limit errors above 4096 UTF-8 bytes. The source says 4 KB without defining decimal or binary units, so the 4001 through 4096 interval remains unresolved.

SDK bidders require a separately identifiable contract. Creative retrieval, signature verification, HTML tracking placement, account hardcoding and viewability declarations remain external. Omitted creative type is accepted because accounts may preconfigure it. The source examples omit required billing URLs, so fixtures follow the explicit field table. Native video is rejected by the creative guide. The Native 1.2 content contract rejects malformed or non-object `adm_native` content. Its wire representation is unspecified, so direct objects and JSON object strings remain accepted, including a `native` envelope.

Sources: [bid responses](https://support.applovin.com/en/max/demand-partners/demand-side-platforms/applovin-ortb-specification/bid-responses), [creative types](https://support.applovin.com/en/max/demand-partners/demand-side-platforms/applovin-ortb-specification/creative-types).

## Commerce Grid validates custom supplier ingest.

`commerce-grid` checks required device, user, currency and supply-chain data; source enums; web buyer IDs; app identifiers and SDK metadata; banner formats; video fields and duration alternatives; deals; metrics; SKAdNetwork and typed extensions.

Six documented Native wire forms reach canonical Native checks: direct object or string, `request` object or string, and `request_native` object or string. The decoded payload also receives recursive type checks for context, placement, assets, title, image, video, data, event trackers, extension objects and API lists. The source's privacy flag is checked on the decoded payload. Zeroed advertising IDs remain accepted.

The guide permits an unknown page to be absent. App publisher metadata depends on supplier role, network ID may arrive through URL parameters, and GPID has conflicting labels. These are not unconditional missing-field errors. Publisher filtering guidance and jurisdiction-specific privacy obligations require external context. Obvious type errors in the guide do not replace canonical array types. The event tracker table lists singular `method`, while the linked Native 1.2 schema requires `methods[]`. Canonical request methods remain an array.

Source: [custom server-to-server guide](https://docs.commercegrid.criteo.com/kb/guide/en/custom-server-to-server-openrtb-Vy9QrGVwyl/Steps/2366154).

## Digital Turbine validates its public exchange contract.

`digital-turbine` checks documented exchange requests, DSP responses and request-response pairs. Checks include nested identifiers and supply chains, privacy and SDK extensions, Native requests, producer video values, response metadata, USD, auction macros, SKAdNetwork and conditional SDK feature behavior. Empty attributes and unknown creative-type strings retain the current guide's tolerant behavior.

The current full guide supersedes the earlier partial public-source audit. Conflicting mandatory and optional labels for attributes, categories and creative type now produce warnings. The validator cannot establish that a macro inside markup belongs to a tracking pixel. Account routing, compression, delivery and creative policy require other evidence. [The depth document](digital-turbine-depth.md) records exact direction, compatibility and remaining source limits.

Sources: [current full guide](https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs), [auction macros](https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs/supported-auction-macros), [DSP onboarding](https://dsp-form.prod.fyber.com/), [technical response fields](https://www.digitalturbine.com/legal/fyber-demand-content-guidelines).

## Sovrn validates outgoing buyer requests.

`sovrn` checks placement tags and publisher IDs, exact banner dimensions or a format alternative, required video duration, dimensions and protocols, private deal extension typing and legacy GDPR flag values.

The dedicated video guide describes minimum duration as recommended, so omission is accepted despite the general table's conflicting label. The profile retains canonical types where article descriptions contain errors. It does not infer response requirements, private onboarding IDs or auction type from prose.

Sources: [OpenRTB fields](https://knowledge.sovrn.com/kb/sovrn-ortb-specs), [video fields](https://knowledge.sovrn.com/kb/sovrn-video-fields-and-parameters).

The reproducible expansion corpus contains 482 cases: 106 ALX, 339 Commerce Grid, 19 Digital Turbine and 18 Sovrn. Sixty-eight are positive alternatives. Commerce Grid includes 234 decoded-field mutations, six rich positive controls and six malformed-carrier cases across all wire forms. ALX includes nine malformed native carriers, wrapped object and string controls, and an optional-null control. The raw size fixtures preserve whitespace and Unicode so byte checks use received input. A separate Digital Turbine depth corpus adds 343 controls.
