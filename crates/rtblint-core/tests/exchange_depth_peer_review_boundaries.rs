//! Source-backed positive and adverse controls for independently reviewed diagnostics.
use rtblint_core::{
    validate_bid_request_with_profile, validate_bid_response_with_profile, Dialect, OpenRtbVersion,
    Profile, Severity,
};
use serde_json::Value;

#[test]
fn peer_review_diagnostics_have_asserted_boundaries() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/exchange-depth/peer-review-boundaries/cases.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let profile = Profile::from_id(case["profile"].as_str().unwrap()).unwrap();
        let input = serde_json::to_string(&case["input"]).unwrap();
        let result = match case["direction"].as_str().unwrap() {
            "request" => validate_bid_request_with_profile(
                OpenRtbVersion::V2_6_202606,
                Dialect::SpecJson,
                profile,
                &input,
            ),
            "response" => validate_bid_response_with_profile(
                OpenRtbVersion::V2_6_202606,
                Dialect::SpecJson,
                profile,
                &input,
            ),
            _ => unreachable!(),
        };
        assert_eq!(
            result.valid,
            case["valid"].as_bool().unwrap(),
            "{}: {:?}",
            case["id"],
            result.issues
        );
        for expected in case["expected"].as_array().unwrap() {
            assert!(
                result.issues.iter().any(|issue| {
                    issue.id == expected["id"].as_str().unwrap()
                        && issue.path.as_deref() == expected["path"].as_str()
                        && issue.severity
                            == if expected["severity"] == "warning" {
                                Severity::Warning
                            } else {
                                Severity::Error
                            }
                }),
                "{} missing {expected}: {:?}",
                case["id"],
                result.issues
            );
        }
        if case["expected"].as_array().unwrap().is_empty() {
            assert!(
                result
                    .issues
                    .iter()
                    .all(|issue| !issue.id.starts_with("openrtb.profile.")),
                "{}: {:?}",
                case["id"],
                result.issues
            );
        }
    }
}
