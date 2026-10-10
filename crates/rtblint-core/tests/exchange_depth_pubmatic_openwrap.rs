//! Source-derived OpenWrap wire declarations and executed CTV boundary controls.
use rtblint_core::{
    validate_bid_request_with_profile, validate_bid_response_against_request_with_profile,
    validate_bid_response_with_profile, Dialect, OpenRtbVersion, Profile, Severity,
};
use serde_json::Value;

fn replay(profile_id: &str, direction: &str) {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../fixtures/exchange-depth/pubmatic-openwrap/cases.json"
    ))
    .unwrap();
    let mut failures = Vec::new();
    let mut count = 0;
    for case in fixture["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["profile"] == profile_id && case["direction"] == direction)
    {
        let profile = Profile::from_id(profile_id).unwrap();
        let input = serde_json::to_string(&case["input"]).unwrap();
        let version = OpenRtbVersion::V2_6_202606;
        let result = match direction {
            "request" => {
                validate_bid_request_with_profile(version, Dialect::SpecJson, profile, &input)
            }
            "response" => {
                validate_bid_response_with_profile(version, Dialect::SpecJson, profile, &input)
            }
            "pair" => validate_bid_response_against_request_with_profile(
                version,
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
            if !result.issues.iter().any(|issue| {
                issue.id == expected["id"].as_str().unwrap()
                    && issue.path.as_deref() == expected["path"].as_str()
                    && matches!(
                        (expected["severity"].as_str(), issue.severity),
                        (Some("error"), Severity::Error) | (Some("warning"), Severity::Warning)
                    )
            }) {
                failures.push(format!(
                    "{} missing {expected:?}: {:?}",
                    case["id"], result.issues
                ));
            }
        }
        count += 1;
    }
    assert!(count > 0);
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn openwrap_request_custom_wire_types() {
    replay("pubmatic-openwrap", "request");
}
#[test]
fn openwrap_response_custom_wire_types() {
    replay("pubmatic-openwrap", "response");
}
#[test]
fn openwrap_ctv_executed_request_boundaries() {
    replay("pubmatic-openwrap-ctv", "request");
}
#[test]
fn openwrap_ctv_response_positive() {
    replay("pubmatic-openwrap-ctv", "response");
}
#[test]
fn openwrap_preserves_generic_pair_checks() {
    replay("pubmatic-openwrap", "pair");
}
#[test]
fn spec_profile_remains_independent() {
    replay("spec", "request");
}
