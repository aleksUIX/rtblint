use rtblint_core::{validate_bid_request_with_dialect, Dialect, OpenRtbVersion};
use serde_json::{json, Value};

fn findings(version: OpenRtbVersion, extra: Value) -> Vec<(String, String)> {
    let mut request =
        json!({"id":"privacy-regression", "imp":[{"id":"1", "banner":{"w":300,"h":250}}]});
    request
        .as_object_mut()
        .unwrap()
        .extend(extra.as_object().unwrap().clone());
    validate_bid_request_with_dialect(version, Dialect::SpecJson, &request.to_string())
        .issues
        .into_iter()
        .filter(|i| i.id.starts_with("openrtb.privacy.") || i.id.ends_with("_malformed"))
        .map(|i| (i.id, i.path.unwrap_or_default()))
        .collect()
}

#[test]
fn legacy_and_modern_signals_receive_the_same_checks_at_their_input_paths() {
    for (version, extra, path, code) in [
        (
            OpenRtbVersion::V2_5,
            json!({"regs":{"ext":{"gdpr":1}},"device":{"ifa":"synthetic-ifa"}}),
            "regs.ext.gdpr",
            "gdpr_without_consent",
        ),
        (
            OpenRtbVersion::V2_6_202606,
            json!({"regs":{"gdpr":1},"device":{"ifa":"synthetic-ifa"}}),
            "regs.gdpr",
            "gdpr_without_consent",
        ),
        (
            OpenRtbVersion::V2_5,
            json!({"user":{"ext":{"consent":"yes"}}}),
            "user.ext.consent",
            "tcf_malformed",
        ),
        (
            OpenRtbVersion::V2_6_202606,
            json!({"user":{"consent":"yes"}}),
            "user.consent",
            "tcf_malformed",
        ),
        (
            OpenRtbVersion::V2_5,
            json!({"regs":{"ext":{"us_privacy":"1Y"}}}),
            "regs.ext.us_privacy",
            "us_privacy_malformed",
        ),
        (
            OpenRtbVersion::V2_1,
            json!({"user":{"ext":{"us_privacy":"1Y"}}}),
            "user.ext.us_privacy",
            "us_privacy_malformed",
        ),
    ] {
        let actual = findings(version, extra);
        assert_eq!(actual.len(), 1, "{version:?}: {actual:?}");
        assert!(actual[0].0.ends_with(code));
        assert_eq!(actual[0].1, path);
    }
}

#[test]
fn selected_contract_wins_conflicts_including_empty_or_null_values() {
    let input = json!({"regs":{"gdpr":0,"ext":{"gdpr":1}},"device":{"ifa":"synthetic"}});
    let old = findings(OpenRtbVersion::V2_5, input.clone());
    assert!(old
        .iter()
        .any(|(id, p)| id.ends_with("signal_conflict") && p == "regs.gdpr"));
    assert!(old
        .iter()
        .any(|(id, p)| id.ends_with("gdpr_without_consent") && p == "regs.ext.gdpr"));
    let modern = findings(OpenRtbVersion::V2_6_202606, input);
    assert_eq!(
        modern,
        vec![(
            "openrtb.privacy.signal_conflict".into(),
            "regs.ext.gdpr".into()
        )]
    );
    for value in [Value::Null, json!("")] {
        let actual = findings(
            OpenRtbVersion::V2_6_202606,
            json!({"regs":{"gdpr":1},"device":{"ifa":"synthetic"}, "user":{"consent":value,"ext":{"consent":"synthetic-present"}}}),
        );
        assert!(actual.iter().any(|(id, _)| id.ends_with("signal_conflict")));
        assert!(actual
            .iter()
            .any(|(id, _)| id.ends_with("gdpr_without_consent")));
    }
}

#[test]
fn legacy_extended_identifiers_participate_in_contradiction_checks() {
    let actual = findings(
        OpenRtbVersion::V2_5,
        json!({"regs":{"coppa":1,"ext":{"us_privacy":"1YYN"}},"user":{"ext":{"eids":[{"source":"synthetic.example","uids":[{"id":"synthetic","atype":3}]}]}}}),
    );
    assert!(actual
        .iter()
        .any(|(id, p)| id.ends_with("coppa_eids") && p == "user.ext.eids"));
    assert!(actual
        .iter()
        .any(|(id, p)| id.ends_with("us_privacy_sale_eids") && p == "user.ext.eids"));
}

#[test]
fn equal_duplicates_and_unrelated_extensions_do_not_add_conflicts() {
    assert!(findings(
        OpenRtbVersion::V2_5,
        json!({"regs":{"gdpr":0,"ext":{"gdpr":0}},"user":{"ext":{"vendor":{"consent":"opaque"}}}})
    )
    .is_empty());
}
