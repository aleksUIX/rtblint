//! Independent source-descriptor boundaries and hand-authored protocol cases.
use rtblint_core::{
    validate_bid_request_with_profile, validate_bid_response_against_request_with_profile,
    validate_bid_response_with_profile, Dialect, OpenRtbVersion, Profile, Severity,
};
use serde_json::Value;

const VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;

fn replay(profile_id: &str, direction: &str) {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/exchange-depth/inmobi-mobilefuse/cases.json"
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
fn inmobi_request_documented_boundaries() {
    replay("inmobi", "request");
}

#[test]
fn inmobi_response_documented_boundaries() {
    replay("inmobi", "response");
}

#[test]
fn inmobi_pair_documented_boundaries() {
    replay("inmobi", "pair");
}

#[test]
fn inmobi_supplier_request_documented_boundaries() {
    replay("inmobi-supplier", "request");
}

#[test]
fn inmobi_supplier_response_documented_boundaries() {
    replay("inmobi-supplier", "response");
}

#[test]
fn mobilefuse_request_documented_boundaries() {
    replay("mobilefuse", "request");
}

#[test]
fn mobilefuse_response_documented_boundaries() {
    replay("mobilefuse", "response");
}

#[test]
fn mobilefuse_pair_documented_boundaries() {
    replay("mobilefuse", "pair");
}

#[test]
fn mobilefuse_sdk_request_documented_boundaries() {
    replay("mobilefuse-sdk", "request");
}

#[test]
fn mobilefuse_sdk_response_documented_boundaries() {
    replay("mobilefuse-sdk", "response");
}

#[test]
fn spec_request_documented_boundaries() {
    replay("spec", "request");
}
