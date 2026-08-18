//! Nested payloads OpenRTB carries as strings or opaque ext objects:
//! Native Ads 1.2, GPP/TCF, substitution macros, EID/SUA, SKAdNetwork.

use rtblint_core::{
    validate_bid_request_for_version, validate_bid_response_against_request,
    validate_bid_response_for_version, OpenRtbVersion, ValidationResult,
};

const VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;

fn has_issue(result: &ValidationResult, id: &str, path: &str) -> bool {
    result
        .issues
        .iter()
        .any(|issue| issue.id == id && issue.path.as_deref() == Some(path))
}

fn request(body: &str) -> String {
    format!(
        r#"{{"id":"req-1","at":1,"tmax":200,"site":{{"id":"s","domain":"publisher.example"}},{body}}}"#
    )
}

fn native_request(inner: &str) -> String {
    let encoded = serde_json::to_string(inner).expect("encode native.request");
    request(&format!(
        r#""imp":[{{"id":"1","native":{{"ver":"1.2","request":{encoded}}}}}]"#
    ))
}

fn bid_response(bid_fields: &str) -> String {
    format!(
        r#"{{"id":"req-1","seatbid":[{{"bid":[{{"id":"b1","impid":"1","price":1.0{bid_fields}}}]}}]}}"#
    )
}

// -- Native request --

#[test]
fn native_request_requires_assets() {
    let result = validate_bid_request_for_version(VERSION, &native_request(r#"{"ver":"1.2"}"#));
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.native.assets_missing",
        "imp[0].native.request.assets"
    ));
}

#[test]
fn native_request_requires_title_len() {
    let result = validate_bid_request_for_version(
        VERSION,
        &native_request(r#"{"ver":"1.2","assets":[{"id":1,"title":{}}]}"#),
    );
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.native.field_required",
        "imp[0].native.request.assets[0].title.len"
    ));
}

#[test]
fn native_request_duplicate_asset_id() {
    let result = validate_bid_request_for_version(
        VERSION,
        &native_request(
            r#"{"ver":"1.2","assets":[{"id":1,"title":{"len":90}},{"id":1,"data":{"type":2}}]}"#,
        ),
    );
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.native.asset.id_duplicate",
        "imp[0].native.request.assets[1].id"
    ));
}

#[test]
fn native_layout_removed_on_1_2() {
    let result = validate_bid_request_for_version(
        VERSION,
        &native_request(r#"{"ver":"1.2","layout":1,"assets":[{"id":1,"title":{"len":90}}]}"#),
    );
    assert!(result.valid);
    assert!(has_issue(
        &result,
        "openrtb.native.layout_removed",
        "imp[0].native.request.layout"
    ));
}

#[test]
fn native_1_0_layout_is_allowed() {
    let inner = r#"{"ver":"1.0","layout":1,"assets":[{"id":1,"required":1,"title":{"len":90}}]}"#;
    let encoded = serde_json::to_string(inner).unwrap();
    let payload = request(&format!(
        r#""imp":[{{"id":"1","native":{{"ver":"1.0","request":{encoded}}}}}]"#
    ));
    let result = validate_bid_request_for_version(OpenRtbVersion::V2_3, &payload);
    assert!(
        !result
            .issues
            .iter()
            .any(|issue| issue.id == "openrtb.native.layout_removed"),
        "{:?}",
        result.issues
    );
}

// -- Native response + pair --

#[test]
fn native_response_requires_link_url() {
    let response = bid_response(
        r#", "mtype": 4, "adm": "{\"ver\":\"1.2\",\"assets\":[{\"id\":1,\"title\":{\"text\":\"Hi\"}}]}""#,
    );
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.native.field_required",
        "seatbid[0].bid[0].adm.link.url"
    ));
}

#[test]
fn native_pair_reports_missing_required_asset() {
    let req = native_request(
        r#"{"ver":"1.2","assets":[{"id":1,"required":1,"title":{"len":90}},{"id":2,"img":{"type":3,"wmin":100}}]}"#,
    );
    let response = bid_response(
        r#", "mtype": 4, "adm": "{\"ver\":\"1.2\",\"link\":{\"url\":\"https://ex\"},\"assets\":[{\"id\":2,\"img\":{\"url\":\"https://cdn/x.png\"}}]}""#,
    );
    let result = validate_bid_response_against_request(VERSION, &req, &response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.native.asset.required_missing",
        "seatbid[0].bid[0].adm"
    ));
}

#[test]
fn native_pair_reports_type_mismatch() {
    let req =
        native_request(r#"{"ver":"1.2","assets":[{"id":1,"required":1,"title":{"len":90}}]}"#);
    let response = bid_response(
        r#", "mtype": 4, "adm": "{\"ver\":\"1.2\",\"link\":{\"url\":\"https://ex\"},\"assets\":[{\"id\":1,\"img\":{\"url\":\"https://cdn/x.png\"}}]}""#,
    );
    let result = validate_bid_response_against_request(VERSION, &req, &response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.native.asset.type_mismatch",
        "seatbid[0].bid[0].adm.assets[0]"
    ));
}

// -- GPP / TCF --

#[test]
fn gpp_malformed_without_separator() {
    let payload = request(
        r#""regs":{"gpp":"not-a-gpp-string","gpp_sid":[2]},"imp":[{"id":"1","banner":{"w":300,"h":250}}]"#,
    );
    let result = validate_bid_request_for_version(VERSION, &payload);
    assert!(has_issue(&result, "openrtb.regs.gpp_malformed", "regs.gpp"));
}

#[test]
fn gpp_sid_mismatch_against_header() {
    let payload = request(
        r#""regs":{"gpp":"DBABM~CPXxRfAPXxRfAAfKABENAPCgAAAAAAAAAAAYgAAAAAAAA","gpp_sid":[7]},"imp":[{"id":"1","banner":{"w":300,"h":250}}]"#,
    );
    let result = validate_bid_request_for_version(VERSION, &payload);
    assert!(has_issue(
        &result,
        "openrtb.regs.gpp_section_mismatch",
        "regs.gpp_sid"
    ));
}

#[test]
fn tcf_malformed_user_consent() {
    let payload =
        request(r#""user":{"consent":"yes"},"imp":[{"id":"1","banner":{"w":300,"h":250}}]"#);
    let result = validate_bid_request_for_version(VERSION, &payload);
    assert!(has_issue(
        &result,
        "openrtb.regs.tcf_malformed",
        "user.consent"
    ));
}

#[test]
fn tcf_accepts_core_string() {
    let payload = request(
        r#""user":{"consent":"COwK9wAOwK9wAABABBENAPCgAAAAAAAAAAAYgAAAAAAAA"},"imp":[{"id":"1","banner":{"w":300,"h":250}}]"#,
    );
    let result = validate_bid_request_for_version(VERSION, &payload);
    assert!(
        !result
            .issues
            .iter()
            .any(|issue| issue.id == "openrtb.regs.tcf_malformed"),
        "{:?}",
        result.issues
    );
}

// -- Macros --

#[test]
fn unknown_macro_on_nurl() {
    let response = bid_response(r#", "nurl": "https://dsp.example/win?p=${AUCTION_PRCIE}""#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(has_issue(
        &result,
        "openrtb.macro.unknown",
        "seatbid[0].bid[0].nurl"
    ));
    assert!(has_issue(
        &result,
        "openrtb.bid.price_macro_missing",
        "seatbid[0].bid[0].nurl"
    ));
}

#[test]
fn price_macro_with_encoding_suffix_counts() {
    let response = bid_response(r#", "burl": "https://dsp.example/bill?p=${AUCTION_PRICE:B64}""#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(
        !result
            .issues
            .iter()
            .any(|issue| issue.id == "openrtb.bid.price_macro_missing"
                || issue.id == "openrtb.macro.unknown"),
        "{:?}",
        result.issues
    );
}

#[test]
fn lurl_without_loss_macro() {
    let response = bid_response(r#", "lurl": "https://dsp.example/loss""#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(has_issue(
        &result,
        "openrtb.bid.loss_macro_missing",
        "seatbid[0].bid[0].lurl"
    ));
}

// -- EID / SUA --

#[test]
fn eid_requires_source_and_uids() {
    let payload = request(r#""user":{"eids":[{}]},"imp":[{"id":"1","banner":{"w":300,"h":250}}]"#);
    let result = validate_bid_request_for_version(VERSION, &payload);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.eid.field_required",
        "user.eids[0].source"
    ));
    assert!(has_issue(
        &result,
        "openrtb.eid.field_required",
        "user.eids[0].uids"
    ));
}

#[test]
fn sua_empty_browsers_warns() {
    let payload = request(
        r#""device":{"ua":"Mozilla/5.0","sua":{}},"imp":[{"id":"1","banner":{"w":300,"h":250}}]"#,
    );
    let result = validate_bid_request_for_version(VERSION, &payload);
    assert!(result.valid);
    assert!(has_issue(
        &result,
        "openrtb.sua.browsers_empty",
        "device.sua.browsers"
    ));
}

#[test]
fn sua_browser_requires_brand() {
    let payload = request(
        r#""device":{"ua":"Mozilla/5.0","sua":{"browsers":[{}]}},"imp":[{"id":"1","banner":{"w":300,"h":250}}]"#,
    );
    let result = validate_bid_request_for_version(VERSION, &payload);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.field.required",
        "device.sua.browsers[0].brand"
    ));
}

// -- SKAdNetwork --

#[test]
fn skadn_request_requires_versions_sourceapp_ids() {
    let payload = request(r#""imp":[{"id":"1","banner":{"w":300,"h":250},"ext":{"skadn":{}}}]"#);
    let result = validate_bid_request_for_version(VERSION, &payload);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.skadn.field_required",
        "imp[0].ext.skadn.versions"
    ));
    assert!(has_issue(
        &result,
        "openrtb.skadn.field_required",
        "imp[0].ext.skadn.sourceapp"
    ));
    assert!(has_issue(
        &result,
        "openrtb.skadn.field_required",
        "imp[0].ext.skadn.skadnetids"
    ));
}

#[test]
fn skadn_response_requires_network() {
    let response = bid_response(r#", "ext": {"skadn": {"version": "3.0"}}"#);
    let result = validate_bid_response_for_version(VERSION, &response);
    assert!(!result.valid);
    assert!(has_issue(
        &result,
        "openrtb.skadn.field_required",
        "seatbid[0].bid[0].ext.skadn.network"
    ));
}
