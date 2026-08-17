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
    // These three carry adm without mtype, which 2.6 warns about but does not
    // reject: the buyer is leaving the markup type to be sniffed.
    FixtureCase {
        name: "valid-openrtb-2.6-202211-multi-seat",
        version: OpenRtbVersion::V2_6_202211,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202211-multi-seat.json"),
        valid: true,
        expected_issues: &[
            ("openrtb.bid.mtype_missing", "seatbid[0].bid[0].mtype"),
            ("openrtb.bid.mtype_missing", "seatbid[1].bid[0].mtype"),
        ],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202309-pod-package",
        version: OpenRtbVersion::V2_6_202309,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202309-pod-package.json"),
        valid: true,
        expected_issues: &[("openrtb.bid.mtype_missing", "seatbid[0].bid[0].mtype")],
    },
    FixtureCase {
        name: "valid-minimal-2.6-202505",
        version: OpenRtbVersion::V2_6_202505,
        input: include_str!("fixtures/bid-responses/2.6-202505/valid-minimal.json"),
        valid: true,
        expected_issues: &[("openrtb.bid.mtype_missing", "seatbid[0].bid[0].mtype")],
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
    // One response per tracked version, each exercising markup and fields that
    // version actually defines. Verdicts are verified against the CLI.
    FixtureCase {
        name: "valid-openrtb-2.0-banner-win",
        version: OpenRtbVersion::V2_0,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.0-banner-win.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.1-video-win",
        version: OpenRtbVersion::V2_1,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.1-video-win.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.2-deal-bid",
        version: OpenRtbVersion::V2_2,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.2-deal-bid.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.3-native-bid",
        version: OpenRtbVersion::V2_3,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.3-native-bid.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.3.1-app-bid",
        version: OpenRtbVersion::V2_3_1,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.3.1-app-bid.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.4-api-protocol-bid",
        version: OpenRtbVersion::V2_4,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.4-api-protocol-bid.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202210-mtype-video",
        version: OpenRtbVersion::V2_6_202210,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202210-mtype-video.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202303-pod-slot",
        version: OpenRtbVersion::V2_6_202303,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202303-pod-slot.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202402-flex-banner",
        version: OpenRtbVersion::V2_6_202402,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202402-flex-banner.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202409-native-bid",
        version: OpenRtbVersion::V2_6_202409,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202409-native-bid.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202501-audio-bid",
        version: OpenRtbVersion::V2_6_202501,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202501-audio-bid.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202606-multi-seat",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202606-multi-seat.json"),
        valid: true,
        expected_issues: &[],
    },
    // 3.0 responses validate through the same envelope; bid.media is the
    // AdCOM Ad object behind the Appendix C wrapper.
    FixtureCase {
        name: "valid-openrtb-3.0-layered-response",
        version: OpenRtbVersion::V3_0,
        input: include_str!("fixtures/bid-responses/valid-openrtb-3.0-layered-response.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-3.0-adcom-media",
        version: OpenRtbVersion::V3_0,
        input: include_str!("fixtures/bid-responses/valid-openrtb-3.0-adcom-media.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "invalid-openrtb-3.0-ad-no-subtype",
        version: OpenRtbVersion::V3_0,
        input: include_str!("fixtures/bid-responses/invalid-openrtb-3.0-ad-no-subtype.json"),
        valid: false,
        expected_issues: &[(
            "adcom.ad.subtype_required",
            "openrtb.response.seatbid[0].bid[0].media.ad",
        )],
    },
    FixtureCase {
        name: "invalid-openrtb-3.0-missing-response",
        version: OpenRtbVersion::V3_0,
        input: include_str!("fixtures/bid-responses/invalid-openrtb-3.0-missing-response.json"),
        valid: false,
        expected_issues: &[("openrtb.field.required", "openrtb.response")],
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

/// Bid responses used to be covered on six versions only, which is how the
/// 2.0-2.2 catalogs shipped with empty BidResponse objects (every response
/// validated clean) without a test noticing.
#[test]
fn every_tracked_version_has_a_validated_response_fixture() {
    for version in OpenRtbVersion::all() {
        assert!(
            VALIDATED_FIXTURES
                .iter()
                .any(|fixture| fixture.version == *version),
            "no validated bid response fixture for {}",
            version.id()
        );
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
