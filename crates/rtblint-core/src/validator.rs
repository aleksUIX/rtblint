use std::fmt::Write as _;

use serde_json::{Map, Value};

use crate::{
    adcom_lists::adcom_list_by_name, canonical_object, path_status,
    version_rules::rule_path_leaves, ExpectedShape, Issue, OpenRtbVersion, PathStateKind, Severity,
    StaticField, ValidationResult,
};

/// The two OpenRTB 2.x payload types the validator understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PayloadKind {
    BidRequest,
    BidResponse,
}

impl PayloadKind {
    fn root_object(self) -> &'static str {
        match self {
            Self::BidRequest => "BidRequest",
            Self::BidResponse => "BidResponse",
        }
    }

    fn root_path_prefix(self) -> &'static str {
        match self {
            Self::BidRequest => "bidrequest",
            Self::BidResponse => "bidresponse",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::BidRequest => "bid request",
            Self::BidResponse => "bid response",
        }
    }
}

pub(crate) fn validate_bid_request(version: OpenRtbVersion, input: &str) -> ValidationResult {
    validate_payload(version, input, PayloadKind::BidRequest)
}

pub(crate) fn validate_bid_response(version: OpenRtbVersion, input: &str) -> ValidationResult {
    validate_payload(version, input, PayloadKind::BidResponse)
}

fn validate_payload(version: OpenRtbVersion, input: &str, kind: PayloadKind) -> ValidationResult {
    if canonical_object(version, kind.root_object()).is_none() {
        return ValidationResult {
            valid: false,
            issues: vec![Issue {
                id: String::from("openrtb.version.unsupported"),
                severity: Severity::Error,
                message: format!(
                    "OpenRTB {} has no canonical {} catalog in this build; {} validation is not supported for this version.",
                    version.id(),
                    kind.root_object(),
                    kind.label()
                ),
                path: None,
                section: None,
            }],
        };
    }

    let value = match serde_json::from_str::<Value>(input) {
        Ok(value) => value,
        Err(error) => {
            return ValidationResult {
                valid: false,
                issues: vec![Issue {
                    id: String::from("openrtb.payload.invalid_json"),
                    severity: Severity::Error,
                    message: format!("Invalid JSON payload: {error}"),
                    path: None,
                    section: None,
                }],
            };
        }
    };

    let Some(root) = value.as_object() else {
        return ValidationResult {
            valid: false,
            issues: vec![Issue {
                id: String::from("openrtb.payload.root_not_object"),
                severity: Severity::Error,
                message: format!(
                    "OpenRTB {}s must be JSON objects at the top level.",
                    kind.label()
                ),
                path: None,
                section: None,
            }],
        };
    };

    let mut issues = Vec::new();
    validate_known_object(
        version,
        kind,
        kind.root_object(),
        root,
        &mut Vec::new(),
        &mut String::new(),
        &mut issues,
    );

    ValidationResult {
        valid: !issues.iter().any(|issue| issue.severity == Severity::Error),
        issues,
    }
}

/// Walks a known catalog object. `instance_path` is a cursor mutated in
/// place (push a segment, recurse, truncate back); path strings are only
/// materialized when an issue is actually pushed, so a clean payload
/// allocates almost nothing here.
#[allow(clippy::too_many_arguments)]
fn validate_known_object<'a>(
    version: OpenRtbVersion,
    kind: PayloadKind,
    object_name: &str,
    object: &'a Map<String, Value>,
    logical_segments: &mut Vec<&'a str>,
    instance_path: &mut String,
    issues: &mut Vec<Issue>,
) {
    let Some(definition) = canonical_object(version, object_name) else {
        return;
    };

    // Objects whose field tables could not be extracted from the archived
    // spec (some legacy PDF snapshots) carry no field list; flagging every
    // payload field as undefined would be a false positive, so field-level
    // checks are skipped and only object semantics run.
    if definition.fields.is_empty() {
        validate_object_semantics(
            object_name,
            definition.section,
            object,
            instance_path,
            issues,
        );
        return;
    }

    for field in definition.fields.iter().filter(|field| field.required) {
        if !object.contains_key(field.name) {
            issues.push(Issue {
                id: String::from("openrtb.field.required"),
                severity: Severity::Error,
                message: format!(
                    "{} is required on OpenRTB {} {}.",
                    field.name,
                    version.id(),
                    object_name
                ),
                path: Some(join_instance_path(instance_path, field.name)),
                section: Some(String::from(field.citation.section)),
            });
        }
    }

    for (field_name, value) in object {
        logical_segments.push(field_name.as_str());
        let parent_path_length = push_path_segment(instance_path, field_name);

        if field_name == "ext" {
            validate_extension_value(
                version,
                kind,
                value,
                logical_segments,
                instance_path,
                issues,
            );
            instance_path.truncate(parent_path_length);
            logical_segments.pop();
            continue;
        }

        let Some(field_definition) = definition
            .fields
            .iter()
            .find(|field| field.name == field_name.as_str())
        else {
            issues.push(Issue {
                id: String::from("openrtb.field.undefined"),
                severity: Severity::Error,
                message: format!(
                    "{}.{} is not defined in the canonical OpenRTB {} catalog.",
                    object_name,
                    field_name,
                    version.id()
                ),
                path: Some(instance_path.clone()),
                section: Some(String::from(definition.section)),
            });
            instance_path.truncate(parent_path_length);
            logical_segments.pop();
            continue;
        };

        // Rule matching is only worth running when this field name can match
        // a rule path at all, or the catalog itself marks it deprecated.
        if field_definition.deprecated || rule_path_leaves().contains(field_name.as_str()) {
            push_path_status_issues(
                version,
                kind,
                logical_segments,
                instance_path,
                field_definition.deprecated,
                Some(field_definition.citation.section),
                issues,
            );
        }
        validate_field_value_shape(field_definition, value, instance_path, issues);
        validate_required_array_contents(field_definition, value, instance_path, issues);
        validate_catalog_value_set(field_definition, value, instance_path, issues);

        if matches!(field_definition.shape, ExpectedShape::Object) && value.is_object() {
            if let Some(child_object_name) = field_definition.child_object {
                validate_known_object(
                    version,
                    kind,
                    child_object_name,
                    value.as_object().expect("checked object shape"),
                    logical_segments,
                    instance_path,
                    issues,
                );
            }
        }

        if matches!(field_definition.shape, ExpectedShape::ObjectArray) && value.is_array() {
            if let Some(child_object_name) = field_definition.child_object {
                for (index, item) in value
                    .as_array()
                    .expect("checked array shape")
                    .iter()
                    .enumerate()
                {
                    if let Some(item_object) = item.as_object() {
                        let item_path_length = push_index_segment(instance_path, index);
                        validate_known_object(
                            version,
                            kind,
                            child_object_name,
                            item_object,
                            logical_segments,
                            instance_path,
                            issues,
                        );
                        instance_path.truncate(item_path_length);
                    }
                }
            }
        }

        instance_path.truncate(parent_path_length);
        logical_segments.pop();
    }

    validate_object_semantics(
        object_name,
        definition.section,
        object,
        instance_path,
        issues,
    );
}

/// Appends `.segment` (or just `segment` at the root) to the path cursor
/// and returns the length to truncate back to afterwards.
fn push_path_segment(path: &mut String, segment: &str) -> usize {
    let previous_length = path.len();
    if !path.is_empty() {
        path.push('.');
    }
    path.push_str(segment);
    previous_length
}

/// Appends `[index]` to the path cursor and returns the length to truncate
/// back to afterwards.
fn push_index_segment(path: &mut String, index: usize) -> usize {
    let previous_length = path.len();
    write!(path, "[{index}]").expect("writing to a String cannot fail");
    previous_length
}

fn validate_extension_value<'a>(
    version: OpenRtbVersion,
    kind: PayloadKind,
    value: &'a Value,
    logical_segments: &mut Vec<&'a str>,
    instance_path: &mut String,
    issues: &mut Vec<Issue>,
) {
    match value {
        Value::Object(map) => {
            for (field_name, child) in map {
                logical_segments.push(field_name.as_str());
                let parent_path_length = push_path_segment(instance_path, field_name);
                if rule_path_leaves().contains(field_name.as_str()) {
                    push_path_status_issues(
                        version,
                        kind,
                        logical_segments,
                        instance_path,
                        false,
                        None,
                        issues,
                    );
                }
                validate_extension_value(
                    version,
                    kind,
                    child,
                    logical_segments,
                    instance_path,
                    issues,
                );
                instance_path.truncate(parent_path_length);
                logical_segments.pop();
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let parent_path_length = push_index_segment(instance_path, index);
                validate_extension_value(
                    version,
                    kind,
                    item,
                    logical_segments,
                    instance_path,
                    issues,
                );
                instance_path.truncate(parent_path_length);
            }
        }
        _ => {}
    }
}

fn validate_required_array_contents(
    field: &StaticField,
    value: &Value,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    if field.required
        && matches!(
            field.shape,
            ExpectedShape::ObjectArray
                | ExpectedShape::StringArray
                | ExpectedShape::IntegerArray
                | ExpectedShape::FloatArray
                | ExpectedShape::AnyArray
        )
    {
        if let Some(values) = value.as_array() {
            if values.is_empty() {
                issues.push(Issue {
                    id: String::from("openrtb.field.required"),
                    severity: Severity::Error,
                    message: format!("{} must not be an empty array.", instance_path),
                    path: Some(String::from(instance_path)),
                    section: Some(String::from(field.citation.section)),
                });
            }
        }
    }
}

fn validate_catalog_value_set(
    field: &StaticField,
    value: &Value,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(value_set) = field_value_set(field) else {
        return;
    };
    let section = field.citation.section;

    match value {
        Value::Number(_) => {
            if let Some(integer) = integer_value(value) {
                validate_integer_against_value_set(
                    &value_set,
                    integer,
                    instance_path,
                    section,
                    issues,
                );
            }
        }
        Value::Array(values) => {
            for (index, item) in values.iter().enumerate() {
                if let Some(integer) = integer_value(item) {
                    validate_integer_against_value_set(
                        &value_set,
                        integer,
                        &format!("{}[{}]", instance_path, index),
                        section,
                        issues,
                    );
                }
            }
        }
        _ => {}
    }
}

fn field_value_set(field: &StaticField) -> Option<IntegerValueSet> {
    if let Some(list_name) = field.adcom_list {
        let list = adcom_list_by_name(list_name)?;
        return Some(IntegerValueSet {
            source: Some(list.name),
            allowed_values: list.allowed_values,
            minimum_inclusive: list.minimum_inclusive,
        });
    }

    field.value_set.map(|value_set| IntegerValueSet {
        source: None,
        allowed_values: value_set.values,
        minimum_inclusive: value_set.minimum_inclusive,
    })
}

fn validate_integer_against_value_set(
    value_set: &IntegerValueSet,
    integer: i64,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    if value_set.contains(integer) {
        return;
    }

    let message = if let Some(source) = value_set.source {
        format!(
            "{} has unsupported value {}. Allowed values from {} are {}.",
            instance_path,
            integer,
            source,
            value_set.render()
        )
    } else {
        format!(
            "{} has unsupported value {}. Allowed values from the extracted spec are {}.",
            instance_path,
            integer,
            value_set.render()
        )
    };

    issues.push(Issue {
        id: String::from("openrtb.value.invalid"),
        severity: Severity::Error,
        message,
        path: Some(String::from(instance_path)),
        section: Some(String::from(section)),
    });
}

fn validate_object_semantics(
    object_name: &str,
    object_section: &str,
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    validate_generic_exclusive_pairs(object, instance_path, object_section, issues);

    match object_name {
        "BidRequest" => {
            validate_bid_request_semantics(object, instance_path, object_section, issues)
        }
        "BidResponse" => {
            validate_bid_response_semantics(object, instance_path, object_section, issues)
        }
        "Imp" => validate_imp_semantics(object, instance_path, object_section, issues),
        "Video" => validate_video_semantics(object, instance_path, object_section, issues),
        "Audio" => validate_audio_semantics(object, instance_path, object_section, issues),
        _ => {}
    }
}

fn validate_generic_exclusive_pairs(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    for (left, right) in [
        ("wseat", "bseat"),
        ("wlang", "wlangb"),
        ("acat", "bcat"),
        ("keywords", "kwarray"),
        ("language", "langb"),
    ] {
        if object.contains_key(left) && object.contains_key(right) {
            push_mutually_exclusive_issue(left, right, instance_path, section, issues);
        }
    }
}

fn validate_bid_request_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    let media_contexts = ["site", "app", "dooh"]
        .into_iter()
        .filter(|field| object.contains_key(*field))
        .collect::<Vec<_>>();

    if media_contexts.len() > 1 {
        issues.push(Issue {
            id: String::from("openrtb.fields.mutually_exclusive"),
            severity: Severity::Error,
            message: String::from(
                "Only one of site, app, or dooh may be present on the same bid request.",
            ),
            path: Some(join_instance_path(instance_path, media_contexts[0])),
            section: Some(String::from(section)),
        });
    }
}

fn validate_bid_response_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    let has_seatbid = object
        .get("seatbid")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());

    if !has_seatbid && !object.contains_key("nbr") {
        issues.push(Issue {
            id: String::from("openrtb.response.seatbid_or_nbr.required"),
            severity: Severity::Error,
            message: String::from(
                "A bid response must contain at least one seatbid, or a no-bid reason code (nbr).",
            ),
            path: Some(if instance_path.is_empty() {
                String::from("seatbid")
            } else {
                join_instance_path(instance_path, "seatbid")
            }),
            section: Some(String::from(section)),
        });
    }
}

fn validate_imp_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    let has_media_type = ["banner", "video", "audio", "native"]
        .into_iter()
        .any(|field| object.contains_key(field));

    if !has_media_type {
        issues.push(Issue {
            id: String::from("openrtb.imp.media_type.required"),
            severity: Severity::Error,
            message: String::from(
                "Each Imp object must offer at least one media subtype such as banner, video, audio, or native.",
            ),
            path: Some(String::from(instance_path)),
            section: Some(String::from(section)),
        });
    }
}

fn validate_video_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    validate_duration_semantics(object, instance_path, section, issues);

    let skippable = object.get("skip").and_then(integer_value) == Some(1);
    for dependent_field in ["skipmin", "skipafter"] {
        if object.contains_key(dependent_field) && !skippable {
            issues.push(Issue {
                id: String::from("openrtb.field.requires_skippable_video"),
                severity: Severity::Error,
                message: format!(
                    "{} may only be present when video.skip is set to 1.",
                    join_instance_path(instance_path, dependent_field)
                ),
                path: Some(join_instance_path(instance_path, dependent_field)),
                section: Some(String::from(section)),
            });
        }
    }
}

fn validate_audio_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    validate_duration_semantics(object, instance_path, section, issues);
}

fn validate_duration_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    if object.contains_key("rqddurs") && object.contains_key("minduration") {
        push_mutually_exclusive_issue("minduration", "rqddurs", instance_path, section, issues);
    }

    if object.contains_key("rqddurs") && object.contains_key("maxduration") {
        push_mutually_exclusive_issue("maxduration", "rqddurs", instance_path, section, issues);
    }
}

fn push_mutually_exclusive_issue(
    left: &str,
    right: &str,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    issues.push(Issue {
        id: String::from("openrtb.fields.mutually_exclusive"),
        severity: Severity::Error,
        message: format!(
            "{} and {} are mutually exclusive and must not both be present.",
            join_instance_path(instance_path, left),
            join_instance_path(instance_path, right)
        ),
        path: Some(join_instance_path(instance_path, left)),
        section: Some(String::from(section)),
    });
}

fn integer_value(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_u64()
            .and_then(|integer| i64::try_from(integer).ok())
    })
}

/// Borrows the static sorted value slices directly; `contains` is a binary
/// search, so no per-check set construction happens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct IntegerValueSet {
    source: Option<&'static str>,
    allowed_values: &'static [i64],
    minimum_inclusive: Option<i64>,
}

impl IntegerValueSet {
    fn contains(&self, value: i64) -> bool {
        self.allowed_values.binary_search(&value).is_ok()
            || self
                .minimum_inclusive
                .is_some_and(|minimum_inclusive| value >= minimum_inclusive)
    }

    fn render(&self) -> String {
        let mut values = self
            .allowed_values
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if let Some(minimum_inclusive) = self.minimum_inclusive {
            values.push(format!(">={minimum_inclusive}"));
        }

        match values.as_slice() {
            [] => String::from("no values"),
            [single] => single.clone(),
            [left, right] => format!("{left} or {right}"),
            _ => {
                let last = values.pop().expect("checked non-empty values");
                format!("{} or {last}", values.join(", "))
            }
        }
    }
}

fn validate_field_value_shape(
    field: &StaticField,
    value: &Value,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let valid = match field.shape {
        ExpectedShape::Unknown => true,
        ExpectedShape::Object => value.is_object(),
        ExpectedShape::ObjectArray => value
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_object)),
        ExpectedShape::String => value.is_string(),
        ExpectedShape::StringArray => value
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_string)),
        ExpectedShape::Integer => value.is_i64() || value.is_u64(),
        ExpectedShape::IntegerArray => value
            .as_array()
            .is_some_and(|items| items.iter().all(|item| item.is_i64() || item.is_u64())),
        ExpectedShape::Float => value.is_number(),
        ExpectedShape::FloatArray => value
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_number)),
        ExpectedShape::Boolean => value.is_boolean(),
        ExpectedShape::BooleanArray => value
            .as_array()
            .is_some_and(|items| items.iter().all(Value::is_boolean)),
        ExpectedShape::AnyArray => value.is_array(),
    };

    if !valid {
        issues.push(Issue {
            id: String::from("openrtb.type.mismatch"),
            severity: Severity::Error,
            message: format!(
                "{} expects {} but received {}.",
                instance_path,
                field.shape.label(),
                json_type_label(value)
            ),
            path: Some(String::from(instance_path)),
            section: Some(String::from(field.citation.section)),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn push_path_status_issues(
    version: OpenRtbVersion,
    kind: PayloadKind,
    logical_segments: &[&str],
    instance_path: &str,
    deprecated_in_catalog: bool,
    catalog_section: Option<&str>,
    issues: &mut Vec<Issue>,
) {
    let Some(schema_path) = schema_path(kind, logical_segments) else {
        return;
    };

    let status = path_status(version, &schema_path);
    // Prefer the section recorded on the matching version rule; fall back to
    // the catalog citation of the field itself.
    let rule_section = status
        .matched_rules
        .first()
        .map(|matched| String::from(matched.rule.section))
        .or_else(|| catalog_section.map(String::from));

    match status.kind {
        PathStateKind::Deprecated => issues.push(Issue {
            id: String::from("openrtb.field.deprecated"),
            severity: Severity::Warning,
            message: format!("{} is deprecated in OpenRTB {}.", schema_path, version.id()),
            path: Some(String::from(instance_path)),
            section: rule_section,
        }),
        PathStateKind::Removed => issues.push(Issue {
            id: String::from("openrtb.field.removed"),
            severity: Severity::Error,
            message: format!(
                "{} was removed before OpenRTB {}.",
                schema_path,
                version.id()
            ),
            path: Some(String::from(instance_path)),
            section: rule_section,
        }),
        PathStateKind::Moved => {
            let replacements = if status.replacement_paths.is_empty() {
                String::from("no replacement path is recorded")
            } else {
                status.replacement_paths.join(", ")
            };
            issues.push(Issue {
                id: String::from("openrtb.field.moved"),
                severity: Severity::Error,
                message: format!(
                    "{} moved in OpenRTB {}; use {}.",
                    schema_path,
                    version.id(),
                    replacements
                ),
                path: Some(String::from(instance_path)),
                section: rule_section,
            });
        }
        PathStateKind::NotYetAvailable => issues.push(Issue {
            id: String::from("openrtb.field.not_yet_available"),
            severity: Severity::Error,
            message: format!(
                "{} is not available in OpenRTB {}.",
                schema_path,
                version.id()
            ),
            path: Some(String::from(instance_path)),
            section: rule_section,
        }),
        PathStateKind::Available | PathStateKind::Unknown => {
            if deprecated_in_catalog {
                issues.push(Issue {
                    id: String::from("openrtb.field.deprecated"),
                    severity: Severity::Warning,
                    message: format!(
                        "{} is marked deprecated in the canonical catalog.",
                        schema_path
                    ),
                    path: Some(String::from(instance_path)),
                    section: catalog_section.map(String::from),
                });
            }
        }
    }
}

fn schema_path(kind: PayloadKind, logical_segments: &[&str]) -> Option<String> {
    if logical_segments.is_empty() {
        return None;
    }

    if logical_segments.len() == 1 {
        return Some(format!(
            "{}.{}",
            kind.root_path_prefix(),
            logical_segments[0]
        ));
    }

    Some(logical_segments.join("."))
}

fn join_instance_path(base: &str, segment: &str) -> String {
    if base.is_empty() {
        return String::from(segment);
    }

    format!("{base}.{segment}")
}

fn json_type_label(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(number) if number.is_i64() || number.is_u64() => "integer",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}
