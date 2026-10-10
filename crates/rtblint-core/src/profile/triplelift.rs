//! TripleLift's September 2026 supplier contract and Native Ads 1.2 tables.
use super::contract::{self, Field, Kind};
use super::{join_instance_path, value_at};
use crate::Issue;
use serde_json::{Map, Value};

macro_rules! f {
    ($name:literal, $kind:expr) => {
        Field {
            name: $name,
            kind: $kind,
        }
    };
}

static REQUEST_IMAGE: &[Field] = &[
    f!("type", Kind::Integer),
    f!("w", Kind::Integer),
    f!("wmin", Kind::Integer),
    f!("h", Kind::Integer),
    f!("hmin", Kind::Integer),
    f!("mimes", Kind::Array(&Kind::String)),
];
static REQUEST_ASSET: &[Field] = &[
    f!("id", Kind::Integer),
    f!("required", Kind::Flag),
    f!("title", Kind::Object(&[f!("len", Kind::Integer)])),
    f!("img", Kind::Object(REQUEST_IMAGE)),
    f!(
        "video",
        Kind::Object(&[
            f!("mimes", Kind::Array(&Kind::String)),
            f!("minduration", Kind::Integer),
            f!("maxduration", Kind::Integer),
            f!("protocols", Kind::Array(&Kind::Integer)),
        ])
    ),
    f!(
        "data",
        Kind::Object(&[f!("type", Kind::Integer), f!("len", Kind::Integer)])
    ),
];
static REQUEST: &[Field] = &[
    f!("ver", Kind::String),
    f!("context", Kind::Integer),
    f!("plcmttype", Kind::Integer),
    f!("plcmtcnt", Kind::Integer),
    f!("aurlsupport", Kind::Flag),
    f!("assets", Kind::Array(&Kind::Object(REQUEST_ASSET))),
    f!(
        "eventtrackers",
        Kind::Array(&Kind::Object(&[
            f!("event", Kind::Integer),
            f!("methods", Kind::Array(&Kind::Integer)),
        ]))
    ),
];
static LINK: &[Field] = &[
    f!("url", Kind::String),
    f!("clicktrackers", Kind::Array(&Kind::String)),
    f!("fallback", Kind::String),
];
static RESPONSE_ASSET: &[Field] = &[
    f!("id", Kind::Integer),
    f!("required", Kind::Flag),
    f!(
        "title",
        Kind::Object(&[f!("text", Kind::String), f!("len", Kind::Integer)])
    ),
    f!(
        "img",
        Kind::Object(&[
            f!("type", Kind::Integer),
            f!("url", Kind::String),
            f!("w", Kind::Integer),
            f!("h", Kind::Integer)
        ])
    ),
    f!("video", Kind::Object(&[f!("vasttag", Kind::String)])),
    f!("link", Kind::Object(LINK)),
];
static RESPONSE: &[Field] = &[
    f!("ver", Kind::String),
    f!("assets", Kind::Array(&Kind::Object(RESPONSE_ASSET))),
    f!("assetsurl", Kind::String),
    f!("dcourl", Kind::String),
    f!("link", Kind::Object(LINK)),
    f!("imptrackers", Kind::Array(&Kind::String)),
    f!("jstracker", Kind::String),
    f!("privacy", Kind::String),
    f!(
        "eventtrackers",
        Kind::Array(&Kind::Object(&[
            f!("event", Kind::Integer),
            f!("method", Kind::Integer),
            f!("url", Kind::String),
            f!("customdata", Kind::Object(&[])),
        ]))
    ),
];

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    match object_name {
        "Imp" => contract::required(object, "tagid", path, "TripleLift supplier placement", issues),
        "Banner" => {
            let formats = object.get("format").and_then(Value::as_array);
            if formats.is_some_and(Vec::is_empty) {
                contract::error("openrtb.profile.triplelift.banner_format_empty", "TripleLift's required format array must describe at least one banner size.", join_instance_path(path, "format"), issues);
            } else if formats.is_none() && !contract::present(object, "format") {
                let exact = object.get("w").and_then(Value::as_u64).is_some_and(|v| v > 0) && object.get("h").and_then(Value::as_u64).is_some_and(|v| v > 0);
                if exact {
                    contract::warning("openrtb.profile.triplelift.banner_format_fallback", "TripleLift labels format required but recommends w/h when it is absent. This request uses that documented fallback.", join_instance_path(path, "format"), issues);
                } else { contract::required(object, "format", path, "TripleLift supplier banner", issues); }
            }
        }
        "App" if !contract::present(object, "bundle") => contract::warning("openrtb.profile.triplelift.bundle_contract_conflict", "TripleLift's app table requires bundle while its prose says an app ID or bundle is not strictly required. Confirm the active supplier contract.", join_instance_path(path, "bundle"), issues),
        "Regs" => {
            if let Some(ext) = object.get("ext").and_then(Value::as_object) {
                contract::fields(ext, &[
                    f!("gdpr", Kind::Flag), f!("us_privacy", Kind::String),
                    f!("gpp", Kind::String), f!("gpp_sid", Kind::Array(&Kind::Integer)),
                ], &join_instance_path(path, "ext"), issues);
            }
        }
        "Native" => {
            if let Some((native, wrapper)) = object.get("request").and_then(decode_native) {
                let native_path = join_instance_path(path, if wrapper { "request.native" } else { "request" });
                contract::fields(&native, REQUEST, &native_path, issues);
                if let Some(assets) = native.get("assets").and_then(Value::as_array) {
                    for (index, asset) in assets.iter().enumerate() {
                        if let Some(image) = asset.get("img").and_then(Value::as_object) {
                            let image_path = format!("{native_path}.assets[{index}].img");
                            contract::required(image, "type", &image_path, "TripleLift native image", issues);
                            for (exact, minimum) in [("w", "wmin"), ("h", "hmin")] {
                                if !contract::present(image, exact) && !contract::present(image, minimum) { contract::warning("openrtb.profile.triplelift.image_size_recommended", "TripleLift recommends an exact or minimum native image dimension.", join_instance_path(&image_path, minimum), issues); }
                            }
                        }
                    }
                }
            }
        }
        "Bid" => validate_native_response(object, path, issues),
        _ => {}
    }
}

fn validate_native_response(bid: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some((native, wrapper)) = bid.get("adm").and_then(decode_native) else {
        return;
    };
    if bid.get("mtype").and_then(Value::as_i64) != Some(4)
        && !native.contains_key("assets")
        && !native.contains_key("assetsurl")
        && !native.contains_key("link")
    {
        return;
    }
    let native_path = join_instance_path(path, if wrapper { "adm.native" } else { "adm" });
    contract::fields(&native, RESPONSE, &native_path, issues);
    if let Some(script) = native.get("jstracker").and_then(Value::as_str) {
        let lower = script.to_ascii_lowercase();
        if !script.is_empty() && !(lower.contains("<script") && lower.contains("</script>")) {
            contract::error(
                "openrtb.profile.triplelift.jstracker_markup",
                "TripleLift Native 1.2 jstracker contains HTML with script tags.",
                join_instance_path(&native_path, "jstracker"),
                issues,
            );
        }
    }
    if let Some(trackers) = native.get("eventtrackers").and_then(Value::as_array) {
        for (index, tracker) in trackers.iter().enumerate() {
            let Some(tracker) = tracker.as_object() else {
                continue;
            };
            let tracker_path = format!("{native_path}.eventtrackers[{index}]");
            for field in ["event", "method"] {
                contract::required(
                    tracker,
                    field,
                    &tracker_path,
                    "TripleLift native event tracker",
                    issues,
                );
            }
            for (field, maximum) in [("event", 4), ("method", 2)] {
                if let Some(value) = tracker.get(field).and_then(Value::as_i64) {
                    if !(1..=maximum).contains(&value) && value < 500 {
                        contract::error("openrtb.profile.triplelift.event_enum", "Native event and method IDs must be standard IDs or exchange-specific values of at least 500.", join_instance_path(&tracker_path, field), issues);
                    }
                }
            }
        }
    }
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    for (bid_path, bid) in contract::bids(response) {
        let Some(imp) = contract::matching_imp(request, bid) else {
            continue;
        };
        let Some((native_request, _)) = value_at(imp, "native.request").and_then(decode_native)
        else {
            continue;
        };
        let Some((native_response, wrapper)) = bid.get("adm").and_then(decode_native) else {
            continue;
        };
        let native_path = join_instance_path(&bid_path, if wrapper { "adm.native" } else { "adm" });
        if contract::present(&native_response, "assetsurl") {
            match native_request.get("aurlsupport") {
                None | Some(Value::Null) => assets_url_unsupported(&native_path, issues),
                Some(value) if value.as_i64() == Some(0) => {
                    assets_url_unsupported(&native_path, issues)
                }
                _ => {}
            }
        }
        let Some(offered) = native_request
            .get("eventtrackers")
            .and_then(Value::as_array)
        else {
            continue;
        };
        let Some(returned) = native_response
            .get("eventtrackers")
            .and_then(Value::as_array)
        else {
            continue;
        };
        // A malformed offer cannot prove that a returned event was unoffered.
        if !offered.iter().all(|tracker| {
            tracker
                .get("event")
                .and_then(Value::as_i64)
                .is_some_and(|id| (1..=4).contains(&id) || id >= 500)
                && tracker
                    .get("methods")
                    .and_then(Value::as_array)
                    .is_some_and(|methods| {
                        !methods.is_empty()
                            && methods.iter().all(|value| {
                                value
                                    .as_i64()
                                    .is_some_and(|id| (1..=2).contains(&id) || id >= 500)
                            })
                    })
        }) {
            continue;
        }
        for (index, tracker) in returned.iter().enumerate() {
            let (Some(event), Some(method)) = (
                tracker.get("event").and_then(Value::as_i64),
                tracker.get("method").and_then(Value::as_i64),
            ) else {
                continue;
            };
            let allowed = offered.iter().any(|offer| {
                offer.get("event").and_then(Value::as_i64) == Some(event)
                    && offer
                        .get("methods")
                        .and_then(Value::as_array)
                        .is_some_and(|methods| {
                            methods.iter().any(|value| value.as_i64() == Some(method))
                        })
            });
            if !allowed {
                contract::error("openrtb.profile.triplelift.event_unoffered", "The returned Native event and tracking method must be among the request's declared options.", format!("{native_path}.eventtrackers[{index}]"), issues);
            }
        }
    }
}

fn assets_url_unsupported(path: &str, issues: &mut Vec<Issue>) {
    contract::error(
        "openrtb.profile.triplelift.assetsurl_unsupported",
        "Native assetsurl requires request aurlsupport=1; omission means unsupported.",
        join_instance_path(path, "assetsurl"),
        issues,
    );
}

fn decode_native(value: &Value) -> Option<(Map<String, Value>, bool)> {
    let parsed: Value = serde_json::from_str(value.as_str()?).ok()?;
    let mut object = parsed.as_object()?.clone();
    if let Some(inner) = object
        .remove("native")
        .and_then(|value| value.as_object().cloned())
    {
        Some((inner, true))
    } else {
        Some((object, false))
    }
}
