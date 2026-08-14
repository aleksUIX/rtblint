//! ARTF envelope, mutation, and apply-then-revalidate coverage.
//!
//! Fixtures are hand written after the shapes in the ARTF reference
//! implementation's sample corpus, not copied from it.

use serde_json::Value;

use rtblint_core::{
    apply_artf_mutations, validate_artf_mutations_applied, validate_artf_request,
    validate_artf_response_against_request, OpenRtbVersion, Severity, ValidationResult,
};

const VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;

const PUBLISHER_REQUEST: &str = include_str!("fixtures/artf/valid-publisher-request.json");
const MIXED_FLAGS_REQUEST: &str = include_str!("fixtures/artf/invalid-mixed-flag-encoding.json");
const DSP_REQUEST: &str = include_str!("fixtures/artf/valid-dsp-response-request.json");
const VALID_MUTATIONS: &str = include_str!("fixtures/artf/valid-mutations.json");
const INVALID_MUTATIONS: &str = include_str!("fixtures/artf/invalid-mutations.json");
const BID_SHADE_MUTATIONS: &str = include_str!("fixtures/artf/bid-shade-mutations.json");
const BREAKING_MUTATIONS: &str = include_str!("fixtures/artf/breaking-mutations.json");

fn has_issue(result: &ValidationResult, id: &str) -> bool {
    result.issues.iter().any(|issue| issue.id == id)
}

fn has_issue_at(result: &ValidationResult, id: &str, path: &str) -> bool {
    result
        .issues
        .iter()
        .any(|issue| issue.id == id && issue.path.as_deref() == Some(path))
}

fn issue_message(result: &ValidationResult, id: &str) -> String {
    result
        .issues
        .iter()
        .find(|issue| issue.id == id)
        .map(|issue| issue.message.clone())
        .unwrap_or_else(|| panic!("no {id} issue: {:?}", result.issues))
}

fn errors(result: &ValidationResult) -> Vec<&str> {
    result
        .issues
        .iter()
        .filter(|issue| issue.severity == Severity::Error)
        .map(|issue| issue.id.as_str())
        .collect()
}

// -- envelope --

#[test]
fn valid_publisher_envelope_passes_clean() {
    let result = validate_artf_request(VERSION, PUBLISHER_REQUEST);
    assert!(result.valid, "issues: {:?}", result.issues);
    assert!(result.issues.is_empty(), "issues: {:?}", result.issues);
}

/// The carried OpenRTB payload is protobuf JSON, so integer flags are the
/// error there, and the finding has to be reported inside the envelope.
#[test]
fn envelope_reports_integer_flags_in_the_carried_bid_request() {
    let result = validate_artf_request(VERSION, MIXED_FLAGS_REQUEST);

    assert!(!result.valid);
    assert!(has_issue_at(
        &result,
        "openrtb.dialect.integer_for_bool",
        "bid_request.imp[0].secure"
    ));
    assert!(has_issue_at(
        &result,
        "openrtb.dialect.integer_for_bool",
        "bid_request.regs.coppa"
    ));
}

#[test]
fn envelope_requires_the_proto_required_members() {
    let result = validate_artf_request(VERSION, r#"{ "originator": { "id": "ssp-1" } }"#);

    assert!(!result.valid);
    for member in ["id", "lifecycle", "tmax", "bid_request"] {
        assert!(
            has_issue_at(&result, "artf.field.required", member),
            "{member} should be reported required: {:?}",
            result.issues
        );
    }
}

#[test]
fn envelope_rejects_members_the_proto_does_not_define() {
    let result = validate_artf_request(
        VERSION,
        r#"{
            "id": "ep-1",
            "tmax": 100,
            "lifecycle": "LIFECYCLE_PUBLISHER_BID_REQUEST",
            "bid_request": { "id": "a", "imp": [{ "id": "1", "banner": { "w": 300, "h": 250 } }], "site": { "id": "s" } },
            "applicable_intents": ["ACTIVATE_DEALS"],
            "bidRequest": {}
        }"#,
    );

    assert!(!result.valid);
    assert!(has_issue_at(&result, "artf.field.undefined", "bidRequest"));
}

#[test]
fn envelope_reports_response_stage_lifecycle_without_a_bid_response() {
    let result = validate_artf_request(
        VERSION,
        r#"{
            "id": "ep-1",
            "tmax": 100,
            "lifecycle": "LIFECYCLE_DSP_BID_RESPONSE",
            "bid_request": { "id": "a", "imp": [{ "id": "1", "banner": { "w": 300, "h": 250 } }], "site": { "id": "s" } },
            "applicable_intents": ["BID_SHADE"]
        }"#,
    );

    assert!(!result.valid);
    assert!(has_issue(&result, "artf.lifecycle.payload_mismatch"));
}

#[test]
fn envelope_reports_implausible_and_impossible_tmax() {
    let template = |tmax: &str| {
        format!(
            r#"{{
                "id": "ep-1",
                "tmax": {tmax},
                "lifecycle": "LIFECYCLE_PUBLISHER_BID_REQUEST",
                "bid_request": {{ "id": "a", "imp": [{{ "id": "1", "banner": {{ "w": 300, "h": 250 }} }}], "site": {{ "id": "s" }} }},
                "applicable_intents": ["ACTIVATE_DEALS"]
            }}"#
        )
    };

    let zero = validate_artf_request(VERSION, &template("0"));
    assert!(!zero.valid);
    assert!(has_issue(&zero, "artf.tmax.non_positive"));

    let seconds = validate_artf_request(VERSION, &template("30"));
    assert!(seconds.valid, "issues: {:?}", seconds.issues);

    let too_long = validate_artf_request(VERSION, &template("5000"));
    assert!(
        too_long.valid,
        "an implausible tmax is a warning, not an error"
    );
    assert!(has_issue(&too_long, "artf.tmax.implausible"));
}

#[test]
fn envelope_reports_unknown_enum_values() {
    let result = validate_artf_request(
        VERSION,
        r#"{
            "id": "ep-1",
            "tmax": 100,
            "lifecycle": "LIFECYCLE_MIDROLL",
            "originator": { "type": "TYPE_CURATOR", "id": "c-1" },
            "bid_request": { "id": "a", "imp": [{ "id": "1", "banner": { "w": 300, "h": 250 } }], "site": { "id": "s" } },
            "applicable_intents": ["ACTIVATE_DEALS", "RESHAPE_AUCTION"]
        }"#,
    );

    assert!(!result.valid);
    assert!(has_issue(&result, "artf.lifecycle.unknown"));
    assert!(has_issue(&result, "artf.originator.type_unknown"));
    assert!(has_issue_at(
        &result,
        "artf.intent.unknown",
        "applicable_intents[1]"
    ));
}

/// protobuf JSON carries enums as either the value name or its number.
#[test]
fn envelope_accepts_numeric_enum_values() {
    let result = validate_artf_request(
        VERSION,
        r#"{
            "id": "ep-1",
            "tmax": 100,
            "lifecycle": 1,
            "originator": { "type": 3, "id": "exchange-1" },
            "bid_request": { "id": "a", "imp": [{ "id": "1", "banner": { "w": 300, "h": 250 } }], "site": { "id": "s" } },
            "applicable_intents": [2]
        }"#,
    );

    assert!(result.valid, "issues: {:?}", result.issues);
    assert!(result.issues.is_empty(), "issues: {:?}", result.issues);
}

// -- mutations --

#[test]
fn coherent_mutation_set_passes_clean() {
    let result =
        validate_artf_response_against_request(VERSION, PUBLISHER_REQUEST, VALID_MUTATIONS);
    assert!(result.valid, "issues: {:?}", result.issues);
    assert!(result.issues.is_empty(), "issues: {:?}", result.issues);
}

#[test]
fn mutation_response_must_echo_the_extension_point_id() {
    let result =
        validate_artf_response_against_request(VERSION, PUBLISHER_REQUEST, INVALID_MUTATIONS);

    assert!(has_issue_at(&result, "artf.response.id_mismatch", "id"));
    // The envelope id is not the bid request id, and that is the mistake the
    // reference implementation itself made.
    let message = issue_message(&result, "artf.response.id_mismatch");
    assert!(
        message.contains("not the bid request id"),
        "the message should name the confusion: {message}"
    );
}

#[test]
fn mutation_paths_are_resolved_against_the_auction() {
    let result =
        validate_artf_response_against_request(VERSION, PUBLISHER_REQUEST, INVALID_MUTATIONS);

    assert!(!result.valid);
    assert!(has_issue_at(
        &result,
        "artf.mutation.imp_unknown",
        "mutations[1].path"
    ));
    assert!(has_issue_at(
        &result,
        "artf.mutation.deal_unknown",
        "mutations[2].path"
    ));
    assert!(has_issue_at(
        &result,
        "artf.mutation.target_payload_missing",
        "mutations[0].path"
    ));
}

#[test]
fn mutation_payload_must_match_the_declared_intent() {
    let result =
        validate_artf_response_against_request(VERSION, PUBLISHER_REQUEST, INVALID_MUTATIONS);

    assert!(has_issue_at(
        &result,
        "artf.mutation.payload_intent_mismatch",
        "mutations[3].adjust_bid"
    ));
    assert!(has_issue_at(
        &result,
        "artf.mutation.price_negative",
        "mutations[0].adjust_bid.price"
    ));
}

#[test]
fn intents_outside_the_declared_set_are_rejected() {
    let result =
        validate_artf_response_against_request(VERSION, PUBLISHER_REQUEST, INVALID_MUTATIONS);

    assert!(has_issue_at(
        &result,
        "artf.mutation.intent_not_applicable",
        "mutations[0].intent"
    ));
}

/// The ARTF v1.0 document's examples use a vocabulary its own .proto does not
/// define. A mutation written from the document is still readable, so it is
/// mapped and reported rather than dismissed as unknown.
#[test]
fn document_vocabulary_is_recognised_and_flagged() {
    let result =
        validate_artf_response_against_request(VERSION, PUBLISHER_REQUEST, INVALID_MUTATIONS);

    assert!(has_issue_at(
        &result,
        "artf.mutation.legacy_spec_encoding",
        "mutations[4]"
    ));
    // Mapped, so the deal path still resolved: no unknown-intent finding.
    assert!(!errors(&result).contains(&"artf.mutation.intent_unknown"));
}

#[test]
fn margin_adjustments_are_reported_as_unrepresentable() {
    let result =
        validate_artf_response_against_request(VERSION, PUBLISHER_REQUEST, INVALID_MUTATIONS);

    assert!(has_issue(&result, "artf.mutation.no_openrtb_target"));
    assert!(has_issue(&result, "artf.mutation.margin_implausible"));
}

#[test]
fn an_empty_mutation_set_is_reported_as_a_wasted_call() {
    let result = validate_artf_response_against_request(
        VERSION,
        PUBLISHER_REQUEST,
        r#"{ "id": "extension-point-001", "mutations": [], "metadata": {} }"#,
    );

    assert!(result.valid);
    assert!(has_issue(&result, "artf.mutations.empty"));
}

#[test]
fn undocumented_semantic_paths_warn_rather_than_fail() {
    let result = validate_artf_response_against_request(
        VERSION,
        PUBLISHER_REQUEST,
        r#"{
            "id": "extension-point-001",
            "mutations": [
                {
                    "intent": "ACTIVATE_DEALS",
                    "op": "OPERATION_ADD",
                    "path": "/curation/pods/pod-1",
                    "ids": { "id": ["deal-x"] }
                }
            ],
            "metadata": { "api_version": "1.0.0", "model_version": "x" }
        }"#,
    );

    assert!(result.valid, "issues: {:?}", result.issues);
    assert!(has_issue(&result, "artf.mutation.path_unrecognized"));
}

// -- apply --

#[test]
fn applying_mutations_rewrites_the_bid_request() {
    let application = apply_artf_mutations(PUBLISHER_REQUEST, VALID_MUTATIONS);
    let mutated: Value = serde_json::from_str(
        application
            .bid_request
            .as_deref()
            .expect("a mutated bid request"),
    )
    .expect("mutated request is JSON");

    assert_eq!(application.applied, vec![0, 1, 2, 3]);
    assert!(application.skipped.is_empty());

    let imp = &mutated["imp"][0];
    let deals = imp["pmp"]["deals"].as_array().expect("deals");
    assert_eq!(deals.len(), 3, "the curated deal should have been added");
    let premium = deals
        .iter()
        .find(|deal| deal["id"] == "deal-premium")
        .expect("deal-premium");
    assert_eq!(premium["bidfloor"], 14.5);

    let segments = mutated["user"]["data"][0]["segment"]
        .as_array()
        .expect("segments");
    assert_eq!(
        segments.len(),
        3,
        "two segments appended to the existing one"
    );

    let metrics = imp["metric"].as_array().expect("metrics");
    assert_eq!(metrics.len(), 1);
    assert_eq!(metrics[0]["type"], "viewability");
}

#[test]
fn applying_a_shading_mutation_rewrites_the_bid_price() {
    let application = apply_artf_mutations(DSP_REQUEST, BID_SHADE_MUTATIONS);
    let mutated: Value = serde_json::from_str(
        application
            .bid_response
            .as_deref()
            .expect("a mutated bid response"),
    )
    .expect("mutated response is JSON");

    assert_eq!(application.applied, vec![0]);
    assert_eq!(mutated["seatbid"][0]["bid"][0]["price"], 3.15);
}

/// Intents with no OpenRTB field to write to are reported as skipped rather
/// than guessed at.
#[test]
fn unapplicable_intents_are_skipped_not_invented() {
    let application = apply_artf_mutations(PUBLISHER_REQUEST, INVALID_MUTATIONS);
    assert!(application.applied.is_empty() || !application.skipped.is_empty());
    assert!(
        application.skipped.contains(&5),
        "the margin mutation has no OpenRTB target: {:?}",
        application
    );
}

#[test]
fn coherent_mutations_introduce_no_openrtb_findings() {
    let outcome = validate_artf_mutations_applied(VERSION, PUBLISHER_REQUEST, VALID_MUTATIONS);
    assert!(outcome.result.valid, "issues: {:?}", outcome.result.issues);
    assert!(
        outcome.result.issues.is_empty(),
        "issues: {:?}",
        outcome.result.issues
    );
}

/// The point of the applied pass: a mutation that is structurally fine on its
/// own but leaves the request invalid once written in.
#[test]
fn applied_pass_reports_findings_the_mutations_introduced() {
    let outcome = validate_artf_mutations_applied(VERSION, PUBLISHER_REQUEST, BREAKING_MUTATIONS);

    let static_only =
        validate_artf_response_against_request(VERSION, PUBLISHER_REQUEST, BREAKING_MUTATIONS);
    assert!(
        static_only.valid,
        "the mutation passes the static checks: {:?}",
        static_only.issues
    );

    assert!(!outcome.result.valid);
    assert!(has_issue_at(
        &outcome.result,
        "openrtb.type.mismatch",
        "bid_request.imp[0].metric[0].value"
    ));
    let message = issue_message(&outcome.result, "openrtb.type.mismatch");
    assert!(
        message.starts_with("After applying the mutations:"),
        "the finding should be attributed to the mutation: {message}"
    );
}

/// Pre-existing findings are the payload's problem, not the agent's, so they
/// stay out of the applied report.
#[test]
fn applied_pass_filters_out_pre_existing_findings() {
    let outcome = validate_artf_mutations_applied(
        VERSION,
        MIXED_FLAGS_REQUEST,
        r#"{
            "id": "extension-point-002",
            "mutations": [
                {
                    "intent": "ACTIVATE_DEALS",
                    "op": "OPERATION_ADD",
                    "path": "/imp/imp-1",
                    "ids": { "id": ["deal-curated"] }
                }
            ],
            "metadata": { "api_version": "1.0.0", "model_version": "x" }
        }"#,
    );

    assert!(
        !has_issue(&outcome.result, "openrtb.dialect.integer_for_bool"),
        "the flag encoding was already broken before the mutation: {:?}",
        outcome.result.issues
    );
    assert_eq!(outcome.application.applied, vec![0]);
}
