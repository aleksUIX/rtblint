/* tslint:disable */
/* eslint-disable */

/**
 * The rtblint-core version this build was compiled against.
 */
export function core_version(): string;

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
 * Validate an OpenRTB bid response payload against the latest tracked 2.6 snapshot.
 *
 * Returns `{ valid: boolean, issues: Array<{ id, severity, message, path }> }`.
 */
export function validate_response(input: string): any;

/**
 * Validate an OpenRTB bid response payload against a specific tracked version id
 * (for example "2.6-202505"). Unknown ids fall back to the latest 2.6 snapshot.
 */
export function validate_response_version(version_id: string, input: string): any;

/**
 * Validate an OpenRTB bid request payload against a specific tracked version id
 * (for example "2.6-202505"). Unknown ids fall back to the latest 2.6 snapshot.
 */
export function validate_version(version_id: string, input: string): any;

/**
 * List every tracked OpenRTB version id, newest snapshots last.
 */
export function versions(): any;
