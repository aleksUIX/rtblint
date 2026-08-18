/* tslint:disable */
/* eslint-disable */

/**
 * The rtblint-core version this build was compiled against.
 */
export function core_version(): string;

/**
 * Every field where the IAB OpenRTB protobuf schema and the specification
 * disagree on the type, as `{ object, field }` pairs. Drives the dialect
 * documentation.
 */
export function proto_bool_divergences(): any;

/**
 * The full versioned rule catalog: one entry per coded rule across every
 * tracked OpenRTB version. Drives the per-rule documentation pages.
 */
export function rules(): any;

/**
 * Validate an OpenRTB bid request payload against the latest tracked 2.6 snapshot.
 *
 * Returns `{ valid: boolean, issues: Array<{ id, severity, message, path }> }`.
 */
export function validate(input: string): any;

/**
 * Validate an ARTF RTBRequest envelope and the OpenRTB payloads it carries.
 */
export function validate_artf_request(version_id: string, input: string): any;

/**
 * Validate an ARTF RTBResponse mutation set against the RTBRequest envelope
 * it answers: intent eligibility, operation and payload coherence, and
 * semantic path resolution against the auction.
 */
export function validate_artf_response(version_id: string, rtb_request: string, rtb_response: string): any;

/**
 * Apply an ARTF mutation set and revalidate: returns `{ result, application }`
 * where result carries the mutation findings plus the OpenRTB findings the
 * mutations introduced, and application carries the mutated payloads.
 */
export function validate_artf_response_applied(version_id: string, rtb_request: string, rtb_response: string): any;

/**
 * Validate an OpenRTB bid request written in a specific JSON dialect
 * ("spec-json" or "proto-json"). protobuf JSON declares 28 of the spec's
 * integer flag fields as bool, so the two encodings disagree in both
 * directions; see `proto_bool_divergences`.
 */
export function validate_dialect(version_id: string, dialect_id: string, input: string): any;

/**
 * Validate an OpenRTB bid request against a JSON dialect and an exchange
 * profile ("spec", "google-ab", or "prebid-server"). Empty profile id means the specification
 * only. Unknown ids are rejected.
 */
export function validate_profile(version_id: string, dialect_id: string, profile_id: string, input: string): any;

/**
 * Validate an OpenRTB bid response payload against the latest tracked 2.6 snapshot.
 *
 * Returns `{ valid: boolean, issues: Array<{ id, severity, message, path }> }`.
 */
export function validate_response(input: string): any;

/**
 * Validate an OpenRTB bid response against the bid request it answers, for
 * a specific tracked version id. Runs the full response validation plus
 * cross-checks: impid resolution, mtype and adm markup coherence against
 * the referenced Imp's media subtypes, dealid, seat, and currency
 * constraints. Unknown version ids fall back to the latest 2.6 snapshot.
 */
export function validate_response_against_request(version_id: string, request: string, response: string): any;

/**
 * Validate an OpenRTB bid response against the bid request it answers, for
 * a specific dialect and exchange profile.
 */
export function validate_response_against_request_profile(version_id: string, dialect_id: string, profile_id: string, request: string, response: string): any;

/**
 * Validate an OpenRTB bid response written in a specific JSON dialect.
 */
export function validate_response_dialect(version_id: string, dialect_id: string, input: string): any;

/**
 * Validate an OpenRTB bid response against a JSON dialect and an exchange
 * profile.
 */
export function validate_response_profile(version_id: string, dialect_id: string, profile_id: string, input: string): any;

/**
 * Validate an OpenRTB bid response payload against a specific tracked version id
 * (for example "2.6-202606"). Unknown ids fall back to the latest 2.6 snapshot.
 */
export function validate_response_version(version_id: string, input: string): any;

/**
 * Validate an OpenRTB bid request payload against a specific tracked version id
 * (for example "2.6-202606"). Unknown ids fall back to the latest 2.6 snapshot.
 */
export function validate_version(version_id: string, input: string): any;

/**
 * List every tracked OpenRTB version id, newest snapshots last.
 */
export function versions(): any;
