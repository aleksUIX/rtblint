//! OpenWrap's published custom wire models and explicitly selected CTV endpoint.
use super::contract;
use super::{join_instance_path, value_at};
use crate::Issue;
use serde_json::{Map, Value};

const ROOT: &str = "https://github.com/PubMatic-OpenWrap/prebid-server/blob/24ff50ca1f00ca1e80ab80bcfa1c812c35f87797/";

#[derive(Clone, Copy)]
enum Wire {
    String,
    Integer,
    Int8,
    Int32,
    Number,
    Boolean,
    Object(&'static [Field]),
    Array(&'static Wire),
    Map(&'static Wire),
    Opaque,
}

struct Field {
    name: &'static str,
    wire: Wire,
}

macro_rules! f {
    ($name:literal, $wire:expr) => {
        Field {
            name: $name,
            wire: $wire,
        }
    };
}

static ADPOD: &[Field] = &[
    f!("minads", Wire::Integer),
    f!("maxads", Wire::Integer),
    f!("adminduration", Wire::Integer),
    f!("admaxduration", Wire::Integer),
    f!("excladv", Wire::Integer),
    f!("excliabcat", Wire::Integer),
];
static REQUEST_ADPOD: &[Field] = &[
    f!("minads", Wire::Integer),
    f!("maxads", Wire::Integer),
    f!("adminduration", Wire::Integer),
    f!("admaxduration", Wire::Integer),
    f!("excladv", Wire::Integer),
    f!("excliabcat", Wire::Integer),
    f!("crosspodexcladv", Wire::Integer),
    f!("crosspodexcliabcat", Wire::Integer),
    f!("excliabcatwindow", Wire::Integer),
    f!("excladvwindow", Wire::Integer),
];
static WRAPPER: &[Field] = &[
    f!("profileid", Wire::Integer),
    f!("versionid", Wire::Integer),
    f!("ssauction", Wire::Integer),
    f!("sumry_disable", Wire::Integer),
    f!("clientconfig", Wire::Integer),
    f!("supportdeals", Wire::Boolean),
    f!("includebrandcategory", Wire::Integer),
    f!("abtest", Wire::Integer),
    f!("wiid", Wire::String),
    f!("ssai", Wire::String),
    f!("kv", Wire::Map(&Wire::Opaque)),
    f!("video", Wire::Object(&[f!("adrule", Wire::Boolean)])),
    f!("sdksubintegration", Wire::Integer),
    f!("edsstatus", Wire::Integer),
];
static PREBID: &[Field] = &[
    f!(
        "transparency",
        Wire::Object(&[f!(
            "content",
            Wire::Map(&Wire::Object(&[
                f!("include", Wire::Boolean),
                f!("keys", Wire::Array(&Wire::String)),
            ]))
        )])
    ),
    f!("keyval", Wire::Map(&Wire::Opaque)),
    f!("tracker_disabled", Wire::Boolean),
    f!("googlessufeature", Wire::Boolean),
    f!("debug_override", Wire::Boolean),
];
static REQUEST: &[Field] = &[
    f!("wrapper", Wire::Object(WRAPPER)),
    f!("bidder", Wire::Map(&Wire::Map(&Wire::Opaque))),
    f!("adpod", Wire::Object(REQUEST_ADPOD)),
    f!("prebid", Wire::Object(PREBID)),
];
static IMP: &[Field] = &[
    f!(
        "wrapper",
        Wire::Object(&[f!("adserverurl", Wire::String), f!("div", Wire::String)])
    ),
    f!("reward", Wire::Int8),
    f!(
        "bidder",
        Wire::Map(&Wire::Object(&[
            f!(
                "keywords",
                Wire::Array(&Wire::Object(&[
                    f!("key", Wire::String),
                    f!("value", Wire::Array(&Wire::String)),
                ]))
            ),
            f!(
                "dealtier",
                Wire::Object(&[f!("prefix", Wire::String), f!("minDealTier", Wire::Integer),])
            ),
        ]))
    ),
    f!("data", Wire::Object(&[])),
    f!("gpid", Wire::String),
    f!("prebid", Wire::Object(&[])),
    f!("owsdk", Wire::Map(&Wire::Opaque)),
    f!("billing_id", Wire::Array(&Wire::String)),
    f!("publisher_setting_list_id", Wire::Array(&Wire::String)),
    f!("allowed_vendor_type", Wire::Array(&Wire::Integer)),
    f!(
        "excluded_creatives",
        Wire::Array(&Wire::Object(&[f!("buyer_creative_id", Wire::String)]))
    ),
    f!("is_app_open_ad", Wire::Int8),
    f!("allowed_restricted_category", Wire::Array(&Wire::Integer)),
    f!(
        "creative_enforcement_settings",
        Wire::Object(&[
            f!("policy_enforcement", Wire::Integer),
            f!("scan_enforcement", Wire::Integer),
            f!("publisher_blocks_enforcement", Wire::Integer),
        ])
    ),
    f!("dfp_ad_unit_code", Wire::String),
];
static VIDEO: &[Field] = &[
    f!("offset", Wire::Integer),
    f!("adpod", Wire::Object(ADPOD)),
];
static BANNER: &[Field] = &[f!(
    "flexslot",
    Wire::Object(&[
        f!("wmin", Wire::Int32),
        f!("wmax", Wire::Int32),
        f!("hmin", Wire::Int32),
        f!("hmax", Wire::Int32),
    ])
)];
static BID_VIDEO: &[Field] = &[
    f!("minduration", Wire::Integer),
    f!("maxduration", Wire::Integer),
    f!("skip", Wire::Int8),
    f!("skipmin", Wire::Integer),
    f!("skipafter", Wire::Integer),
    f!("battr", Wire::Array(&Wire::Integer)),
    f!("playbackmethod", Wire::Array(&Wire::Integer)),
];
static BID: &[Field] = &[
    f!("errorCode", Wire::Integer),
    f!("errorMessage", Wire::String),
    f!("refreshInterval", Wire::Integer),
    f!("crtype", Wire::String),
    f!(
        "summary",
        Wire::Array(&Wire::Object(&[
            f!("vastTagID", Wire::String),
            f!("bidder", Wire::String),
            f!("bid", Wire::Number),
            f!("errorCode", Wire::Integer),
            f!("errorMessage", Wire::String),
            f!("width", Wire::Integer),
            f!("height", Wire::Integer),
            f!("regex", Wire::String),
        ]))
    ),
    f!("video", Wire::Object(BID_VIDEO)),
    f!("banner", Wire::Object(&[])),
    f!("dspid", Wire::Integer),
    f!("winner", Wire::Integer),
    f!("netecpm", Wire::Number),
    f!("origbidcpm", Wire::Number),
    f!("origbidcur", Wire::String),
    f!("origbidcpmusd", Wire::Number),
    f!("fsc", Wire::Integer),
    // Both production response writers use this container with different keys.
    f!(
        "adpod",
        Wire::Object(&[
            f!("isAdpodBid", Wire::Boolean),
            f!("targeting", Wire::Map(&Wire::String)),
            f!(
                "debug",
                Wire::Object(&[
                    f!("Targeting", Wire::Map(&Wire::String)),
                    f!("targeting", Wire::Map(&Wire::String)),
                ])
            ),
            f!("aprc", Wire::Integer),
            f!("refbids", Wire::Array(&Wire::String)),
        ])
    ),
    f!("ibv", Wire::Boolean),
    f!("clicktrackers", Wire::Array(&Wire::String)),
    f!("owsdk", Wire::Map(&Wire::Opaque)),
    f!("act", Wire::Integer),
    f!("bidexp_enf", Wire::Integer),
];

fn fields(object: &Map<String, Value>, schema: &[Field], path: &str, issues: &mut Vec<Issue>) {
    for field in schema {
        if let Some(value) = object.get(field.name) {
            wire(
                value,
                field.wire,
                &join_instance_path(path, field.name),
                issues,
            );
        }
    }
}

fn wire(value: &Value, kind: Wire, path: &str, issues: &mut Vec<Issue>) {
    // encoding/json accepts null into scalar, pointer, slice and map targets.
    if value.is_null() {
        return;
    }
    let valid = match kind {
        Wire::String => value.is_string(),
        Wire::Integer => value.as_i64().is_some(),
        Wire::Int8 => value.as_i64().is_some_and(|v| (-128..=127).contains(&v)),
        Wire::Int32 => value
            .as_i64()
            .is_some_and(|v| (i32::MIN as i64..=i32::MAX as i64).contains(&v)),
        Wire::Number => value.is_number(),
        Wire::Boolean => value.is_boolean(),
        Wire::Opaque => true,
        Wire::Object(schema) => {
            if let Some(object) = value.as_object() {
                fields(object, schema, path, issues);
                return;
            }
            false
        }
        Wire::Array(item) => {
            if let Some(values) = value.as_array() {
                for (index, value) in values.iter().enumerate() {
                    wire(value, *item, &format!("{path}[{index}]"), issues);
                }
                return;
            }
            false
        }
        Wire::Map(item) => {
            if let Some(values) = value.as_object() {
                for (key, value) in values {
                    wire(value, *item, &join_instance_path(path, key), issues);
                }
                return;
            }
            false
        }
    };
    if !valid {
        contract::error(
            "openrtb.profile.field_type",
            "The field does not match OpenWrap's published JSON wire type or integer width.",
            path.to_owned(),
            issues,
        );
    }
}

fn extension(object: &Map<String, Value>, schema: &[Field], path: &str, issues: &mut Vec<Issue>) {
    if let Some(ext) = object.get("ext") {
        if let Some(object) = ext.as_object() {
            fields(object, schema, &join_instance_path(path, "ext"), issues);
        } else if !ext.is_null() {
            contract::error(
                "openrtb.profile.field_type",
                "OpenWrap's published extension wire container must be an object.",
                join_instance_path(path, "ext"),
                issues,
            );
        }
    }
}

fn zero_default(object: &Map<String, Value>, key: &str, default: i64) -> Option<i64> {
    match object.get(key) {
        None | Some(Value::Null) => Some(default),
        Some(value) => value.as_i64().map(|v| if v == 0 { default } else { v }),
    }
}

fn ctv_adpod(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let min = zero_default(object, "minads", 1);
    let max = zero_default(object, "maxads", 3);
    for (key, value) in [("minads", min), ("maxads", max)] {
        if value.is_some_and(|v| v < 0) {
            contract::error("openrtb.profile.openwrap.adpod_count", "CTV ad counts must be positive after OpenWrap's zero defaults (minads 1, maxads 3).", join_instance_path(path, key), issues);
        }
    }
    let durations: Vec<_> = ["adminduration", "admaxduration"]
        .iter()
        .map(|key| {
            let value = object.get(*key).filter(|v| !v.is_null());
            let duration = value.map_or(Some(0), Value::as_i64);
            if duration.is_some_and(|v| v <= 0) {
                contract::error(
                    "openrtb.profile.openwrap.adpod_duration",
                    "An explicit CTV adpod requires positive adminduration and admaxduration.",
                    join_instance_path(path, key),
                    issues,
                );
            }
            duration
        })
        .collect();
    for key in ["excladv", "excliabcat"] {
        if object
            .get(key)
            .and_then(Value::as_i64)
            .is_some_and(|v| !(0..=100).contains(&v))
        {
            contract::error(
                "openrtb.profile.openwrap.adpod_exclusion",
                "CTV exclusion percentages must be between 0 and 100.",
                join_instance_path(path, key),
                issues,
            );
        }
    }
    if matches!((min, max), (Some(min), Some(max)) if min > 0 && max > 0 && min > max) {
        contract::error(
            "openrtb.profile.openwrap.adpod_count_order",
            "CTV minads cannot exceed maxads after applying defaults.",
            join_instance_path(path, "minads"),
            issues,
        );
    }
    if matches!((durations[0], durations[1]), (Some(min), Some(max)) if min > 0 && max > 0 && min > max)
    {
        contract::error(
            "openrtb.profile.openwrap.adpod_duration_order",
            "CTV adminduration cannot exceed admaxduration.",
            join_instance_path(path, "adminduration"),
            issues,
        );
    }
}

fn ctv_schain(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(schain) = object.get("schain").and_then(Value::as_object) else {
        return;
    };
    let nodes = schain.get("nodes").and_then(Value::as_array);
    let dropped = schain.get("ver").and_then(Value::as_str) != Some("1.0")
        || !zero_default(schain, "complete", 0).is_some_and(|v| matches!(v, 0 | 1))
        || nodes.map_or(true, |nodes| {
            nodes.is_empty()
                || nodes.iter().any(|node| {
                    let text = |key| node.get(key).and_then(Value::as_str).unwrap_or("");
                    text("asi").is_empty()
                        || text("sid").is_empty()
                        || text("sid").chars().count() > 64
                        || node.get("hp").and_then(Value::as_i64) != Some(1)
                })
        });
    if dropped {
        contract::warning("openrtb.profile.openwrap.schain_removed", "The CTV middleware removes this supply chain because its helper requires version 1.0, a complete flag, nonempty nodes, nonempty ASI/SID, SID at most 64 characters, and hp 1.", join_instance_path(path, "schain"), issues);
    }
}

pub(super) fn validate(
    ctv: bool,
    name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let start = issues.len();
    let source = match name {
        "BidRequest" => {
            extension(object, REQUEST, path, issues);
            if ctv {
                let has_video = object
                    .get("imp")
                    .and_then(Value::as_array)
                    .is_some_and(|imps| {
                        imps.iter()
                            .any(|imp| imp.get("video").is_some_and(Value::is_object))
                    });
                if !has_video {
                    contract::error("openrtb.profile.openwrap.ctv_video_required", "The selected OpenWrap CTV endpoint requires at least one video impression.", join_instance_path(path, "imp"), issues);
                }
            }
            "modules/pubmatic/openwrap/models/request.go"
        }
        "Imp" => {
            extension(object, IMP, path, issues);
            "modules/pubmatic/openwrap/models/request.go"
        }
        "Video" => {
            extension(object, VIDEO, path, issues);
            if ctv {
                if let Some(adpod) = value_at(object, "ext.adpod").and_then(Value::as_object) {
                    ctv_adpod(adpod, &join_instance_path(path, "ext.adpod"), issues);
                }
            }
            "modules/pubmatic/openwrap/adpod/adpod.go"
        }
        "Banner" => {
            extension(object, BANNER, path, issues);
            "openrtb_ext/pubmatic_ow.go"
        }
        "Regs" => {
            extension(
                object,
                &[f!("gdpr", Wire::Integer), f!("us_privacy", Wire::String)],
                path,
                issues,
            );
            "modules/pubmatic/openwrap/models/request.go"
        }
        "User" => {
            extension(object, &[f!("consent", Wire::String)], path, issues);
            "openrtb_ext/user.go"
        }
        "Source" => {
            extension(
                object,
                &[f!("omidpv", Wire::String), f!("omidpn", Wire::String)],
                path,
                issues,
            );
            if ctv {
                ctv_schain(object, path, issues);
            }
            "modules/pubmatic/openwrap/models/source.go"
        }
        "Bid" => {
            extension(object, BID, path, issues);
            if let Some(skip) = value_at(object, "ext.video.skip").and_then(Value::as_i64) {
                if !matches!(skip, 0 | 1) {
                    contract::error(
                        "openrtb.profile.value_invalid",
                        "OpenWrap bid.ext.video.skip is a documented 0/1 flag.",
                        join_instance_path(path, "ext.video.skip"),
                        issues,
                    );
                }
            }
            "modules/pubmatic/openwrap/models/response.go"
        }
        _ => return,
    };
    for issue in &mut issues[start..] {
        let source = if issue.id == "openrtb.profile.openwrap.ctv_video_required"
            || issue.id == "openrtb.profile.openwrap.schain_removed"
        {
            "modules/pubmatic/openwrap/endpoints/legacy/ctv/video.go"
        } else {
            source
        };
        issue.section = Some(format!("{ROOT}{source}"));
    }
}
