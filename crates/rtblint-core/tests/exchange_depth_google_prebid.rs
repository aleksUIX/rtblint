//! Independent source-descriptor boundaries and hand-authored protocol cases.
use rtblint_core::{
    validate_bid_request_with_profile, validate_bid_response_against_request_with_profile,
    validate_bid_response_with_profile, Dialect, OpenRtbVersion, Profile, Severity,
};
use serde_json::Value;

const VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;

fn replay(profile_id: &str, direction: &str) {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/exchange-depth/google-prebid/cases.json"
    ))
    .unwrap();
    let cases = fixture["cases"].as_array().unwrap();
    let mut count = 0;
    let mut failures = Vec::new();
    for case in cases
        .iter()
        .filter(|case| case["profile"] == profile_id && case["direction"] == direction)
    {
        let profile = Profile::from_id(profile_id).unwrap();
        let input = serde_json::to_string(&case["input"]).unwrap();
        let result = match direction {
            "request" => {
                validate_bid_request_with_profile(VERSION, Dialect::SpecJson, profile, &input)
            }
            "response" => {
                validate_bid_response_with_profile(VERSION, Dialect::SpecJson, profile, &input)
            }
            "pair" => validate_bid_response_against_request_with_profile(
                VERSION,
                Dialect::SpecJson,
                profile,
                &serde_json::to_string(&case["request"]).unwrap(),
                &input,
            ),
            _ => unreachable!(),
        };
        if result.valid != case["valid"].as_bool().unwrap() {
            failures.push(format!(
                "{}: expected valid {}, found {:?}",
                case["id"], case["valid"], result.issues
            ));
        }
        for expected in case["expected"].as_array().unwrap() {
            let found = result.issues.iter().any(|issue| {
                issue.id == expected["id"].as_str().unwrap()
                    && expected
                        .get("path")
                        .map_or(true, |path| issue.path.as_deref() == path.as_str())
                    && expected.get("severity").map_or(true, |severity| {
                        matches!(
                            (severity.as_str(), issue.severity),
                            (Some("warning"), Severity::Warning) | (Some("error"), Severity::Error)
                        )
                    })
            });
            if !found {
                failures.push(format!(
                    "{} missing expected {expected:?}: {:?}",
                    case["id"], result.issues
                ));
            }
        }
        // Descriptor positives establish that published field shapes are not
        // rejected by an unrelated enum/range/oneof assertion.
        if case["source"] == "google-v210-descriptor"
            && case["valid"] == true
            && result
                .issues
                .iter()
                .any(|issue| issue.id.starts_with("openrtb.profile.google"))
        {
            failures.push(format!("{}: {:?}", case["id"], result.issues));
        }
        count += 1;
    }
    assert!(count > 0);
    assert!(
        failures.is_empty(),
        "{} case failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn google_request_protocol_boundaries() {
    replay("google-ab", "request");
}

#[test]
fn google_response_protocol_boundaries() {
    replay("google-ab", "response");
}

#[test]
fn google_paired_protocol_boundaries() {
    replay("google-ab", "pair");
}

#[test]
fn prebid_request_protocol_boundaries() {
    replay("prebid-server", "request");
}

#[test]
fn prebid_response_protocol_boundaries() {
    replay("prebid-server", "response");
}

#[test]
fn prebid_paired_protocol_boundaries() {
    replay("prebid-server", "pair");
}

#[test]
fn default_spec_keeps_extension_values_open() {
    let request = r#"{"id":"req","imp":[{"id":"1","banner":{"w":300,"h":250},"ext":{"billing_id":[123],"prebid":{"is_rewarded_inventory":true}}}],"ext":{"prebid":{"debug":1}}}"#;
    let result =
        validate_bid_request_with_profile(VERSION, Dialect::SpecJson, Profile::Spec, request);
    assert!(result.valid, "{:?}", result.issues);
    assert!(result
        .issues
        .iter()
        .all(|issue| !issue.id.starts_with("openrtb.profile")));
}
