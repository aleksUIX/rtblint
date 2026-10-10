//! Current public Digital Turbine contract, including retained tolerant alternatives.
use rtblint_core::{
    validate_bid_request_with_profile, validate_bid_response_against_request_with_profile,
    validate_bid_response_with_profile, Dialect, OpenRtbVersion, Profile, Severity,
};
use serde_json::Value;
use std::{fs, path::PathBuf};
#[test]
fn digital_turbine_current_public_contract_corpus() {
    let root =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/exchange-depth/dt-depth");
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(root.join("manifest.json")).unwrap()).unwrap();
    let mut failures = Vec::new();
    for case in manifest["cases"].as_array().unwrap() {
        let input = fs::read_to_string(root.join(case["file"].as_str().unwrap())).unwrap();
        let profile = Profile::from_id(case["profile"].as_str().unwrap()).unwrap();
        let version = OpenRtbVersion::from_id(case["version"].as_str().unwrap()).unwrap();
        let result = match case["direction"].as_str().unwrap() {
            "request" => {
                validate_bid_request_with_profile(version, Dialect::SpecJson, profile, &input)
            }
            "response" => {
                validate_bid_response_with_profile(version, Dialect::SpecJson, profile, &input)
            }
            "pair" => {
                let request =
                    fs::read_to_string(root.join(case["request_file"].as_str().unwrap())).unwrap();
                validate_bid_response_against_request_with_profile(
                    version,
                    Dialect::SpecJson,
                    profile,
                    &request,
                    &input,
                )
            }
            _ => unreachable!(),
        };
        if result.valid != case["expected_valid"].as_bool().unwrap() {
            failures.push(format!(
                "{} validity {}: {:?}",
                case["id"], result.valid, result.issues
            ));
        }
        let expected = case["expected_profile_findings"].as_array().unwrap();
        for finding in expected {
            let severity = if finding["severity"] == "warning" {
                Severity::Warning
            } else {
                Severity::Error
            };
            if !result.issues.iter().any(|issue| {
                issue.id == finding["id"].as_str().unwrap()
                    && issue.path.as_deref() == finding["path"].as_str()
                    && issue.severity == severity
            }) {
                failures.push(format!(
                    "{} missing {}: {:?}",
                    case["id"], finding, result.issues
                ));
            }
        }
        if expected.is_empty()
            && result
                .issues
                .iter()
                .any(|i| i.id.starts_with("openrtb.profile."))
        {
            failures.push(format!(
                "{} positive has profile findings: {:?}",
                case["id"], result.issues
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
