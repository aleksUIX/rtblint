//! The spec-JSON / protobuf-JSON split.
//!
//! The IAB OpenRTB protobuf schema declares 28 fields `bool` that the
//! specification types as integer flags. Each encoding is correct on its own
//! transport and wrong on the other, so the validator reports against the
//! dialect the caller declares.

use rtblint_core::{
    proto_bool_fields, validate_bid_request_for_version, validate_bid_request_with_dialect,
    Dialect, OpenRtbVersion, Severity, ValidationResult,
};

const VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;

fn has_issue(result: &ValidationResult, id: &str, path: &str) -> bool {
    result
        .issues
        .iter()
        .any(|issue| issue.id == id && issue.path.as_deref() == Some(path))
}

fn issue_message(result: &ValidationResult, id: &str) -> String {
    result
        .issues
        .iter()
        .find(|issue| issue.id == id)
        .map(|issue| issue.message.clone())
        .unwrap_or_else(|| panic!("no {id} issue: {:?}", result.issues))
}

/// A request whose flag fields are written the protobuf way.
fn proto_flavoured_request() -> &'static str {
    r#"{
        "id": "req-1",
        "imp": [
            {
                "id": "imp-1",
                "secure": true,
                "instl": false,
                "banner": { "w": 300, "h": 250, "topframe": true },
                "pmp": { "private_auction": true }
            }
        ],
        "site": { "id": "site-1", "domain": "news.example", "mobile": false },
        "regs": { "coppa": false, "gdpr": true }
    }"#
}

/// The same request with spec-typed integer flags.
fn spec_flavoured_request() -> &'static str {
    r#"{
        "id": "req-1",
        "imp": [
            {
                "id": "imp-1",
                "secure": 1,
                "instl": 0,
                "banner": { "w": 300, "h": 250, "topframe": 1 },
                "pmp": { "private_auction": 1 }
            }
        ],
        "site": { "id": "site-1", "domain": "news.example", "mobile": 0 },
        "regs": { "coppa": 0, "gdpr": 1 }
    }"#
}

#[test]
fn spec_dialect_accepts_integer_flags() {
    let result = validate_bid_request_for_version(VERSION, spec_flavoured_request());
    assert!(result.valid, "issues: {:?}", result.issues);
    assert!(result.issues.is_empty());
}

#[test]
fn proto_dialect_accepts_boolean_flags() {
    let result =
        validate_bid_request_with_dialect(VERSION, Dialect::ProtoJson, proto_flavoured_request());
    assert!(result.valid, "issues: {:?}", result.issues);
    assert!(result.issues.is_empty());
}

#[test]
fn spec_dialect_reports_boolean_flags_as_a_dialect_finding() {
    let result = validate_bid_request_for_version(VERSION, proto_flavoured_request());

    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.dialect.bool_for_integer",
        "imp[0].secure"
    ));
    assert!(has_issue(
        &result,
        "openrtb.dialect.bool_for_integer",
        "regs.coppa"
    ));
    // A plain type mismatch would leave the reader guessing; naming the
    // protobuf schema and the integer to send instead is the fix.
    let message = issue_message(&result, "openrtb.dialect.bool_for_integer");
    assert!(
        message.contains("protobuf") && message.contains("proto-json"),
        "the message should point at the dialect: {message}"
    );
    // The generic shape finding must not also fire for the same field.
    assert!(!has_issue(
        &result,
        "openrtb.type.mismatch",
        "imp[0].secure"
    ));
}

#[test]
fn proto_dialect_rejects_integer_flags_protojson_cannot_parse() {
    let result =
        validate_bid_request_with_dialect(VERSION, Dialect::ProtoJson, spec_flavoured_request());

    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.dialect.integer_for_bool",
        "imp[0].secure"
    ));
    assert!(
        has_issue(
            &result,
            "openrtb.dialect.integer_for_bool",
            "pmp.private_auction"
        ) || has_issue(
            &result,
            "openrtb.dialect.integer_for_bool",
            "imp[0].pmp.private_auction"
        )
    );
}

/// Fields the spec types as integers but does not share with the protobuf
/// schema keep the ordinary type finding in both dialects.
#[test]
fn non_proto_bool_integer_fields_keep_the_plain_type_finding() {
    let payload = r#"{
        "id": "req-1",
        "at": true,
        "imp": [{ "id": "imp-1", "banner": { "w": 300, "h": 250 } }],
        "site": { "id": "site-1" }
    }"#;

    for dialect in [Dialect::SpecJson, Dialect::ProtoJson] {
        let result = validate_bid_request_with_dialect(VERSION, dialect, payload);
        assert!(
            has_issue(&result, "openrtb.type.mismatch", "at"),
            "{dialect} should report a plain type mismatch: {:?}",
            result.issues
        );
    }
}

#[test]
fn proto_dialect_resolves_camel_cased_field_names() {
    let payload = r#"{
        "id": "req-1",
        "imp": [
            {
                "id": "imp-1",
                "banner": { "w": 300, "h": 250 },
                "pmp": { "privateAuction": true }
            }
        ],
        "site": { "id": "site-1" }
    }"#;

    let proto = validate_bid_request_with_dialect(VERSION, Dialect::ProtoJson, payload);
    assert!(proto.valid, "issues: {:?}", proto.issues);
    assert!(has_issue(
        &proto,
        "openrtb.dialect.camel_case_name",
        "imp[0].pmp.privateAuction"
    ));
    assert_eq!(
        proto
            .issues
            .iter()
            .find(|issue| issue.id == "openrtb.dialect.camel_case_name")
            .map(|issue| issue.severity),
        Some(Severity::Warning)
    );

    // Spec JSON has no such spelling, so it stays an undefined field.
    let spec = validate_bid_request_for_version(VERSION, payload);
    assert!(!spec.valid);
    assert!(has_issue(
        &spec,
        "openrtb.field.undefined",
        "imp[0].pmp.privateAuction"
    ));
}

/// The divergence set is data, and it is only useful if it stays honest about
/// which fields it covers.
#[test]
fn proto_bool_divergence_set_is_published_and_stable() {
    let fields = proto_bool_fields();
    assert_eq!(fields.len(), 28);
    assert!(fields.contains(&("Regs", "coppa")));
    assert!(fields.contains(&("Content", "livestream")));
    assert!(fields.contains(&("SupplyChainNode", "hp")));
}

/// Content.livestream is typed "int" in the 2.6 spec tables rather than
/// "integer". That short spelling used to leave the field shapeless, which
/// silently disabled type checking on it.
#[test]
fn short_int_type_spellings_are_still_type_checked() {
    let payload = r#"{
        "id": "req-1",
        "imp": [{ "id": "imp-1", "video": { "mimes": ["video/mp4"] } }],
        "app": {
            "bundle": "com.example.tv",
            "content": { "id": "c1", "livestream": "yes" }
        }
    }"#;

    let result = validate_bid_request_for_version(VERSION, payload);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.type.mismatch",
        "app.content.livestream"
    ));
}
