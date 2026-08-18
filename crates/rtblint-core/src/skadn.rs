//! IAB community SKAdNetwork extension on `imp.ext.skadn` and `bid.ext.skadn`.
//!
//! Lives under `ext`, so the OpenRTB catalog will not walk it. Only the
//! documented protocol fields are checked; campaign targeting stays out.

use serde_json::{Map, Value};

use crate::{Issue, Severity};

const SECTION: &str = "IAB SKAdNetwork OpenRTB extension";

pub(crate) fn validate_request(
    skadn: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let versions = skadn.get("versions").and_then(Value::as_array);
    let version = skadn.get("version").and_then(Value::as_str);
    if versions.map_or(true, |items| items.is_empty())
        && version.map_or(true, |text| text.is_empty())
    {
        issues.push(issue(
            "openrtb.skadn.field_required",
            Severity::Error,
            String::from(
                "imp.ext.skadn must declare versions (array of SKAdNetwork versions, 2.0+) or \
                 the deprecated version string.",
            ),
            join_path(instance_path, "versions"),
        ));
    }

    if !non_empty_string(skadn.get("sourceapp")) {
        issues.push(issue(
            "openrtb.skadn.field_required",
            Severity::Error,
            String::from(
                "imp.ext.skadn.sourceapp is required: the publisher app's App Store id, matching \
                 app.bundle.",
            ),
            join_path(instance_path, "sourceapp"),
        ));
    }

    let skadnetids = skadn
        .get("skadnetids")
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty());
    let skadnetlist = skadn.get("skadnetlist").and_then(Value::as_object);
    let list_populated = skadnetlist.is_some_and(|list| {
        list.get("max").is_some()
            || list
                .get("addl")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty())
    });
    if !skadnetids && !list_populated {
        issues.push(issue(
            "openrtb.skadn.field_required",
            Severity::Error,
            String::from(
                "imp.ext.skadn must list SKAdNetwork ids: skadnetids, or skadnetlist.max / \
                 skadnetlist.addl.",
            ),
            join_path(instance_path, "skadnetids"),
        ));
    }
}

pub(crate) fn validate_response(
    skadn: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let version = skadn.get("version").and_then(Value::as_str).unwrap_or("");
    if version.is_empty() {
        issues.push(issue(
            "openrtb.skadn.field_required",
            Severity::Error,
            String::from("bid.ext.skadn.version is required and must be 2.0 or higher."),
            join_path(instance_path, "version"),
        ));
    }

    for field in ["network", "itunesitem", "sourceapp"] {
        if !non_empty_string(skadn.get(field)) {
            issues.push(issue(
                "openrtb.skadn.field_required",
                Severity::Error,
                format!(
                    "bid.ext.skadn.{field} is required when the SKAdNetwork extension is present."
                ),
                join_path(instance_path, field),
            ));
        }
    }

    let is_v4 = version.starts_with('4');
    if is_v4 {
        if !non_empty_string(skadn.get("sourceidentifier")) {
            issues.push(issue(
                "openrtb.skadn.field_required",
                Severity::Error,
                String::from(
                    "bid.ext.skadn.sourceidentifier is required for SKAdNetwork 4.0+ (replaces \
                     campaign).",
                ),
                join_path(instance_path, "sourceidentifier"),
            ));
        }
    } else if !version.is_empty() && !non_empty_string(skadn.get("campaign")) {
        issues.push(issue(
            "openrtb.skadn.field_required",
            Severity::Error,
            String::from(
                "bid.ext.skadn.campaign is required for SKAdNetwork 3.x and below, as a \
                 string campaign id 1-100.",
            ),
            join_path(instance_path, "campaign"),
        ));
    }

    if let Some(fidelities) = skadn.get("fidelities").and_then(Value::as_array) {
        if fidelities.is_empty() {
            issues.push(issue(
                "openrtb.skadn.field_required",
                Severity::Error,
                String::from("bid.ext.skadn.fidelities must not be an empty array."),
                join_path(instance_path, "fidelities"),
            ));
        }
        for (index, fidelity) in fidelities.iter().enumerate() {
            let Some(fidelity) = fidelity.as_object() else {
                continue;
            };
            let path = format!("{instance_path}.fidelities[{index}]");
            for field in ["nonce", "timestamp", "signature"] {
                if !non_empty_string(fidelity.get(field)) {
                    issues.push(issue(
                        "openrtb.skadn.field_required",
                        Severity::Error,
                        format!("bid.ext.skadn.fidelities[].{field} is required on each fidelity."),
                        join_path(&path, field),
                    ));
                }
            }
        }
        return;
    }

    if version.is_empty() || is_v4 {
        return;
    }
    for field in ["nonce", "timestamp", "signature"] {
        if !non_empty_string(skadn.get(field)) {
            issues.push(issue(
                "openrtb.skadn.field_required",
                Severity::Error,
                format!(
                    "bid.ext.skadn.{field} is required when fidelities is absent (SKAdNetwork \
                     2.0/2.1)."
                ),
                join_path(instance_path, field),
            ));
        }
    }
}

fn non_empty_string(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| !text.is_empty())
}

fn join_path(base: &str, segment: &str) -> String {
    if base.is_empty() {
        String::from(segment)
    } else {
        format!("{base}.{segment}")
    }
}

fn issue(id: &'static str, severity: Severity, message: String, path: String) -> Issue {
    Issue {
        id: String::from(id),
        severity,
        message,
        path: Some(path),
        section: Some(String::from(SECTION)),
    }
}
