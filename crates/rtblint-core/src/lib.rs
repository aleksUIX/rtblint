mod adcom_lists;
mod artf;
mod canonical_catalog;
#[doc(hidden)]
pub mod catalog_extract;
mod dialect;
mod pair;
mod schema_manifest;
mod validator;
mod version_rules;

use serde::Serialize;

pub use artf::{
    apply_artf_mutations, validate_artf_mutations_applied, validate_artf_request,
    validate_artf_response_against_request, ArtfApplication, ArtfMutationOutcome,
};
pub use canonical_catalog::{
    canonical_field, canonical_object, canonical_object_catalog, canonical_object_catalog_versions,
    CanonicalField, CanonicalObject, CanonicalObjectCatalog, CatalogCitation, CatalogValueSet,
    ExpectedShape, StaticCatalog, StaticCitation, StaticField, StaticObject, StaticValueSet,
};
pub use dialect::{proto_bool_fields, Dialect};
pub use schema_manifest::{
    schema_manifest, schema_manifest_versions, schema_path_entry, SchemaCoverage, SchemaManifest,
    SchemaPathEntry, SchemaPathState,
};
pub use version_rules::{
    path_status, rules_for_path, version_profile, version_profiles, OpenRtbFamily, OpenRtbVersion,
    PathRuleMatch, PathStateKind, PathStatus, VersionProfile, VersionRule, VersionRuleKind,
};

/// The values an AdCOM list allows, as the validator sees them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct AdcomListValues {
    /// Documented values, ascending.
    pub allowed_values: &'static [i64],
    /// Floor above which exchange-specific values are permitted, when the
    /// list defines one.
    pub minimum_inclusive: Option<i64>,
}

/// Looks up an AdCOM list by the name catalogs cite (for example
/// "List: Device Types"), so callers can resolve a field's `adcom_list`
/// reference to concrete values.
pub fn adcom_list_values(name: &str) -> Option<AdcomListValues> {
    adcom_lists::adcom_list_by_name(name).map(|list| AdcomListValues {
        allowed_values: list.allowed_values,
        minimum_inclusive: list.minimum_inclusive,
    })
}

/// Validates an OpenRTB 2.6 bid request payload.
///
/// This first implementation focuses on deterministic structural issues:
/// JSON parsing, required fields, unknown fields, basic type mismatches,
/// and versioned path status such as deprecated or moved fields.
pub fn validate(_input: &str) -> ValidationResult {
    validate_bid_request_for_version(OpenRtbVersion::V2_6_202606, _input)
}

/// Validates an OpenRTB bid request payload for a specific tracked version.
pub fn validate_bid_request_for_version(version: OpenRtbVersion, input: &str) -> ValidationResult {
    validator::validate_bid_request(version, Dialect::SpecJson, input)
}

/// Validates an OpenRTB bid response payload for a specific tracked version.
pub fn validate_bid_response_for_version(version: OpenRtbVersion, input: &str) -> ValidationResult {
    validator::validate_bid_response(version, Dialect::SpecJson, input)
}

/// Validates an OpenRTB bid request payload written in a specific JSON
/// dialect.
///
/// [`Dialect::SpecJson`] is what [`validate_bid_request_for_version`] uses and
/// what the spec describes. [`Dialect::ProtoJson`] is the protobuf JSON
/// mapping of the IAB OpenRTB protobuf schema, which gRPC bidstream
/// integrations (ARTF among them) speak: there, the flag fields the spec types
/// as integers are `bool`, so `"secure": true` is correct and `"secure": 1` is
/// the error.
pub fn validate_bid_request_with_dialect(
    version: OpenRtbVersion,
    dialect: Dialect,
    input: &str,
) -> ValidationResult {
    validator::validate_bid_request(version, dialect, input)
}

/// Validates an OpenRTB bid response payload written in a specific JSON
/// dialect. See [`validate_bid_request_with_dialect`].
pub fn validate_bid_response_with_dialect(
    version: OpenRtbVersion,
    dialect: Dialect,
    input: &str,
) -> ValidationResult {
    validator::validate_bid_response(version, dialect, input)
}

/// Validates an OpenRTB bid response payload against the bid request it
/// answers.
///
/// Runs the full single-payload response validation, then cross-checks the
/// response against the request: every `bid.impid` must reference a request
/// Imp, `bid.mtype` and sniffed `bid.adm` markup must match a media subtype
/// that Imp offers, `dealid` is checked against the Imp's pmp deals, the
/// response id must echo the request id, and seat and currency constraints
/// (`wseat`/`bseat`/`cur`) are enforced.
///
/// The request itself is not validated here; run
/// [`validate_bid_request_for_version`] on it separately. An unparseable
/// request is reported as `openrtb.pair.request_unusable`.
pub fn validate_bid_response_against_request(
    version: OpenRtbVersion,
    request_input: &str,
    response_input: &str,
) -> ValidationResult {
    pair::validate_bid_response_against_request(
        version,
        Dialect::SpecJson,
        request_input,
        response_input,
    )
}

/// Validates a bid response against its bid request, with both payloads
/// written in a specific JSON dialect. See
/// [`validate_bid_response_with_dialect`].
pub fn validate_bid_response_against_request_with_dialect(
    version: OpenRtbVersion,
    dialect: Dialect,
    request_input: &str,
    response_input: &str,
) -> ValidationResult {
    pair::validate_bid_response_against_request(version, dialect, request_input, response_input)
}

/// Validates whether an object field exists in the canonical catalog for a specific OpenRTB version.
pub fn validate_object_field(
    version: OpenRtbVersion,
    object_name: &str,
    field_name: &str,
) -> ValidationResult {
    if canonical_field(version, object_name, field_name).is_some() {
        return ValidationResult {
            valid: true,
            issues: vec![],
        };
    }

    let catalog_message = if canonical_object_catalog(version).is_some() {
        format!(
            "{}.{} is not defined in the canonical OpenRTB {} catalog.",
            object_name,
            field_name,
            version.id()
        )
    } else {
        format!(
            "No canonical object catalog is loaded for OpenRTB {} yet; cannot validate {}.{}.",
            version.id(),
            object_name,
            field_name
        )
    };

    ValidationResult {
        valid: false,
        issues: vec![Issue {
            id: String::from("openrtb.field.undefined"),
            severity: Severity::Error,
            message: catalog_message,
            path: Some(format!("{}.{}", object_name, field_name)),
            section: canonical_object(version, object_name)
                .map(|object| String::from(object.section)),
        }],
    }
}

/// Result of a validation run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ValidationResult {
    pub valid: bool,
    pub issues: Vec<Issue>,
}

/// Severity of a validation issue. Serializes as a lowercase string
/// ("error", "warning").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum Severity {
    Error,
    Warning,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A single validation issue.
///
/// `section` cites the OpenRTB spec section the finding derives from (for
/// example "3.2.7"), when the underlying catalog or version rule records one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct Issue {
    pub id: String,
    pub severity: Severity,
    pub message: String,
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub section: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_issue(result: &ValidationResult, id: &str, path: &str) -> bool {
        result
            .issues
            .iter()
            .any(|issue| issue.id == id && issue.path.as_deref() == Some(path))
    }

    /// The message reported at `path`, for assertions about wording that is
    /// load-bearing (a named arrival version is the fix, not decoration).
    fn issue_message(result: &ValidationResult, path: &str) -> String {
        result
            .issues
            .iter()
            .find(|issue| issue.path.as_deref() == Some(path))
            .map(|issue| issue.message.clone())
            .unwrap_or_else(|| panic!("no issue at {path}: {:?}", result.issues))
    }

    #[test]
    fn validate_accepts_minimal_video_bid_request() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"]
                        }
                    }
                ]
            }"#,
        );

        assert!(result.valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn validate_reports_invalid_json() {
        let result = validate("{");

        assert!(!result.valid);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].id, "openrtb.payload.invalid_json");
        assert_eq!(result.issues[0].severity, Severity::Error);
    }

    #[test]
    fn validate_reports_unknown_top_level_field() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"]
                        }
                    }
                ],
                "unexpected": true
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(&result, "openrtb.field.undefined", "unexpected"));
    }

    #[test]
    fn validate_warns_on_deprecated_video_placement() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "placement": 1
                        }
                    }
                ]
            }"#,
        );

        assert!(result.valid);
        assert!(has_issue(
            &result,
            "openrtb.field.deprecated",
            "imp[0].video.placement"
        ));
    }

    #[test]
    fn validate_reports_moved_regs_ext_gdpr() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"]
                        }
                    }
                ],
                "regs": {
                    "ext": {
                        "gdpr": 1
                    }
                }
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(&result, "openrtb.field.moved", "regs.ext.gdpr"));
    }

    #[test]
    fn validate_reports_missing_required_video_mimes() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {}
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.field.required",
            "imp[0].video.mimes"
        ));

        let issue = result
            .issues
            .iter()
            .find(|issue| issue.id == "openrtb.field.required")
            .expect("required issue should be present");
        assert_eq!(issue.severity, Severity::Error);
        assert_eq!(issue.section.as_deref(), Some("3.2.7"));
    }

    #[test]
    fn validate_reports_invalid_inline_enum_value() {
        let result = validate(
            r#"{
                "id": "request-1",
                "test": 2,
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"]
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(&result, "openrtb.value.invalid", "test"));
    }

    #[test]
    fn validate_accepts_exchange_specific_auction_type_range() {
        let result = validate(
            r#"{
                "id": "request-1",
                "at": 500,
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"]
                        }
                    }
                ]
            }"#,
        );

        assert!(result.valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn validate_reports_invalid_inline_enum_array_member() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "banner": {
                            "btype": [9]
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.value.invalid",
            "imp[0].banner.btype[0]"
        ));
    }

    #[test]
    fn validate_reports_mutually_exclusive_site_and_app() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"]
                        }
                    }
                ],
                "site": {
                    "id": "site-1"
                },
                "app": {
                    "bundle": "com.example.app"
                }
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.fields.mutually_exclusive",
            "site"
        ));
    }

    #[test]
    fn validate_reports_imp_without_media_type() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1"
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.imp.media_type.required",
            "imp[0]"
        ));
    }

    #[test]
    fn validate_reports_keywords_and_kwarray_as_mutually_exclusive() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"]
                        }
                    }
                ],
                "site": {
                    "id": "site-1",
                    "keywords": "news,sports",
                    "kwarray": ["news", "sports"]
                }
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.fields.mutually_exclusive",
            "site.keywords"
        ));
    }

    #[test]
    fn validate_reports_mutually_exclusive_video_duration_fields() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "minduration": 15,
                            "rqddurs": [15]
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.fields.mutually_exclusive",
            "imp[0].video.minduration"
        ));
    }

    #[test]
    fn validate_reports_skipmin_without_skippable_video() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "skipmin": 5
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.field.requires_skippable_video",
            "imp[0].video.skipmin"
        ));
    }

    #[test]
    fn validate_reports_invalid_adcom_api_framework_value() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "api": [10]
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.value.invalid",
            "imp[0].video.api[0]"
        ));
    }

    #[test]
    fn validate_accepts_vendor_specific_adcom_api_framework_value() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "api": [500]
                        }
                    }
                ]
            }"#,
        );

        assert!(result.valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn validate_reports_invalid_adcom_plcmt_value() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "plcmt": 10
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.value.invalid",
            "imp[0].video.plcmt"
        ));
    }

    /// AdCOM 1.0-202607 added the CTV Ad Portfolio enumerations. A pause ad
    /// carrying all four of them has to validate clean.
    #[test]
    fn validate_accepts_ctv_ad_portfolio_adcom_values() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "plcmt": 5,
                            "pos": 14,
                            "playbackmethod": [9],
                            "battr": [19, 21, 23],
                            "linearity": 2
                        }
                    }
                ]
            }"#,
        );

        assert!(result.valid, "unexpected issues: {:?}", result.issues);
    }

    #[test]
    fn validate_accepts_positive_startdelay_seconds() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "startdelay": 30
                        }
                    }
                ]
            }"#,
        );

        assert!(result.valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn validate_reports_invalid_delivery_method() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "delivery": [4]
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.value.invalid",
            "imp[0].video.delivery[0]"
        ));
    }

    #[test]
    fn validate_reports_invalid_qty_source_type() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"]
                        },
                        "qty": {
                            "multiplier": 14.2,
                            "sourcetype": 9
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.value.invalid",
            "imp[0].qty.sourcetype"
        ));
    }

    #[test]
    fn validate_reports_invalid_category_taxonomy() {
        let result = validate(
            r#"{
                "id": "request-1",
                "cattax": 10,
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"]
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(&result, "openrtb.value.invalid", "cattax"));
    }

    #[test]
    fn validate_2_5_rejects_video_plcmt() {
        let result = validate_bid_request_for_version(
            OpenRtbVersion::V2_5,
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "plcmt": 1
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        // plcmt exists, just not yet at 2.5. Saying "not available, arrives in
        // 2.6-202303" answers the version question; "undefined" only says the
        // catalog has never heard of it, which sends people hunting for a typo.
        assert!(has_issue(
            &result,
            "openrtb.field.not_yet_available",
            "imp[0].video.plcmt"
        ));
        let message = issue_message(&result, "imp[0].video.plcmt");
        assert!(
            message.contains("2.6-202303"),
            "the arrival version should be named: {message}"
        );
    }

    #[test]
    fn validate_latest_2_6_accepts_video_plcmt() {
        let result = validate_bid_request_for_version(
            OpenRtbVersion::V2_6_202505,
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "video": {
                            "mimes": ["video/mp4"],
                            "plcmt": 1
                        }
                    }
                ]
            }"#,
        );

        assert!(result.valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn object_field_validation_accepts_2_5_bidrequest_source() {
        let result = validate_object_field(OpenRtbVersion::V2_5, "BidRequest", "source");

        assert!(result.valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn object_field_validation_rejects_2_5_video_plcmt() {
        let result = validate_object_field(OpenRtbVersion::V2_5, "Video", "plcmt");

        assert!(!result.valid);
        assert_eq!(result.issues.len(), 1);
        assert_eq!(result.issues[0].id, "openrtb.field.undefined");
        assert_eq!(result.issues[0].path.as_deref(), Some("Video.plcmt"));
    }

    #[test]
    fn object_field_validation_accepts_latest_2_6_video_plcmt() {
        let result = validate_object_field(OpenRtbVersion::V2_6_202505, "Video", "plcmt");

        assert!(result.valid);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn validate_accepts_dooh_request_with_catalog_fields() {
        let result = validate(
            r#"{
                "id": "req-dooh-1",
                "dooh": {
                    "id": "screen-88",
                    "name": "Airport Arrivals Billboard",
                    "venuetype": ["transit.airports"],
                    "venuetypetax": 1,
                    "publisher": { "id": "pub-oh-4" },
                    "domain": "cityscreens.example",
                    "keywords": "billboard,airport"
                },
                "imp": [
                    {
                        "id": "1",
                        "banner": { "w": 1080, "h": 1920 },
                        "qty": { "multiplier": 42.0, "sourcetype": 1 }
                    }
                ]
            }"#,
        );

        assert!(result.valid, "issues: {:?}", result.issues);
        assert!(result.issues.is_empty());
    }

    #[test]
    fn validate_2_6_202606_accepts_content_liveness_fields() {
        let payload = r#"{
            "id": "req-live-1",
            "app": {
                "bundle": "com.example.tv",
                "content": { "id": "c1", "livestream": 1, "realtime": 1, "firstbroadcast": 1 }
            },
            "imp": [
                { "id": "1", "video": { "mimes": ["video/mp4"] } }
            ]
        }"#;

        let at_202606 = validate_bid_request_for_version(OpenRtbVersion::V2_6_202606, payload);
        let at_202505 = validate_bid_request_for_version(OpenRtbVersion::V2_6_202505, payload);

        assert!(at_202606.valid, "issues: {:?}", at_202606.issues);
        assert!(!at_202505.valid);
        assert!(has_issue(
            &at_202505,
            "openrtb.field.not_yet_available",
            "app.content.realtime"
        ));
        let message = issue_message(&at_202505, "app.content.realtime");
        assert!(
            message.contains("2.6-202606"),
            "the arrival version should be named: {message}"
        );
    }

    /// The version-rule lookup must not swallow ordinary typos.
    ///
    /// `not_yet_available` and `removed` are only correct for paths a version
    /// rule actually knows. An invented key matches nothing, stays Unknown, and
    /// has to keep reporting as undefined, or every misspelling turns into a
    /// misleading "arrives in a later version".
    #[test]
    fn unknown_field_names_still_report_as_undefined() {
        let result = validate(
            r#"{
                "id": "req-1",
                "site": { "id": "s1" },
                "imp": [{ "id": "1", "bidflor": 2.5, "banner": { "w": 300, "h": 250 } }]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.field.undefined",
            "imp[0].bidflor"
        ));
    }

    /// A field removed by a later revision reports as removed, not undefined.
    #[test]
    fn removed_fields_report_as_removed() {
        let payload = r#"{
            "id": "req-1",
            "site": { "id": "s1" },
            "imp": [{ "id": "1", "banner": { "w": 300, "h": 250, "wmax": 728 } }]
        }"#;

        let at_2_6 = validate_bid_request_for_version(OpenRtbVersion::V2_6_202606, payload);
        assert!(!at_2_6.valid);
        assert!(has_issue(
            &at_2_6,
            "openrtb.field.removed",
            "imp[0].banner.wmax"
        ));

        // Still only deprecated at 2.5, where it had not been removed yet.
        let at_2_5 = validate_bid_request_for_version(OpenRtbVersion::V2_5, payload);
        assert!(has_issue(
            &at_2_5,
            "openrtb.field.deprecated",
            "imp[0].banner.wmax"
        ));
    }

    #[test]
    fn validate_response_accepts_no_bid_reason_code() {
        let result = validate_bid_response_for_version(
            OpenRtbVersion::V2_6_202606,
            r#"{ "id": "req-1", "nbr": 8 }"#,
        );

        assert!(result.valid, "issues: {:?}", result.issues);
    }

    #[test]
    fn validate_response_reports_invalid_no_bid_reason_code() {
        let result = validate_bid_response_for_version(
            OpenRtbVersion::V2_6_202606,
            r#"{ "id": "req-1", "nbr": 99 }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(&result, "openrtb.value.invalid", "nbr"));
    }

    #[test]
    fn validate_skips_field_checks_for_legacy_objects_without_extracted_fields() {
        let result = validate_bid_request_for_version(
            OpenRtbVersion::V2_2,
            r#"{
                "id": "req-legacy-1",
                "imp": [
                    { "id": "1", "banner": { "w": 300, "h": 250 } }
                ],
                "site": { "id": "site-9", "domain": "news.example" }
            }"#,
        );

        assert!(
            !result
                .issues
                .iter()
                .any(|issue| issue.id == "openrtb.field.undefined"
                    && issue
                        .path
                        .as_deref()
                        .is_some_and(|p| p.starts_with("site."))),
            "legacy Site fields should not be flagged undefined: {:?}",
            result.issues
        );
    }

    #[test]
    fn validate_reports_double_encoded_native_request() {
        // native.request has been JSON.stringify'd twice: parsing it once
        // yields another JSON string, not the Native Markup Request object.
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "native": {
                            "request": "\"{\\\"ver\\\":\\\"1.2\\\"}\""
                        }
                    }
                ]
            }"#,
        );

        assert!(!result.valid);
        assert!(has_issue(
            &result,
            "openrtb.native.request.double_encoded",
            "imp[0].native.request"
        ));
    }

    #[test]
    fn validate_reports_native_request_legacy_wrapper() {
        // Pre-Native-1.1 convention: the request is wrapped in a root
        // object with a single "native" key instead of being the root
        // Native Markup Request object itself.
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "native": {
                            "request": "{\"native\":{\"ver\":\"1.2\"}}"
                        }
                    }
                ]
            }"#,
        );

        assert!(result.valid);
        assert!(has_issue(
            &result,
            "openrtb.native.request.legacy_wrapper",
            "imp[0].native.request"
        ));
    }

    #[test]
    fn validate_reports_unparseable_native_request() {
        let result = validate(
            r#"{
                "id": "request-1",
                "imp": [
                    {
                        "id": "imp-1",
                        "native": {
                            "request": "not json"
                        }
                    }
                ]
            }"#,
        );

        assert!(result.valid);
        assert!(has_issue(
            &result,
            "openrtb.native.request.unparseable",
            "imp[0].native.request"
        ));
    }
}
