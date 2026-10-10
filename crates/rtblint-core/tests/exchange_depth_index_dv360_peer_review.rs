//! Source-table field shapes plus independently authored contract boundaries.
use rtblint_core::{
    validate_bid_request_with_profile, validate_bid_response_against_request_with_profile,
    validate_bid_response_with_profile, Dialect, OpenRtbVersion, Profile, Severity,
};
use serde_json::Value;

#[test]
fn independently_reviewed_index_and_dv360_boundaries() {
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../fixtures/exchange-depth/index-dv360-peer-review/cases.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    for case in manifest["cases"].as_array().unwrap() {
        let profile = Profile::from_id(case["profile"].as_str().unwrap()).unwrap();
        let input = case["input"].to_string();
        let run = |profile| match case["direction"].as_str().unwrap() {
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
            "pair" => validate_bid_response_against_request_with_profile(
                OpenRtbVersion::V2_6_202606,
                Dialect::SpecJson,
                profile,
                &case["request"].to_string(),
                &input,
            ),
            _ => unreachable!(),
        };
        let result = run(profile);
        if result.valid != case["valid"].as_bool().unwrap() {
            failures.push(format!("{}: {:?}", case["id"], result.issues));
        }
        for expected in case["expected"].as_array().unwrap() {
            if !result.issues.iter().any(|issue| {
                issue.id == expected["id"].as_str().unwrap()
                    && issue.path.as_deref() == expected["path"].as_str()
            }) {
                failures.push(format!(
                    "{} missing {expected}: {:?}",
                    case["id"], result.issues
                ));
            }
        }
        for forbidden in case["forbidden"].as_array().unwrap() {
            if result.issues.iter().any(|issue| {
                issue.id == forbidden["id"].as_str().unwrap()
                    && issue.path.as_deref() == forbidden["path"].as_str()
            }) {
                failures.push(format!(
                    "{} unexpected {forbidden}: {:?}",
                    case["id"], result.issues
                ));
            }
        }
        if result.valid
            && result.issues.iter().any(|issue| {
                issue.id.starts_with("openrtb.profile.") && issue.severity == Severity::Error
            })
        {
            failures.push(format!("{} positive profile error", case["id"]));
        }
        assert!(
            run(Profile::Spec)
                .issues
                .iter()
                .all(|issue| !issue.id.starts_with("openrtb.profile.")),
            "{} leaks profile rules into Spec",
            case["id"]
        );
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
