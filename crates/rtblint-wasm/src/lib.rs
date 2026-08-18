//! # rtblint-wasm
//!
//! WASM bindings for `rtblint-core`. Exposes OpenRTB bid request validation to
//! JavaScript/TypeScript via `wasm-bindgen`.
//!
//! Build with:
//! ```sh
//! wasm-pack build crates/rtblint-wasm --target web --out-dir <out>
//! ```
//!
//! `rtblint-core`'s result types already derive `Serialize`, so results are
//! converted straight to JS objects via `serde-wasm-bindgen`.

use rtblint_core::{
    proto_bool_fields, validate as core_validate, validate_artf_mutations_applied,
    validate_artf_request as core_validate_artf_request, validate_artf_response_against_request,
    validate_bid_request_for_version, validate_bid_request_with_dialect,
    validate_bid_request_with_profile, validate_bid_response_against_request,
    validate_bid_response_against_request_with_profile, validate_bid_response_for_version,
    validate_bid_response_with_dialect, validate_bid_response_with_profile, version_profiles,
    Dialect, OpenRtbVersion, Profile, VersionRuleKind,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Resolve a tracked version id. Unknown ids fall back to the latest 2.6
/// snapshot, which is the documented behaviour of every version-taking export
/// here; an empty string is how the JS wrappers say "default".
fn resolve_version(version_id: &str) -> OpenRtbVersion {
    OpenRtbVersion::ALL
        .into_iter()
        .find(|candidate| candidate.id() == version_id)
        .unwrap_or(OpenRtbVersion::V2_6_202606)
}

#[derive(Serialize)]
struct JsProtoBoolField {
    object: &'static str,
    field: &'static str,
}

fn kind_str(kind: VersionRuleKind) -> &'static str {
    match kind {
        VersionRuleKind::AddedField => "added-field",
        VersionRuleKind::AddedObject => "added-object",
        VersionRuleKind::AddedMacro => "added-macro",
        VersionRuleKind::AddedHeader => "added-header",
        VersionRuleKind::AddedList => "added-list",
        VersionRuleKind::AddedGuidance => "added-guidance",
        VersionRuleKind::AddedBehavior => "added-behavior",
        VersionRuleKind::DeprecatedField => "deprecated-field",
        VersionRuleKind::RemovedField => "removed-field",
        VersionRuleKind::MovedField => "moved-field",
        VersionRuleKind::CorrectedField => "corrected-field",
        VersionRuleKind::StructuralShift => "structural-shift",
    }
}

#[derive(Serialize)]
struct JsRule {
    code: &'static str,
    version: &'static str,
    release_date: &'static str,
    kind: &'static str,
    paths: Vec<&'static str>,
    replacement_paths: Vec<&'static str>,
    summary: &'static str,
    section: &'static str,
    source: &'static str,
}

/// The full versioned rule catalog: one entry per coded rule across every
/// tracked OpenRTB version. Drives the per-rule documentation pages.
#[wasm_bindgen]
pub fn rules() -> Result<JsValue, JsValue> {
    let mut out: Vec<JsRule> = Vec::new();
    for profile in version_profiles() {
        for rule in profile.rules {
            out.push(JsRule {
                code: rule.code,
                version: profile.version.id(),
                release_date: profile.release_date,
                kind: kind_str(rule.kind),
                paths: rule.paths.to_vec(),
                replacement_paths: rule.replacement_paths.to_vec(),
                summary: rule.summary,
                section: rule.section,
                source: rule.source,
            });
        }
    }
    to_js(&out)
}

/// Validate an OpenRTB bid request payload against the latest tracked 2.6 snapshot.
///
/// Returns `{ valid: boolean, issues: Array<{ id, severity, message, path }> }`.
#[wasm_bindgen]
pub fn validate(input: &str) -> Result<JsValue, JsValue> {
    to_js(&core_validate(input))
}

/// Validate an OpenRTB bid request payload against a specific tracked version id
/// (for example "2.6-202606"). Unknown ids fall back to the latest 2.6 snapshot.
#[wasm_bindgen]
pub fn validate_version(version_id: &str, input: &str) -> Result<JsValue, JsValue> {
    to_js(&validate_bid_request_for_version(
        resolve_version(version_id),
        input,
    ))
}

/// Validate an OpenRTB bid response payload against the latest tracked 2.6 snapshot.
///
/// Returns `{ valid: boolean, issues: Array<{ id, severity, message, path }> }`.
#[wasm_bindgen]
pub fn validate_response(input: &str) -> Result<JsValue, JsValue> {
    to_js(&validate_bid_response_for_version(
        OpenRtbVersion::V2_6_202606,
        input,
    ))
}

/// Validate an OpenRTB bid response payload against a specific tracked version id
/// (for example "2.6-202606"). Unknown ids fall back to the latest 2.6 snapshot.
#[wasm_bindgen]
pub fn validate_response_version(version_id: &str, input: &str) -> Result<JsValue, JsValue> {
    to_js(&validate_bid_response_for_version(
        resolve_version(version_id),
        input,
    ))
}

/// Validate an OpenRTB bid response against the bid request it answers, for
/// a specific tracked version id. Runs the full response validation plus
/// cross-checks: impid resolution, mtype and adm markup coherence against
/// the referenced Imp's media subtypes, dealid, seat, and currency
/// constraints. Unknown version ids fall back to the latest 2.6 snapshot.
#[wasm_bindgen]
pub fn validate_response_against_request(
    version_id: &str,
    request: &str,
    response: &str,
) -> Result<JsValue, JsValue> {
    to_js(&validate_bid_response_against_request(
        resolve_version(version_id),
        request,
        response,
    ))
}

/// Resolve a dialect id, defaulting to spec JSON for anything unrecognised so
/// a stale caller keeps the behaviour it had before dialects existed.
fn resolve_dialect(dialect_id: &str) -> Dialect {
    Dialect::from_id(dialect_id).unwrap_or(Dialect::SpecJson)
}

/// Resolve a profile id. Empty means the specification only. An unknown id is
/// an error rather than a silent fallback: a typo would otherwise validate
/// against the spec and look like a green Authorized Buyers check.
fn resolve_profile(profile_id: &str) -> Result<Profile, JsValue> {
    if profile_id.is_empty() {
        return Ok(Profile::Spec);
    }
    Profile::from_id(profile_id).ok_or_else(|| {
        JsValue::from_str(&format!(
            "Unsupported profile: {profile_id}. Use one of: {}",
            Profile::ids().join(", ")
        ))
    })
}

/// Validate an OpenRTB bid request written in a specific JSON dialect
/// ("spec-json" or "proto-json"). protobuf JSON declares 28 of the spec's
/// integer flag fields as bool, so the two encodings disagree in both
/// directions; see `proto_bool_divergences`.
#[wasm_bindgen]
pub fn validate_dialect(
    version_id: &str,
    dialect_id: &str,
    input: &str,
) -> Result<JsValue, JsValue> {
    to_js(&validate_bid_request_with_dialect(
        resolve_version(version_id),
        resolve_dialect(dialect_id),
        input,
    ))
}

/// Validate an OpenRTB bid response written in a specific JSON dialect.
#[wasm_bindgen]
pub fn validate_response_dialect(
    version_id: &str,
    dialect_id: &str,
    input: &str,
) -> Result<JsValue, JsValue> {
    to_js(&validate_bid_response_with_dialect(
        resolve_version(version_id),
        resolve_dialect(dialect_id),
        input,
    ))
}

/// Validate an OpenRTB bid request against a JSON dialect and an exchange
/// profile ("spec", "google-ab", "prebid-server", "xandr", or "magnite"). Empty profile id means the specification
/// only. Unknown ids are rejected.
#[wasm_bindgen]
pub fn validate_profile(
    version_id: &str,
    dialect_id: &str,
    profile_id: &str,
    input: &str,
) -> Result<JsValue, JsValue> {
    to_js(&validate_bid_request_with_profile(
        resolve_version(version_id),
        resolve_dialect(dialect_id),
        resolve_profile(profile_id)?,
        input,
    ))
}

/// Validate an OpenRTB bid response against a JSON dialect and an exchange
/// profile.
#[wasm_bindgen]
pub fn validate_response_profile(
    version_id: &str,
    dialect_id: &str,
    profile_id: &str,
    input: &str,
) -> Result<JsValue, JsValue> {
    to_js(&validate_bid_response_with_profile(
        resolve_version(version_id),
        resolve_dialect(dialect_id),
        resolve_profile(profile_id)?,
        input,
    ))
}

/// Validate an OpenRTB bid response against the bid request it answers, for
/// a specific dialect and exchange profile.
#[wasm_bindgen]
pub fn validate_response_against_request_profile(
    version_id: &str,
    dialect_id: &str,
    profile_id: &str,
    request: &str,
    response: &str,
) -> Result<JsValue, JsValue> {
    to_js(&validate_bid_response_against_request_with_profile(
        resolve_version(version_id),
        resolve_dialect(dialect_id),
        resolve_profile(profile_id)?,
        request,
        response,
    ))
}

/// Every field where the IAB OpenRTB protobuf schema and the specification
/// disagree on the type, as `{ object, field }` pairs. Drives the dialect
/// documentation.
#[wasm_bindgen]
pub fn proto_bool_divergences() -> Result<JsValue, JsValue> {
    let pairs: Vec<JsProtoBoolField> = proto_bool_fields()
        .iter()
        .map(|(object, field)| JsProtoBoolField { object, field })
        .collect();
    to_js(&pairs)
}

/// Validate an ARTF RTBRequest envelope and the OpenRTB payloads it carries.
#[wasm_bindgen]
pub fn validate_artf_request(version_id: &str, input: &str) -> Result<JsValue, JsValue> {
    to_js(&core_validate_artf_request(
        resolve_version(version_id),
        input,
    ))
}

/// Validate an ARTF RTBResponse mutation set against the RTBRequest envelope
/// it answers: intent eligibility, operation and payload coherence, and
/// semantic path resolution against the auction.
#[wasm_bindgen]
pub fn validate_artf_response(
    version_id: &str,
    rtb_request: &str,
    rtb_response: &str,
) -> Result<JsValue, JsValue> {
    to_js(&validate_artf_response_against_request(
        resolve_version(version_id),
        rtb_request,
        rtb_response,
    ))
}

/// Apply an ARTF mutation set and revalidate: returns `{ result, application }`
/// where result carries the mutation findings plus the OpenRTB findings the
/// mutations introduced, and application carries the mutated payloads.
#[wasm_bindgen]
pub fn validate_artf_response_applied(
    version_id: &str,
    rtb_request: &str,
    rtb_response: &str,
) -> Result<JsValue, JsValue> {
    to_js(&validate_artf_mutations_applied(
        resolve_version(version_id),
        rtb_request,
        rtb_response,
    ))
}

/// List every tracked OpenRTB version id, newest snapshots last.
#[wasm_bindgen]
pub fn versions() -> Result<JsValue, JsValue> {
    let ids: Vec<&'static str> = OpenRtbVersion::ALL.into_iter().map(|v| v.id()).collect();
    to_js(&ids)
}

/// The rtblint-core version this build was compiled against.
#[wasm_bindgen]
pub fn core_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
