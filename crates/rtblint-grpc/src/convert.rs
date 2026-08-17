//! Mapping between `rtblint-core` types and the wire contract.
//!
//! Kept in one place and kept dumb. The transport must not decide anything
//! about validation.

use rtblint_core as core;
use tonic::Status;

use crate::proto;
use crate::provenance::provenance;

/// Version used when a request names none.
///
/// The newest tracked 2.6 revision rather than 3.0. 3.0 is a different object
/// model, not a newer one, so defaulting to it would silently validate the vast
/// majority of real traffic against a specification it does not use.
pub const DEFAULT_VERSION: core::OpenRtbVersion = core::OpenRtbVersion::V2_6_202606;

pub fn severity(value: core::Severity) -> proto::Severity {
    match value {
        core::Severity::Error => proto::Severity::Error,
        core::Severity::Warning => proto::Severity::Warning,
        // `rtblint_core::Severity` is #[non_exhaustive], so a level added
        // upstream lands here rather than failing to compile. Reporting it as
        // unspecified is the honest answer: this contract has no value for it
        // yet, and inventing one would misreport the finding.
        _ => proto::Severity::Unspecified,
    }
}

pub fn family(value: core::OpenRtbFamily) -> proto::OpenrtbFamily {
    match value {
        // prost strips the enum-name prefix and leaves the numeric part, so
        // OPENRTB_FAMILY_2_X becomes OpenrtbFamily2X rather than TwoX.
        core::OpenRtbFamily::TwoX => proto::OpenrtbFamily::OpenrtbFamily2X,
        core::OpenRtbFamily::ThreeZero => proto::OpenrtbFamily::OpenrtbFamily30,
    }
}

pub fn issue(value: &core::Issue) -> proto::Issue {
    proto::Issue {
        rule_id: value.id.clone(),
        severity: severity(value.severity) as i32,
        message: value.message.clone(),
        // Proto3 has no null, so an absent path is the empty string. A JSON
        // path is never legitimately empty, so the two do not collide.
        path: value.path.clone().unwrap_or_default(),
        section: value.section.clone().unwrap_or_default(),
    }
}

pub fn verdict(result: &core::ValidationResult, version: core::OpenRtbVersion) -> proto::Verdict {
    let errors = result
        .issues
        .iter()
        .filter(|issue| matches!(issue.severity, core::Severity::Error))
        .count();
    let warnings = result
        .issues
        .iter()
        .filter(|issue| matches!(issue.severity, core::Severity::Warning))
        .count();

    proto::Verdict {
        valid: result.valid,
        issues: result.issues.iter().map(issue).collect(),
        summary: Some(proto::Summary {
            errors: errors.try_into().unwrap_or(u32::MAX),
            warnings: warnings.try_into().unwrap_or(u32::MAX),
        }),
        effective_version: version.id().to_string(),
        provenance: Some(provenance()),
    }
}

/// Resolves the version a request asked for.
///
/// An unknown version is rejected rather than silently replaced with the
/// default. Validating against the wrong specification revision and reporting
/// success is the worst outcome available here: the caller gets a verdict that
/// looks authoritative and answers a question it did not ask.
pub fn version(context: Option<&proto::ValidationContext>) -> Result<core::OpenRtbVersion, Status> {
    let Some(requested) = context.map(|context| context.version.trim()) else {
        return Ok(DEFAULT_VERSION);
    };

    if requested.is_empty() {
        return Ok(DEFAULT_VERSION);
    }

    core::OpenRtbVersion::from_id(requested).ok_or_else(|| {
        let known: Vec<&str> = core::OpenRtbVersion::all()
            .iter()
            .map(|version| version.id())
            .collect();
        Status::invalid_argument(format!(
            "unknown OpenRTB version {requested:?}; known versions: {}",
            known.join(", ")
        ))
    })
}

/// Resolves the JSON dialect a request asked for.
///
/// Unset means spec JSON. Unlike the version, an unrecognised value cannot be
/// rejected with a list of alternatives without inventing one, because proto
/// enums are open: a value this build does not know is a value a later build
/// added. Treating it as the unspecified case is what the contract's stability
/// posture tells consumers to do, so the server does it too.
pub fn dialect(context: Option<&proto::ValidationContext>) -> core::Dialect {
    match context.map(|context| proto::JsonDialect::try_from(context.dialect)) {
        Some(Ok(proto::JsonDialect::Proto)) => core::Dialect::ProtoJson,
        _ => core::Dialect::SpecJson,
    }
}

/// Resolves the exchange profile a request asked for.
///
/// Empty means the specification only. An unknown id is rejected rather than
/// silently ignored: unlike JsonDialect, this is a string, so a typo is a
/// typo, not a value a later build added.
pub fn profile(context: Option<&proto::ValidationContext>) -> Result<core::Profile, Status> {
    let raw = context.map(|context| context.profile.trim()).unwrap_or("");
    if raw.is_empty() {
        return Ok(core::Profile::Spec);
    }
    core::Profile::from_id(raw).ok_or_else(|| {
        Status::invalid_argument(format!(
            "Unsupported profile: {raw}. Use one of: {}",
            core::Profile::ids().join(", ")
        ))
    })
}

/// Refuses a dialect on the ARTF RPCs.
///
/// ARTF transports its OpenRTB payloads as protobuf messages, so their JSON is
/// protobuf JSON by construction. A caller setting the field either believes
/// otherwise or is reusing a context object without meaning to, and both are
/// worth saying out loud: accepting JSON_DIALECT_PROTO silently would teach
/// the first caller that the choice was theirs to make.
pub fn reject_dialect_on_artf(context: Option<&proto::ValidationContext>) -> Result<(), Status> {
    let declared = context.map(|context| context.dialect).unwrap_or_default();
    if declared == proto::JsonDialect::Unspecified as i32 {
        return Ok(());
    }

    Err(Status::invalid_argument(
        "ValidationContext.dialect does not apply to ARTF payloads: ARTF carries its OpenRTB \
         messages as protobuf, so they are protobuf JSON by definition. Leave the field unset",
    ))
}

/// Refuses an exchange profile on the ARTF RPCs.
pub fn reject_profile_on_artf(context: Option<&proto::ValidationContext>) -> Result<(), Status> {
    let raw = context.map(|context| context.profile.trim()).unwrap_or("");
    if raw.is_empty() {
        return Ok(());
    }
    Err(Status::invalid_argument(
        "ValidationContext.profile does not apply to ARTF payloads: ARTF is not an exchange \
         dialect. Leave the field unset",
    ))
}

/// Mutation indexes for the wire, which counts in u32.
///
/// A mutation set large enough to overflow is not a real mutation set, and
/// saturating keeps a nonsense input from wrapping into a plausible index.
pub fn indexes(values: &[usize]) -> Vec<u32> {
    values
        .iter()
        .map(|value| u32::try_from(*value).unwrap_or(u32::MAX))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context(version: &str) -> proto::ValidationContext {
        proto::ValidationContext {
            version: version.to_string(),
            dialect: proto::JsonDialect::Unspecified as i32,
            profile: String::new(),
        }
    }

    fn dialect_context(dialect: proto::JsonDialect) -> proto::ValidationContext {
        proto::ValidationContext {
            version: String::new(),
            dialect: dialect as i32,
            profile: String::new(),
        }
    }

    #[test]
    fn an_absent_or_unspecified_dialect_is_spec_json() {
        assert_eq!(dialect(None), core::Dialect::SpecJson);
        assert_eq!(dialect(Some(&context(""))), core::Dialect::SpecJson);
        assert_eq!(
            dialect(Some(&dialect_context(proto::JsonDialect::Spec))),
            core::Dialect::SpecJson
        );
    }

    #[test]
    fn the_proto_dialect_maps_through() {
        assert_eq!(
            dialect(Some(&dialect_context(proto::JsonDialect::Proto))),
            core::Dialect::ProtoJson
        );
    }

    /// Proto enums are open by policy, so an unknown value must degrade to the
    /// unspecified case rather than fail the call.
    #[test]
    fn an_unknown_dialect_value_falls_back_to_spec_json() {
        let context = proto::ValidationContext {
            version: String::new(),
            dialect: 99,
            profile: String::new(),
        };
        assert_eq!(dialect(Some(&context)), core::Dialect::SpecJson);
    }

    #[test]
    fn artf_refuses_a_declared_dialect_including_the_correct_one() {
        assert!(reject_dialect_on_artf(None).is_ok());
        assert!(reject_dialect_on_artf(Some(&context(""))).is_ok());

        for declared in [proto::JsonDialect::Spec, proto::JsonDialect::Proto] {
            let status = reject_dialect_on_artf(Some(&dialect_context(declared)))
                .expect_err("a declared dialect is refused on ARTF");
            assert_eq!(status.code(), tonic::Code::InvalidArgument);
        }
    }

    fn profile_context(profile: &str) -> proto::ValidationContext {
        proto::ValidationContext {
            version: String::new(),
            dialect: proto::JsonDialect::Unspecified as i32,
            profile: profile.to_string(),
        }
    }

    #[test]
    fn an_absent_or_empty_profile_is_spec() {
        assert_eq!(profile(None).unwrap(), core::Profile::Spec);
        assert_eq!(
            profile(Some(&profile_context(""))).unwrap(),
            core::Profile::Spec
        );
        assert_eq!(
            profile(Some(&profile_context("  "))).unwrap(),
            core::Profile::Spec
        );
    }

    #[test]
    fn google_ab_aliases_map_through() {
        assert_eq!(
            profile(Some(&profile_context("google-ab"))).unwrap(),
            core::Profile::GoogleAuthorizedBuyers
        );
        assert_eq!(
            profile(Some(&profile_context("adx"))).unwrap(),
            core::Profile::GoogleAuthorizedBuyers
        );
    }

    #[test]
    fn an_unknown_profile_is_rejected_and_lists_what_is_available() {
        let error = profile(Some(&profile_context("magnite"))).unwrap_err();
        assert_eq!(error.code(), tonic::Code::InvalidArgument);
        assert!(error.message().contains("magnite"));
        assert!(error.message().contains("google-ab"));
    }

    #[test]
    fn artf_refuses_a_declared_profile() {
        assert!(reject_profile_on_artf(None).is_ok());
        assert!(reject_profile_on_artf(Some(&profile_context(""))).is_ok());
        let status = reject_profile_on_artf(Some(&profile_context("google-ab")))
            .expect_err("a declared profile is refused on ARTF");
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    #[test]
    fn indexes_saturate_rather_than_wrap() {
        assert_eq!(indexes(&[0, 3, 7]), vec![0, 3, 7]);
        assert_eq!(indexes(&[usize::MAX]), vec![u32::MAX]);
    }

    #[test]
    fn absent_context_uses_the_default_version() {
        assert_eq!(version(None).unwrap(), DEFAULT_VERSION);
    }

    #[test]
    fn an_empty_version_uses_the_default() {
        assert_eq!(version(Some(&context(""))).unwrap(), DEFAULT_VERSION);
        assert_eq!(version(Some(&context("  "))).unwrap(), DEFAULT_VERSION);
    }

    #[test]
    fn every_tracked_version_resolves() {
        for tracked in core::OpenRtbVersion::all() {
            assert_eq!(
                version(Some(&context(tracked.id()))).unwrap(),
                *tracked,
                "{} should resolve",
                tracked.id()
            );
        }
    }

    /// Silently falling back would hand the caller a verdict against a
    /// specification they did not name, which reads as authoritative and is not.
    #[test]
    fn an_unknown_version_is_rejected_and_says_what_is_available() {
        let error = version(Some(&context("2.6-209901"))).unwrap_err();
        assert_eq!(error.code(), tonic::Code::InvalidArgument);
        assert!(error.message().contains("2.6-209901"));
        assert!(error.message().contains("2.6-202606"));
    }

    #[test]
    fn the_default_is_a_2_x_version_not_3_0() {
        assert_eq!(DEFAULT_VERSION.family(), core::OpenRtbFamily::TwoX);
    }

    #[test]
    fn every_family_maps_to_a_specified_value() {
        for tracked in core::OpenRtbVersion::all() {
            assert_ne!(
                family(tracked.family()),
                proto::OpenrtbFamily::Unspecified,
                "{} has an unmapped family",
                tracked.id()
            );
        }
    }

    /// `rtblint_core::Issue` is #[non_exhaustive], so it cannot be constructed
    /// here. That is a feature for this test: the findings come from a real
    /// validation run, so the mapping is exercised against what the core
    /// actually produces rather than against a hand-built approximation of it.
    #[test]
    fn findings_from_a_real_run_map_every_field() {
        let result = core::validate_bid_request_for_version(
            DEFAULT_VERSION,
            r#"{"id":"r1","imp":[{"id":"i1","video":{}}]}"#,
        );

        assert!(
            !result.issues.is_empty(),
            "the fixture should produce findings"
        );

        for finding in &result.issues {
            let wire = issue(finding);

            assert_eq!(wire.rule_id, finding.id);
            assert_eq!(wire.message, finding.message);
            assert_ne!(
                wire.severity,
                proto::Severity::Unspecified as i32,
                "every finding must carry a severity the contract can express"
            );
            assert_eq!(wire.path, finding.path.clone().unwrap_or_default());
            assert_eq!(wire.section, finding.section.clone().unwrap_or_default());
        }
    }

    /// Proto3 has no null, so absent values become empty strings. A finding
    /// about the payload as a whole has no path, and rendering that as the
    /// string "null" or as a missing key would both be worse.
    #[test]
    fn a_document_level_finding_has_an_empty_path() {
        let result = core::validate_bid_request_for_version(DEFAULT_VERSION, "{ not json");

        let document_level = result
            .issues
            .iter()
            .find(|finding| finding.path.is_none())
            .expect("an unparseable payload produces a finding with no path");

        assert_eq!(issue(document_level).path, "");
    }

    #[test]
    fn the_summary_counts_match_the_findings() {
        let result = core::validate_bid_request_for_version(
            DEFAULT_VERSION,
            r#"{"id":"r1","imp":[{"id":"i1","video":{}}]}"#,
        );

        let wire = verdict(&result, DEFAULT_VERSION);
        let summary = wire.summary.expect("summary present");

        assert_eq!(
            (summary.errors + summary.warnings) as usize,
            wire.issues.len(),
            "every finding must be counted exactly once"
        );
        assert_eq!(wire.valid, summary.errors == 0);
    }
}
