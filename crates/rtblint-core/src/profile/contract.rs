//! Shared primitives for published exchange contracts. Unknown fields stay open.
use super::{join_instance_path, profile_issue, value_at};
use crate::{Issue, Severity};
use serde_json::{Map, Value};

#[derive(Clone, Copy)]
pub(super) enum Kind {
    String,
    Integer,
    Number,
    Flag,
    CompatibleFlag,
    Object(&'static [Field]),
    Array(&'static Kind),
}

pub(super) struct Field {
    pub name: &'static str,
    pub kind: Kind,
}

pub(super) fn fields(
    object: &Map<String, Value>,
    schema: &[Field],
    path: &str,
    issues: &mut Vec<Issue>,
) {
    for field in schema {
        if let Some(value) = object.get(field.name).filter(|value| !value.is_null()) {
            shape(
                value,
                field.kind,
                &join_instance_path(path, field.name),
                issues,
            );
        }
    }
}

fn shape(value: &Value, kind: Kind, path: &str, issues: &mut Vec<Issue>) {
    let valid = match kind {
        Kind::String => value.is_string(),
        Kind::Integer => value.is_i64() || value.is_u64(),
        Kind::Number => value.is_number(),
        Kind::Flag => value.as_i64().is_some_and(|v| matches!(v, 0 | 1)),
        Kind::CompatibleFlag => {
            value.is_boolean() || value.as_i64().is_some_and(|v| matches!(v, 0 | 1))
        }
        Kind::Object(schema) => {
            if let Some(object) = value.as_object() {
                fields(object, schema, path, issues);
                return;
            }
            false
        }
        Kind::Array(item) => {
            if let Some(values) = value.as_array() {
                for (index, value) in values.iter().enumerate() {
                    shape(value, *item, &format!("{path}[{index}]"), issues);
                }
                return;
            }
            false
        }
    };
    if !valid {
        error(
            "openrtb.profile.field_type",
            "The field does not match the exchange's documented JSON type.",
            path.to_owned(),
            issues,
        );
    }
}

pub(super) fn required(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    label: &str,
    issues: &mut Vec<Issue>,
) {
    if !present(object, field) {
        error(
            "openrtb.profile.field_required",
            &format!("{label} requires {field} for this contract."),
            join_instance_path(path, field),
            issues,
        );
    }
}

pub(super) fn present(object: &Map<String, Value>, field: &str) -> bool {
    value_at(object, field).is_some_and(|value| match value {
        Value::Null => false,
        Value::String(text) => !text.is_empty(),
        _ => true,
    })
}

pub(super) fn integer_enum(
    object: &Map<String, Value>,
    field: &str,
    values: &[i64],
    path: &str,
    issues: &mut Vec<Issue>,
) {
    if let Some(value) = value_at(object, field).filter(|value| !value.is_null()) {
        if !value.as_i64().is_some_and(|v| values.contains(&v)) {
            error(
                "openrtb.profile.value_invalid",
                &format!("{field} must be one of {values:?} in this exchange contract."),
                join_instance_path(path, field),
                issues,
            );
        }
    }
}

pub(super) fn error(id: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    issues.push(profile_issue(id, message.to_owned(), path));
}

pub(super) fn warning(id: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    let mut issue = profile_issue(id, message.to_owned(), path);
    issue.severity = Severity::Warning;
    issues.push(issue);
}

/// Resolve a response's impression only when the request supplies a unique ID.
pub(super) fn matching_imp<'a>(
    request: &'a Map<String, Value>,
    bid: &Map<String, Value>,
) -> Option<&'a Map<String, Value>> {
    let id = bid.get("impid")?.as_str()?;
    let mut matches = request
        .get("imp")?
        .as_array()?
        .iter()
        .filter_map(Value::as_object)
        .filter(|imp| imp.get("id").and_then(Value::as_str) == Some(id));
    let first = matches.next()?;
    if matches.next().is_some() {
        None
    } else {
        Some(first)
    }
}

pub(super) fn bids(
    response: &Map<String, Value>,
) -> impl Iterator<Item = (String, &Map<String, Value>)> {
    response
        .get("seatbid")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
        .flat_map(|(seat_index, seat)| {
            seat.get("bid")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
                .filter_map(move |(bid_index, bid)| {
                    bid.as_object()
                        .map(|bid| (format!("seatbid[{seat_index}].bid[{bid_index}]"), bid))
                })
        })
}
