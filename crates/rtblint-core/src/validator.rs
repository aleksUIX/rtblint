use std::collections::BTreeSet;

use serde_json::{Map, Value};

use crate::{
    adcom_lists::adcom_list_by_name, canonical_object, path_status, CanonicalField, Issue,
    OpenRtbVersion, PathStateKind, ValidationResult,
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
                severity: String::from("error"),
                message: format!(
                    "OpenRTB {} has no canonical {} catalog in this build; {} validation is not supported for this version.",
                    version.id(),
                    kind.root_object(),
                    kind.label()
                ),
                path: None,
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
                    severity: String::from("error"),
                    message: format!("Invalid JSON payload: {error}"),
                    path: None,
                }],
            };
        }
    };

    let Some(root) = value.as_object() else {
        return ValidationResult {
            valid: false,
            issues: vec![Issue {
                id: String::from("openrtb.payload.root_not_object"),
                severity: String::from("error"),
                message: format!(
                    "OpenRTB {}s must be JSON objects at the top level.",
                    kind.label()
                ),
                path: None,
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
        String::new(),
        &mut issues,
    );

    ValidationResult {
        valid: !issues.iter().any(|issue| issue.severity == "error"),
        issues,
    }
}

#[allow(clippy::too_many_arguments)]
fn validate_known_object(
    version: OpenRtbVersion,
    kind: PayloadKind,
    object_name: &str,
    object: &Map<String, Value>,
    logical_segments: &mut Vec<String>,
    instance_path: String,
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
        validate_object_semantics(object_name, object, &instance_path, issues);
        return;
    }

    for field in definition
        .fields
        .iter()
        .filter(|field| is_required(field.type_spec.as_str()))
    {
        if !object.contains_key(&field.name) {
            issues.push(Issue {
                id: String::from("openrtb.field.required"),
                severity: String::from("error"),
                message: format!(
                    "{} is required on OpenRTB {} {}.",
                    field.name,
                    version.id(),
                    object_name
                ),
                path: Some(join_instance_path(&instance_path, &field.name)),
            });
        }
    }

    for (field_name, value) in object {
        logical_segments.push(field_name.clone());
        let field_instance_path = join_instance_path(&instance_path, field_name);

        if field_name == "ext" {
            validate_extension_value(
                version,
                kind,
                value,
                logical_segments,
                field_instance_path,
                issues,
            );
            logical_segments.pop();
            continue;
        }

        let Some(field_definition) = definition
            .fields
            .iter()
            .find(|field| field.name == *field_name)
        else {
            issues.push(Issue {
                id: String::from("openrtb.field.undefined"),
                severity: String::from("error"),
                message: format!(
                    "{}.{} is not defined in the canonical OpenRTB {} catalog.",
                    object_name,
                    field_name,
                    version.id()
                ),
                path: Some(field_instance_path),
            });
            logical_segments.pop();
            continue;
        };

        push_path_status_issues(
            version,
            kind,
            logical_segments,
            &field_instance_path,
            field_definition.type_spec.as_str(),
            issues,
        );
        validate_field_value_shape(
            field_definition.type_spec.as_str(),
            value,
            &field_instance_path,
            issues,
        );
        validate_required_array_contents(
            field_definition.type_spec.as_str(),
            value,
            &field_instance_path,
            issues,
        );
        validate_catalog_value_set(field_definition, value, &field_instance_path, issues);

        if matches!(
            expected_shape(field_definition.type_spec.as_str()),
            ExpectedShape::Object
        ) && value.is_object()
        {
            if let Some(child_object_name) = field_definition.child_object.as_deref() {
                validate_known_object(
                    version,
                    kind,
                    child_object_name,
                    value.as_object().expect("checked object shape"),
                    logical_segments,
                    field_instance_path.clone(),
                    issues,
                );
            }
        }

        if matches!(
            expected_shape(field_definition.type_spec.as_str()),
            ExpectedShape::ObjectArray
        ) && value.is_array()
        {
            if let Some(child_object_name) = field_definition.child_object.as_deref() {
                for (index, item) in value
                    .as_array()
                    .expect("checked array shape")
                    .iter()
                    .enumerate()
                {
                    if let Some(item_object) = item.as_object() {
                        validate_known_object(
                            version,
                            kind,
                            child_object_name,
                            item_object,
                            logical_segments,
                            format!("{}[{}]", field_instance_path, index),
                            issues,
                        );
                    }
                }
            }
        }

        logical_segments.pop();
    }

    validate_object_semantics(object_name, object, &instance_path, issues);
}

fn validate_extension_value(
    version: OpenRtbVersion,
    kind: PayloadKind,
    value: &Value,
    logical_segments: &mut Vec<String>,
    instance_path: String,
    issues: &mut Vec<Issue>,
) {
    match value {
        Value::Object(map) => {
            for (field_name, child) in map {
                logical_segments.push(field_name.clone());
                let child_instance_path = join_instance_path(&instance_path, field_name);
                push_path_status_issues(
                    version,
                    kind,
                    logical_segments,
                    &child_instance_path,
                    "",
                    issues,
                );
                validate_extension_value(
                    version,
                    kind,
                    child,
                    logical_segments,
                    child_instance_path,
                    issues,
                );
                logical_segments.pop();
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                validate_extension_value(
                    version,
                    kind,
                    item,
                    logical_segments,
                    format!("{}[{}]", instance_path, index),
                    issues,
                );
            }
        }
        _ => {}
    }
}

fn validate_required_array_contents(
    type_spec: &str,
    value: &Value,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    if is_required(type_spec)
        && matches!(
            expected_shape(type_spec),
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
                    severity: String::from("error"),
                    message: format!("{} must not be an empty array.", instance_path),
                    path: Some(String::from(instance_path)),
                });
            }
        }
    }
}

fn validate_catalog_value_set(
    field: &CanonicalField,
    value: &Value,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(value_set) = field_value_set(field) else {
        return;
    };

    match value {
        Value::Number(_) => {
            if let Some(integer) = integer_value(value) {
                validate_integer_against_value_set(&value_set, integer, instance_path, issues);
            }
        }
        Value::Array(values) => {
            for (index, item) in values.iter().enumerate() {
                if let Some(integer) = integer_value(item) {
                    validate_integer_against_value_set(
                        &value_set,
                        integer,
                        &format!("{}[{}]", instance_path, index),
                        issues,
                    );
                }
            }
        }
        _ => {}
    }
}

fn field_value_set(field: &CanonicalField) -> Option<IntegerValueSet> {
    if let Some(list_name) = field.adcom_list.as_deref() {
        let list = adcom_list_by_name(list_name)?;
        return Some(IntegerValueSet {
            source: Some(list.name),
            allowed_values: list.allowed_values.iter().copied().collect(),
            minimum_inclusive: list.minimum_inclusive,
        });
    }

    field.value_set.as_ref().map(|value_set| IntegerValueSet {
        source: None,
        allowed_values: value_set.values.iter().copied().collect(),
        minimum_inclusive: value_set.minimum_inclusive,
    })
}

fn validate_integer_against_value_set(
    value_set: &IntegerValueSet,
    integer: i64,
    instance_path: &str,
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
        severity: String::from("error"),
        message,
        path: Some(String::from(instance_path)),
    });
}

fn validate_object_semantics(
    object_name: &str,
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    validate_generic_exclusive_pairs(object, instance_path, issues);

    match object_name {
        "BidRequest" => validate_bid_request_semantics(object, instance_path, issues),
        "BidResponse" => validate_bid_response_semantics(object, instance_path, issues),
        "Imp" => validate_imp_semantics(object, instance_path, issues),
        "Video" => validate_video_semantics(object, instance_path, issues),
        "Audio" => validate_audio_semantics(object, instance_path, issues),
        _ => {}
    }
}

fn validate_generic_exclusive_pairs(
    object: &Map<String, Value>,
    instance_path: &str,
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
            push_mutually_exclusive_issue(left, right, instance_path, issues);
        }
    }
}

fn validate_bid_request_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let media_contexts = ["site", "app", "dooh"]
        .into_iter()
        .filter(|field| object.contains_key(*field))
        .collect::<Vec<_>>();

    if media_contexts.len() > 1 {
        issues.push(Issue {
            id: String::from("openrtb.fields.mutually_exclusive"),
            severity: String::from("error"),
            message: String::from(
                "Only one of site, app, or dooh may be present on the same bid request.",
            ),
            path: Some(join_instance_path(instance_path, media_contexts[0])),
        });
    }
}

fn validate_bid_response_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let has_seatbid = object
        .get("seatbid")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());

    if !has_seatbid && !object.contains_key("nbr") {
        issues.push(Issue {
            id: String::from("openrtb.response.seatbid_or_nbr.required"),
            severity: String::from("error"),
            message: String::from(
                "A bid response must contain at least one seatbid, or a no-bid reason code (nbr).",
            ),
            path: Some(if instance_path.is_empty() {
                String::from("seatbid")
            } else {
                join_instance_path(instance_path, "seatbid")
            }),
        });
    }
}

fn validate_imp_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let has_media_type = ["banner", "video", "audio", "native"]
        .into_iter()
        .any(|field| object.contains_key(field));

    if !has_media_type {
        issues.push(Issue {
            id: String::from("openrtb.imp.media_type.required"),
            severity: String::from("error"),
            message: String::from(
                "Each Imp object must offer at least one media subtype such as banner, video, audio, or native.",
            ),
            path: Some(String::from(instance_path)),
        });
    }
}

fn validate_video_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    validate_duration_semantics(object, instance_path, issues);

    let skippable = object.get("skip").and_then(integer_value) == Some(1);
    for dependent_field in ["skipmin", "skipafter"] {
        if object.contains_key(dependent_field) && !skippable {
            issues.push(Issue {
                id: String::from("openrtb.field.requires_skippable_video"),
                severity: String::from("error"),
                message: format!(
                    "{} may only be present when video.skip is set to 1.",
                    join_instance_path(instance_path, dependent_field)
                ),
                path: Some(join_instance_path(instance_path, dependent_field)),
            });
        }
    }
}

fn validate_audio_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    validate_duration_semantics(object, instance_path, issues);
}

fn validate_duration_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    if object.contains_key("rqddurs") && object.contains_key("minduration") {
        push_mutually_exclusive_issue("minduration", "rqddurs", instance_path, issues);
    }

    if object.contains_key("rqddurs") && object.contains_key("maxduration") {
        push_mutually_exclusive_issue("maxduration", "rqddurs", instance_path, issues);
    }
}

fn push_mutually_exclusive_issue(
    left: &str,
    right: &str,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    issues.push(Issue {
        id: String::from("openrtb.fields.mutually_exclusive"),
        severity: String::from("error"),
        message: format!(
            "{} and {} are mutually exclusive and must not both be present.",
            join_instance_path(instance_path, left),
            join_instance_path(instance_path, right)
        ),
        path: Some(join_instance_path(instance_path, left)),
    });
}

fn integer_value(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_u64()
            .and_then(|integer| i64::try_from(integer).ok())
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct IntegerValueSet {
    source: Option<&'static str>,
    allowed_values: BTreeSet<i64>,
    minimum_inclusive: Option<i64>,
}

impl IntegerValueSet {
    fn contains(&self, value: i64) -> bool {
        self.allowed_values.contains(&value)
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
    type_spec: &str,
    value: &Value,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let valid = match expected_shape(type_spec) {
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
            severity: String::from("error"),
            message: format!(
                "{} expects {} but received {}.",
                instance_path,
                expected_shape(type_spec).label(),
                json_type_label(value)
            ),
            path: Some(String::from(instance_path)),
        });
    }
}

fn push_path_status_issues(
    version: OpenRtbVersion,
    kind: PayloadKind,
    logical_segments: &[String],
    instance_path: &str,
    type_spec: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(schema_path) = schema_path(kind, logical_segments) else {
        return;
    };

    let status = path_status(version, &schema_path);
    match status.kind {
        PathStateKind::Deprecated => issues.push(Issue {
            id: String::from("openrtb.field.deprecated"),
            severity: String::from("warning"),
            message: format!("{} is deprecated in OpenRTB {}.", schema_path, version.id()),
            path: Some(String::from(instance_path)),
        }),
        PathStateKind::Removed => issues.push(Issue {
            id: String::from("openrtb.field.removed"),
            severity: String::from("error"),
            message: format!(
                "{} was removed before OpenRTB {}.",
                schema_path,
                version.id()
            ),
            path: Some(String::from(instance_path)),
        }),
        PathStateKind::Moved => {
            let replacements = if status.replacement_paths.is_empty() {
                String::from("no replacement path is recorded")
            } else {
                status.replacement_paths.join(", ")
            };
            issues.push(Issue {
                id: String::from("openrtb.field.moved"),
                severity: String::from("error"),
                message: format!(
                    "{} moved in OpenRTB {}; use {}.",
                    schema_path,
                    version.id(),
                    replacements
                ),
                path: Some(String::from(instance_path)),
            });
        }
        PathStateKind::NotYetAvailable => issues.push(Issue {
            id: String::from("openrtb.field.not_yet_available"),
            severity: String::from("error"),
            message: format!(
                "{} is not available in OpenRTB {}.",
                schema_path,
                version.id()
            ),
            path: Some(String::from(instance_path)),
        }),
        PathStateKind::Available | PathStateKind::Unknown => {
            if type_spec.to_ascii_lowercase().contains("deprecated") {
                issues.push(Issue {
                    id: String::from("openrtb.field.deprecated"),
                    severity: String::from("warning"),
                    message: format!(
                        "{} is marked deprecated in the canonical catalog.",
                        schema_path
                    ),
                    path: Some(String::from(instance_path)),
                });
            }
        }
    }
}

fn schema_path(kind: PayloadKind, logical_segments: &[String]) -> Option<String> {
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

/// A field is unconditionally required only when the type column says so as
/// its own segment ("string; required", "scope: required; type: ...").
/// Conditional phrasings bleeding in from the spec's prose ("required for
/// Flex Ads", "required if sourcetype is present") must not count.
fn is_required(type_spec: &str) -> bool {
    type_spec
        .to_ascii_lowercase()
        .split(';')
        .map(str::trim)
        .any(|segment| {
            segment == "required" || segment == "required *" || segment == "scope: required"
        })
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpectedShape {
    Unknown,
    Object,
    ObjectArray,
    String,
    StringArray,
    Integer,
    IntegerArray,
    Float,
    FloatArray,
    Boolean,
    BooleanArray,
    AnyArray,
}

impl ExpectedShape {
    fn label(self) -> &'static str {
        match self {
            Self::Unknown => "a supported type",
            Self::Object => "object",
            Self::ObjectArray => "array of objects",
            Self::String => "string",
            Self::StringArray => "array of strings",
            Self::Integer => "integer",
            Self::IntegerArray => "array of integers",
            Self::Float => "number",
            Self::FloatArray => "array of numbers",
            Self::Boolean => "boolean",
            Self::BooleanArray => "array of booleans",
            Self::AnyArray => "array",
        }
    }
}

fn expected_shape(type_spec: &str) -> ExpectedShape {
    let normalized = type_spec.to_ascii_lowercase();

    if normalized.contains("object array") {
        return ExpectedShape::ObjectArray;
    }

    if normalized.contains("string array") {
        return ExpectedShape::StringArray;
    }

    if normalized.contains("integer array") {
        return ExpectedShape::IntegerArray;
    }

    if normalized.contains("float array") {
        return ExpectedShape::FloatArray;
    }

    if normalized.contains("boolean array") {
        return ExpectedShape::BooleanArray;
    }

    if normalized.contains("enum array") {
        return ExpectedShape::AnyArray;
    }

    if normalized.contains("object") {
        return ExpectedShape::Object;
    }

    if normalized.contains("string") {
        return ExpectedShape::String;
    }

    if normalized.contains("integer") {
        return ExpectedShape::Integer;
    }

    if normalized.contains("float") {
        return ExpectedShape::Float;
    }

    if normalized.contains("boolean") {
        return ExpectedShape::Boolean;
    }

    ExpectedShape::Unknown
}
