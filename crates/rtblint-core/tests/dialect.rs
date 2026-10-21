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

fn native_request(field: &str) -> serde_json::Value {
    serde_json::json!({
        "id": "r", "imp": [{"id": "i", "secure": true, "native": {
            (field): {"ver": "1.2", "privacy": true, "assets": [
                {"id": 1, "required": true, "title": {"len": 40}}
            ]}
        }}]
    })
}

#[test]
fn proto_native_objects_accept_both_names_and_retain_asset_checks() {
    for field in ["request_native", "requestNative"] {
        let mut payload = native_request(field);
        let result =
            validate_bid_request_with_dialect(VERSION, Dialect::ProtoJson, &payload.to_string());
        assert!(result.valid, "{field}: {:?}", result.issues);
        assert!(result.issues.is_empty(), "{field}: {:?}", result.issues);
        let spec = validate_bid_request_for_version(VERSION, &payload.to_string());
        assert!(has_issue(
            &spec,
            "openrtb.field.required",
            "imp[0].native.request"
        ));
        assert!(has_issue(
            &spec,
            "openrtb.field.undefined",
            &format!("imp[0].native.{field}")
        ));

        payload["imp"][0]["native"][field]["assets"][0]["title"] = serde_json::json!({});
        let result =
            validate_bid_request_with_dialect(VERSION, Dialect::ProtoJson, &payload.to_string());
        assert!(has_issue(
            &result,
            "openrtb.native.field_required",
            &format!("imp[0].native.{field}.assets[0].title.len")
        ));
    }
}

#[test]
fn proto_native_rejects_bad_shapes_integer_flags_and_oneof_conflicts() {
    let mut payload = native_request("request_native");
    payload["imp"][0]["native"]["request_native"] = serde_json::json!("{}");
    let result =
        validate_bid_request_with_dialect(VERSION, Dialect::ProtoJson, &payload.to_string());
    assert!(has_issue(
        &result,
        "openrtb.type.mismatch",
        "imp[0].native.request_native"
    ));
    payload = native_request("request_native");
    payload["imp"][0]["native"]["request_native"]["assets"][0]["required"] = serde_json::json!(1);
    let result =
        validate_bid_request_with_dialect(VERSION, Dialect::ProtoJson, &payload.to_string());
    assert!(has_issue(
        &result,
        "openrtb.dialect.integer_for_bool",
        "imp[0].native.request_native.assets[0].required"
    ));
    payload["imp"][0]["native"]["request"] = serde_json::json!("{}");
    let result =
        validate_bid_request_with_dialect(VERSION, Dialect::ProtoJson, &payload.to_string());
    assert!(has_issue(
        &result,
        "openrtb.dialect.oneof_conflict",
        "imp[0].native.request_native"
    ));
}

#[test]
fn proto_native_responses_preserve_required_asset_pairing() {
    use rtblint_core::{
        validate_bid_response_against_request_with_dialect, validate_bid_response_with_dialect,
    };
    for field in ["adm_native", "admNative"] {
        let request = native_request("requestNative").to_string();
        let mut response = serde_json::json!({"id": "r", "seatbid": [{"bid": [{
            "id": "b", "impid": "i", "price": 1.0, "mtype": 4,
            (field): {"ver": "1.2", "link": {"url": "https://advertiser.example"}, "assets": [
                {"id": 1, "required": true, "title": {"text": "Example"}}
            ]}
        }]}]});
        let result = validate_bid_response_against_request_with_dialect(
            VERSION,
            Dialect::ProtoJson,
            &request,
            &response.to_string(),
        );
        assert!(result.valid, "{field}: {:?}", result.issues);
        response["seatbid"][0]["bid"][0][field]["assets"][0]["id"] = serde_json::json!(2);
        let result = validate_bid_response_against_request_with_dialect(
            VERSION,
            Dialect::ProtoJson,
            &request,
            &response.to_string(),
        );
        assert!(has_issue(
            &result,
            "openrtb.native.asset.required_missing",
            &format!("seatbid[0].bid[0].{field}")
        ));
        response["seatbid"][0]["bid"][0][field]
            .as_object_mut()
            .unwrap()
            .remove("link");
        let result =
            validate_bid_response_with_dialect(VERSION, Dialect::ProtoJson, &response.to_string());
        assert!(has_issue(
            &result,
            "openrtb.native.field_required",
            &format!("seatbid[0].bid[0].{field}.link.url")
        ));
    }
}

#[test]
fn site_domain_warns_on_url_syntax_without_rejecting_bare_domains() {
    for domain in [
        "https://publisher.example/",
        "publisher.example/path",
        "publisher.example:443",
        "publisher.example?x=1",
        " publisher.example",
    ] {
        let payload = serde_json::json!({"id":"r", "imp":[{"id":"i", "banner":{}}], "site":{"domain": domain}});
        let result = validate_bid_request_for_version(VERSION, &payload.to_string());
        assert!(
            has_issue(&result, "openrtb.site.domain_not_domain", "site.domain"),
            "{domain}"
        );
    }
    for domain in [
        "publisher.example",
        "news.publisher.example",
        "xn--bcher-kva.example",
        "PUBLISHER.example",
    ] {
        let payload = serde_json::json!({"id":"r", "imp":[{"id":"i", "banner":{}}], "site":{"domain": domain}});
        let result = validate_bid_request_for_version(VERSION, &payload.to_string());
        assert!(result.issues.is_empty(), "{domain}: {:?}", result.issues);
    }
}

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
