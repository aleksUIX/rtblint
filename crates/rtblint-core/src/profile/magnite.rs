//! Magnite DV+ xAPI extensions. Does not describe Streaming or SpringServe.
//! Descriptor types follow the pinned public proto2 model. Optional fields
//! remain optional; integration-time settings are not reconstructed locally.

use super::{join_instance_path, profile_issue, value_at};
use crate::{Issue, Severity};
use serde_json::{Map, Value};

#[derive(Clone, Copy)]
enum Kind {
    I32,
    U32,
    U64,
    Number,
    String,
    Flag,
    Object(&'static [Field]),
    Array(&'static Kind),
    KeyValues,
}

struct Field {
    name: &'static str,
    kind: Kind,
    deprecated: bool,
}

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let fields = match object_name {
        "BidRequest" => BID_REQUEST_EXT,
        "Imp" => IMP_EXT,
        "Video" => VIDEO_EXT,
        "Banner" => BANNER_EXT,
        "App" => APP_EXT,
        "Site" => SITE_EXT,
        "DOOH" => DOOH_EXT,
        "Publisher" => PUBLISHER_EXT,
        "Device" => DEVICE_EXT,
        "Geo" => GEO_EXT,
        "User" => USER_EXT,
        "Regs" => REGS_EXT,
        "Source" => SOURCE_EXT,
        "Native" => NATIVE_EXT,
        "Audio" => AUDIO_EXT,
        "BidResponse" => BID_RESPONSE_EXT,
        "SeatBid" => SEAT_BID_EXT,
        "Bid" => BID_EXT,
        _ => return,
    };
    if let Some(ext) = object.get("ext").filter(|value| !value.is_null()) {
        check_kind(
            ext,
            Kind::Object(fields),
            &join_instance_path(path, "ext"),
            issues,
        );
    }
    match object_name {
        "BidRequest" => {
            for (field, max) in [("ext.rp.badvid", 20), ("ext.rp.baindid", 50)] {
                if value_at(object, field)
                    .and_then(Value::as_array)
                    .is_some_and(|a| a.len() > max)
                {
                    error(
                        "openrtb.profile.magnite.array_limit",
                        &format!("Magnite xAPI {field} permits at most {max} entries."),
                        join_instance_path(path, field),
                        issues,
                    );
                }
            }
        }
        "Video" => enum_string(object, "ext.orientation", &["h", "v"], path, issues),
        "Banner" => enum_string(
            object,
            "ext.rp.mime",
            &["application/javascript", "text/html"],
            path,
            issues,
        ),
        "Site" | "App" => enum_string(
            object,
            "ext.rp.aq.sensitivity",
            &["high", "low"],
            path,
            issues,
        ),
        "Geo" => enum_int(object, "ext.rp.consent", &[0, 1], path, issues),
        "BidResponse" => {
            if let Some(time) = value_at(object, "ext.rp.time").and_then(Value::as_i64) {
                if time < 0 || time % 10 != 0 {
                    error("openrtb.profile.value_invalid", "Magnite xAPI response time is nonnegative and rounded up to a multiple of 10 milliseconds.", join_instance_path(path, "ext.rp.time"), issues);
                }
            }
        }
        "Bid" => {
            enum_string(object, "ext.rp.adtype", &["banner", "video"], path, issues);
            enum_int(object, "ext.rp.estimated", &[0, 1], path, issues);
            enum_int(
                object,
                "ext.rp.response_format",
                &[0, 1, 2, 3, 4],
                path,
                issues,
            );
            check_native_response(object, path, issues);
        }
        "Native" => check_native_request(object, path, issues),
        _ => {}
    }
}

fn check_kind(value: &Value, kind: Kind, path: &str, issues: &mut Vec<Issue>) {
    // Proto2 optional fields also accept an unset/null value in JSON mappings.
    if value.is_null() {
        return;
    }
    let scalar_valid = match kind {
        Kind::I32 => value.as_i64().is_some_and(|v| i32::try_from(v).is_ok()),
        Kind::U32 => value.as_u64().is_some_and(|v| u32::try_from(v).is_ok()),
        Kind::U64 => {
            value.as_u64().is_some()
                || value.as_str().is_some_and(|v| {
                    !v.is_empty()
                        && v.bytes().all(|b| b.is_ascii_digit())
                        && v.parse::<u64>().is_ok()
                })
        }
        Kind::Number => value.is_number(),
        Kind::String => value.is_string(),
        // xAPI's public model uses protobuf bool. Accept integer OpenRTB flag
        // spellings as well until an accessible JSON wire guide resolves them.
        Kind::Flag => value.is_boolean() || matches!(value.as_i64(), Some(0 | 1)),
        Kind::Object(fields) => {
            let Some(object) = value.as_object() else {
                type_error(path, issues);
                return;
            };
            for field in fields {
                if let Some(child) = object.get(field.name) {
                    let child_path = join_instance_path(path, field.name);
                    check_kind(child, field.kind, &child_path, issues);
                    if field.deprecated && !child.is_null() {
                        let mut issue = profile_issue("openrtb.profile.magnite.deprecated", String::from("The public Magnite xAPI model marks this extension field deprecated; no account-side behavior is inferred."), child_path);
                        issue.severity = Severity::Warning;
                        issues.push(issue);
                    }
                }
            }
            return;
        }
        Kind::Array(inner) => {
            let Some(values) = value.as_array() else {
                type_error(path, issues);
                return;
            };
            for (index, child) in values.iter().enumerate() {
                if child.is_null() {
                    type_error(&format!("{path}[{index}]"), issues);
                } else {
                    check_kind(child, *inner, &format!("{path}[{index}]"), issues);
                }
            }
            return;
        }
        Kind::KeyValues => {
            // The public model calls these repeated KeyValuePair messages,
            // while its comments describe JSON key-to-string-array maps.
            if let Some(map) = value.as_object() {
                for (key, values) in map {
                    check_kind(
                        values,
                        Kind::Array(&Kind::String),
                        &join_instance_path(path, key),
                        issues,
                    );
                }
            } else {
                check_kind(
                    value,
                    Kind::Array(&Kind::Object(KEY_VALUE_PAIR)),
                    path,
                    issues,
                );
            }
            return;
        }
    };
    if !scalar_valid {
        type_error(path, issues);
    }
}

fn type_error(path: &str, issues: &mut Vec<Issue>) {
    error("openrtb.profile.magnite.type_invalid", "The supplied Magnite xAPI extension does not match its public field type or integer width.", String::from(path), issues);
}

fn enum_string(
    object: &Map<String, Value>,
    field: &str,
    allowed: &[&str],
    path: &str,
    issues: &mut Vec<Issue>,
) {
    if let Some(value) = value_at(object, field).and_then(Value::as_str) {
        if !allowed.contains(&value) {
            error(
                "openrtb.profile.value_invalid",
                &format!(
                    "Magnite xAPI {field} must be one of {}.",
                    allowed.join(", ")
                ),
                join_instance_path(path, field),
                issues,
            );
        }
    }
}

fn enum_int(
    object: &Map<String, Value>,
    field: &str,
    allowed: &[i64],
    path: &str,
    issues: &mut Vec<Issue>,
) {
    if let Some(value) = value_at(object, field).and_then(Value::as_i64) {
        if !allowed.contains(&value) {
            error(
                "openrtb.profile.value_invalid",
                &format!("Magnite xAPI {field} is outside its documented value set."),
                join_instance_path(path, field),
                issues,
            );
        }
    }
}

fn check_native_request(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(native) = object
        .get("request")
        .and_then(Value::as_str)
        .and_then(crate::native::parse_encoded_object)
    else {
        return;
    };
    if let Some(ext) = native.get("ext") {
        check_kind(
            ext,
            Kind::Object(NATIVE_REQUEST_EXT),
            &join_instance_path(path, "request.ext"),
            issues,
        );
    }
}

fn check_native_response(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(native) = object
        .get("adm")
        .and_then(Value::as_str)
        .and_then(crate::native::parse_encoded_object)
    else {
        return;
    };
    if let Some(trackers) = native.get("eventtrackers").and_then(Value::as_array) {
        for (index, tracker) in trackers.iter().enumerate() {
            if let Some(ext) = tracker.get("ext") {
                check_kind(
                    ext,
                    Kind::Object(EVENT_TRACKER_EXT),
                    &join_instance_path(path, &format!("adm.eventtrackers[{index}].ext")),
                    issues,
                );
            }
        }
    }
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    let Some(seats) = response.get("seatbid").and_then(Value::as_array) else {
        return;
    };
    let Some(imps) = request.get("imp").and_then(Value::as_array) else {
        return;
    };
    for (seat_index, seat) in seats.iter().enumerate() {
        let Some(bids) = seat.get("bid").and_then(Value::as_array) else {
            continue;
        };
        for (bid_index, bid) in bids.iter().enumerate() {
            let Some(bid) = bid.as_object() else { continue };
            let Some(impid) = bid.get("impid").and_then(Value::as_str) else {
                continue;
            };
            let matches: Vec<_> = imps
                .iter()
                .filter(|imp| imp.get("id").and_then(Value::as_str) == Some(impid))
                .collect();
            if matches.len() != 1 {
                continue;
            }
            let Some(imp) = matches[0].as_object() else {
                continue;
            };
            let base = format!("seatbid[{seat_index}].bid[{bid_index}]");
            if imp.get("secure").and_then(Value::as_i64) == Some(1)
                || imp.get("secure").and_then(Value::as_bool) == Some(true)
            {
                if let Some(trackers) =
                    value_at(bid, "ext.rp.imptrackers").and_then(Value::as_array)
                {
                    for (index, tracker) in trackers.iter().enumerate() {
                        if tracker.as_str().is_some_and(|url| {
                            !url.get(..8)
                                .is_some_and(|scheme| scheme.eq_ignore_ascii_case("https://"))
                        }) {
                            error("openrtb.profile.magnite.insecure_tracker", "Magnite xAPI requires HTTPS impression trackers when the referenced impression is secure.", format!("{base}.ext.rp.imptrackers[{index}]"), issues);
                        }
                    }
                }
            }
            // A malformed media container cannot establish what was offered.
            // Keep independent secure-tracker checks and leave its type to request validation.
            if ["banner", "video", "audio", "native"]
                .iter()
                .any(|field| imp.get(*field).is_some_and(|value| !value.is_object()))
            {
                continue;
            }
            let format = value_at(bid, "ext.rp.response_format").and_then(Value::as_i64);
            let adtype = value_at(bid, "ext.rp.adtype").and_then(Value::as_str);
            let media = match (format, adtype) {
                (Some(1), _) => Some("banner"),
                (Some(2), _) => Some("video"),
                (Some(3), _) => Some("native"),
                (Some(4), _) => Some("audio"),
                (_, Some("banner")) => Some("banner"),
                (_, Some("video")) => Some("video"),
                _ => None,
            };
            if let Some(media) = media {
                if !imp.get(media).is_some_and(Value::is_object) {
                    error("openrtb.profile.magnite.media_not_offered", "The Magnite xAPI creative format was not offered by the referenced impression.", join_instance_path(&base, if format.is_some_and(|v| v != 0) { "ext.rp.response_format" } else { "ext.rp.adtype" }), issues);
                }
                if let Some(offered) = imp.get(media).and_then(Value::as_object) {
                    if let (Some(chosen), Some(api)) = (
                        value_at(bid, "ext.rp.creativeapi").and_then(Value::as_u64),
                        offered.get("api").and_then(Value::as_array),
                    ) {
                        if api.iter().all(|v| v.as_u64().is_some())
                            && !api.iter().any(|v| v.as_u64() == Some(chosen))
                        {
                            error("openrtb.profile.magnite.api_not_offered", "The referenced impression does not offer the Magnite creative API framework.", format!("{base}.ext.rp.creativeapi"), issues);
                        }
                    }
                }
            }
            if let Some(banner) = imp.get("banner").and_then(Value::as_object) {
                if let (Some(requested), Some(returned)) = (
                    value_at(banner, "ext.rp.mime").and_then(Value::as_str),
                    value_at(bid, "ext.rp.mime").and_then(Value::as_str),
                ) {
                    if media == Some("banner") && requested != returned {
                        error("openrtb.profile.magnite.mime_mismatch", "The Magnite banner response MIME type differs from the requested markup format.", format!("{base}.ext.rp.mime"), issues);
                    }
                }
                // Slot-name configuration may add sizes outside the captured JSON.
                if media == Some("banner") && value_at(imp, "ext.rp.slot").is_none() {
                    if let (Some(chosen), Some(primary)) = (
                        value_at(bid, "ext.rp.size_id").and_then(Value::as_u64),
                        value_at(banner, "ext.rp.size_id").and_then(Value::as_u64),
                    ) {
                        let alternatives = value_at(banner, "ext.rp.alt_size_ids");
                        let allowed = match alternatives {
                            None => Some(chosen == primary),
                            Some(Value::Array(values))
                                if values.iter().all(|v| v.as_u64().is_some()) =>
                            {
                                Some(
                                    chosen == primary
                                        || values.iter().any(|v| v.as_u64() == Some(chosen)),
                                )
                            }
                            _ => None,
                        };
                        if allowed == Some(false) {
                            error("openrtb.profile.magnite.size_not_offered", "The Magnite banner size ID was not included in the explicit primary or alternate size IDs.", format!("{base}.ext.rp.size_id"), issues);
                        }
                    }
                }
            }
        }
    }
}

fn error(id: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    issues.push(profile_issue(id, String::from(message), path));
}
// Generated from the pinned xAPI proto2 field descriptors. All fields are optional.
const KEY_VALUE_PAIR: &[Field] = &[
    Field {
        name: "key",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "value",
        kind: Kind::Array(&Kind::String),
        deprecated: false,
    },
];
const WINDOW: &[Field] = &[
    Field {
        name: "url",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "depth",
        kind: Kind::I32,
        deprecated: false,
    },
    Field {
        name: "w",
        kind: Kind::U32,
        deprecated: false,
    },
    Field {
        name: "h",
        kind: Kind::U32,
        deprecated: false,
    },
];
const BID_REQUEST_EXT__RP: &[Field] = &[
    Field {
        name: "zone_id",
        kind: Kind::I32,
        deprecated: false,
    },
    Field {
        name: "badvid",
        kind: Kind::Array(&Kind::I32),
        deprecated: false,
    },
    Field {
        name: "baindid",
        kind: Kind::Array(&Kind::I32),
        deprecated: false,
    },
];
const BID_REQUEST_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(BID_REQUEST_EXT__RP),
    deprecated: false,
}];
const IMP_EXT__PROXY_BID: &[Field] = &[
    Field {
        name: "id",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "price",
        kind: Kind::Number,
        deprecated: false,
    },
];
const IMP_EXT__PROXY_DEMAND: &[Field] = &[
    Field {
        name: "marketrate",
        kind: Kind::Number,
        deprecated: false,
    },
    Field {
        name: "bids",
        kind: Kind::Array(&Kind::Object(IMP_EXT__PROXY_BID)),
        deprecated: false,
    },
];
const IMP_EXT__FLOOR: &[Field] = &[
    Field {
        name: "id",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "price",
        kind: Kind::Number,
        deprecated: false,
    },
    Field {
        name: "dpf",
        kind: Kind::Flag,
        deprecated: true,
    },
    Field {
        name: "pmptier",
        kind: Kind::I32,
        deprecated: false,
    },
    Field {
        name: "pmppriority",
        kind: Kind::I32,
        deprecated: false,
    },
];
const IMP_EXT__RP: &[Field] = &[
    Field {
        name: "zone_id",
        kind: Kind::I32,
        deprecated: false,
    },
    Field {
        name: "enc",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "slot",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "target",
        kind: Kind::KeyValues,
        deprecated: false,
    },
    Field {
        name: "track",
        kind: Kind::KeyValues,
        deprecated: false,
    },
    Field {
        name: "rtb",
        kind: Kind::KeyValues,
        deprecated: false,
    },
    Field {
        name: "nolog",
        kind: Kind::KeyValues,
        deprecated: false,
    },
    Field {
        name: "proxydemand",
        kind: Kind::Object(IMP_EXT__PROXY_DEMAND),
        deprecated: false,
    },
    Field {
        name: "floors",
        kind: Kind::Array(&Kind::Object(IMP_EXT__FLOOR)),
        deprecated: false,
    },
    Field {
        name: "gpid",
        kind: Kind::String,
        deprecated: false,
    },
];
const IMP_EXT: &[Field] = &[
    Field {
        name: "rp",
        kind: Kind::Object(IMP_EXT__RP),
        deprecated: false,
    },
    Field {
        name: "window",
        kind: Kind::Object(WINDOW),
        deprecated: false,
    },
    Field {
        name: "viewabilityvendors",
        kind: Kind::Array(&Kind::String),
        deprecated: false,
    },
    Field {
        name: "gpid",
        kind: Kind::String,
        deprecated: false,
    },
];
const VIDEO_EXT__RP: &[Field] = &[Field {
    name: "size_id",
    kind: Kind::U32,
    deprecated: false,
}];
const VIDEO_EXT__AD: &[Field] = &[
    Field {
        name: "w",
        kind: Kind::I32,
        deprecated: false,
    },
    Field {
        name: "h",
        kind: Kind::I32,
        deprecated: false,
    },
];
const VIDEO_EXT: &[Field] = &[
    Field {
        name: "orientation",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "rp",
        kind: Kind::Object(VIDEO_EXT__RP),
        deprecated: false,
    },
    Field {
        name: "ad",
        kind: Kind::Object(VIDEO_EXT__AD),
        deprecated: false,
    },
    Field {
        name: "skip",
        kind: Kind::Flag,
        deprecated: true,
    },
    Field {
        name: "skipdelay",
        kind: Kind::I32,
        deprecated: true,
    },
    Field {
        name: "placement",
        kind: Kind::I32,
        deprecated: false,
    },
];
const BANNER_EXT__RP: &[Field] = &[
    Field {
        name: "size_id",
        kind: Kind::U32,
        deprecated: false,
    },
    Field {
        name: "alt_size_ids",
        kind: Kind::Array(&Kind::U32),
        deprecated: false,
    },
    Field {
        name: "mime",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "usenurl",
        kind: Kind::Flag,
        deprecated: true,
    },
    Field {
        name: "useimptrackers",
        kind: Kind::Flag,
        deprecated: false,
    },
];
const BANNER_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(BANNER_EXT__RP),
    deprecated: false,
}];
const AD_QUALITY: &[Field] = &[Field {
    name: "sensitivity",
    kind: Kind::String,
    deprecated: false,
}];
const APP_EXT__RP: &[Field] = &[
    Field {
        name: "site_id",
        kind: Kind::U32,
        deprecated: false,
    },
    Field {
        name: "aq",
        kind: Kind::Object(AD_QUALITY),
        deprecated: false,
    },
    Field {
        name: "blocklists",
        kind: Kind::Array(&Kind::String),
        deprecated: false,
    },
];
const APP_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(APP_EXT__RP),
    deprecated: false,
}];
const SITE_EXT__RP: &[Field] = &[
    Field {
        name: "site_id",
        kind: Kind::U32,
        deprecated: false,
    },
    Field {
        name: "aq",
        kind: Kind::Object(AD_QUALITY),
        deprecated: false,
    },
    Field {
        name: "blocklists",
        kind: Kind::Array(&Kind::String),
        deprecated: false,
    },
];
const SITE_EXT: &[Field] = &[
    Field {
        name: "rp",
        kind: Kind::Object(SITE_EXT__RP),
        deprecated: false,
    },
    Field {
        name: "window",
        kind: Kind::Object(WINDOW),
        deprecated: false,
    },
];
const DOOH_EXT__RP: &[Field] = &[
    Field {
        name: "site_id",
        kind: Kind::U32,
        deprecated: false,
    },
    Field {
        name: "blocklists",
        kind: Kind::Array(&Kind::String),
        deprecated: false,
    },
];
const DOOH_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(DOOH_EXT__RP),
    deprecated: false,
}];
const PUBLISHER_EXT__RP: &[Field] = &[Field {
    name: "account_id",
    kind: Kind::I32,
    deprecated: false,
}];
const PUBLISHER_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(PUBLISHER_EXT__RP),
    deprecated: false,
}];
const DEVICE_EXT__RP: &[Field] = &[
    Field {
        name: "res",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "pixelratio",
        kind: Kind::Number,
        deprecated: false,
    },
    Field {
        name: "xff",
        kind: Kind::String,
        deprecated: false,
    },
];
const DEVICE_EXT__DOOH: &[Field] = &[Field {
    name: "impmultiply",
    kind: Kind::Number,
    deprecated: false,
}];
const DEVICE_EXT: &[Field] = &[
    Field {
        name: "rp",
        kind: Kind::Object(DEVICE_EXT__RP),
        deprecated: false,
    },
    Field {
        name: "ipmask",
        kind: Kind::Flag,
        deprecated: false,
    },
    Field {
        name: "dooh",
        kind: Kind::Object(DEVICE_EXT__DOOH),
        deprecated: false,
    },
];
const GEO_EXT__RP: &[Field] = &[Field {
    name: "consent",
    kind: Kind::I32,
    deprecated: false,
}];
const GEO_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(GEO_EXT__RP),
    deprecated: false,
}];
const USER_EXT__DT: &[Field] = &[
    Field {
        name: "id",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "pref",
        kind: Kind::U64,
        deprecated: false,
    },
    Field {
        name: "keyv",
        kind: Kind::U32,
        deprecated: false,
    },
];
const USER_EXT__RP: &[Field] = &[Field {
    name: "target",
    kind: Kind::KeyValues,
    deprecated: false,
}];
const USER_EXT: &[Field] = &[
    Field {
        name: "dt",
        kind: Kind::Object(USER_EXT__DT),
        deprecated: true,
    },
    Field {
        name: "rp",
        kind: Kind::Object(USER_EXT__RP),
        deprecated: false,
    },
    Field {
        name: "liveramp_idl",
        kind: Kind::String,
        deprecated: false,
    },
];
const REGS_EXT: &[Field] = &[Field {
    name: "s22580",
    kind: Kind::Flag,
    deprecated: false,
}];
const SOURCE_EXT: &[Field] = &[Field {
    name: "ssreq",
    kind: Kind::Flag,
    deprecated: false,
}];
const NATIVE_EXT__RP: &[Field] = &[Field {
    name: "size_id",
    kind: Kind::U32,
    deprecated: true,
}];
const NATIVE_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(NATIVE_EXT__RP),
    deprecated: false,
}];
const NATIVE_REQUEST_EXT: &[Field] = &[Field {
    name: "privacy",
    kind: Kind::Flag,
    deprecated: false,
}];
const AUDIO_EXT__RP: &[Field] = &[Field {
    name: "size_id",
    kind: Kind::U32,
    deprecated: false,
}];
const AUDIO_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(AUDIO_EXT__RP),
    deprecated: false,
}];
const BID_RESPONSE_EXT__RP: &[Field] = &[
    Field {
        name: "statuscode",
        kind: Kind::I32,
        deprecated: false,
    },
    Field {
        name: "statusmsg",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "time",
        kind: Kind::I32,
        deprecated: false,
    },
];
const BID_RESPONSE_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(BID_RESPONSE_EXT__RP),
    deprecated: false,
}];
const SEAT_BID_EXT__RP: &[Field] = &[Field {
    name: "buyer",
    kind: Kind::String,
    deprecated: false,
}];
const SEAT_BID_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(SEAT_BID_EXT__RP),
    deprecated: false,
}];
const BID_EXT__RP: &[Field] = &[
    Field {
        name: "advid",
        kind: Kind::I32,
        deprecated: false,
    },
    Field {
        name: "mime",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "adtype",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "size_id",
        kind: Kind::U32,
        deprecated: false,
    },
    Field {
        name: "targeting",
        kind: Kind::KeyValues,
        deprecated: false,
    },
    Field {
        name: "creativeapi",
        kind: Kind::U32,
        deprecated: false,
    },
    Field {
        name: "viewabilityvendors",
        kind: Kind::Array(&Kind::String),
        deprecated: false,
    },
    Field {
        name: "estimated",
        kind: Kind::I32,
        deprecated: false,
    },
    Field {
        name: "adjustbid",
        kind: Kind::Number,
        deprecated: false,
    },
    Field {
        name: "deprecated_pmptier",
        kind: Kind::String,
        deprecated: true,
    },
    Field {
        name: "aqid",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "aindid",
        kind: Kind::Array(&Kind::I32),
        deprecated: false,
    },
    Field {
        name: "burl",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "imptrackers",
        kind: Kind::Array(&Kind::String),
        deprecated: false,
    },
    Field {
        name: "pmptier",
        kind: Kind::I32,
        deprecated: false,
    },
    Field {
        name: "response_format",
        kind: Kind::U32,
        deprecated: false,
    },
];
const BID_EXT: &[Field] = &[Field {
    name: "rp",
    kind: Kind::Object(BID_EXT__RP),
    deprecated: false,
}];
const EVENT_TRACKER_EXT: &[Field] = &[
    Field {
        name: "verification_parameters",
        kind: Kind::String,
        deprecated: false,
    },
    Field {
        name: "vendorKey",
        kind: Kind::String,
        deprecated: false,
    },
];
