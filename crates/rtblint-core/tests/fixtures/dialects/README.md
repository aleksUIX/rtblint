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
