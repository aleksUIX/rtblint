use std::fmt::Write as _;

use serde_json::{Map, Value};

use crate::{
    adcom_lists::adcom_list_by_name,
    canonical_field, canonical_object,
    dialect::{proto_declares_bool, snake_case_of_camel},
    path_status,
    version_rules::rule_path_leaves,
    Dialect, ExpectedShape, Issue, OpenRtbVersion, PathStateKind, Severity, StaticField,
    ValidationResult,
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

    /// The member this payload occupies inside a 3.0 envelope.
    fn layered_member(self) -> &'static str {
        match self {
            Self::BidRequest => "request",
            Self::BidResponse => "response",
        }
    }
}

pub(crate) fn validate_bid_request(
    version: OpenRtbVersion,
    dialect: Dialect,
    input: &str,
) -> ValidationResult {
    validate_payload(version, dialect, input, PayloadKind::BidRequest)
}

pub(crate) fn validate_bid_response(
    version: OpenRtbVersion,
    dialect: Dialect,
    input: &str,
) -> ValidationResult {
    validate_payload(version, dialect, input, PayloadKind::BidResponse)
}

/// OpenRTB 3.0 wraps everything in a single `openrtb` member.
const ENVELOPE_MEMBER: &str = "openrtb";
/// The catalog object describing that wrapper.
const ENVELOPE_OBJECT: &str = "Openrtb";
/// The only domain spec whose objects this build can reason about.
const DEFAULT_DOMAIN_SPEC: &str = "adcom";
/// The only published version of the SupplyChain object.
const SUPPLY_CHAIN_VERSION: &str = "1.0";
/// Node count past which a declared path is worth a second look. Real chains
/// run to a handful of hops; anything near this is usually a concatenation
/// bug, since every hop has to be independently authorised to be buyable.
const SUPPLY_CHAIN_LENGTH_CEILING: usize = 10;

fn validate_payload(
    version: OpenRtbVersion,
    dialect: Dialect,
    input: &str,
    kind: PayloadKind,
) -> ValidationResult {
    if matches!(version.family(), crate::OpenRtbFamily::ThreeZero) {
        return validate_layered_payload(version, dialect, input, kind);
    }

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
        dialect,
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

/// OpenRTB 3.0 splits the transport layer (this catalog: Openrtb, Request,
/// Item, Response, Bid) from the domain layer, which is AdCOM and lives under
/// `item.spec` and `bid.media`. Everything in the transport layer is checked
/// here; the domain objects are accepted as opaque objects, since no AdCOM
/// catalog ships yet.
fn validate_layered_payload(
    version: OpenRtbVersion,
    dialect: Dialect,
    input: &str,
    kind: PayloadKind,
) -> ValidationResult {
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

    let envelope_section = canonical_object(version, ENVELOPE_OBJECT).map(|object| object.section);
    let mut issues = Vec::new();

    let Some(envelope) = root.get(ENVELOPE_MEMBER).and_then(Value::as_object) else {
        issues.push(Issue {
            id: String::from("openrtb.envelope.missing"),
            severity: Severity::Error,
            message: format!(
                "An OpenRTB {} {} is a single \"openrtb\" object wrapping ver, domainspec, \
                 domainver, and the {} payload.{}",
                version.id(),
                kind.label(),
                kind.layered_member(),
                migration_hint(root)
            ),
            path: None,
            section: envelope_section.map(String::from),
        });
        return finalize_result(issues);
    };

    for member in root.keys().filter(|key| key.as_str() != ENVELOPE_MEMBER) {
        issues.push(Issue {
            id: String::from("openrtb.field.undefined"),
            severity: Severity::Error,
            message: format!(
                "{member} sits outside the envelope; an OpenRTB {} payload carries nothing at the \
                 top level but \"openrtb\".",
                version.id()
            ),
            path: Some(member.clone()),
            section: envelope_section.map(String::from),
        });
    }

    let mut instance_path = String::from(ENVELOPE_MEMBER);
    validate_known_object(
        version,
        dialect,
        kind,
        ENVELOPE_OBJECT,
        envelope,
        &mut vec![ENVELOPE_MEMBER],
        &mut instance_path,
        &mut issues,
    );

    validate_envelope_semantics(version, kind, envelope, envelope_section, &mut issues);

    finalize_result(issues)
}

/// Envelope rules the catalog cannot express: the spec marks `request` and
/// `response` "required *", meaning exactly one of them, and which one depends
/// on the payload being validated.
fn validate_envelope_semantics(
    version: OpenRtbVersion,
    kind: PayloadKind,
    envelope: &Map<String, Value>,
    envelope_section: Option<&'static str>,
    issues: &mut Vec<Issue>,
) {
    let has_request = envelope.get("request").is_some_and(Value::is_object);
    let has_response = envelope.get("response").is_some_and(Value::is_object);
    let expected = kind.layered_member();

    if !envelope.contains_key(expected) {
        issues.push(Issue {
            id: String::from("openrtb.field.required"),
            severity: Severity::Error,
            message: format!(
                "openrtb.{expected} is required on an OpenRTB {} {}.",
                version.id(),
                kind.label()
            ),
            path: Some(format!("{ENVELOPE_MEMBER}.{expected}")),
            section: envelope_section.map(String::from),
        });
    }

    if has_request && has_response {
        issues.push(Issue {
            id: String::from("openrtb.fields.mutually_exclusive"),
            severity: Severity::Error,
            message: String::from(
                "An OpenRTB 3.0 envelope carries either a request or a response, never both.",
            ),
            path: Some(format!("{ENVELOPE_MEMBER}.request")),
            section: envelope_section.map(String::from),
        });
    }

    if let Some(declared) = envelope.get("ver").and_then(Value::as_str) {
        if declared != version.id() {
            issues.push(Issue {
                id: String::from("openrtb.envelope.ver_mismatch"),
                severity: Severity::Warning,
                message: format!(
                    "Envelope declares OpenRTB {declared} but the payload is being validated \
                     against {}.",
                    version.id()
                ),
                path: Some(format!("{ENVELOPE_MEMBER}.ver")),
                section: envelope_section.map(String::from),
            });
        }
    }

    if let Some(domainspec) = envelope.get("domainspec").and_then(Value::as_str) {
        if !domainspec.eq_ignore_ascii_case(DEFAULT_DOMAIN_SPEC) {
            issues.push(Issue {
                id: String::from("openrtb.envelope.domainspec_unsupported"),
                severity: Severity::Warning,
                message: format!(
                    "Domain spec \"{domainspec}\" is not AdCOM, so the objects under item.spec \
                     and bid.media are left unchecked.",
                ),
                path: Some(format!("{ENVELOPE_MEMBER}.domainspec")),
                section: envelope_section.map(String::from),
            });
        }
    }
}

/// A 2.x payload sent to a 3.0 validator is a common migration slip, and the
/// bare "no envelope" message does not explain what moved where.
fn migration_hint(root: &Map<String, Value>) -> String {
    if root.contains_key("imp") {
        return String::from(
            " This looks like an OpenRTB 2.x bid request: the 2.x root moves to openrtb.request, \
             imp becomes item, and each impression's media objects (banner, video, audio, native) \
             move into item.spec as AdCOM placements.",
        );
    }

    if root.contains_key("seatbid") || root.contains_key("nbr") {
        return String::from(
            " This looks like an OpenRTB 2.x bid response: the 2.x root moves to openrtb.response, \
             bid.impid becomes bid.item, and bid.adm moves into bid.media as an AdCOM ad.",
        );
    }

    String::new()
}

/// Recomputes `valid` after cross-payload checks have appended issues.
pub(crate) fn finalize_result(issues: Vec<Issue>) -> ValidationResult {
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
    dialect: Dialect,
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
            version,
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

        let field_definition = definition
            .fields
            .iter()
            .find(|field| field.name == field_name.as_str());

        // protojson emits lowerCamelCase names unless the serializer asks for
        // proto names, so a protojson payload can spell a catalogued field
        // `privateAuction`. Resolve it to the spec name and keep validating,
        // rather than reporting the whole subtree as undefined.
        let field_definition = match (field_definition, dialect) {
            (Some(field_definition), _) => Some(field_definition),
            (None, Dialect::ProtoJson) => {
                match snake_case_of_camel(field_name).and_then(|spec_name| {
                    definition
                        .fields
                        .iter()
                        .find(|field| field.name == spec_name.as_str())
                }) {
                    Some(resolved) => {
                        issues.push(Issue {
                            id: String::from("openrtb.dialect.camel_case_name"),
                            severity: Severity::Warning,
                            message: format!(
                                "{field_name} is the lowerCamelCase protobuf JSON spelling of \
                                 {}.{}; OpenRTB JSON readers look for \"{}\". Serialize with \
                                 proto field names to stay readable on both sides.",
                                object_name, resolved.name, resolved.name
                            ),
                            path: Some(String::from(instance_path.as_str())),
                            section: Some(String::from(resolved.citation.section)),
                        });
                        Some(resolved)
                    }
                    None => None,
                }
            }
            (None, Dialect::SpecJson) => None,
        };

        let Some(field_definition) = field_definition else {
            issues.push(uncatalogued_field_issue(
                version,
                kind,
                object_name,
                field_name,
                logical_segments,
                instance_path,
                definition.section,
            ));
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
        validate_field_value_shape(
            dialect,
            object_name,
            field_definition,
            value,
            instance_path,
            issues,
        );
        validate_required_array_contents(field_definition, value, instance_path, issues);
        validate_catalog_value_set(field_definition, value, instance_path, issues);

        if matches!(field_definition.shape, ExpectedShape::Object) && value.is_object() {
            if let Some(child_object_name) = field_definition.child_object {
                validate_known_object(
                    version,
                    dialect,
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
                            dialect,
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
        version,
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
    version: OpenRtbVersion,
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
        "Bid" => validate_bid_semantics(version, object, instance_path, object_section, issues),
        "Imp" => validate_imp_semantics(object, instance_path, object_section, issues),
        "Video" => validate_video_semantics(object, instance_path, object_section, issues),
        "Audio" => validate_audio_semantics(object, instance_path, object_section, issues),
        "Deal" => validate_deal_semantics(object, instance_path, object_section, issues),
        "Regs" => validate_regs_semantics(object, instance_path, object_section, issues),
        "Native" => validate_native_semantics(object, instance_path, object_section, issues),
        "Source" => validate_source_semantics(object, instance_path, object_section, issues),
        "SupplyChain" => {
            validate_supply_chain_semantics(object, instance_path, object_section, issues)
        }
        "SupplyChainNode" => {
            validate_supply_chain_node_semantics(object, instance_path, object_section, issues)
        }
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

    if let Some(tmax) = object.get("tmax").and_then(integer_value) {
        let tmax_path = join_instance_path(instance_path, "tmax");
        if tmax <= 0 {
            issues.push(Issue {
                id: String::from("openrtb.request.tmax_non_positive"),
                severity: Severity::Error,
                message: String::from(
                    "tmax must be a positive number of milliseconds; a zero or negative value \
                     leaves no time for bids to be received.",
                ),
                path: Some(tmax_path),
                section: Some(String::from(section)),
            });
        } else if tmax > 10_000 {
            issues.push(Issue {
                id: String::from("openrtb.request.tmax_implausible"),
                severity: Severity::Warning,
                message: format!(
                    "tmax of {tmax} is unusually high for an RTB auction; confirm tmax is \
                     expressed in milliseconds, not seconds."
                ),
                path: Some(tmax_path),
                section: Some(String::from(section)),
            });
        }
    }

    if let Some(currencies) = object.get("cur").and_then(Value::as_array) {
        let cur_instance_path = join_instance_path(instance_path, "cur");
        for (index, currency) in currencies.iter().enumerate() {
            if let Some(code) = currency.as_str() {
                if !is_alpha3_currency_code(code) {
                    issues.push(Issue {
                        id: String::from("openrtb.request.cur_format_invalid"),
                        severity: Severity::Warning,
                        message: format!(
                            "\"{code}\" does not look like an ISO-4217 currency code (three \
                             uppercase letters, e.g. \"USD\")."
                        ),
                        path: Some(format!("{cur_instance_path}[{index}]")),
                        section: Some(String::from(section)),
                    });
                }
            }
        }
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

/// How `bid.adm` markup classifies after content sniffing. Parsing is cheap
/// for non-JSON markup: serde fails on the first byte of XML or HTML, so
/// only genuine JSON payloads (native responses) are parsed in full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AdmMarkup {
    /// Parses to a JSON string: the markup was JSON-encoded twice.
    DoubleEncodedJson,
    /// Parses to a JSON object: a native response payload (bare Native
    /// Markup Response or the documented `{"native": {...}}` root).
    NativeJson,
    /// Parses as JSON but to neither a string nor an object.
    OtherJson,
    /// XML with a VAST or DAAST document root.
    Vast,
    /// Starts with `<` but is not recognizably VAST/DAAST (HTML, other XML).
    OtherMarkup,
    /// Anything else: JavaScript, a bare URL, free text.
    Other,
}

pub(crate) fn classify_adm(adm: &str) -> AdmMarkup {
    match serde_json::from_str::<Value>(adm) {
        Ok(Value::String(_)) => return AdmMarkup::DoubleEncodedJson,
        Ok(Value::Object(_)) => return AdmMarkup::NativeJson,
        Ok(_) => return AdmMarkup::OtherJson,
        Err(_) => {}
    }

    if adm.trim_start().starts_with('<') {
        if adm.contains("<VAST") || adm.contains("<DAAST") {
            AdmMarkup::Vast
        } else {
            AdmMarkup::OtherMarkup
        }
    } else {
        AdmMarkup::Other
    }
}

/// Markup-type coherence between `bid.mtype` and the `bid.adm` payload.
/// This is the response-side counterpart of the `imp.native.request`
/// encoding checks; without the originating request, `mtype` is the only
/// in-payload declaration of what the markup should be.
fn validate_bid_semantics(
    version: OpenRtbVersion,
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(adm) = object.get("adm").and_then(Value::as_str) else {
        return;
    };
    let adm_path = join_instance_path(instance_path, "adm");
    let markup = classify_adm(adm);

    if markup == AdmMarkup::DoubleEncodedJson {
        issues.push(Issue {
            id: String::from("openrtb.bid.adm.double_encoded"),
            severity: Severity::Error,
            message: String::from(
                "adm parses to another JSON string rather than markup; it looks like the \
                 creative payload was JSON-encoded twice.",
            ),
            path: Some(adm_path.clone()),
            section: Some(String::from(section)),
        });
    }

    let mtype = object.get("mtype").and_then(integer_value);
    match mtype {
        Some(1) => {
            if markup == AdmMarkup::NativeJson {
                issues.push(Issue {
                    id: String::from("openrtb.bid.adm.markup_type_mismatch"),
                    severity: Severity::Error,
                    message: String::from(
                        "mtype 1 declares banner markup, but adm is a JSON object; a JSON \
                         payload in adm is native markup (mtype 4).",
                    ),
                    path: Some(adm_path.clone()),
                    section: Some(String::from(section)),
                });
            }
        }
        Some(2) | Some(3) => {
            let declared = if mtype == Some(2) {
                "mtype 2 declares video markup (VAST XML)"
            } else {
                "mtype 3 declares audio markup (VAST or DAAST XML)"
            };
            match markup {
                AdmMarkup::NativeJson | AdmMarkup::OtherJson => issues.push(Issue {
                    id: String::from("openrtb.bid.adm.markup_type_mismatch"),
                    severity: Severity::Error,
                    message: format!("{declared}, but adm is a JSON payload."),
                    path: Some(adm_path.clone()),
                    section: Some(String::from(section)),
                }),
                AdmMarkup::Other => issues.push(Issue {
                    id: String::from("openrtb.bid.adm.not_markup"),
                    severity: Severity::Warning,
                    message: format!("{declared}, but adm does not start with an XML tag."),
                    path: Some(adm_path.clone()),
                    section: Some(String::from(section)),
                }),
                AdmMarkup::OtherMarkup => issues.push(Issue {
                    id: String::from("openrtb.bid.adm.vast_root_missing"),
                    severity: Severity::Warning,
                    message: format!("{declared}, but adm has no VAST or DAAST document root."),
                    path: Some(adm_path.clone()),
                    section: Some(String::from(section)),
                }),
                AdmMarkup::Vast | AdmMarkup::DoubleEncodedJson => {}
            }
        }
        Some(4) => match markup {
            AdmMarkup::NativeJson | AdmMarkup::DoubleEncodedJson => {}
            _ => issues.push(Issue {
                id: String::from("openrtb.bid.adm.native_not_json"),
                severity: Severity::Error,
                message: String::from(
                    "mtype 4 declares native markup, but adm does not parse as a JSON object; \
                     a native response must be the JSON Native Markup Response.",
                ),
                path: Some(adm_path.clone()),
                section: Some(String::from(section)),
            }),
        },
        _ => {
            // No usable mtype. On versions whose Bid object defines mtype,
            // an adm without one leaves exchanges guessing at the markup
            // type; several majors reject such bids outright.
            if mtype.is_none()
                && !object.contains_key("mtype")
                && canonical_field(version, "Bid", "mtype").is_some()
            {
                issues.push(Issue {
                    id: String::from("openrtb.bid.mtype_missing"),
                    severity: Severity::Warning,
                    message: String::from(
                        "adm is present but mtype is not set; declare the markup type so the \
                         exchange can associate the creative with the right Imp subtype.",
                    ),
                    path: Some(join_instance_path(instance_path, "mtype")),
                    section: Some(String::from(section)),
                });
            }
        }
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

    validate_bidfloor_semantics(object, instance_path, section, issues);
}

fn validate_deal_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    validate_bidfloor_semantics(object, instance_path, section, issues);
}

/// Shared by `Imp` and `Deal`, which both carry a `bidfloor`/`bidfloorcur`
/// pair with identical semantics. Deliberately fires the same
/// `openrtb.imp.*` ids regardless of which object called it, matching the
/// existing pattern of one shared id firing from multiple call sites (see
/// `openrtb.fields.mutually_exclusive`).
fn validate_bidfloor_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    if let Some(bidfloor) = object.get("bidfloor").and_then(Value::as_f64) {
        if bidfloor < 0.0 {
            issues.push(Issue {
                id: String::from("openrtb.imp.bidfloor_negative"),
                severity: Severity::Error,
                message: format!(
                    "bidfloor of {bidfloor} is negative; a CPM price cannot be negative."
                ),
                path: Some(join_instance_path(instance_path, "bidfloor")),
                section: Some(String::from(section)),
            });
        }
    }

    if let Some(currency) = object.get("bidfloorcur").and_then(Value::as_str) {
        if !is_alpha3_currency_code(currency) {
            issues.push(Issue {
                id: String::from("openrtb.imp.bidfloorcur_format_invalid"),
                severity: Severity::Warning,
                message: format!(
                    "\"{currency}\" does not look like an ISO-4217 currency code (three \
                     uppercase letters, e.g. \"USD\")."
                ),
                path: Some(join_instance_path(instance_path, "bidfloorcur")),
                section: Some(String::from(section)),
            });
        }
    }
}

fn validate_video_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    validate_duration_semantics(object, instance_path, section, issues);
    validate_pod_semantics(object, instance_path, section, issues);

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

/// CTV ad-pod duration coherence. Deliberately does not require `podid` on
/// `slotinpod`/`podseq`/`maxseq`/`poddur`: a single Imp can represent an
/// entire dynamic pod with no sibling Imps to correlate against, and an
/// existing fixture (valid-openrtb-2.6-202402-poddedupe-video) documents
/// exactly that shape.
fn validate_pod_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    if object
        .get("rqddurs")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        issues.push(Issue {
            id: String::from("openrtb.video.pod.rqddurs_empty"),
            severity: Severity::Warning,
            message: String::from(
                "rqddurs is an empty array; it should list the exact durations acceptable for \
                 this ad pod slot, or be omitted entirely.",
            ),
            path: Some(join_instance_path(instance_path, "rqddurs")),
            section: Some(String::from(section)),
        });
    }

    let has_dynamic_pod_context = object.contains_key("poddur") || object.contains_key("maxseq");
    if object.contains_key("mincpmpersec") && !has_dynamic_pod_context {
        issues.push(Issue {
            id: String::from("openrtb.video.pod.mincpmpersec_without_pod_context"),
            severity: Severity::Warning,
            message: String::from(
                "mincpmpersec is meant for the dynamic portion of a video ad pod; it is present \
                 without poddur or maxseq, which normally accompany a dynamic pod.",
            ),
            path: Some(join_instance_path(instance_path, "mincpmpersec")),
            section: Some(String::from(section)),
        });
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

fn validate_regs_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    let gpp_present = object
        .get("gpp")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty());
    let gpp_sid_present = object
        .get("gpp_sid")
        .and_then(Value::as_array)
        .is_some_and(|values| !values.is_empty());

    if gpp_sid_present && !gpp_present {
        issues.push(Issue {
            id: String::from("openrtb.regs.gpp_sid_without_gpp"),
            severity: Severity::Warning,
            message: String::from(
                "gpp_sid is present but gpp is absent or empty; section ids with no GPP string \
                 to scope them are incoherent.",
            ),
            path: Some(join_instance_path(instance_path, "gpp_sid")),
            section: Some(String::from(section)),
        });
    }

    if gpp_present && !gpp_sid_present {
        issues.push(Issue {
            id: String::from("openrtb.regs.gpp_without_gpp_sid"),
            severity: Severity::Warning,
            message: String::from(
                "gpp is present but gpp_sid is absent or empty; without section ids a consumer \
                 cannot tell which GPP sections apply.",
            ),
            path: Some(join_instance_path(instance_path, "gpp")),
            section: Some(String::from(section)),
        });
    }

    if let Some(us_privacy) = object.get("us_privacy").and_then(Value::as_str) {
        if !us_privacy.is_empty() && !is_us_privacy_string_shape(us_privacy) {
            issues.push(Issue {
                id: String::from("openrtb.regs.us_privacy_malformed"),
                severity: Severity::Warning,
                message: format!(
                    "\"{us_privacy}\" does not look like a US Privacy string: expected \"1\" \
                     followed by three characters each Y, N, or \"-\"."
                ),
                path: Some(join_instance_path(instance_path, "us_privacy")),
                section: Some(String::from(section)),
            });
        }
    }
}

fn validate_native_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(request_raw) = object.get("request").and_then(Value::as_str) else {
        return;
    };
    let request_path = join_instance_path(instance_path, "request");

    match serde_json::from_str::<Value>(request_raw) {
        Ok(Value::String(_)) => {
            issues.push(Issue {
                id: String::from("openrtb.native.request.double_encoded"),
                severity: Severity::Error,
                message: String::from(
                    "native.request parses to another JSON string rather than an object; it \
                     looks like the Native Markup Request was JSON-encoded twice.",
                ),
                path: Some(request_path),
                section: Some(String::from(section)),
            });
        }
        Ok(Value::Object(fields)) => {
            if fields.len() == 1 && fields.contains_key("native") {
                issues.push(Issue {
                    id: String::from("openrtb.native.request.legacy_wrapper"),
                    severity: Severity::Warning,
                    message: String::from(
                        "native.request is wrapped in a top-level \"native\" key; that \
                         convention predates Native Ads 1.1, which made the Native Markup \
                         Request the root object.",
                    ),
                    path: Some(request_path),
                    section: Some(String::from(section)),
                });
            }
        }
        Ok(_) => {}
        Err(_) => {
            issues.push(Issue {
                id: String::from("openrtb.native.request.unparseable"),
                severity: Severity::Warning,
                message: String::from(
                    "native.request does not parse as JSON; it should be a JSON-encoded Native \
                     Markup Request object.",
                ),
                path: Some(request_path),
                section: Some(String::from(section)),
            });
        }
    }
}

fn validate_source_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    let ext_declares_schain = object
        .get("ext")
        .and_then(Value::as_object)
        .is_some_and(|ext| ext.contains_key("schain"));

    if object.contains_key("schain") && ext_declares_schain {
        issues.push(Issue {
            id: String::from("openrtb.schain.duplicate_location"),
            severity: Severity::Warning,
            message: String::from(
                "A supply chain is declared at both source.schain and source.ext.schain; the two \
                 copies can disagree, and receivers differ on which one they read.",
            ),
            path: Some(join_instance_path(instance_path, "ext.schain")),
            section: Some(String::from(section)),
        });
    }
}

fn validate_supply_chain_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    if let Some(ver) = object.get("ver").and_then(Value::as_str) {
        if ver != SUPPLY_CHAIN_VERSION {
            issues.push(Issue {
                id: String::from("openrtb.schain.ver_unexpected"),
                severity: Severity::Warning,
                message: format!(
                    "ver is \"{ver}\"; {SUPPLY_CHAIN_VERSION} is the only published version of \
                     the SupplyChain object, so receivers may not recognise this chain.",
                ),
                path: Some(join_instance_path(instance_path, "ver")),
                section: Some(String::from(section)),
            });
        }
    }

    if object.get("complete").and_then(integer_value) == Some(0) {
        issues.push(Issue {
            id: String::from("openrtb.schain.incomplete"),
            severity: Severity::Warning,
            message: String::from(
                "complete is 0, which declares that at least one upstream node is missing from \
                 this path; buyers that require a verifiable chain will treat the inventory as \
                 unauthorised.",
            ),
            path: Some(join_instance_path(instance_path, "complete")),
            section: Some(String::from(section)),
        });
    }

    let Some(nodes) = object.get("nodes").and_then(Value::as_array) else {
        return;
    };
    let nodes_path = join_instance_path(instance_path, "nodes");

    // An empty `nodes` array is already an error: the catalog's required-field
    // check treats it as absent. Adding a second finding for the same defect
    // would just be noise, so this only covers chains that do have nodes.
    if nodes.len() > SUPPLY_CHAIN_LENGTH_CEILING {
        issues.push(Issue {
            id: String::from("openrtb.schain.length_implausible"),
            severity: Severity::Warning,
            message: format!(
                "This SupplyChain declares {} nodes; paths that long are rare, and each hop has \
                 to be independently authorised, so check the chain was not appended twice.",
                nodes.len()
            ),
            path: Some(nodes_path.clone()),
            section: Some(String::from(section)),
        });
    }

    for index in 1..nodes.len() {
        let previous = nodes[index - 1].as_object();
        let current = nodes[index].as_object();
        let (Some(previous), Some(current)) = (previous, current) else {
            continue;
        };

        let same_asi = previous.get("asi").and_then(Value::as_str)
            == current.get("asi").and_then(Value::as_str);
        let same_sid = previous.get("sid").and_then(Value::as_str)
            == current.get("sid").and_then(Value::as_str);

        if same_asi && same_sid && previous.get("asi").and_then(Value::as_str).is_some() {
            issues.push(Issue {
                id: String::from("openrtb.schain.duplicate_node"),
                severity: Severity::Warning,
                message: String::from(
                    "Two adjacent SupplyChain nodes share the same asi and sid; verify this \
                     node was not appended twice by mistake.",
                ),
                path: Some(format!("{nodes_path}[{index}]")),
                section: Some(String::from(section)),
            });
        }
    }
}

fn validate_supply_chain_node_semantics(
    object: &Map<String, Value>,
    instance_path: &str,
    section: &str,
    issues: &mut Vec<Issue>,
) {
    if !object.contains_key("hp") {
        issues.push(Issue {
            id: String::from("openrtb.schain.node.hp_missing"),
            severity: Severity::Warning,
            message: String::from(
                "This SupplyChain node has no hp field; the spec expects it to be propagated \
                 on every node once a payment-flow signal is available.",
            ),
            path: Some(String::from(instance_path)),
            section: Some(String::from(section)),
        });
    } else if let Some(hp) = object.get("hp").and_then(integer_value) {
        if hp != 1 {
            issues.push(Issue {
                id: String::from("openrtb.schain.node.hp_unexpected"),
                severity: Severity::Warning,
                message: format!(
                    "hp is {hp}; version {SUPPLY_CHAIN_VERSION} of the SupplyChain object expects \
                     every node on the declared path to be marked as part of the payment flow \
                     with hp set to 1.",
                ),
                path: Some(join_instance_path(instance_path, "hp")),
                section: Some(String::from(section)),
            });
        }
    }

    for field in ["asi", "sid"] {
        if object.get(field).and_then(Value::as_str) == Some("") {
            issues.push(Issue {
                id: String::from("openrtb.schain.node.identifier_empty"),
                severity: Severity::Error,
                message: format!(
                    "{} is an empty string; it must identify the advertising system or seller.",
                    join_instance_path(instance_path, field)
                ),
                path: Some(join_instance_path(instance_path, field)),
                section: Some(String::from(section)),
            });
        }
    }

    let Some(asi) = object.get("asi").and_then(Value::as_str) else {
        return;
    };
    if asi.is_empty() {
        return;
    }
    let asi_path = join_instance_path(instance_path, "asi");

    if let Some(defect) = supply_chain_asi_defect(asi) {
        issues.push(Issue {
            id: String::from("openrtb.schain.node.asi_not_domain"),
            severity: Severity::Warning,
            message: format!(
                "asi is \"{asi}\", which {defect}; asi has to be the bare canonical domain of the \
                 selling system so it can be matched against that domain's sellers.json and the \
                 publisher's ads.txt.",
            ),
            path: Some(asi_path.clone()),
            section: Some(String::from(section)),
        });
    }

    if asi.chars().any(char::is_uppercase) {
        issues.push(Issue {
            id: String::from("openrtb.schain.node.asi_not_lowercase"),
            severity: Severity::Warning,
            message: format!(
                "asi is \"{asi}\"; canonical domains are lowercase, and a case difference breaks \
                 exact matching against sellers.json and ads.txt entries.",
            ),
            path: Some(asi_path),
            section: Some(String::from(section)),
        });
    }
}

/// Why an `asi` value is not a bare canonical domain, if it is not one.
/// Ordered so the most specific defect wins: a full URL trips the scheme
/// check rather than reporting a path separator and a port separately.
fn supply_chain_asi_defect(asi: &str) -> Option<&'static str> {
    if asi.contains("://") {
        Some("includes a URI scheme")
    } else if asi.contains('/') {
        Some("includes a path separator")
    } else if asi.chars().any(char::is_whitespace) {
        Some("contains whitespace")
    } else if asi.contains(':') {
        Some("includes a port")
    } else if asi.starts_with('.') || asi.ends_with('.') {
        Some("has a leading or trailing dot")
    } else if !asi.contains('.') {
        Some("is not a domain name")
    } else {
        None
    }
}

pub(crate) fn integer_value(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_u64()
            .and_then(|integer| i64::try_from(integer).ok())
    })
}

/// Uppercase ISO-4217 alpha style: exactly three ASCII uppercase letters.
fn is_alpha3_currency_code(code: &str) -> bool {
    code.len() == 3 && code.bytes().all(|byte| byte.is_ascii_uppercase())
}

/// IAB US Privacy String v1 shape: "1" followed by three chars each one of
/// Y, N, or "-". Uppercase only; no other version is currently defined, and
/// "-" in any of the three flag positions (including "1---") is the spec's
/// own documented "not applicable" signal, not an error.
fn is_us_privacy_string_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 4
        && bytes[0] == b'1'
        && bytes[1..]
            .iter()
            .all(|byte| matches!(byte, b'Y' | b'N' | b'-'))
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
    dialect: Dialect,
    object_name: &str,
    field: &StaticField,
    value: &Value,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    if matches!(field.shape, ExpectedShape::Integer)
        && proto_declares_bool(object_name, field.name)
        && validate_proto_bool_field(dialect, object_name, field, value, instance_path, issues)
    {
        return;
    }

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

/// Handles the fields the IAB protobuf schema declares `bool` while the spec
/// types them as an integer flag.
///
/// Returns whether the value was settled here. A `false` return sends the
/// value back to the ordinary shape check, which is what should happen for a
/// spec-JSON integer (correct) and for anything that is neither a bool nor a
/// number (a plain type mismatch, dialect notwithstanding).
fn validate_proto_bool_field(
    dialect: Dialect,
    object_name: &str,
    field: &StaticField,
    value: &Value,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) -> bool {
    match (value, dialect) {
        // Correct for the transport it was written for.
        (Value::Bool(_), Dialect::ProtoJson) => true,
        (Value::Bool(actual), Dialect::SpecJson) => {
            issues.push(Issue {
                id: String::from("openrtb.dialect.bool_for_integer"),
                severity: Severity::Error,
                message: format!(
                    "{instance_path} is {actual}, but OpenRTB types {}.{} as an integer flag \
                     (0 or 1). The IAB protobuf schema declares it bool, so this is protobuf \
                     JSON: send {} to spec-JSON readers, or validate with the proto-json \
                     dialect.",
                    object_name,
                    field.name,
                    u8::from(*actual)
                ),
                path: Some(String::from(instance_path)),
                section: Some(String::from(field.citation.section)),
            });
            true
        }
        (Value::Number(number), Dialect::ProtoJson) => {
            issues.push(Issue {
                id: String::from("openrtb.dialect.integer_for_bool"),
                severity: Severity::Error,
                message: format!(
                    "{instance_path} is {number}, but the IAB protobuf schema declares {}.{} as \
                     bool, and protobuf JSON accepts only true or false there. A protojson \
                     parser rejects this payload outright.",
                    object_name, field.name
                ),
                path: Some(String::from(instance_path)),
                section: Some(String::from(field.citation.section)),
            });
            true
        }
        _ => false,
    }
}

/// Look up a walked field against the version rules, in both path vocabularies.
///
/// The two do not share one. The walker builds a full logical path from the
/// document root (`imp.banner.wmax`), while the version rules name a field
/// relative to the object that owns it (`banner.wmax`), because that is how the
/// spec's change appendices are written. Of the 130 rule paths, most are the
/// two-segment `object.field` form, so a full-path lookup only lands when the
/// owning object happens to sit at the root: `regs.gpp` matches, `banner.wmax`
/// never does.
///
/// So try the full path, then the trailing `object.field` pair. Both are exact
/// comparisons against the rule table rather than substring matching, and the
/// tail is the rules' own vocabulary, so this resolves the mismatch without
/// loosening what counts as a match.
fn resolve_path_status(
    version: OpenRtbVersion,
    kind: PayloadKind,
    logical_segments: &[&str],
) -> Option<(String, crate::PathStatus)> {
    let full = schema_path(kind, logical_segments)?;
    let status = path_status(version, &full);
    if status.kind != PathStateKind::Unknown {
        return Some((full, status));
    }

    if logical_segments.len() >= 2 {
        let tail = logical_segments[logical_segments.len() - 2..].join(".");
        if tail != full {
            let tail_status = path_status(version, &tail);
            if tail_status.kind != PathStateKind::Unknown {
                return Some((tail, tail_status));
            }
        }
    }

    Some((full, status))
}

/// The finding for a field the target version's catalog does not define.
///
/// `openrtb.field.undefined` is the fallback, and on its own it is a weak
/// answer: it says the name is absent from this version's catalog, which is
/// true for a typo, for a field that was removed, and for a field that has not
/// shipped yet. Those three want different fixes.
///
/// The version rules already distinguish them, so consult those first. A field
/// absent because it arrives in a later snapshot gets
/// `openrtb.field.not_yet_available` and the version it arrives in, which turns
/// "unknown field" into a version-negotiation answer. A field absent because it
/// was removed gets `openrtb.field.removed`.
///
/// Paths no version rule knows about return `PathStateKind::Unknown` and fall
/// through to `openrtb.field.undefined`, so ordinary typos are unaffected.
#[allow(clippy::too_many_arguments)]
fn uncatalogued_field_issue(
    version: OpenRtbVersion,
    kind: PayloadKind,
    object_name: &str,
    field_name: &str,
    logical_segments: &[&str],
    instance_path: &str,
    catalog_section: &str,
) -> Issue {
    if let Some((schema_path, status)) = resolve_path_status(version, kind, logical_segments) {
        let section = status
            .matched_rules
            .first()
            .map(|matched| String::from(matched.rule.section))
            .unwrap_or_else(|| String::from(catalog_section));

        match status.kind {
            PathStateKind::Removed => {
                return Issue {
                    id: String::from("openrtb.field.removed"),
                    severity: Severity::Error,
                    message: format!(
                        "{} was removed before OpenRTB {}.",
                        schema_path,
                        version.id()
                    ),
                    path: Some(String::from(instance_path)),
                    section: Some(section),
                };
            }
            PathStateKind::NotYetAvailable => {
                let arrives = match status.since {
                    Some(since) => format!(" It arrives in {}.", since.id()),
                    None => String::new(),
                };
                return Issue {
                    id: String::from("openrtb.field.not_yet_available"),
                    severity: Severity::Error,
                    message: format!(
                        "{} is not available in OpenRTB {}.{}",
                        schema_path,
                        version.id(),
                        arrives
                    ),
                    path: Some(String::from(instance_path)),
                    section: Some(section),
                };
            }
            _ => {}
        }
    }

    Issue {
        id: String::from("openrtb.field.undefined"),
        severity: Severity::Error,
        message: format!(
            "{}.{} is not defined in the canonical OpenRTB {} catalog.",
            object_name,
            field_name,
            version.id()
        ),
        path: Some(String::from(instance_path)),
        section: Some(String::from(catalog_section)),
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
