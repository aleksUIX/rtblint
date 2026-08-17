//! Exchange profiles on top of the spec.
//!
//! A profile is not a JSON dialect. Dialect is how flags are serialised;
//! a profile is the extra protocol an exchange documents. Google Authorized
//! Buyers JSON still uses integer flags, and still rejects `at: 3` unless
//! this profile is declared.

use rtblint_core::{
    validate_bid_request_for_version, validate_bid_request_with_profile, Dialect, OpenRtbVersion,
    Profile, ValidationResult,
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
