/**
 * RTBlint: OpenRTB bid request / bid response linter.
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

/**
 * Validate an OpenRTB bid response against the bid request it answers.
 * Runs the full response validation plus cross-checks: impid resolution,
 * mtype and adm markup coherence against the referenced Imp's media
 * subtypes, dealid, seat, and currency constraints.
 * @param {string} response - Raw bid response JSON.
 * @param {string} request - Raw bid request JSON (the originating request).
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateResponseAgainstRequest(response, request, version) {
  return wasm.validate_response_against_request(version ?? "", request, response);
}

/**
 * Validate an OpenRTB bid request written in a specific JSON dialect.
 * "spec-json" (default) types flag fields such as imp.secure, regs.coppa and
 * pmp.private_auction as integers, the way the specification does.
 * "proto-json" follows the IAB OpenRTB protobuf schema, which declares 28 of
 * those fields bool; see protoBoolDivergences().
 * @param {string} input - Raw bid request JSON.
 * @param {"spec-json"|"proto-json"} dialect - JSON dialect the payload is written in.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateDialect(input, dialect, version) {
  return wasm.validate_dialect(version ?? "", dialect, input);
}

/**
 * Validate an OpenRTB bid response written in a specific JSON dialect.
 * @param {string} input - Raw bid response JSON.
 * @param {"spec-json"|"proto-json"} dialect - JSON dialect the payload is written in.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateResponseDialect(input, dialect, version) {
  return wasm.validate_response_dialect(version ?? "", dialect, input);
}

/**
 * Validate an OpenRTB bid request against an exchange profile.
 * "spec" (default) is the specification only. "google-ab" is Google
 * Authorized Buyers OpenRTB: at=3 (FIXED_PRICE) is a valid auction type, and
 * each Imp must carry ext.billing_id. "prebid-server" is Prebid Server
 * /openrtb2/auction: each Imp must name a bidder or stored request, and
 * wseat/bseat are refused.
 * @param {string} input - Raw bid request JSON.
 * @param {"spec"|"google-ab"|"prebid-server"} profile - Exchange profile.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateProfile(input, profile, version) {
  return wasm.validate_profile(version ?? "", "spec-json", profile, input);
}

/**
 * Validate an OpenRTB bid response against an exchange profile.
 * @param {string} input - Raw bid response JSON.
 * @param {"spec"|"google-ab"|"prebid-server"} profile - Exchange profile.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateResponseProfile(input, profile, version) {
  return wasm.validate_response_profile(version ?? "", "spec-json", profile, input);
}

/** Every field the IAB OpenRTB protobuf schema types differently from the spec. */
export function protoBoolDivergences() {
  return wasm.proto_bool_divergences();
}

/**
 * Validate an ARTF RTBRequest envelope and the OpenRTB payloads it carries.
 * @param {string} input - Raw RTBRequest JSON.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateArtfRequest(input, version) {
  return wasm.validate_artf_request(version ?? "", input);
}

/**
 * Validate an ARTF RTBResponse mutation set against the RTBRequest it answers:
 * intent eligibility, operation and payload coherence, and semantic path
 * resolution against the auction.
 * @param {string} rtbResponse - Raw RTBResponse JSON.
 * @param {string} rtbRequest - Raw RTBRequest envelope JSON.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateArtfResponse(rtbResponse, rtbRequest, version) {
  return wasm.validate_artf_response(version ?? "", rtbRequest, rtbResponse);
}

/**
 * Apply an ARTF mutation set and revalidate. Returns { result, application }:
 * result carries the mutation findings plus the OpenRTB findings the mutations
 * introduced, application carries the mutated payloads.
 * @param {string} rtbResponse - Raw RTBResponse JSON.
 * @param {string} rtbRequest - Raw RTBRequest envelope JSON.
 * @param {string} [version] - OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateArtfResponseApplied(rtbResponse, rtbRequest, version) {
  return wasm.validate_artf_response_applied(version ?? "", rtbRequest, rtbResponse);
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
