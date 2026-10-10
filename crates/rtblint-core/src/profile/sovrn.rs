//! Sovrn's public outgoing request contract. No invented response requirements.

use super::{
    contract::{self, Field, Kind},
    join_instance_path, require_integer_in_range,
};
use crate::Issue;
use serde_json::{Map, Value};

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    match object_name {
        "Imp" => {
            contract::required(object, "tagid", path, "Sovrn", issues);
            contract::fields(
                object,
                &[Field {
                    name: "ext",
                    kind: Kind::Object(&[Field {
                        name: "deals",
                        kind: Kind::Array(&Kind::String),
                    }]),
                }],
                path,
                issues,
            );
        }
        "Banner" => {
            // Sovrn describes format as an alternative to exact w/h, despite its required labels.
            let sized_format =
                object
                    .get("format")
                    .and_then(Value::as_array)
                    .is_some_and(|formats| {
                        formats.iter().any(|format| {
                            format
                                .get("w")
                                .and_then(Value::as_i64)
                                .is_some_and(|v| v > 0)
                                && format
                                    .get("h")
                                    .and_then(Value::as_i64)
                                    .is_some_and(|v| v > 0)
                        })
                    });
            if !sized_format {
                for field in ["w", "h"] {
                    contract::required(object, field, path, "Sovrn banner size", issues);
                    require_integer_in_range(
                        object,
                        field,
                        1,
                        i64::MAX,
                        "Sovrn exact banner dimensions must be positive integers.",
                        path,
                        issues,
                    );
                }
            }
        }
        "Video" => {
            // The dedicated Video Fields page labels minduration recommended, so omission is allowed.
            for field in ["maxduration", "w", "h", "protocols"] {
                contract::required(object, field, path, "Sovrn video", issues);
            }
            for field in ["maxduration", "w", "h"] {
                require_integer_in_range(
                    object,
                    field,
                    1,
                    i64::MAX,
                    "Sovrn video duration and dimensions must be positive integers.",
                    path,
                    issues,
                );
            }
            if object
                .get("protocols")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
            {
                contract::error(
                    "openrtb.profile.sovrn.protocols",
                    "Sovrn video requires supported protocols.",
                    join_instance_path(path, "protocols"),
                    issues,
                );
            }
        }
        "Site" => contract::required(object, "publisher.id", path, "Sovrn", issues),
        "Regs" => contract::integer_enum(object, "ext.gdpr", &[0, 1], path, issues),
        _ => {}
    }
}
