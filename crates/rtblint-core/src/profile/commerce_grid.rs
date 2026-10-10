//! Supplier ingest for Criteo Commerce Grid's custom server-to-server endpoint.

use serde_json::{Map, Value};

use super::{
    contract::{self, Field, Kind},
    join_instance_path, require_integer_in_range, value_at,
};
use crate::Issue;

const IMP_EXT: &[Field] = &[
    Field {
        name: "gpid",
        kind: Kind::String,
    },
    Field {
        name: "publisher_sub_id",
        kind: Kind::String,
    },
    Field {
        name: "bidder",
        kind: Kind::Object(&[Field {
            name: "uid",
            kind: Kind::String,
        }]),
    },
    Field {
        name: "skadn",
        kind: Kind::Object(&[
            Field {
                name: "version",
                kind: Kind::String,
            },
            Field {
                name: "sourceapp",
                kind: Kind::String,
            },
            Field {
                name: "skadnetids",
                kind: Kind::Array(&Kind::String),
            },
        ]),
    },
];

const NATIVE_IMAGE: &[Field] = &[
    Field {
        name: "type",
        kind: Kind::Integer,
    },
    Field {
        name: "w",
        kind: Kind::Integer,
    },
    Field {
        name: "h",
        kind: Kind::Integer,
    },
    Field {
        name: "wmin",
        kind: Kind::Integer,
    },
    Field {
        name: "hmin",
        kind: Kind::Integer,
    },
    Field {
        name: "ext",
        kind: Kind::Object(&[]),
    },
];
const NATIVE_VIDEO: &[Field] = &[
    Field {
        name: "mimes",
        kind: Kind::Array(&Kind::String),
    },
    Field {
        name: "minduration",
        kind: Kind::Integer,
    },
    Field {
        name: "maxduration",
        kind: Kind::Integer,
    },
    Field {
        name: "protocols",
        kind: Kind::Array(&Kind::Integer),
    },
    Field {
        name: "ext",
        kind: Kind::Object(&[]),
    },
];
const NATIVE_ASSET: &[Field] = &[
    Field {
        name: "id",
        kind: Kind::Integer,
    },
    Field {
        name: "required",
        kind: Kind::Integer,
    },
    Field {
        name: "title",
        kind: Kind::Object(&[
            Field {
                name: "len",
                kind: Kind::Integer,
            },
            Field {
                name: "ext",
                kind: Kind::Object(&[]),
            },
        ]),
    },
    Field {
        name: "img",
        kind: Kind::Object(NATIVE_IMAGE),
    },
    Field {
        name: "video",
        kind: Kind::Object(NATIVE_VIDEO),
    },
    Field {
        name: "data",
        kind: Kind::Object(&[
            Field {
                name: "type",
                kind: Kind::Integer,
            },
            Field {
                name: "len",
                kind: Kind::Integer,
            },
            Field {
                name: "ext",
                kind: Kind::Object(&[]),
            },
        ]),
    },
    Field {
        name: "ext",
        kind: Kind::Object(&[]),
    },
];
const NATIVE_REQUEST: &[Field] = &[
    Field {
        name: "ver",
        kind: Kind::String,
    },
    Field {
        name: "context",
        kind: Kind::Integer,
    },
    Field {
        name: "contextsubtype",
        kind: Kind::Integer,
    },
    Field {
        name: "plcmttype",
        kind: Kind::Integer,
    },
    Field {
        name: "plcmtcnt",
        kind: Kind::Integer,
    },
    Field {
        name: "assets",
        kind: Kind::Array(&Kind::Object(NATIVE_ASSET)),
    },
    Field {
        name: "aurlsupport",
        kind: Kind::Integer,
    },
    Field {
        name: "privacy",
        kind: Kind::Integer,
    },
    Field {
        name: "eventtrackers",
        kind: Kind::Array(&Kind::Object(&[
            Field {
                name: "event",
                kind: Kind::Integer,
            },
            // The linked Native 1.2 schema uses methods[], unlike the guide's singular row.
            Field {
                name: "methods",
                kind: Kind::Array(&Kind::Integer),
            },
            Field {
                name: "ext",
                kind: Kind::Object(&[]),
            },
        ])),
    },
    Field {
        name: "ext",
        kind: Kind::Object(&[]),
    },
    Field {
        name: "api",
        kind: Kind::Array(&Kind::Integer),
    },
];

/// Types the decoded Native 1.2 payload for every documented wire carrier.
pub(super) fn validate_native(root: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let start = issues.len();
    contract::fields(root, NATIVE_REQUEST, path, issues);
    for issue in &mut issues[start..] {
        issue.section = Some("https://docs.commercegrid.criteo.com/kb/guide/en/custom-server-to-server-openrtb-Vy9QrGVwyl/Steps/2366154".to_owned());
    }
}

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    match object_name {
        "BidRequest" => request(object, path, issues),
        "Source" => {
            for field in ["ext.sourcetype", "ext.sourceorigin", "schain"] {
                required(object, field, path, issues);
            }
            require_integer_in_range(object, "ext.sourcetype", 1, 3, "Commerce Grid source type is 1 (not header bidding), 2 (client header bidding), or 3 (server header bidding).", path, issues);
            require_integer_in_range(
                object,
                "ext.sourceorigin",
                1,
                7,
                "Commerce Grid sourceorigin must identify a documented origin from 1 through 7.",
                path,
                issues,
            );
        }
        "Imp" => {
            required(object, "instl", path, issues);
            contract::fields(
                object,
                &[Field {
                    name: "ext",
                    kind: Kind::Object(IMP_EXT),
                }],
                path,
                issues,
            );
            if object.get("video").is_some_and(Value::is_object) {
                for field in ["displaymanager", "displaymanagerver"] {
                    required(object, field, path, issues);
                }
            }
            if let Some(skadn) = value_at(object, "ext.skadn").and_then(Value::as_object) {
                let base = join_instance_path(path, "ext.skadn");
                for field in ["version", "sourceapp", "skadnetids"] {
                    required(skadn, field, &base, issues);
                }
            }
            // Native decoding and privacy validation are handled by the source-scoped generic hook.
        }
        "Metric" => {
            for field in ["type", "value", "vendor"] {
                required(object, field, path, issues);
            }
            if object
                .get("value")
                .and_then(Value::as_f64)
                .is_some_and(|value| !(0.0..=1.0).contains(&value))
            {
                error(
                    "metric_value",
                    "Commerce Grid metric values must be between 0 and 1.",
                    join_instance_path(path, "value"),
                    issues,
                );
            }
        }
        "Banner" => {
            required(object, "format", path, issues);
            for field in ["w", "h"] {
                required(object, field, path, issues);
            }
            let Some(formats) = object.get("format").and_then(Value::as_array) else {
                return;
            };
            if formats.is_empty() {
                error(
                    "banner_format",
                    "Commerce Grid requires banner formats, even for a single size.",
                    join_instance_path(path, "format"),
                    issues,
                );
            }
            if let Some(first) = formats.first().and_then(Value::as_object) {
                for field in ["w", "h"] {
                    if let (Some(banner), Some(format)) = (
                        object.get(field).and_then(Value::as_i64),
                        first.get(field).and_then(Value::as_i64),
                    ) {
                        if banner != format {
                            error("banner_format", "Commerce Grid banner dimensions must match the first format's dimensions.", join_instance_path(path, field), issues);
                        }
                    }
                }
            }
        }
        "Format" => {
            for field in ["w", "h"] {
                required(object, field, path, issues);
                require_integer_in_range(
                    object,
                    field,
                    1,
                    i64::MAX,
                    "Commerce Grid format dimensions must be positive integers.",
                    path,
                    issues,
                );
            }
        }
        "Video" => {
            for field in [
                "mimes",
                "protocols",
                "w",
                "h",
                "plcmt",
                "skip",
                "playbackmethod",
                "api",
            ] {
                required(object, field, path, issues);
            }
            // Exact durations are expressly an alternative to maxduration.
            let precise = object
                .get("rqddurs")
                .and_then(Value::as_array)
                .is_some_and(|durations| !durations.is_empty());
            if !precise {
                for field in ["minduration", "maxduration"] {
                    required(object, field, path, issues);
                }
            }
        }
        "Pmp" | "PMP" => {
            if object
                .get("deals")
                .and_then(Value::as_array)
                .is_some_and(|deals| !deals.is_empty())
            {
                required(object, "private_auction", path, issues);
            }
        }
        "Deal" => required(object, "bidfloor", path, issues),
        "Site" => {
            required(object, "publisher", path, issues);
            // The page row explicitly permits absence or an empty string when unknown.
        }
        "App" => required(object, "bundle", path, issues),
        "Device" => {
            for field in ["ua", "geo", "dnt", "lmt", "ip"] {
                required(object, field, path, issues);
            }
        }
        "Geo" => required(object, "country", path, issues),
        "Regs" => {
            contract::integer_enum(object, "ext.gdpr", &[0, 1], path, issues);
            contract::fields(
                object,
                &[Field {
                    name: "ext",
                    kind: Kind::Object(&[
                        Field {
                            name: "us_privacy",
                            kind: Kind::String,
                        },
                        Field {
                            name: "gpp",
                            kind: Kind::String,
                        },
                    ]),
                }],
                path,
                issues,
            );
        }
        "User" => contract::fields(
            object,
            &[Field {
                name: "ext",
                kind: Kind::Object(&[Field {
                    name: "consent",
                    kind: Kind::String,
                }]),
            }],
            path,
            issues,
        ),
        _ => {}
    }
}

fn request(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    for field in ["device", "user", "cur", "source"] {
        required(object, field, path, issues);
    }
    if object
        .get("cur")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        error(
            "currency_offer",
            "Commerce Grid requires at least one allowed bid currency.",
            join_instance_path(path, "cur"),
            issues,
        );
    }
    if contract::present(object, "wlang") && contract::present(object, "wlangb") {
        error(
            "language_conflict",
            "Commerce Grid accepts only one of wlang and wlangb.",
            join_instance_path(path, "wlangb"),
            issues,
        );
    }
    contract::integer_enum(object, "cattax", &[1], path, issues);
    let in_app = object.get("app").is_some_and(Value::is_object);
    let web = object.get("site").is_some_and(Value::is_object);
    if web {
        required(object, "user.buyeruid", path, issues);
    }
    if in_app {
        required(object, "device.os", path, issues);
        // A required identifier in this ingest contract may be scrubbed; no live identifier is demanded.
        required(object, "device.ifa", path, issues);
    }
    if let Some(imps) = object.get("imp").and_then(Value::as_array) {
        for (index, imp) in imps.iter().enumerate() {
            let Some(imp) = imp.as_object() else { continue };
            let ip = format!("{}imp[{index}]", super::prefix(path));
            if in_app {
                for field in [
                    "displaymanager",
                    "displaymanagerver",
                    "clickbrowser",
                    "rwdd",
                ] {
                    required(imp, field, &ip, issues);
                }
            }
            if let (Some(sourceapp), Some(bundle)) = (
                value_at(imp, "ext.skadn.sourceapp").and_then(Value::as_str),
                value_at(object, "app.bundle").and_then(Value::as_str),
            ) {
                if sourceapp != bundle {
                    error(
                        "skadn_mismatch",
                        "Commerce Grid SKAdNetwork sourceapp must match publisher app.bundle.",
                        join_instance_path(&ip, "ext.skadn.sourceapp"),
                        issues,
                    );
                }
            }
        }
    }
}

fn required(object: &Map<String, Value>, field: &str, path: &str, issues: &mut Vec<Issue>) {
    contract::required(object, field, path, "Commerce Grid supplier ingest", issues);
}

fn error(suffix: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    contract::error(
        &format!("openrtb.profile.commerce_grid.{suffix}"),
        message,
        path,
        issues,
    );
}
