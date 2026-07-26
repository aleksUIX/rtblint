"use strict";

/**
 * RTBlint: OpenRTB bid request / bid response linter.
 * CJS entry point backed by the rtblint-core WASM build.
 */

const wasm = require("./wasm/rtblint_wasm.js");

/**
 * Validate an OpenRTB bid request JSON string.
 * @param {string} input - Raw bid request JSON.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 * @returns {{valid: boolean, issues: Array<{id: string, severity: string, message: string, path?: string}>}}
 */
function validate(input, version) {
  return version === undefined ? wasm.validate(input) : wasm.validate_version(version, input);
}

/**
 * Validate an OpenRTB bid response JSON string.
 * @param {string} input - Raw bid response JSON.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
function validateResponse(input, version) {
  return version === undefined
    ? wasm.validate_response(input)
    : wasm.validate_response_version(version, input);
}

/**
 * Validate an OpenRTB bid response against the bid request it answers.
 * Runs the full response validation plus cross-checks: impid resolution,
 * mtype and adm markup coherence against the referenced Imp's media
 * subtypes, dealid, seat, and currency constraints.
 * @param {string} response - Raw bid response JSON.
 * @param {string} request - Raw bid request JSON (the originating request).
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
function validateResponseAgainstRequest(response, request, version) {
  return wasm.validate_response_against_request(version ?? "", request, response);
}

/** Every tracked OpenRTB version id. */
function versions() {
  return wasm.versions();
}

/** The full versioned rule catalog, one entry per coded rule. */
function rules() {
  return wasm.rules();
}

/** The rtblint-core version this build was compiled against. */
function coreVersion() {
  return wasm.core_version();
}

module.exports = {
  validate,
  validateResponse,
  validateResponseAgainstRequest,
  versions,
  rules,
  coreVersion,
};
