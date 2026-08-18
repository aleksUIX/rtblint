//! Exchange profiles on top of the spec.
//!
//! A profile is not a JSON dialect. Dialect is how flags are serialised;
//! a profile is the extra protocol an exchange documents. Google Authorized
//! Buyers JSON still uses integer flags, and still rejects `at: 3` unless
//! this profile is declared.

use rtblint_core::{
    validate_bid_request_for_version, validate_bid_request_with_profile,
    validate_bid_response_with_profile, Dialect, OpenRtbVersion, Profile, ValidationResult,
};

const VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;

fn has_issue(result: &ValidationResult, id: &str, path: &str) -> bool {
    result
        .issues
        .iter()
        .any(|issue| issue.id == id && issue.path.as_deref() == Some(path))
}

fn google_request(at: i64, billing_id: bool) -> String {
    let billing = if billing_id {
        r#", "ext": { "billing_id": ["123"] }"#
    } else {
        ""
    };
    format!(
        r#"{{
            "id": "req-1",
            "at": {at},
            "imp": [{{ "id": "1", "banner": {{ "w": 300, "h": 250 }}{billing} }}]
        }}"#
    )
}

#[test]
fn spec_profile_rejects_google_fixed_price_auction_type() {
    let result = validate_bid_request_for_version(VERSION, &google_request(3, true));
    assert!(!result.valid);
    assert!(has_issue(&result, "openrtb.value.invalid", "at"));
}

#[test]
fn google_ab_accepts_fixed_price_auction_type() {
    let result = validate_bid_request_with_profile(
        VERSION,
        Dialect::SpecJson,
        Profile::GoogleAuthorizedBuyers,
        &google_request(3, true),
    );
    assert!(
        result.valid,
        "google-ab should accept at=3: {:?}",
        result.issues
    );
}

#[test]
fn google_ab_still_rejects_undocumented_auction_types() {
    let result = validate_bid_request_with_profile(
        VERSION,
        Dialect::SpecJson,
        Profile::GoogleAuthorizedBuyers,
        &google_request(4, true),
    );
    assert!(!result.valid);
    assert!(has_issue(&result, "openrtb.value.invalid", "at"));
}

#[test]
fn google_ab_requires_imp_billing_id() {
    let result = validate_bid_request_with_profile(
        VERSION,
        Dialect::SpecJson,
        Profile::GoogleAuthorizedBuyers,
        &google_request(1, false),
    );
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.profile.field_required",
        "imp[0].ext.billing_id"
    ));
    let message = result
        .issues
        .iter()
        .find(|issue| issue.id == "openrtb.profile.field_required")
        .map(|issue| issue.message.as_str())
        .unwrap_or("");
    assert!(
        message.contains("Google Authorized Buyers"),
        "the finding should name the profile: {message}"
    );
}

#[test]
fn spec_profile_does_not_require_google_billing_id() {
    let result = validate_bid_request_for_version(VERSION, &google_request(1, false));
    assert!(
        result.valid,
        "spec profile should not require billing_id: {:?}",
        result.issues
    );
}

#[test]
fn empty_billing_id_array_is_not_populated() {
    let payload = r#"{
        "id": "req-1",
        "imp": [{ "id": "1", "banner": { "w": 300, "h": 250 }, "ext": { "billing_id": [] } }]
    }"#;
    let result = validate_bid_request_with_profile(
        VERSION,
        Dialect::SpecJson,
        Profile::GoogleAuthorizedBuyers,
        payload,
    );
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.profile.field_required",
        "imp[0].ext.billing_id"
    ));
}

fn pbs_imp(ext: &str) -> String {
    let ext_field = if ext.is_empty() {
        String::new()
    } else {
        format!(", {ext}")
    };
    format!(
        r#"{{
            "id": "req-1",
            "imp": [{{ "id": "1", "banner": {{ "w": 300, "h": 250 }}{ext_field} }}]
        }}"#
    )
}

fn pbs_validate(payload: &str) -> ValidationResult {
    validate_bid_request_with_profile(VERSION, Dialect::SpecJson, Profile::PrebidServer, payload)
}

#[test]
fn spec_profile_does_not_require_a_prebid_bidder() {
    let result = validate_bid_request_for_version(VERSION, &pbs_imp(""));
    assert!(
        result.valid,
        "spec profile should not require a PBS bidder: {:?}",
        result.issues
    );
}

#[test]
fn prebid_server_requires_a_bidder_on_each_imp() {
    let result = pbs_validate(&pbs_imp(""));
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.profile.prebid.bidder_required",
        "imp[0].ext"
    ));
    let message = result
        .issues
        .iter()
        .find(|issue| issue.id == "openrtb.profile.prebid.bidder_required")
        .map(|issue| issue.message.as_str())
        .unwrap_or("");
    assert!(
        message.contains("Prebid Server"),
        "the finding should name the profile: {message}"
    );
}

#[test]
fn prebid_server_accepts_imp_ext_prebid_bidder() {
    let result = pbs_validate(&pbs_imp(
        r#""ext": { "prebid": { "bidder": { "appnexus": { "placementId": 1 } } } }"#,
    ));
    assert!(
        result.valid,
        "prebid.bidder.appnexus should satisfy targeting: {:?}",
        result.issues
    );
}

#[test]
fn prebid_server_accepts_legacy_imp_ext_bidder() {
    let result = pbs_validate(&pbs_imp(r#""ext": { "appnexus": { "placementId": 1 } }"#));
    assert!(
        result.valid,
        "legacy imp.ext.appnexus should count as a bidder: {:?}",
        result.issues
    );
}

#[test]
fn prebid_server_does_not_treat_skadn_as_a_bidder() {
    let result = pbs_validate(&pbs_imp(
        r#""ext": { "skadn": { "versions": ["2.0"], "sourceapp": "123", "skadnetids": ["abc"] } }"#,
    ));
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.profile.prebid.bidder_required",
        "imp[0].ext"
    ));
}

#[test]
fn prebid_server_rejects_an_empty_bidder_object() {
    let result = pbs_validate(&pbs_imp(r#""ext": { "prebid": { "bidder": {} } }"#));
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.profile.prebid.bidder_required",
        "imp[0].ext"
    ));
}

#[test]
fn prebid_server_requires_storedrequest_id() {
    let result = pbs_validate(&pbs_imp(r#""ext": { "prebid": { "storedrequest": {} } }"#));
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.profile.field_required",
        "imp[0].ext.prebid.storedrequest.id"
    ));
    assert!(!result
        .issues
        .iter()
        .any(|issue| { issue.id == "openrtb.profile.prebid.bidder_required" }));
}

#[test]
fn prebid_server_accepts_imp_storedrequest_id_instead_of_a_bidder() {
    let result = pbs_validate(&pbs_imp(
        r#""ext": { "prebid": { "storedrequest": { "id": "sr-imp-1" } } }"#,
    ));
    assert!(
        result.valid,
        "storedrequest.id should cover bidder targeting: {:?}",
        result.issues
    );
}

#[test]
fn prebid_server_top_level_storedrequest_covers_imps() {
    let payload = r#"{
        "id": "req-1",
        "imp": [{ "id": "1", "banner": { "w": 300, "h": 250 } }],
        "ext": { "prebid": { "storedrequest": { "id": "sr-1" } } }
    }"#;
    let result = pbs_validate(payload);
    assert!(
        result.valid,
        "top-level storedrequest should cover imps: {:?}",
        result.issues
    );
}

#[test]
fn prebid_server_refuses_wseat() {
    let payload = r#"{
        "id": "req-1",
        "wseat": ["appnexus"],
        "imp": [{
            "id": "1",
            "banner": { "w": 300, "h": 250 },
            "ext": { "prebid": { "bidder": { "appnexus": {} } } }
        }]
    }"#;
    let result = pbs_validate(payload);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.profile.field_forbidden",
        "wseat"
    ));
}

#[test]
fn prebid_server_rejects_unknown_trace() {
    let payload = r#"{
        "id": "req-1",
        "imp": [{
            "id": "1",
            "banner": { "w": 300, "h": 250 },
            "ext": { "prebid": { "bidder": { "appnexus": {} } } }
        }],
        "ext": { "prebid": { "trace": "debug" } }
    }"#;
    let result = pbs_validate(payload);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.profile.value_invalid",
        "ext.prebid.trace"
    ));
}

#[test]
fn prebid_server_allows_native_request_assets_without_ids() {
    let native =
        serde_json::to_string(r#"{"ver":"1.2","assets":[{"required":1,"title":{"len":90}}]}"#)
            .expect("encode native.request");
    let payload = format!(
        r#"{{
            "id": "req-1",
            "imp": [{{
                "id": "1",
                "native": {{ "ver": "1.2", "request": {native} }},
                "ext": {{ "prebid": {{ "bidder": {{ "appnexus": {{}} }} }} }}
            }}]
        }}"#
    );
    let spec = validate_bid_request_for_version(VERSION, &payload);
    assert!(has_issue(
        &spec,
        "openrtb.native.asset.id_required",
        "imp[0].native.request.assets[0].id"
    ));
    let pbs = pbs_validate(&payload);
    assert!(pbs.valid, "PBS fills native asset ids: {:?}", pbs.issues);
}

#[test]
fn prebid_server_rejects_invalid_bid_type() {
    let payload = r#"{
        "id": "req-1",
        "seatbid": [{
            "bid": [{
                "id": "b1",
                "impid": "1",
                "price": 1.0,
                "ext": { "prebid": { "type": "dooh" } }
            }]
        }]
    }"#;
    let result = validate_bid_response_with_profile(
        VERSION,
        Dialect::SpecJson,
        Profile::PrebidServer,
        payload,
    );
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.profile.value_invalid",
        "seatbid[0].bid[0].ext.prebid.type"
    ));
}
