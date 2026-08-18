//! bid.adm markup coherence: standalone mtype-driven checks and the
//! two-pass request/response cross-validation.

use rtblint_core::{
    validate_bid_response_against_request, validate_bid_response_for_version, OpenRtbVersion,
    Severity, ValidationResult,
};

const VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;

fn has_issue(result: &ValidationResult, id: &str, path: &str) -> bool {
    result
        .issues
        .iter()
        .any(|issue| issue.id == id && issue.path.as_deref() == Some(path))
}

fn issue_severity(result: &ValidationResult, id: &str) -> Option<Severity> {
    result
        .issues
        .iter()
        .find(|issue| issue.id == id)
        .map(|issue| issue.severity)
}

fn response_with_bid(bid_fields: &str) -> String {
    format!(
        r#"{{
            "id": "req-1",
            "seatbid": [
                {{ "bid": [ {{ "id": "bid-1", "impid": "imp-1", "price": 1.5{bid_fields} }} ] }}
            ]
        }}"#
    )
}

// -- standalone mtype/adm coherence --

#[test]
fn native_mtype_accepts_json_adm() {
    let response = response_with_bid(
        r#", "mtype": 4, "adm": "{\"native\":{\"ver\":\"1.2\",\"link\":{\"url\":\"https://example.com\"},\"assets\":[{\"id\":1,\"title\":{\"text\":\"Hello\"}}]}}""#,
    );
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(result.valid, "issues: {:?}", result.issues);
    assert!(result.issues.is_empty());
}

#[test]
fn native_mtype_rejects_html_adm() {
    let response = response_with_bid(r#", "mtype": 4, "adm": "<div>hello</div>""#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.bid.adm.native_not_json",
        "seatbid[0].bid[0].adm"
    ));
}

#[test]
fn double_encoded_adm_is_reported() {
    // adm parses to a JSON *string*: the native payload was encoded twice.
    let response = response_with_bid(
        r#", "mtype": 4, "adm": "\"{\\\"native\\\":{\\\"ver\\\":\\\"1.2\\\"}}\"""#,
    );
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.bid.adm.double_encoded",
        "seatbid[0].bid[0].adm"
    ));
}

#[test]
fn video_mtype_accepts_vast_adm() {
    let response = response_with_bid(
        r#", "mtype": 2, "adm": "<?xml version=\"1.0\"?><VAST version=\"4.2\"></VAST>""#,
    );
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(result.valid, "issues: {:?}", result.issues);
    assert!(result.issues.is_empty());
}

#[test]
fn video_mtype_rejects_json_adm() {
    let response = response_with_bid(r#", "mtype": 2, "adm": "{\"native\":{}}""#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.bid.adm.markup_type_mismatch",
        "seatbid[0].bid[0].adm"
    ));
}

#[test]
fn video_mtype_warns_on_markup_without_vast_root() {
    let response = response_with_bid(r#", "mtype": 2, "adm": "<div>not vast</div>""#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(result.valid, "warning must not fail validation");
    assert!(has_issue(
        &result,
        "openrtb.bid.adm.vast_root_missing",
        "seatbid[0].bid[0].adm"
    ));
}

#[test]
fn audio_mtype_accepts_daast_adm() {
    let response = response_with_bid(r#", "mtype": 3, "adm": "<DAAST version=\"1.0\"></DAAST>""#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(result.valid, "issues: {:?}", result.issues);
    assert!(result.issues.is_empty());
}

#[test]
fn video_mtype_warns_on_non_markup_adm() {
    let response = response_with_bid(r#", "mtype": 2, "adm": "https://vast.example.com/tag""#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(result.valid);
    assert!(has_issue(
        &result,
        "openrtb.bid.adm.not_markup",
        "seatbid[0].bid[0].adm"
    ));
}

#[test]
fn banner_mtype_rejects_native_json_adm() {
    let response = response_with_bid(r#", "mtype": 1, "adm": "{\"native\":{\"ver\":\"1.2\"}}""#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.bid.adm.markup_type_mismatch",
        "seatbid[0].bid[0].adm"
    ));
}

#[test]
fn adm_without_mtype_warns_on_2_6() {
    let response = response_with_bid(r#", "adm": "<div>banner</div>""#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(result.valid);
    assert!(has_issue(
        &result,
        "openrtb.bid.mtype_missing",
        "seatbid[0].bid[0].mtype"
    ));
}

#[test]
fn adm_without_mtype_does_not_warn_on_2_5() {
    // 2.5's Bid object predates mtype; recommending it would be wrong.
    let response = response_with_bid(r#", "adm": "<div>banner</div>""#);
    let result = validate_bid_response_for_version(OpenRtbVersion::V2_5, &response);
    assert!(!has_issue(
        &result,
        "openrtb.bid.mtype_missing",
        "seatbid[0].bid[0].mtype"
    ));
}

// -- two-pass cross-validation --

const BANNER_NATIVE_REQUEST: &str = r#"{
    "id": "req-1",
    "cur": ["USD", "EUR"],
    "imp": [
        { "id": "imp-1", "banner": { "w": 300, "h": 250 } },
        { "id": "imp-2", "native": { "request": "{\"ver\":\"1.2\",\"assets\":[{\"id\":1,\"required\":1,\"title\":{\"len\":90}}]}" } },
        {
            "id": "imp-3",
            "video": { "mimes": ["video/mp4"] },
            "pmp": { "deals": [ { "id": "deal-1" } ] }
        }
    ]
}"#;

#[test]
fn cross_validation_accepts_coherent_pair() {
    let response = r#"{
        "id": "req-1",
        "cur": "USD",
        "seatbid": [
            { "bid": [ { "id": "b1", "impid": "imp-1", "price": 1.0, "mtype": 1, "adm": "<div>ad</div>" } ] }
        ]
    }"#;
    let result = validate_bid_response_against_request(VERSION, BANNER_NATIVE_REQUEST, response);
    assert!(result.valid, "issues: {:?}", result.issues);
}

#[test]
fn cross_validation_reports_unknown_impid() {
    let response = r#"{
        "id": "req-1",
        "seatbid": [ { "bid": [ { "id": "b1", "impid": "imp-99", "price": 1.0, "mtype": 1 } ] } ]
    }"#;
    let result = validate_bid_response_against_request(VERSION, BANNER_NATIVE_REQUEST, response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.bid.impid_unknown",
        "seatbid[0].bid[0].impid"
    ));
}

#[test]
fn cross_validation_reports_mtype_not_offered() {
    // imp-1 offers banner only; a video bid against it is incoherent.
    let response = r#"{
        "id": "req-1",
        "seatbid": [ { "bid": [ { "id": "b1", "impid": "imp-1", "price": 1.0, "mtype": 2,
            "adm": "<VAST version=\"4.2\"></VAST>" } ] } ]
    }"#;
    let result = validate_bid_response_against_request(VERSION, BANNER_NATIVE_REQUEST, response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.bid.mtype_not_offered",
        "seatbid[0].bid[0].mtype"
    ));
    assert!(has_issue(
        &result,
        "openrtb.bid.adm.media_type_mismatch",
        "seatbid[0].bid[0].adm"
    ));
}

#[test]
fn cross_validation_reports_native_adm_against_banner_imp() {
    let response = r#"{
        "id": "req-1",
        "seatbid": [ { "bid": [ { "id": "b1", "impid": "imp-1", "price": 1.0, "mtype": 4,
            "adm": "{\"native\":{\"ver\":\"1.2\"}}" } ] } ]
    }"#;
    let result = validate_bid_response_against_request(VERSION, BANNER_NATIVE_REQUEST, response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.bid.adm.media_type_mismatch",
        "seatbid[0].bid[0].adm"
    ));
}

#[test]
fn cross_validation_reports_non_json_adm_against_native_only_imp() {
    let response = r#"{
        "id": "req-1",
        "seatbid": [ { "bid": [ { "id": "b1", "impid": "imp-2", "price": 1.0, "mtype": 4,
            "adm": "<div>not native</div>" } ] } ]
    }"#;
    let result = validate_bid_response_against_request(VERSION, BANNER_NATIVE_REQUEST, response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.bid.adm.media_type_mismatch",
        "seatbid[0].bid[0].adm"
    ));
}

#[test]
fn cross_validation_reports_request_id_mismatch() {
    let response = r#"{ "id": "other-request", "nbr": 2 }"#;
    let result = validate_bid_response_against_request(VERSION, BANNER_NATIVE_REQUEST, response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.response.request_id_mismatch",
        "id"
    ));
}

#[test]
fn cross_validation_reports_disallowed_currency() {
    let response = r#"{
        "id": "req-1",
        "cur": "JPY",
        "seatbid": [ { "bid": [ { "id": "b1", "impid": "imp-1", "price": 1.0, "mtype": 1 } ] } ]
    }"#;
    let result = validate_bid_response_against_request(VERSION, BANNER_NATIVE_REQUEST, response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.response.cur_not_allowed",
        "cur"
    ));
}

#[test]
fn cross_validation_enforces_seat_constraints() {
    let request = r#"{
        "id": "req-1",
        "wseat": ["seat-a"],
        "imp": [ { "id": "imp-1", "banner": { "w": 300, "h": 250 } } ]
    }"#;
    let response = r#"{
        "id": "req-1",
        "seatbid": [ { "seat": "seat-b",
            "bid": [ { "id": "b1", "impid": "imp-1", "price": 1.0, "mtype": 1 } ] } ]
    }"#;
    let result = validate_bid_response_against_request(VERSION, request, response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.seatbid.seat_not_allowed",
        "seatbid[0].seat"
    ));
}

#[test]
fn cross_validation_warns_on_unknown_dealid() {
    let response = r#"{
        "id": "req-1",
        "seatbid": [ { "bid": [ { "id": "b1", "impid": "imp-3", "price": 1.0, "mtype": 2,
            "dealid": "deal-999", "adm": "<VAST version=\"4.2\"></VAST>" } ] } ]
    }"#;
    let result = validate_bid_response_against_request(VERSION, BANNER_NATIVE_REQUEST, response);
    assert_eq!(
        issue_severity(&result, "openrtb.bid.dealid_unknown"),
        Some(Severity::Warning)
    );
    assert!(has_issue(
        &result,
        "openrtb.bid.dealid_unknown",
        "seatbid[0].bid[0].dealid"
    ));
}

#[test]
fn cross_validation_accepts_known_dealid() {
    let response = r#"{
        "id": "req-1",
        "seatbid": [ { "bid": [ { "id": "b1", "impid": "imp-3", "price": 1.0, "mtype": 2,
            "dealid": "deal-1", "adm": "<VAST version=\"4.2\"></VAST>" } ] } ]
    }"#;
    let result = validate_bid_response_against_request(VERSION, BANNER_NATIVE_REQUEST, response);
    assert!(result.valid, "issues: {:?}", result.issues);
    assert!(!has_issue(
        &result,
        "openrtb.bid.dealid_unknown",
        "seatbid[0].bid[0].dealid"
    ));
}

#[test]
fn cross_validation_reports_unusable_request() {
    let response = r#"{ "id": "req-1", "nbr": 2 }"#;
    let result = validate_bid_response_against_request(VERSION, "not json", response);
    assert!(!result.valid);
    assert!(result
        .issues
        .iter()
        .any(|issue| issue.id == "openrtb.pair.request_unusable"));
}
