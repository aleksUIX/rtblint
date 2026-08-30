//! Payload-versus-declaration privacy findings on full bid requests.

use rtblint_core::{validate_bid_request_for_version, OpenRtbVersion, ValidationResult};

const VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;

fn has_issue(result: &ValidationResult, id: &str, path: &str) -> bool {
    result
        .issues
        .iter()
        .any(|issue| issue.id == id && issue.path.as_deref() == Some(path))
}

fn request(extra: &str) -> String {
    format!(
        r#"{{"id":"req-1","at":1,"tmax":200,"site":{{"id":"s","domain":"publisher.example"}},"imp":[{{"id":"1","banner":{{"w":300,"h":250}}}}],{extra}}}"#
    )
}

#[test]
fn coppa_and_live_ifa() {
    let result = validate_bid_request_for_version(
        VERSION,
        &request(r#""regs":{"coppa":1},"device":{"ifa":"aebe4057-cebe-4301-8cc6-5e6e41c0bf47"}"#),
    );
    assert!(result.valid);
    assert!(has_issue(
        &result,
        "openrtb.privacy.coppa_identifier",
        "device.ifa"
    ));
}

#[test]
fn lmt_and_live_ifa() {
    let result = validate_bid_request_for_version(
        VERSION,
        &request(r#""device":{"lmt":1,"ifa":"aebe4057-cebe-4301-8cc6-5e6e41c0bf47"}"#),
    );
    assert!(result.valid);
    assert!(has_issue(&result, "openrtb.privacy.lmt_ifa", "device.ifa"));
}

#[test]
fn dnt_and_user_id() {
    let result = validate_bid_request_for_version(
        VERSION,
        &request(r#""device":{"dnt":1},"user":{"id":"cookie-1"}"#),
    );
    assert!(result.valid);
    assert!(has_issue(
        &result,
        "openrtb.privacy.dnt_identifiers",
        "user.id"
    ));
}

#[test]
fn gdpr_without_consent_and_buyeruid() {
    let result = validate_bid_request_for_version(
        VERSION,
        &request(r#""regs":{"gdpr":1},"user":{"buyeruid":"ssp-uid"}"#),
    );
    assert!(result.valid);
    assert!(has_issue(
        &result,
        "openrtb.privacy.gdpr_without_consent",
        "regs.gdpr"
    ));
}

#[test]
fn email_in_site_page_query() {
    let result = validate_bid_request_for_version(
        VERSION,
        r#"{"id":"req-1","at":1,"tmax":200,"site":{"id":"s","domain":"publisher.example","page":"https://publisher.example/a?email=buyer@example.com"},"imp":[{"id":"1","banner":{"w":300,"h":250}}]}"#,
    );
    assert!(result.valid);
    assert!(has_issue(&result, "openrtb.privacy.pii_email", "site.page"));
}

#[test]
fn coppa_precise_geo() {
    let result = validate_bid_request_for_version(
        VERSION,
        &request(r#""regs":{"coppa":1},"device":{"geo":{"lat":37.7749,"lon":-122.4194}}"#),
    );
    assert!(result.valid);
    assert!(has_issue(
        &result,
        "openrtb.privacy.coppa_geo",
        "device.geo.lat"
    ));
}
