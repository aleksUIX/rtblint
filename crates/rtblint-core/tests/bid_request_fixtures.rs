use serde_json::Value;

use rtblint_core::{validate_bid_request_for_version, OpenRtbVersion, ValidationResult};

struct FixtureCase {
    name: &'static str,
    version: OpenRtbVersion,
    input: &'static str,
    valid: bool,
    expected_issues: &'static [(&'static str, &'static str)],
}

struct InventoryFixtureCase {
    name: &'static str,
    version: OpenRtbVersion,
    input: &'static str,
    family: OpenRtbFixtureFamily,
}

#[derive(Clone, Copy)]
enum OpenRtbFixtureFamily {
    TwoXRequest,
    ThreeZeroRequest,
}

const VALIDATED_FIXTURES: &[FixtureCase] = &[
    FixtureCase {
        name: "valid-openrtb-2.5-header-bidding-video",
        version: OpenRtbVersion::V2_5,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.5-header-bidding-video.json"),
        valid: true,
        expected_issues: &[],
    },
    // The 2.6-202204 catalog is an empty stub, so validation must refuse
    // loudly instead of passing everything silently.
    FixtureCase {
        name: "valid-openrtb-2.6-202204-ctv-consent",
        version: OpenRtbVersion::V2_6_202204,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.6-202204-ctv-consent.json"),
        valid: false,
        expected_issues: &[("openrtb.version.unsupported", "")],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202210-ctv-baseline",
        version: OpenRtbVersion::V2_6_202210,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.6-202210-ctv-baseline.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202211-dooh-gpp",
        version: OpenRtbVersion::V2_6_202211,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.6-202211-dooh-gpp.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202309-adpod-floors",
        version: OpenRtbVersion::V2_6_202309,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.6-202309-adpod-floors.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202402-poddedupe-video",
        version: OpenRtbVersion::V2_6_202402,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.6-202402-poddedupe-video.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-web-video",
        version: OpenRtbVersion::V2_6_202505,
        input: include_str!("fixtures/bid-requests/valid-web-video.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "warning-deprecated-video-placement",
        version: OpenRtbVersion::V2_6_202303,
        input: include_str!("fixtures/bid-requests/warning-deprecated-video-placement.json"),
        valid: true,
        expected_issues: &[("openrtb.field.deprecated", "imp[0].video.placement")],
    },
    FixtureCase {
        name: "invalid-openrtb-2.4-skipmin-without-skip",
        version: OpenRtbVersion::V2_4,
        input: include_str!("fixtures/bid-requests/invalid-openrtb-2.4-skipmin-without-skip.json"),
        valid: false,
        expected_issues: &[("openrtb.field.requires_skippable_video", "imp[0].video.skipmin")],
    },
    FixtureCase {
        name: "invalid-openrtb-2.5-video-plcmt-too-early",
        version: OpenRtbVersion::V2_5,
        input: include_str!("fixtures/bid-requests/invalid-openrtb-2.5-video-plcmt-too-early.json"),
        valid: false,
        expected_issues: &[("openrtb.field.undefined", "imp[0].video.plcmt")],
    },
    FixtureCase {
        name: "invalid-moved-gdpr",
        version: OpenRtbVersion::V2_6_202505,
        input: include_str!("fixtures/bid-requests/invalid-moved-gdpr.json"),
        valid: false,
        expected_issues: &[("openrtb.field.moved", "regs.ext.gdpr")],
    },
    FixtureCase {
        name: "invalid-missing-video-mimes",
        version: OpenRtbVersion::V2_6_202505,
        input: include_str!("fixtures/bid-requests/invalid-missing-video-mimes.json"),
        valid: false,
        expected_issues: &[("openrtb.field.required", "imp[0].video.mimes")],
    },
    FixtureCase {
        name: "invalid-site-app-conflict",
        version: OpenRtbVersion::V2_6_202505,
        input: include_str!("fixtures/bid-requests/invalid-site-app-conflict.json"),
        valid: false,
        expected_issues: &[("openrtb.fields.mutually_exclusive", "site")],
    },
];

const INVENTORY_FIXTURES: &[InventoryFixtureCase] = &[
    InventoryFixtureCase {
        name: "valid-openrtb-2.0-mobile-video",
        version: OpenRtbVersion::V2_0,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.0-mobile-video.json"),
        family: OpenRtbFixtureFamily::TwoXRequest,
    },
    InventoryFixtureCase {
        name: "valid-openrtb-2.1-geo-video",
        version: OpenRtbVersion::V2_1,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.1-geo-video.json"),
        family: OpenRtbFixtureFamily::TwoXRequest,
    },
    InventoryFixtureCase {
        name: "valid-openrtb-2.2-secure-pmp-video",
        version: OpenRtbVersion::V2_2,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.2-secure-pmp-video.json"),
        family: OpenRtbFixtureFamily::TwoXRequest,
    },
    InventoryFixtureCase {
        name: "valid-openrtb-2.3-native-feed",
        version: OpenRtbVersion::V2_3,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.3-native-feed.json"),
        family: OpenRtbFixtureFamily::TwoXRequest,
    },
    InventoryFixtureCase {
        name: "valid-openrtb-2.3.1-buyeruid-video",
        version: OpenRtbVersion::V2_3_1,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.3.1-buyeruid-video.json"),
        family: OpenRtbFixtureFamily::TwoXRequest,
    },
    InventoryFixtureCase {
        name: "valid-openrtb-2.4-skippable-video",
        version: OpenRtbVersion::V2_4,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.4-skippable-video.json"),
        family: OpenRtbFixtureFamily::TwoXRequest,
    },
    InventoryFixtureCase {
        name: "valid-openrtb-2.6-202303-refresh-video",
        version: OpenRtbVersion::V2_6_202303,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.6-202303-refresh-video.json"),
        family: OpenRtbFixtureFamily::TwoXRequest,
    },
    InventoryFixtureCase {
        name: "valid-app-video",
        version: OpenRtbVersion::V2_6_202409,
        input: include_str!("fixtures/bid-requests/valid-app-video.json"),
        family: OpenRtbFixtureFamily::TwoXRequest,
    },
    InventoryFixtureCase {
        name: "valid-openrtb-2.6-202501-content-taxonomy",
        version: OpenRtbVersion::V2_6_202501,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.6-202501-content-taxonomy.json"),
        family: OpenRtbFixtureFamily::TwoXRequest,
    },
    InventoryFixtureCase {
        name: "valid-openrtb-3.0-layered-request",
        version: OpenRtbVersion::V3_0,
        input: include_str!("fixtures/bid-requests/valid-openrtb-3.0-layered-request.json"),
        family: OpenRtbFixtureFamily::ThreeZeroRequest,
    },
];

#[test]
fn bid_request_fixtures_match_expected_outcomes() {
    for fixture in VALIDATED_FIXTURES {
        let result = validate_bid_request_for_version(fixture.version, fixture.input);
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
fn bid_request_inventory_fixtures_are_parseable() {
    for fixture in INVENTORY_FIXTURES {
        let value: Value = serde_json::from_str(fixture.input).unwrap_or_else(|error| {
            panic!(
                "inventory fixture {} for {} is not valid JSON: {}",
                fixture.name,
                fixture.version.id(),
                error
            )
        });

        let root = value.as_object().unwrap_or_else(|| {
            panic!(
                "inventory fixture {} for {} must be a JSON object",
                fixture.name,
                fixture.version.id()
            )
        });

        match fixture.family {
            OpenRtbFixtureFamily::TwoXRequest => {
                assert!(
                    root.contains_key("id"),
                    "inventory fixture {} for {} should include a top-level id",
                    fixture.name,
                    fixture.version.id()
                );
                assert!(
                    root.contains_key("imp"),
                    "inventory fixture {} for {} should include a top-level imp",
                    fixture.name,
                    fixture.version.id()
                );
            }
            OpenRtbFixtureFamily::ThreeZeroRequest => {
                let openrtb = root.get("openrtb").and_then(Value::as_object).unwrap_or_else(|| {
                    panic!(
                        "inventory fixture {} for {} should include an openrtb object",
                        fixture.name,
                        fixture.version.id()
                    )
                });

                assert!(
                    openrtb.contains_key("request"),
                    "inventory fixture {} for {} should include a request payload",
                    fixture.name,
                    fixture.version.id()
                );
            }
        }
    }
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