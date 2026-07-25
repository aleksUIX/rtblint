use serde_json::Value;

use rtblint_core::{validate_bid_request_for_version, OpenRtbVersion, ValidationResult};

struct FixtureCase {
    name: &'static str,
    version: OpenRtbVersion,
    input: &'static str,
    valid: bool,
    expected_issues: &'static [(&'static str, &'static str)],
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
        expected_issues: &[(
            "openrtb.field.requires_skippable_video",
            "imp[0].video.skipmin",
        )],
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
    // -- Semantic rule pack --
    FixtureCase {
        name: "valid-schain-baseline",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/valid-schain-baseline.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "warning-schain-duplicate-node",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-schain-duplicate-node.json"),
        valid: true,
        expected_issues: &[("openrtb.schain.duplicate_node", "source.schain.nodes[1]")],
    },
    FixtureCase {
        name: "valid-schain-non-adjacent-repeat",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/valid-schain-non-adjacent-repeat.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "warning-schain-node-hp-missing",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-schain-node-hp-missing.json"),
        valid: true,
        expected_issues: &[("openrtb.schain.node.hp_missing", "source.schain.nodes[0]")],
    },
    FixtureCase {
        name: "invalid-schain-node-empty-asi",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/invalid-schain-node-empty-asi.json"),
        valid: false,
        expected_issues: &[(
            "openrtb.schain.node.identifier_empty",
            "source.schain.nodes[0].asi",
        )],
    },
    FixtureCase {
        name: "warning-regs-gpp-sid-without-gpp",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-regs-gpp-sid-without-gpp.json"),
        valid: true,
        expected_issues: &[("openrtb.regs.gpp_sid_without_gpp", "regs.gpp_sid")],
    },
    FixtureCase {
        name: "warning-regs-gpp-without-gpp-sid",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-regs-gpp-without-gpp-sid.json"),
        valid: true,
        expected_issues: &[("openrtb.regs.gpp_without_gpp_sid", "regs.gpp")],
    },
    FixtureCase {
        name: "warning-regs-us-privacy-malformed",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-regs-us-privacy-malformed.json"),
        valid: true,
        expected_issues: &[("openrtb.regs.us_privacy_malformed", "regs.us_privacy")],
    },
    FixtureCase {
        name: "valid-regs-us-privacy-not-applicable",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/valid-regs-us-privacy-not-applicable.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "warning-video-pod-rqddurs-empty",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-video-pod-rqddurs-empty.json"),
        valid: true,
        expected_issues: &[("openrtb.video.pod.rqddurs_empty", "imp[0].video.rqddurs")],
    },
    FixtureCase {
        name: "warning-video-pod-mincpmpersec-without-context",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!(
            "fixtures/bid-requests/warning-video-pod-mincpmpersec-without-context.json"
        ),
        valid: true,
        expected_issues: &[(
            "openrtb.video.pod.mincpmpersec_without_pod_context",
            "imp[0].video.mincpmpersec",
        )],
    },
    FixtureCase {
        name: "valid-video-pod-mincpmpersec-with-poddur",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/valid-video-pod-mincpmpersec-with-poddur.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "invalid-native-request-double-encoded",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/invalid-native-request-double-encoded.json"),
        valid: false,
        expected_issues: &[(
            "openrtb.native.request.double_encoded",
            "imp[0].native.request",
        )],
    },
    FixtureCase {
        name: "warning-native-request-unparseable",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-native-request-unparseable.json"),
        valid: true,
        expected_issues: &[(
            "openrtb.native.request.unparseable",
            "imp[0].native.request",
        )],
    },
    FixtureCase {
        name: "warning-native-request-legacy-wrapper",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-native-request-legacy-wrapper.json"),
        valid: true,
        expected_issues: &[(
            "openrtb.native.request.legacy_wrapper",
            "imp[0].native.request",
        )],
    },
    FixtureCase {
        name: "valid-openrtb-2.3-native-feed-semantic-check",
        version: OpenRtbVersion::V2_3,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.3-native-feed.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "invalid-tmax-non-positive",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/invalid-tmax-non-positive.json"),
        valid: false,
        expected_issues: &[("openrtb.request.tmax_non_positive", "tmax")],
    },
    FixtureCase {
        name: "warning-tmax-implausible",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-tmax-implausible.json"),
        valid: true,
        expected_issues: &[("openrtb.request.tmax_implausible", "tmax")],
    },
    FixtureCase {
        name: "warning-cur-format-invalid",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-cur-format-invalid.json"),
        valid: true,
        expected_issues: &[("openrtb.request.cur_format_invalid", "cur[0]")],
    },
    FixtureCase {
        name: "invalid-bidfloor-negative",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/invalid-bidfloor-negative.json"),
        valid: false,
        expected_issues: &[
            ("openrtb.imp.bidfloor_negative", "imp[0].bidfloor"),
            (
                "openrtb.imp.bidfloor_negative",
                "imp[0].pmp.deals[0].bidfloor",
            ),
        ],
    },
    FixtureCase {
        name: "warning-bidfloorcur-format-invalid",
        version: OpenRtbVersion::V2_6_202606,
        input: include_str!("fixtures/bid-requests/warning-bidfloorcur-format-invalid.json"),
        valid: true,
        expected_issues: &[
            (
                "openrtb.imp.bidfloorcur_format_invalid",
                "imp[0].bidfloorcur",
            ),
            (
                "openrtb.imp.bidfloorcur_format_invalid",
                "imp[0].pmp.deals[0].bidfloorcur",
            ),
        ],
    },
    // One request per tracked version, each exercising something that version
    // actually defines. Verdicts are verified against the CLI.
    FixtureCase {
        name: "valid-openrtb-2.0-mobile-video",
        version: OpenRtbVersion::V2_0,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.0-mobile-video.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.1-geo-video",
        version: OpenRtbVersion::V2_1,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.1-geo-video.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.2-secure-pmp-video",
        version: OpenRtbVersion::V2_2,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.2-secure-pmp-video.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.3-native-feed",
        version: OpenRtbVersion::V2_3,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.3-native-feed.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.3.1-buyeruid-video",
        version: OpenRtbVersion::V2_3_1,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.3.1-buyeruid-video.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.4-skippable-video",
        version: OpenRtbVersion::V2_4,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.4-skippable-video.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202303-dooh-qty",
        version: OpenRtbVersion::V2_6_202303,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.6-202303-dooh-qty.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-app-video",
        version: OpenRtbVersion::V2_6_202409,
        input: include_str!("fixtures/bid-requests/valid-app-video.json"),
        valid: true,
        expected_issues: &[],
    },
    FixtureCase {
        name: "valid-openrtb-2.6-202501-eid-provenance",
        version: OpenRtbVersion::V2_6_202501,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.6-202501-eid-provenance.json"),
        valid: true,
        expected_issues: &[],
    },
    // 202505 types Content.genres as a plain string; 202606 corrects it to a
    // string array. The fixture follows the snapshot as published.
    FixtureCase {
        name: "valid-openrtb-2.6-202505-content-taxonomy",
        version: OpenRtbVersion::V2_6_202505,
        input: include_str!("fixtures/bid-requests/valid-openrtb-2.6-202505-content-taxonomy.json"),
        valid: true,
        expected_issues: &[],
    },
    // 3.0 has no 2.x-style BidRequest object; layered validation is not
    // implemented, so it must refuse rather than pass the payload.
    FixtureCase {
        name: "valid-openrtb-3.0-layered-request",
        version: OpenRtbVersion::V3_0,
        input: include_str!("fixtures/bid-requests/valid-openrtb-3.0-layered-request.json"),
        valid: false,
        expected_issues: &[("openrtb.version.unsupported", "")],
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

/// Every tracked version needs a fixture whose verdict is asserted, including
/// the ones that must refuse (the 2.6-202204 stub catalog and 3.0). Without
/// this a new snapshot can ship with no coverage at all.
#[test]
fn every_tracked_version_has_a_validated_request_fixture() {
    for version in OpenRtbVersion::all() {
        assert!(
            VALIDATED_FIXTURES
                .iter()
                .any(|fixture| fixture.version == *version),
            "no validated bid request fixture for {}",
            version.id()
        );
    }
}

/// The 3.0 fixture is the layered envelope, not a 2.x request. Keep its shape
/// asserted so the refusal above stays meaningful.
#[test]
fn three_zero_layered_request_fixture_keeps_its_shape() {
    let value: Value = serde_json::from_str(include_str!(
        "fixtures/bid-requests/valid-openrtb-3.0-layered-request.json"
    ))
    .expect("3.0 request fixture should be valid JSON");

    let request = value
        .get("openrtb")
        .and_then(Value::as_object)
        .and_then(|openrtb| openrtb.get("request"))
        .and_then(Value::as_object)
        .expect("3.0 request fixture should include openrtb.request");

    assert!(request.contains_key("id"));
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
