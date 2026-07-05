use serde_json::Value;

use rtblint_core::{validate_bid_response_for_version, OpenRtbVersion, ValidationResult};

struct FixtureCase {
    name: &'static str,
    version: OpenRtbVersion,
    input: &'static str,
    valid: bool,
    expected_issues: &'static [(&'static str, &'static str)],
}

const VALIDATED_FIXTURES: &[FixtureCase] = &[
    FixtureCase {
        name: "valid-openrtb-2.5-win-notice",
        version: OpenRtbVersion::V2_5,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.5-win-notice.json"),
        valid: true,
        expected_issues: &[],
    },
    // The 2.6-202204 catalog is an empty stub, so validation must refuse
    // loudly instead of passing everything silently.
    FixtureCase {
        name: "valid-openrtb-2.6-202204-apis-markup",
        version: OpenRtbVersion::V2_6_202204,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202204-apis-markup.json"),
        valid: false,
        expected_issues: &[("openrtb.version.unsupported", "")],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202211-multi-seat",
        version: OpenRtbVersion::V2_6_202211,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202211-multi-seat.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202309-pod-package",
        version: OpenRtbVersion::V2_6_202309,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202309-pod-package.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-minimal-2.6-202505",
        version: OpenRtbVersion::V2_6_202505,
        input: include_str!("fixtures/bid-responses/2.6-202505/valid-minimal.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "invalid-empty-seatbid-2.6-202505",
        version: OpenRtbVersion::V2_6_202505,
        input: include_str!("fixtures/bid-responses/2.6-202505/invalid-empty-seatbid.json"),
        valid: false,
        expected_issues: &[("openrtb.response.seatbid_or_nbr.required", "seatbid")],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202505-no-bid",
        version: OpenRtbVersion::V2_6_202505,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202505-no-bid.json"),
        valid: true,
        expected_issues: &[],
    },
    // The 3.0 catalog has no 2.x-style BidResponse object; layered 3.0
    // response validation is not implemented yet and must say so.
    FixtureCase {
        name: "valid-openrtb-3.0-layered-response",
        version: OpenRtbVersion::V3_0,
        input: include_str!("fixtures/bid-responses/valid-openrtb-3.0-layered-response.json"),
        valid: false,
        expected_issues: &[("openrtb.version.unsupported", "")],
    },
];

#[test]
fn bid_response_fixtures_match_expected_outcomes() {
    for fixture in VALIDATED_FIXTURES {
        let result = validate_bid_response_for_version(fixture.version, fixture.input);
        assert_eq!(
            result.valid, fixture.valid,
            "fixture {} returned unexpected validity: {:?}",
            fixture.name, result
        );

        for (issue_id, issue_path) in fixture.expected_issues {
            assert!(
                has_issue(&result, issue_id, issue_path),
                "fixture {} missing expected issue {} at {}: {:?}",
                fixture.name,
                issue_id,
                issue_path,
                result
            );
        }
    }
}

#[test]
fn three_zero_layered_response_fixture_keeps_its_shape() {
    let value: Value = serde_json::from_str(include_str!(
        "fixtures/bid-responses/valid-openrtb-3.0-layered-response.json"
    ))
    .expect("3.0 response fixture should be valid JSON");

    let response = value
        .get("openrtb")
        .and_then(Value::as_object)
        .and_then(|openrtb| openrtb.get("response"))
        .and_then(Value::as_object)
        .expect("3.0 response fixture should include openrtb.response");

    assert!(response.contains_key("id"));
}

fn has_issue(result: &ValidationResult, id: &str, path: &str) -> bool {
    result.issues.iter().any(|issue| {
        issue.id == id
            && if path.is_empty() {
                issue.path.is_none()
            } else {
                issue.path.as_deref() == Some(path)
            }
    })
}
