# Dialect fixtures preserve the native object regression cases.

These synthetic payloads cover the shapes found in tester usage without
copying visitor data or identifiers. `cases.json` pins the version, dialect,
validation mode, validity, and complete finding IDs, severities and paths.
The fixture runner rejects extra findings as well as missing findings.

Coverage includes protobuf native requests and responses under both field
spellings, boolean native flags, oneof conflicts, null alternatives, malformed
objects, required assets in paired responses, and encoded string alternatives.
The mixed request must fail under both dialects for the appropriate fields.
Site domain, legacy GDPR, invalid JSON, and zero-valued video skip controls
keep real payload errors distinct from validator regressions.

The wire alternatives and boolean fields follow the [IAB protobuf schema](https://github.com/IABTechLab/openrtb-proto-v2/blob/master/openrtb-core/src/main/protobuf/openrtb.proto).
The site domain control follows [OpenRTB 2.6, section 3.2.13](https://github.com/InteractiveAdvertisingBureau/openrtb2.x/blob/main/2.6.md#3213---object-site).

Run `cargo test -p rtblint-core --test dialect_fixtures`.

Privacy fixtures also cover legacy 2.5 GDPR and consent, the 2.0/2.1 User
extension location for US Privacy, and legacy Regs US Privacy. They pin malformed
strings and base-spec extension types, canonical/legacy conflict precedence,
null canonical consent, equal duplicates, legacy EIDs under COPPA and US
Privacy, Purpose 1 checks, and the actual GPP source path. Vendor extension
wire types remain governed by their profiles. No stored visitor values appear
in these fixtures.

The legacy locations follow the [OpenRTB 2.6 migration notes](https://github.com/InteractiveAdvertisingBureau/openrtb2.x/blob/main/2.6.md)
and [IAB US Privacy extension](https://github.com/InteractiveAdvertisingBureau/USPrivacy/blob/master/CCPA/OpenRTB%20Extension%20for%20USPrivacy.md).
