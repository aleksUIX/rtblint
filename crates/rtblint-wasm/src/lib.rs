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
    validate as core_validate, validate_bid_request_for_version,
    validate_bid_response_against_request, validate_bid_response_for_version, version_profiles,
    OpenRtbVersion, VersionRuleKind,
};
use serde::Serialize;
use wasm_bindgen::prelude::*;

fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|err| JsValue::from_str(&err.to_string()))
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
    let version = OpenRtbVersion::ALL
        .into_iter()
        .find(|candidate| candidate.id() == version_id)
        .unwrap_or(OpenRtbVersion::V2_6_202606);
    to_js(&validate_bid_request_for_version(version, input))
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
    let version = OpenRtbVersion::ALL
        .into_iter()
        .find(|candidate| candidate.id() == version_id)
        .unwrap_or(OpenRtbVersion::V2_6_202606);
    to_js(&validate_bid_response_for_version(version, input))
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
    let version = OpenRtbVersion::ALL
        .into_iter()
        .find(|candidate| candidate.id() == version_id)
        .unwrap_or(OpenRtbVersion::V2_6_202606);
    to_js(&validate_bid_response_against_request(
        version, request, response,
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
