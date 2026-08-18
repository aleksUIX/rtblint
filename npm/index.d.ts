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

/**
 * JSON dialect a payload is written in. "spec-json" types flag fields such as
 * imp.secure, regs.coppa and pmp.private_auction as integers, the way the
 * OpenRTB specification does. "proto-json" follows the IAB OpenRTB protobuf
 * schema, which declares 28 of those fields bool.
 */
export type Dialect = "spec-json" | "proto-json";

/** A field the OpenRTB protobuf schema types differently from the spec. */
export interface ProtoBoolField {
  object: string;
  field: string;
}

/** What applying an ARTF mutation set produced. */
export interface ArtfApplication {
  /** The bid request after every applicable mutation was applied. */
  bid_request: string | null;
  /** The bid response after every applicable mutation was applied. */
  bid_response: string | null;
  /** Indexes into `mutations` that were applied. */
  applied: number[];
  /** Indexes that were not applied: unresolved target, or no OpenRTB field to write to. */
  skipped: number[];
}

export interface ArtfMutationOutcome {
  result: ValidationResult;
  application: ArtfApplication;
}

/**
 * Validate an OpenRTB bid request written in a specific JSON dialect.
 * @param input Raw bid request JSON.
 * @param dialect Dialect the payload is written in.
 * @param version OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateDialect(
  input: string,
  dialect: Dialect,
  version?: string
): ValidationResult;

/** Validate an OpenRTB bid response written in a specific JSON dialect. */
export function validateResponseDialect(
  input: string,
  dialect: Dialect,
  version?: string
): ValidationResult;

/**
 * Exchange profile applied on top of the spec. "spec" is the specification
 * only. "google-ab" is Google Authorized Buyers OpenRTB: at=3 (FIXED_PRICE)
 * is a valid auction type, and each Imp must carry ext.billing_id.
 * "prebid-server" is Prebid Server /openrtb2/auction: each Imp must name a
 * bidder or stored request, and wseat/bseat are refused. "xandr" is Microsoft
 * Monetize outgoing requests: ext.appnexus.seller_member_id and
 * video.ext.appnexus.context. "magnite" is Magnite xAPI identity fields:
 * imp.ext.rp.zone_id, site/app ext.rp.site_id, publisher.ext.rp.account_id.
 */
export type Profile = "spec" | "google-ab" | "prebid-server" | "xandr" | "magnite";

/**
 * Validate an OpenRTB bid request against an exchange profile.
 * @param input Raw bid request JSON.
 * @param profile Exchange profile.
 * @param version OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateProfile(
  input: string,
  profile: Profile,
  version?: string
): ValidationResult;

/** Validate an OpenRTB bid response against an exchange profile. */
export function validateResponseProfile(
  input: string,
  profile: Profile,
  version?: string
): ValidationResult;

/** Every field the OpenRTB protobuf schema types differently from the spec. */
export function protoBoolDivergences(): ProtoBoolField[];

/**
 * Validate an ARTF RTBRequest envelope and the OpenRTB payloads it carries.
 * @param input Raw RTBRequest JSON.
 * @param version OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateArtfRequest(input: string, version?: string): ValidationResult;

/**
 * Validate an ARTF RTBResponse mutation set against the RTBRequest it answers:
 * intent eligibility, operation and payload coherence, and semantic path
 * resolution against the auction.
 * @param rtbResponse Raw RTBResponse JSON.
 * @param rtbRequest Raw RTBRequest envelope JSON.
 * @param version OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateArtfResponse(
  rtbResponse: string,
  rtbRequest: string,
  version?: string
): ValidationResult;

/**
 * Apply an ARTF mutation set and revalidate, reporting the mutation findings
 * plus the OpenRTB findings the mutations introduced.
 * @param rtbResponse Raw RTBResponse JSON.
 * @param rtbRequest Raw RTBRequest envelope JSON.
 * @param version OpenRTB version id (default: latest tracked 2.6 snapshot).
 */
export function validateArtfResponseApplied(
  rtbResponse: string,
  rtbRequest: string,
  version?: string
): ArtfMutationOutcome;

/** Every tracked OpenRTB version id. */
export function versions(): string[];

/** The full versioned rule catalog, one entry per coded rule. */
export function rules(): Rule[];

/** The rtblint-core version this build was compiled against. */
export function coreVersion(): string;
