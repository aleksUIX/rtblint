//! Checks the generated JSON Schemas in `schemas/` at the repository root.
//!
//! They are produced by `examples/export_json_schemas.rs` and published at
//! <https://rtblint.org/schemas/>, so they must stay structurally sound and
//! must agree with the validator: a payload the linter calls valid cannot be
//! rejected by the schema derived from the same catalog.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use rtblint_core::{canonical_object_catalog, OpenRtbVersion};

fn schemas_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../schemas")
}

fn load_schema(version: OpenRtbVersion, payload_slug: &str) -> Option<Value> {
    let path = schemas_dir().join(format!(
        "openrtb-{}-{}.schema.json",
        version.id(),
        payload_slug
    ));
    let raw = fs::read_to_string(path).ok()?;
    Some(serde_json::from_str(&raw).expect("schema should be valid JSON"))
}

/// A version with a usable catalog gets both schemas; the ones that cannot be
/// validated at all (the 2.6-202204 stub, 3.0's layered envelope) get none,
/// rather than an empty schema that would accept anything.
#[test]
fn every_validatable_version_has_both_schemas() {
    for version in OpenRtbVersion::all() {
        let has_request_root = canonical_object_catalog(*version).is_some_and(|catalog| {
            catalog
                .objects
                .iter()
                .any(|object| object.name == "BidRequest" && !object.fields.is_empty())
        });

        for slug in ["bid-request", "bid-response"] {
            let schema = load_schema(*version, slug);
            if has_request_root {
                assert!(
                    schema.is_some(),
                    "{} has a usable catalog but no {slug} schema",
                    version.id()
                );
            }
        }
    }
}

#[test]
fn schemas_declare_their_dialect_and_identity() {
    for version in OpenRtbVersion::all() {
        for slug in ["bid-request", "bid-response"] {
            let Some(schema) = load_schema(*version, slug) else {
                continue;
            };

            assert_eq!(
                schema["$schema"].as_str(),
                Some("https://json-schema.org/draft/2020-12/schema"),
                "{} {slug} schema declares the wrong dialect",
                version.id()
            );
            assert_eq!(
                schema["$id"].as_str(),
                Some(
                    format!(
                        "https://rtblint.org/schemas/openrtb-{}-{}.schema.json",
                        version.id(),
                        slug
                    )
                    .as_str()
                ),
                "{} {slug} schema has the wrong $id",
                version.id()
            );
            assert!(schema["title"].is_string());
            assert!(schema["properties"].is_object());
        }
    }
}

/// A `$ref` that names a definition the schema does not carry makes the whole
/// document unusable in most validators.
#[test]
fn schema_refs_resolve_within_their_document() {
    for version in OpenRtbVersion::all() {
        for slug in ["bid-request", "bid-response"] {
            let Some(schema) = load_schema(*version, slug) else {
                continue;
            };

            let defs = schema
                .get("$defs")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();

            let mut refs = Vec::new();
            collect_refs(&schema, &mut refs);
            for reference in refs {
                let name = reference
                    .strip_prefix("#/$defs/")
                    .unwrap_or_else(|| panic!("{}: unexpected $ref {reference}", version.id()));
                assert!(
                    defs.contains_key(name),
                    "{} {slug}: $ref {reference} has no definition",
                    version.id()
                );
            }
        }
    }
}

/// The schemas and the validator come from one catalog, so they must not
/// disagree: every fixture the linter accepts has to satisfy its schema.
#[test]
fn valid_fixtures_satisfy_their_version_schema() {
    let cases: &[(&str, OpenRtbVersion, &str, &str)] = &[
        (
            "valid-openrtb-2.0-mobile-video.json",
            OpenRtbVersion::V2_0,
            "bid-requests",
            "bid-request",
        ),
        (
            "valid-openrtb-2.2-secure-pmp-video.json",
            OpenRtbVersion::V2_2,
            "bid-requests",
            "bid-request",
        ),
        (
            "valid-openrtb-2.5-header-bidding-video.json",
            OpenRtbVersion::V2_5,
            "bid-requests",
            "bid-request",
        ),
        (
            "valid-openrtb-2.6-202211-dooh-gpp.json",
            OpenRtbVersion::V2_6_202211,
            "bid-requests",
            "bid-request",
        ),
        (
            "valid-openrtb-2.6-202309-adpod-floors.json",
            OpenRtbVersion::V2_6_202309,
            "bid-requests",
            "bid-request",
        ),
        (
            "valid-schain-baseline.json",
            OpenRtbVersion::V2_6_202606,
            "bid-requests",
            "bid-request",
        ),
        (
            "valid-web-video.json",
            OpenRtbVersion::V2_6_202505,
            "bid-requests",
            "bid-request",
        ),
        (
            "valid-openrtb-2.0-banner-win.json",
            OpenRtbVersion::V2_0,
            "bid-responses",
            "bid-response",
        ),
        (
            "valid-openrtb-2.4-api-protocol-bid.json",
            OpenRtbVersion::V2_4,
            "bid-responses",
            "bid-response",
        ),
        (
            "valid-openrtb-2.6-202606-multi-seat.json",
            OpenRtbVersion::V2_6_202606,
            "bid-responses",
            "bid-response",
        ),
        (
            "valid-openrtb-3.0-layered-request.json",
            OpenRtbVersion::V3_0,
            "bid-requests",
            "bid-request",
        ),
        (
            "valid-openrtb-3.0-layered-response.json",
            OpenRtbVersion::V3_0,
            "bid-responses",
            "bid-response",
        ),
    ];

    for (fixture, version, directory, slug) in cases {
        let schema = load_schema(*version, slug)
            .unwrap_or_else(|| panic!("{}: missing {slug} schema", version.id()));
        let validator = jsonschema::validator_for(&schema).unwrap_or_else(|error| {
            panic!("{} {slug} schema is not usable: {error}", version.id())
        });

        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(directory)
            .join(fixture);
        let raw = fs::read_to_string(&path).expect("fixture should exist");
        let payload: Value = serde_json::from_str(&raw).expect("fixture should be valid JSON");

        let errors: Vec<String> = validator
            .iter_errors(&payload)
            .map(|error| format!("{} at {}", error, error.instance_path))
            .collect();
        assert!(
            errors.is_empty(),
            "fixture {fixture} does not satisfy the {} {slug} schema: {errors:?}",
            version.id()
        );
    }
}

/// The complement of the test above: a schema that accepts everything would
/// pass that one too, so check that structural breakage is actually caught.
#[test]
fn schemas_reject_structurally_invalid_payloads() {
    let schema = load_schema(OpenRtbVersion::V2_6_202606, "bid-request")
        .expect("2.6-202606 bid request schema");
    let validator = jsonschema::validator_for(&schema).expect("schema should compile");

    let missing_required = serde_json::json!({ "imp": [{ "id": "1" }] });
    assert!(
        !validator.is_valid(&missing_required),
        "schema accepted a request with no id"
    );

    let wrong_type = serde_json::json!({ "id": "req-1", "imp": { "id": "1" } });
    assert!(
        !validator.is_valid(&wrong_type),
        "schema accepted imp as an object instead of an array"
    );

    let out_of_range_enum = serde_json::json!({
        "id": "req-1",
        "imp": [{ "id": "1", "banner": { "w": 300, "h": 250 } }],
        "device": { "devicetype": 99 }
    });
    assert!(
        !validator.is_valid(&out_of_range_enum),
        "schema accepted an undocumented devicetype"
    );
}

/// The 3.0 schemas are rooted at the envelope and pin which payload member
/// belongs inside, which is the whole difference between the two documents.
#[test]
fn layered_schemas_pin_their_envelope_member() {
    for (slug, member, excluded) in [
        ("bid-request", "request", "response"),
        ("bid-response", "response", "request"),
    ] {
        let schema = load_schema(OpenRtbVersion::V3_0, slug)
            .unwrap_or_else(|| panic!("3.0 {slug} schema should exist"));

        assert_eq!(schema["required"], serde_json::json!(["openrtb"]));
        let envelope = &schema["properties"]["openrtb"];
        assert!(
            envelope["properties"].get(member).is_some(),
            "3.0 {slug} schema does not carry openrtb.{member}"
        );
        assert!(
            envelope["properties"].get(excluded).is_none(),
            "3.0 {slug} schema still allows openrtb.{excluded}"
        );
        assert!(envelope["required"]
            .as_array()
            .expect("envelope required")
            .contains(&Value::String(String::from(member))));

        let validator = jsonschema::validator_for(&schema).expect("schema should compile");
        assert!(
            !validator.is_valid(&serde_json::json!({ "openrtb": { "domainver": "1.0" } })),
            "3.0 {slug} schema accepted an envelope with no {member}"
        );
        assert!(
            !validator.is_valid(&serde_json::json!({ "id": "req-1", "imp": [] })),
            "3.0 {slug} schema accepted an unwrapped 2.x payload"
        );
    }
}

fn collect_refs(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                if key == "$ref" {
                    if let Some(reference) = child.as_str() {
                        out.push(String::from(reference));
                    }
                    continue;
                }
                collect_refs(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_refs(item, out);
            }
        }
        _ => {}
    }
}
