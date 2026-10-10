//! Regressions for generic catalog bugs and documented vendor wire alternatives.
use rtblint_core::{
    canonical_field, validate_bid_request_with_profile,
    validate_bid_response_against_request_with_profile, validate_bid_response_with_profile,
    Dialect, OpenRtbVersion, Profile,
};
use serde_json::json;

#[test]
fn index_structured_supplier_pods_follow_the_single_pod_contract() {
    let manifest: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/exchange-depth/index-structured-pods/cases.json"
    ))
    .unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let profile = Profile::from_id(case["profile"].as_str().unwrap()).unwrap();
        let result = validate_bid_request_with_profile(
            OpenRtbVersion::V2_6_202606,
            Dialect::SpecJson,
            profile,
            &case["input"].to_string(),
        );
        assert_eq!(
            result.valid,
            case["valid"].as_bool().unwrap(),
            "{}: {:?}",
            case["id"],
            result.issues
        );
        for expected in case["expected"].as_array().unwrap() {
            assert!(
                result
                    .issues
                    .iter()
                    .any(|issue| issue.id == expected["id"].as_str().unwrap()
                        && issue.path.as_deref() == expected["path"].as_str()),
                "{} missing {expected}: {:?}",
                case["id"],
                result.issues
            );
        }
        for forbidden in case["forbidden"].as_array().unwrap() {
            assert!(
                !result
                    .issues
                    .iter()
                    .any(|issue| issue.id == forbidden.as_str().unwrap()),
                "{} unexpected {forbidden}: {:?}",
                case["id"],
                result.issues
            );
        }
    }
}

#[test]
fn video_skip_is_a_binary_flag_across_every_catalog_that_declares_it() {
    for version in OpenRtbVersion::all() {
        if canonical_field(*version, "Video", "skip").is_none() {
            continue;
        }
        for (flag, valid) in [(0, true), (1, true), (2, false), (16, false), (500, false)] {
            let input =
                json!({"id":"r", "imp":[{"id":"i", "video":{"mimes":["video/mp4"], "skip":flag}}]});
            let result = validate_bid_request_with_profile(
                *version,
                Dialect::SpecJson,
                Profile::Spec,
                &input.to_string(),
            );
            let wrong_flag = result.issues.iter().any(|issue| {
                issue.id == "openrtb.value.invalid"
                    && issue.path.as_deref() == Some("imp[0].video.skip")
            });
            assert_eq!(
                !wrong_flag,
                valid,
                "{} skip={flag}: {:?}",
                version.id(),
                result.issues
            );
        }
    }
}

#[test]
fn alternate_native_markup_keeps_pair_checks_when_adm_is_empty() {
    let request = json!({"id":"r", "imp":[{"id":"i", "native":{"request":json!({"ver":"1.2", "assets":[{"id":1,"required":1,"title":{"len":25}}]}).to_string()}}]});
    for adm in [json!(null), json!(""), json!("  ")] {
        for (assets, missing) in [
            (json!([]), true),
            (json!([{"id":1,"title":{"text":"A title"}}]), false),
        ] {
            let response = json!({"id":"r", "seatbid":[{"bid":[{"id":"b", "impid":"i", "price":1.2, "adm":adm, "adm_native":{"ver":"1.2", "assets":assets, "link":{"url":"https://example.com"}}, "adomain":["example.com"], "burl":"https://example.com/win", "cat":["IAB1"], "crid":"c"}]}]});
            let result = validate_bid_response_against_request_with_profile(
                OpenRtbVersion::V2_6_202606,
                Dialect::SpecJson,
                Profile::AppLovinAlx,
                &request.to_string(),
                &response.to_string(),
            );
            let missing_asset = result
                .issues
                .iter()
                .any(|issue| issue.id == "openrtb.native.asset.required_missing");
            assert_eq!(
                missing_asset, missing,
                "adm={adm}, missing={missing}: {:?}",
                result.issues
            );
        }
    }
}

#[test]
fn bidswitch_empty_no_bid_is_scoped_and_does_not_hide_malformed_seatbid() {
    for profile in [
        Profile::Spec,
        Profile::BidSwitch,
        Profile::BidSwitchSupplier,
    ] {
        let empty = validate_bid_response_with_profile(
            OpenRtbVersion::V2_6_202606,
            Dialect::SpecJson,
            profile,
            r#"{"id":"r","seatbid":[],"ext":{}}"#,
        );
        assert_eq!(
            empty.valid,
            profile != Profile::Spec,
            "{profile}: {:?}",
            empty.issues
        );
        let malformed = validate_bid_response_with_profile(
            OpenRtbVersion::V2_6_202606,
            Dialect::SpecJson,
            profile,
            r#"{"id":"r","seatbid":{}}"#,
        );
        assert!(!malformed.valid, "{profile} must reject malformed seatbid");
    }
}

#[test]
fn commerce_grid_native_envelope_metadata_keeps_spec_type_checks() {
    let native = json!({"ver":"1.2", "privacy":1, "assets":[{"id":1,"title":{"len":25}}]});
    for (field, value) in [
        ("ver", json!(12)),
        ("api", json!("3")),
        ("battr", json!(["1"])),
        ("ext", json!([])),
    ] {
        let mut envelope = json!({"request":native.clone()});
        envelope[field] = value;
        let input = json!({"id":"r", "imp":[{"id":"i","native":envelope}]});
        let result = validate_bid_request_with_profile(
            OpenRtbVersion::V2_6_202606,
            Dialect::SpecJson,
            Profile::CommerceGrid,
            &input.to_string(),
        );
        let path = format!("imp[0].native.{field}");
        assert!(
            result
                .issues
                .iter()
                .any(|issue| issue.id == "openrtb.type.mismatch"
                    && issue.path.as_deref() == Some(&path)),
            "{path}: {:?}",
            result.issues
        );
    }
}

#[test]
fn unsupported_vendor_directions_preserve_spec_checks_and_report_scope() {
    {
        let profile = Profile::AppLovinAlx;
        let result = validate_bid_request_with_profile(
            OpenRtbVersion::V2_6_202606,
            Dialect::SpecJson,
            profile,
            r#"{"id":"r","imp":[{"id":"i","banner":{}}]}"#,
        );
        assert!(result.valid, "{profile}: {:?}", result.issues);
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.id == "openrtb.profile.scope.unsupported"));
    }
    for profile in [
        Profile::CommerceGrid,
        Profile::Sovrn,
        Profile::IndexExchangeSeller,
    ] {
        let result = validate_bid_response_with_profile(
            OpenRtbVersion::V2_6_202606,
            Dialect::SpecJson,
            profile,
            r#"{"id":"r","nbr":0}"#,
        );
        assert!(result.valid, "{profile}: {:?}", result.issues);
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.id == "openrtb.profile.scope.unsupported"));
    }
    let input = include_str!("fixtures/bid-responses/valid-openrtb-3.0-layered-response.json");
    let spec = validate_bid_response_with_profile(
        OpenRtbVersion::V3_0,
        Dialect::SpecJson,
        Profile::Spec,
        input,
    );
    for id in Profile::ids().iter().filter(|id| **id != "spec") {
        let mut vendor = validate_bid_response_with_profile(
            OpenRtbVersion::V3_0,
            Dialect::SpecJson,
            Profile::from_id(id).unwrap(),
            input,
        );
        let scope = vendor.issues.pop().unwrap();
        assert_eq!(scope.id, "openrtb.profile.scope.unsupported", "{id}");
        assert_eq!(vendor, spec, "{id} must preserve canonical 3.0 behavior");
    }
}
