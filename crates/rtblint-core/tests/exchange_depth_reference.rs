//! Source-backed rule-reference branches and unsupported-direction controls.
use rtblint_core::{
    validate_bid_request_with_profile, validate_bid_response_against_request_with_profile,
    validate_bid_response_with_profile, Dialect, OpenRtbVersion, Profile, Severity,
};
use serde_json::Value;

#[test]
fn source_derived_reference_and_scope_cases() {
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../fixtures/exchange-depth/reference/cases.json"
    ))
    .unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let name = case["id"].as_str().unwrap();
        let profile = Profile::from_id(case["profile"].as_str().unwrap()).unwrap();
        let version = OpenRtbVersion::from_id(case["version"].as_str().unwrap()).unwrap();
        let input = case["input"].to_string();
        let run = |profile| match case["direction"].as_str().unwrap() {
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
                &case["request"].to_string(),
                &input,
            ),
            _ => unreachable!(),
        };
        let result = run(profile);
        if let Some(valid) = case["valid"].as_bool() {
            assert_eq!(result.valid, valid, "{name}: {:?}", result.issues);
        }
        for (key, required) in [("expected", true), ("forbidden", false)] {
            for expected in case[key].as_array().unwrap() {
                let found = result.issues.iter().any(|issue| {
                    issue.id == expected["id"].as_str().unwrap()
                        && issue.path.as_deref() == expected["path"].as_str()
                        && matches!(
                            (issue.severity, expected["severity"].as_str()),
                            (Severity::Error, Some("error")) | (Severity::Warning, Some("warning"))
                        )
                });
                assert_eq!(
                    found, required,
                    "{name}: {key}: {expected}: {:?}",
                    result.issues
                );
            }
        }
        assert!(
            run(Profile::Spec)
                .issues
                .iter()
                .all(|i| !i.id.starts_with("openrtb.profile.")),
            "{name}: profile leaked into Spec"
        );
    }
}
