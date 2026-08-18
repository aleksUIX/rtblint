//! Native Ads 1.2 markup that OpenRTB carries as an encoded string.
//!
//! `imp.native.request` and `bid.adm` (when mtype is native) are JSON text,
//! not catalog objects, so the OpenRTB walk cannot see assets, eventtrackers,
//! or the response link. This module is that walk. Pair checks (required
//! asset ids, type match) live here too and are called from `pair`.

use std::collections::{HashMap, HashSet};

use serde_json::{Map, Value};

use crate::{Issue, Severity};

const SECTION_REQUEST: &str = "Native Ads 1.2 §4";
const SECTION_RESPONSE: &str = "Native Ads 1.2 §5";
const VENDOR_RANGE: i64 = 500;

const DATA_TYPES: &[i64] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
const IMAGE_TYPES: &[i64] = &[1, 2, 3];
const EVENT_TYPES: &[i64] = &[1, 2, 3, 4];
const EVENT_METHODS: &[i64] = &[1, 2];

const ASSET_KEYS: [&str; 4] = ["title", "img", "video", "data"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeAssetKind {
    Title,
    Image,
    Video,
    Data,
}

impl NativeAssetKind {
    fn from_object(asset: &Map<String, Value>) -> Option<Self> {
        let mut found = None;
        for key in ASSET_KEYS {
            if asset.contains_key(key) {
                if found.is_some() {
                    return None;
                }
                found = Some(match key {
                    "title" => Self::Title,
                    "img" => Self::Image,
                    "video" => Self::Video,
                    "data" => Self::Data,
                    _ => unreachable!(),
                });
            }
        }
        found
    }

    fn label(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Image => "img",
            Self::Video => "video",
            Self::Data => "data",
        }
    }
}

/// Request asset ids and their kinds, for pair checks against `bid.adm`.
#[derive(Debug, Default)]
pub(crate) struct NativeRequestIndex {
    pub assets: HashMap<i64, NativeAssetKind>,
    pub required: Vec<i64>,
}

/// Decodes the Native Markup Request string, unwrapping the pre-1.1
/// `{"native": {...}}` root when present.
pub(crate) fn parse_encoded_object(encoded: &str) -> Option<Map<String, Value>> {
    let value: Value = serde_json::from_str(encoded).ok()?;
    unwrap_native_root(value)
}

fn unwrap_native_root(value: Value) -> Option<Map<String, Value>> {
    match value {
        Value::Object(mut fields) => {
            if fields.len() == 1 {
                if let Some(Value::Object(inner)) = fields.remove("native") {
                    return Some(inner);
                }
            }
            Some(fields)
        }
        _ => None,
    }
}

pub(crate) fn index_markup_request(request: &Map<String, Value>) -> NativeRequestIndex {
    let mut index = NativeRequestIndex::default();
    let Some(assets) = request.get("assets").and_then(Value::as_array) else {
        return index;
    };
    for asset in assets.iter().filter_map(Value::as_object) {
        let Some(id) = integer_id(asset.get("id")) else {
            continue;
        };
        if let Some(kind) = NativeAssetKind::from_object(asset) {
            index.assets.insert(id, kind);
        }
        if integer_id(asset.get("required")) == Some(1) {
            index.required.push(id);
        }
    }
    index
}

pub(crate) fn validate_markup_request(
    request: &Map<String, Value>,
    instance_path: &str,
    ver: Option<&str>,
    require_asset_id: bool,
    issues: &mut Vec<Issue>,
) {
    if layout_removed(ver) && request.contains_key("layout") {
        issues.push(issue(
            "openrtb.native.layout_removed",
            Severity::Warning,
            String::from(
                "layout was removed in Native Ads 1.1; Native 1.1+ uses plcmttype and context \
                 instead.",
            ),
            join_path(instance_path, "layout"),
            SECTION_REQUEST,
        ));
    }

    let Some(assets) = request.get("assets").and_then(Value::as_array) else {
        issues.push(issue(
            "openrtb.native.assets_missing",
            Severity::Error,
            String::from(
                "Native Markup Request must contain a non-empty assets array; without it a buyer \
                 cannot tell which assets to return.",
            ),
            join_path(instance_path, "assets"),
            SECTION_REQUEST,
        ));
        return;
    };
    if assets.is_empty() {
        issues.push(issue(
            "openrtb.native.assets_missing",
            Severity::Error,
            String::from("Native Markup Request assets must not be an empty array."),
            join_path(instance_path, "assets"),
            SECTION_REQUEST,
        ));
        return;
    }

    let mut seen_ids: HashSet<i64> = HashSet::new();
    for (index, asset) in assets.iter().enumerate() {
        let asset_path = format!("{instance_path}.assets[{index}]");
        let Some(asset) = asset.as_object() else {
            continue;
        };
        validate_request_asset(asset, &asset_path, &mut seen_ids, require_asset_id, issues);
    }

    if let Some(trackers) = request.get("eventtrackers").and_then(Value::as_array) {
        for (index, tracker) in trackers.iter().enumerate() {
            let Some(tracker) = tracker.as_object() else {
                continue;
            };
            validate_event_tracker(
                tracker,
                &format!("{instance_path}.eventtrackers[{index}]"),
                issues,
            );
        }
    }
}

fn validate_request_asset(
    asset: &Map<String, Value>,
    asset_path: &str,
    seen_ids: &mut HashSet<i64>,
    require_asset_id: bool,
    issues: &mut Vec<Issue>,
) {
    match integer_id(asset.get("id")) {
        Some(id) if !seen_ids.insert(id) => issues.push(issue(
            "openrtb.native.asset.id_duplicate",
            Severity::Error,
            format!(
                "Native asset id {id} is used more than once; the response cannot map \
                 assets unambiguously."
            ),
            join_path(asset_path, "id"),
            SECTION_REQUEST,
        )),
        Some(_) => {}
        None if require_asset_id => issues.push(issue(
            "openrtb.native.asset.id_required",
            Severity::Error,
            String::from("Each native asset must have an integer id unique within the request."),
            join_path(asset_path, "id"),
            SECTION_REQUEST,
        )),
        None => {}
    }

    let present: Vec<_> = ASSET_KEYS
        .iter()
        .copied()
        .filter(|key| asset.contains_key(*key))
        .collect();
    match present.as_slice() {
        [] => issues.push(issue(
            "openrtb.native.asset.subtype_required",
            Severity::Error,
            String::from("A native asset needs exactly one of title, img, video, or data."),
            String::from(asset_path),
            SECTION_REQUEST,
        )),
        [_] => {}
        _ => issues.push(issue(
            "openrtb.native.asset.subtype_required",
            Severity::Error,
            format!(
                "A native asset must contain exactly one of title, img, video, or data; this one \
                 has {}.",
                present.join(", ")
            ),
            String::from(asset_path),
            SECTION_REQUEST,
        )),
    }

    if let Some(title) = asset.get("title").and_then(Value::as_object) {
        if integer_id(title.get("len")).is_none() {
            issues.push(issue(
                "openrtb.native.field_required",
                Severity::Error,
                String::from(
                    "title.len is required on a native title asset (maximum text length).",
                ),
                format!("{asset_path}.title.len"),
                SECTION_REQUEST,
            ));
        }
    }

    if let Some(img) = asset.get("img").and_then(Value::as_object) {
        if let Some(image_type) = integer_id(img.get("type")) {
            if !allowed_enum(image_type, IMAGE_TYPES) {
                issues.push(invalid_enum(
                    "img.type",
                    image_type,
                    "1 (icon), 2 (logo, Native 1.1), 3 (main), or 500+",
                    format!("{asset_path}.img.type"),
                    SECTION_REQUEST,
                ));
            }
        }
    }

    if let Some(data) = asset.get("data").and_then(Value::as_object) {
        match integer_id(data.get("type")) {
            None => issues.push(issue(
                "openrtb.native.field_required",
                Severity::Error,
                String::from("data.type is required on a native data asset."),
                format!("{asset_path}.data.type"),
                SECTION_REQUEST,
            )),
            Some(data_type) if !allowed_enum(data_type, DATA_TYPES) => {
                issues.push(invalid_enum(
                    "data.type",
                    data_type,
                    "1-12 or 500+",
                    format!("{asset_path}.data.type"),
                    SECTION_REQUEST,
                ));
            }
            Some(_) => {}
        }
    }

    if let Some(video) = asset.get("video").and_then(Value::as_object) {
        for field in ["mimes", "protocols", "minduration", "maxduration"] {
            if !video_field_populated(video, field) {
                issues.push(issue(
                    "openrtb.native.field_required",
                    Severity::Error,
                    format!(
                        "video.{field} is required on a native video asset (Native Ads 1.2 §4.5)."
                    ),
                    format!("{asset_path}.video.{field}"),
                    SECTION_REQUEST,
                ));
            }
        }
    }
}

fn validate_event_tracker(tracker: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    match integer_id(tracker.get("event")) {
        None => issues.push(issue(
            "openrtb.native.field_required",
            Severity::Error,
            String::from("eventtrackers.event is required (1=impression, 2-4=viewability, 500+)."),
            join_path(path, "event"),
            SECTION_REQUEST,
        )),
        Some(event) if !allowed_enum(event, EVENT_TYPES) => {
            issues.push(invalid_enum(
                "event",
                event,
                "1-4 or 500+",
                join_path(path, "event"),
                SECTION_REQUEST,
            ));
        }
        Some(_) => {}
    }

    let methods = tracker.get("methods").and_then(Value::as_array);
    if methods.map_or(true, |items| items.is_empty()) {
        issues.push(issue(
            "openrtb.native.field_required",
            Severity::Error,
            String::from(
                "eventtrackers.methods is required and must not be empty (1=image, 2=js, 500+).",
            ),
            join_path(path, "methods"),
            SECTION_REQUEST,
        ));
        return;
    }
    if let Some(methods) = methods {
        for (index, method) in methods.iter().enumerate() {
            if let Some(method) = integer_id(Some(method)) {
                if !allowed_enum(method, EVENT_METHODS) {
                    issues.push(invalid_enum(
                        "methods",
                        method,
                        "1, 2, or 500+",
                        format!("{path}.methods[{index}]"),
                        SECTION_REQUEST,
                    ));
                }
            }
        }
    }
}

pub(crate) fn validate_markup_response(
    response: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let link_url = response
        .get("link")
        .and_then(Value::as_object)
        .and_then(|link| link.get("url").and_then(Value::as_str))
        .is_some_and(|url| !url.is_empty());
    if !link_url {
        issues.push(issue(
            "openrtb.native.field_required",
            Severity::Error,
            String::from("Native Markup Response must contain link.url, the clickthrough."),
            join_path(instance_path, "link.url"),
            SECTION_RESPONSE,
        ));
    }

    let assetsurl = response
        .get("assetsurl")
        .and_then(Value::as_str)
        .is_some_and(|url| !url.is_empty());
    let assets = response.get("assets").and_then(Value::as_array);
    if !assetsurl && assets.map_or(true, |items| items.is_empty()) {
        issues.push(issue(
            "openrtb.native.assets_missing",
            Severity::Error,
            String::from(
                "Native Markup Response must contain assets, or assetsurl for a third-party \
                 native ad.",
            ),
            join_path(instance_path, "assets"),
            SECTION_RESPONSE,
        ));
        return;
    }

    let Some(assets) = assets else {
        return;
    };
    for (index, asset) in assets.iter().enumerate() {
        let Some(asset) = asset.as_object() else {
            continue;
        };
        let asset_path = format!("{instance_path}.assets[{index}]");
        validate_response_asset(asset, &asset_path, issues);
    }
}

fn validate_response_asset(asset: &Map<String, Value>, asset_path: &str, issues: &mut Vec<Issue>) {
    if integer_id(asset.get("id")).is_none() {
        issues.push(issue(
            "openrtb.native.asset.id_required",
            Severity::Error,
            String::from(
                "Each native response asset must have an integer id matching the request asset.",
            ),
            join_path(asset_path, "id"),
            SECTION_RESPONSE,
        ));
    }

    let present: Vec<_> = ASSET_KEYS
        .iter()
        .copied()
        .filter(|key| asset.contains_key(*key))
        .collect();
    match present.as_slice() {
        [] => issues.push(issue(
            "openrtb.native.asset.subtype_required",
            Severity::Error,
            String::from(
                "A native response asset needs exactly one of title, img, video, or data.",
            ),
            String::from(asset_path),
            SECTION_RESPONSE,
        )),
        [_] => {}
        _ => issues.push(issue(
            "openrtb.native.asset.subtype_required",
            Severity::Error,
            format!(
                "A native response asset must contain exactly one of title, img, video, or data; \
                 this one has {}.",
                present.join(", ")
            ),
            String::from(asset_path),
            SECTION_RESPONSE,
        )),
    }

    if let Some(title) = asset.get("title").and_then(Value::as_object) {
        if !non_empty_string(title.get("text")) {
            issues.push(issue(
                "openrtb.native.field_required",
                Severity::Error,
                String::from("title.text is required on a native title response asset."),
                format!("{asset_path}.title.text"),
                SECTION_RESPONSE,
            ));
        }
    }
    if let Some(img) = asset.get("img").and_then(Value::as_object) {
        if !non_empty_string(img.get("url")) {
            issues.push(issue(
                "openrtb.native.field_required",
                Severity::Error,
                String::from("img.url is required on a native image response asset."),
                format!("{asset_path}.img.url"),
                SECTION_RESPONSE,
            ));
        }
    }
    if let Some(video) = asset.get("video").and_then(Value::as_object) {
        if !non_empty_string(video.get("vasttag")) {
            issues.push(issue(
                "openrtb.native.field_required",
                Severity::Error,
                String::from("video.vasttag is required on a native video response asset."),
                format!("{asset_path}.video.vasttag"),
                SECTION_RESPONSE,
            ));
        }
    }
    if let Some(data) = asset.get("data").and_then(Value::as_object) {
        if !non_empty_string(data.get("value")) {
            issues.push(issue(
                "openrtb.native.field_required",
                Severity::Error,
                String::from("data.value is required on a native data response asset."),
                format!("{asset_path}.data.value"),
                SECTION_RESPONSE,
            ));
        }
    }
}

pub(crate) fn validate_response_against_request(
    request: &NativeRequestIndex,
    response: &Map<String, Value>,
    adm_path: &str,
    issues: &mut Vec<Issue>,
) {
    if request.assets.is_empty() {
        return;
    }

    let Some(assets) = response.get("assets").and_then(Value::as_array) else {
        return;
    };

    let mut seen: HashMap<i64, NativeAssetKind> = HashMap::new();
    for (index, asset) in assets.iter().enumerate() {
        let Some(asset) = asset.as_object() else {
            continue;
        };
        let Some(id) = integer_id(asset.get("id")) else {
            continue;
        };
        let asset_path = format!("{adm_path}.assets[{index}]");
        match request.assets.get(&id) {
            None => issues.push(issue(
                "openrtb.native.asset.id_unknown",
                Severity::Error,
                format!(
                    "Native response asset id {id} is not among the ids the request asked for."
                ),
                join_path(&asset_path, "id"),
                SECTION_RESPONSE,
            )),
            Some(expected) => {
                if let Some(actual) = NativeAssetKind::from_object(asset) {
                    if actual != *expected {
                        issues.push(issue(
                            "openrtb.native.asset.type_mismatch",
                            Severity::Error,
                            format!(
                                "Native response asset {id} is {}, but the request asked for {}.",
                                actual.label(),
                                expected.label()
                            ),
                            asset_path,
                            SECTION_RESPONSE,
                        ));
                    }
                    seen.insert(id, actual);
                }
            }
        }
    }

    for id in &request.required {
        if !seen.contains_key(id) {
            issues.push(issue(
                "openrtb.native.asset.required_missing",
                Severity::Error,
                format!(
                    "The native request marked asset id {id} as required, but the response does \
                     not return it."
                ),
                String::from(adm_path),
                SECTION_RESPONSE,
            ));
        }
    }
}

fn layout_removed(ver: Option<&str>) -> bool {
    match ver {
        Some(ver) if ver.starts_with("1.0") => false,
        None => false,
        Some(_) => true,
    }
}

fn video_field_populated(video: &Map<String, Value>, field: &str) -> bool {
    match video.get(field) {
        Some(Value::Array(items)) => !items.is_empty(),
        Some(Value::Number(_)) => true,
        Some(Value::String(text)) => !text.is_empty(),
        _ => false,
    }
}

fn integer_id(value: Option<&Value>) -> Option<i64> {
    value.and_then(|value| {
        value
            .as_i64()
            .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
    })
}

fn allowed_enum(value: i64, documented: &[i64]) -> bool {
    documented.contains(&value) || value >= VENDOR_RANGE
}

fn non_empty_string(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| !text.is_empty())
}

fn invalid_enum(
    field: &str,
    value: i64,
    allowed: &str,
    path: String,
    section: &'static str,
) -> Issue {
    issue(
        "openrtb.native.value.invalid",
        Severity::Error,
        format!("{field} value {value} is not a documented Native Ads type ({allowed})."),
        path,
        section,
    )
}

fn join_path(base: &str, segment: &str) -> String {
    if base.is_empty() {
        String::from(segment)
    } else {
        format!("{base}.{segment}")
    }
}

fn issue(
    id: &'static str,
    severity: Severity,
    message: String,
    path: String,
    section: &'static str,
) -> Issue {
    Issue {
        id: String::from(id),
        severity,
        message,
        path: Some(path),
        section: Some(String::from(section)),
    }
}
