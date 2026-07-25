export interface Issue {
  id: string;
  severity: "error" | "warning";
  message: string;
  path?: string;
  /** OpenRTB spec section the finding derives from, e.g. "3.2.7". */
  section?: string;
}

export interface ValidationResult {
  valid: boolean;
  issues: Issue[];
}

export interface Rule {
  code: string;
  version: string;
  release_date: string;
  kind: string;
  paths: string[];
  replacement_paths: string[];
  summary: string;
  section: string;
  source: string;
}

/**
 * Validate an OpenRTB bid request JSON string.
 * @param input Raw bid request JSON.
 * @param version OpenRTB version id, e.g. "2.6-202505" (default: latest tracked 2.6 snapshot).
 */
export function validate(input: string, version?: string): ValidationResult;

/**
 * Validate an OpenRTB bid response JSON string.
 * @param input Raw bid response JSON.
 * @param version OpenRTB version id, e.g. "2.6-202505" (default: latest tracked 2.6 snapshot).
 */
export function validateResponse(input: string, version?: string): ValidationResult;

/**
 * Validate an OpenRTB bid response against the bid request it answers.
 * Runs the full response validation plus cross-checks: impid resolution,
 * mtype and adm markup coherence against the referenced Imp's media
 * subtypes, dealid, seat, and currency constraints.
 * @param response Raw bid response JSON.
 * @param request Raw bid request JSON (the originating request).
 * @param version OpenRTB version id, e.g. "2.6-202505" (default: latest tracked 2.6 snapshot).
 */
export function validateResponseAgainstRequest(
  response: string,
  request: string,
  version?: string
): ValidationResult;

/** Every tracked OpenRTB version id. */
export function versions(): string[];

/** The full versioned rule catalog, one entry per coded rule. */
export function rules(): Rule[];

/** The rtblint-core version this build was compiled against. */
export function coreVersion(): string;
