//! Fixture oracles for documented exchange requirements and allowed alternatives.

use std::{fs, path::PathBuf};

use rtblint_core::{
    validate_bid_request_with_profile, validate_bid_response_against_request_with_profile,
    validate_bid_response_with_profile, Dialect, OpenRtbVersion, Profile,
};
use serde_json::Value;

#[test]
fn unity_vungle_documented_contract_corpus() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/exchange-depth/unity-vungle");
    let manifest: Value =
        serde_json::from_str(&fs::read_to_string(directory.join("manifest.json")).unwrap())
            .unwrap();
    let cases = manifest["cases"].as_array().unwrap();
    assert!(
        cases.len() >= 150,
        "The fixture corpus must retain its alternatives and adversarial cases."
    );
    let mut failures = Vec::new();
    for case in cases {
        let id = case["id"].as_str().unwrap();
        let version = OpenRtbVersion::from_id(case["version"].as_str().unwrap()).unwrap();
        let profile = Profile::from_id(case["profile"].as_str().unwrap()).unwrap();
        let payload = fs::read_to_string(directory.join(case["file"].as_str().unwrap())).unwrap();
        let result = match case["direction"].as_str().unwrap() {
            "request" => {
                validate_bid_request_with_profile(version, Dialect::SpecJson, profile, &payload)
            }
            "response" => {
                validate_bid_response_with_profile(version, Dialect::SpecJson, profile, &payload)
            }
            "pair" => {
                let request =
                    fs::read_to_string(directory.join(case["request_file"].as_str().unwrap()))
                        .unwrap();
                validate_bid_response_against_request_with_profile(
                    version,
                    Dialect::SpecJson,
                    profile,
                    &request,
                    &payload,
                )
            }
            unknown => panic!("Unrecognized fixture direction: {unknown}"),
        };
        let expected = case["expected_profile_findings"].as_array().unwrap();
        if expected.is_empty() {
            if !result.valid
                || result
                    .issues
                    .iter()
                    .any(|issue| issue.id.starts_with("openrtb.profile."))
            {
                failures.push(format!(
                    "{id}: valid documented alternative rejected: {:?}",
                    result.issues
                ));
            }
        } else {
            for finding in expected {
                let finding_id = finding["id"].as_str().unwrap();
                let path = finding["path"].as_str().unwrap();
                if !result
                    .issues
                    .iter()
                    .any(|issue| issue.id == finding_id && issue.path.as_deref() == Some(path))
                {
                    failures.push(format!(
                        "{id}: missing {finding_id} at {path}: {:?}",
                        result.issues
                    ));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
