mod adcom_lists;
mod canonical_catalog;
#[doc(hidden)]
pub mod catalog_extract;
mod schema_manifest;
mod validator;
mod version_rules;

use serde::Serialize;

pub use canonical_catalog::{
    canonical_field, canonical_object, canonical_object_catalog, canonical_object_catalog_versions,
    CanonicalField, CanonicalObject, CanonicalObjectCatalog, CatalogCitation, CatalogValueSet,
};
pub use schema_manifest::{
    schema_manifest, schema_manifest_versions, schema_path_entry, SchemaCoverage, SchemaManifest,
    SchemaPathEntry, SchemaPathState,
};
pub use version_rules::{
    path_status, rules_for_path, version_profile, version_profiles, OpenRtbFamily, OpenRtbVersion,
    PathRuleMatch, PathStateKind, PathStatus, VersionProfile, VersionRule, VersionRuleKind,
};

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
    validator::validate_bid_request(version, input)
}

/// Validates an OpenRTB bid response payload for a specific tracked version.
pub fn validate_bid_response_for_version(version: OpenRtbVersion, input: &str) -> ValidationResult {
    validator::validate_bid_response(version, input)
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
            section: canonical_object(version, object_name).map(|object| object.section.clone()),
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
                            "plcmt": 5
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
        assert!(has_issue(
            &result,
            "openrtb.field.undefined",
            "imp[0].video.plcmt"
        ));
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
                    "venuetype": "transit.airports",
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
            "openrtb.field.undefined",
            "app.content.realtime"
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
}
