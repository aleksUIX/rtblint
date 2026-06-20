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

use rtblint_core::{validate as core_validate, validate_bid_request_for_version, OpenRtbVersion};
use wasm_bindgen::prelude::*;

fn to_js<T: serde::Serialize>(value: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(value).map_err(|err| JsValue::from_str(&err.to_string()))
}

/// Validate an OpenRTB bid request payload against the latest tracked 2.6 snapshot.
///
/// Returns `{ valid: boolean, issues: Array<{ id, severity, message, path }> }`.
#[wasm_bindgen]
pub fn validate(input: &str) -> Result<JsValue, JsValue> {
    to_js(&core_validate(input))
}

/// Validate an OpenRTB bid request payload against a specific tracked version id
/// (for example "2.6-202505"). Unknown ids fall back to the latest 2.6 snapshot.
#[wasm_bindgen]
pub fn validate_version(version_id: &str, input: &str) -> Result<JsValue, JsValue> {
    let version = OpenRtbVersion::ALL
        .into_iter()
        .find(|candidate| candidate.id() == version_id)
        .unwrap_or(OpenRtbVersion::V2_6_202505);
    to_js(&validate_bid_request_for_version(version, input))
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
