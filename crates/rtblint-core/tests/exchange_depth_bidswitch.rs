//! Vendor-source fixtures include optional absence, malformed extension types,
//! protocol boundaries and paired capture facts. No remote acceptance is inferred.
use rtblint_core::{
    validate_bid_request_with_profile, validate_bid_response_against_request_with_profile,
    validate_bid_response_with_profile, Dialect, OpenRtbVersion, Profile, Severity,
};
use serde_json::Value;

#[test]
fn source_derived_bidswitch_directional_cases() {
    for line in include_str!("fixtures/exchange-depth/bidswitch/cases.jsonl").lines() {
        let case: Value = serde_json::from_str(line).expect("synthetic fixture JSON");
        let name = case["name"].as_str().unwrap();
        let profile = Profile::from_id(case["profile"].as_str().unwrap()).unwrap();
        let payload = case["payload"].to_string();
        let version = OpenRtbVersion::V2_6_202606;
        let result = match case["mode"].as_str().unwrap() {
            "request" => {
                validate_bid_request_with_profile(version, Dialect::SpecJson, profile, &payload)
            }
            "response" => {
                validate_bid_response_with_profile(version, Dialect::SpecJson, profile, &payload)
            }
            "pair" => validate_bid_response_against_request_with_profile(
                version,
                Dialect::SpecJson,
                profile,
                &case["request"].to_string(),
                &payload,
            ),
            mode => panic!("unknown mode {mode}"),
        };
        if let Some(valid) = case.get("valid").and_then(Value::as_bool) {
            assert_eq!(result.valid, valid, "{name}: {:?}", result.issues);
        }
        let actual: Vec<_> = result
            .issues
            .iter()
            .filter(|issue| {
                issue.id.starts_with("openrtb.profile.") && issue.severity == Severity::Error
            })
            .map(|issue| serde_json::json!({"id":issue.id,"path":issue.path}))
            .collect();
        if let Some(expected) = case.get("expected_profile_errors") {
            assert_eq!(
                Value::Array(actual),
                *expected,
                "{name}: {:?}",
                result.issues
            );
        }
        for (key, should_exist) in [("required_issues", true), ("forbidden_issues", false)] {
            if let Some(expected) = case[key].as_array() {
                for expected in expected {
                    let found = result.issues.iter().any(|i| {
                        i.id == expected["id"].as_str().unwrap()
                            && i.path.as_deref() == expected["path"].as_str()
                    });
                    assert_eq!(
                        found, should_exist,
                        "{name}: {key}: {expected}: {:?}",
                        result.issues
                    );
                }
            }
        }
        if let Some(warnings) = case
            .get("expected_profile_warnings")
            .and_then(Value::as_array)
        {
            for warning in warnings {
                assert!(
                    result
                        .issues
                        .iter()
                        .any(|issue| issue.severity == Severity::Warning
                            && issue.id == warning["id"].as_str().unwrap()
                            && issue.path.as_deref() == warning["path"].as_str()),
                    "{name}: missing warning {warning}: {:?}",
                    result.issues
                );
            }
        }
        // Declaring a vendor profile must not make these extensions closed for Spec.
        let spec = match case["mode"].as_str().unwrap() {
            "request" => validate_bid_request_with_profile(
                version,
                Dialect::SpecJson,
                Profile::Spec,
                &payload,
            ),
            "response" => validate_bid_response_with_profile(
                version,
                Dialect::SpecJson,
                Profile::Spec,
                &payload,
            ),
            _ => validate_bid_response_against_request_with_profile(
                version,
                Dialect::SpecJson,
                Profile::Spec,
                &case["request"].to_string(),
                &payload,
            ),
        };
        assert!(
            !spec
                .issues
                .iter()
                .any(|issue| issue.id.starts_with("openrtb.profile.")),
            "{name}: Spec profile leaked vendor requirements"
        );
    }
}
