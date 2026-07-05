/**
 * rtblint: OpenRTB bid request / bid response linter.
 * ESM entry point backed by the rtblint-core WASM build.
 */

import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
const wasm = require("./wasm/rtblint_wasm.js");

/**
 * Validate an OpenRTB bid request JSON string.
 * @param {string} input - Raw bid request JSON.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 * @returns {{valid: boolean, issues: Array<{id: string, severity: string, message: string, path?: string}>}}
 */
export function validate(input, version) {
  return version === undefined ? wasm.validate(input) : wasm.validate_version(version, input);
}

/**
 * Validate an OpenRTB bid response JSON string.
 * @param {string} input - Raw bid response JSON.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateResponse(input, version) {
  return version === undefined
    ? wasm.validate_response(input)
    : wasm.validate_response_version(version, input);
}

/** Every tracked OpenRTB version id. */
export function versions() {
  return wasm.versions();
}

/** The full versioned rule catalog, one entry per coded rule. */
export function rules() {
  return wasm.rules();
}

/** The rtblint-core version this build was compiled against. */
export function coreVersion() {
  return wasm.core_version();
}
