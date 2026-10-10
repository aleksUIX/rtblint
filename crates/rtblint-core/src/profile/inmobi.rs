//! InMobi's two documented directions have different acceptance contracts.
use super::contract::{self, Field, Kind};
use super::{join_instance_path, value_at};
use crate::{native, Issue};
use serde_json::{Map, Value};

macro_rules! f {
    ($name:literal, $kind:expr) => {
        Field {
            name: $name,
            kind: $kind,
        }
    };
}

static IAP: &[Field] = &[
    f!("bundle", Kind::String),
    f!("recenttransactiontime", Kind::Integer),
    f!("purchasecount", Kind::Integer),
    f!("subscriptioncount", Kind::Integer),
];
static RELEVANCY: &[Field] = &[
    f!("adomain", Kind::String),
    f!("bundle", Kind::String),
    f!("lastbidtime", Kind::Integer),
    f!("recentbids", Kind::Integer),
];
static SUPPLY_CHAIN_NODE: &[Field] = &[
    f!("asi", Kind::String),
    f!("sid", Kind::String),
    f!("hp", Kind::Flag),
    f!("rid", Kind::String),
    f!("name", Kind::String),
    f!("domain", Kind::String),
    f!("ext", Kind::Object(&[])),
];
static SUPPLY_CHAIN: &[Field] = &[
    f!("ver", Kind::String),
    f!("complete", Kind::Flag),
    f!("nodes", Kind::Array(&Kind::Object(SUPPLY_CHAIN_NODE))),
    f!("ext", Kind::Object(&[])),
];
static DEVICE_EXT: &[Field] = &[
    f!("boottime", Kind::Array(&Kind::Integer)),
    f!("lowmemory", Kind::Flag),
    f!("emulator", Kind::Flag),
    f!("iap", Kind::Array(&Kind::Object(IAP))),
    f!("advertiserrelevancy", Kind::Array(&Kind::Object(RELEVANCY))),
    f!("lastimpressioncpm", Kind::Array(&Kind::Number)),
    f!("psdisable", Kind::Flag),
    f!("psv", Kind::String),
    f!("headset", Kind::Flag),
    f!("cpucount", Kind::Integer),
    f!("airplane", Kind::Flag),
    f!("locale", Kind::String),
    f!("apilevel", Kind::Integer),
    f!("currentmccmnc", Kind::String),
    f!("batterylevel", Kind::Integer),
    f!("batterysaver", Kind::Flag),
    f!("inputlanguage", Kind::Array(&Kind::String)),
    f!("volumelevel", Kind::Integer),
    f!("dnd", Kind::Flag),
    f!("darkmode", Kind::Flag),
    f!("totaldisk", Kind::Integer),
    f!("diskspace", Kind::Integer),
    f!("atts", Kind::Integer),
    f!("ifv", Kind::String),
    f!("appsetid", Kind::String),
    f!("appsetscope", Kind::Integer),
];
static SKOVERLAY: &[Field] = &[
    f!("position", Kind::Flag),
    f!("dismissable", Kind::Flag),
    f!("video_delay", Kind::Integer),
    f!("companion_delay", Kind::Integer),
    f!("sk_dismiss_delay", Kind::Integer),
];
static BID_EXT: &[Field] = &[
    f!("dspId", Kind::String),
    f!("advId", Kind::String),
    f!("advName", Kind::String),
    f!("video", Kind::Object(&[f!("experience", Kind::Integer)])),
    f!("imptrackers", Kind::Array(&Kind::String)),
    f!("clicktrackers", Kind::Array(&Kind::String)),
    f!("engagetracker", Kind::String),
    f!("landingurl", Kind::String),
    f!("deeplink", Kind::String),
    f!("openbrowsermode", Kind::Integer),
    f!("campaigntype", Kind::Integer),
    f!(
        "skadn",
        Kind::Object(&[f!("skoverlay", Kind::Object(SKOVERLAY))])
    ),
];

pub(super) fn validate(
    supplier: bool,
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    match object_name {
        "BidRequest" => validate_request(supplier, object, path, issues),
        "Imp" => {
            if supplier {
                if !contract::present(object, "tagid")
                    && !contract::present(object, "ext.placementid")
                {
                    contract::error(
                        "openrtb.profile.inmobi.placement_required",
                        "InMobi supplier ingest requires tagid or ext.placementid.",
                        join_instance_path(path, "tagid"),
                        issues,
                    );
                }
                typed_ext(object, &[f!("placementid", Kind::Integer)], path, issues);
                for unsupported in ["audio", "pmp"] {
                    if contract::present(object, unsupported) {
                        contract::warning("openrtb.profile.inmobi.supplier_unsupported", "The supplier request guide labels this object unsupported in its current version; negotiated integration support is not visible here.", join_instance_path(path, unsupported), issues);
                    }
                }
            } else {
                typed_ext(
                    object,
                    &[
                        f!("viewabilityvendors", Kind::Array(&Kind::String)),
                        f!("autostore", Kind::Flag),
                    ],
                    path,
                    issues,
                );
                if let Some(hb) = value_at(object, "ext.hb").filter(|v| !v.is_null()) {
                    if !hb.is_boolean() {
                        contract::error(
                            "openrtb.profile.field_type",
                            "InMobi imp.ext.hb is a JSON boolean.",
                            join_instance_path(path, "ext.hb"),
                            issues,
                        );
                    }
                }
                let media_count = ["banner", "video", "audio", "native"]
                    .iter()
                    .filter(|name| object.get(**name).is_some_and(Value::is_object))
                    .count();
                if media_count > 1 {
                    contract::error(
                        "openrtb.profile.inmobi.single_format",
                        "InMobi sends each format opportunity to DSPs in a separate request.",
                        path.to_owned(),
                        issues,
                    );
                }
            }
        }
        "Banner" if supplier => {
            let exact = positive_integer(object.get("w")) && positive_integer(object.get("h"));
            let format = object
                .get("format")
                .and_then(Value::as_array)
                .is_some_and(|values| {
                    values.iter().any(|format| {
                        positive_integer(format.get("w")) && positive_integer(format.get("h"))
                    })
                });
            if !exact && !format {
                contract::error(
                    "openrtb.profile.inmobi.banner_size_required",
                    "InMobi supplier banners require positive w/h or a format with positive w/h.",
                    path.to_owned(),
                    issues,
                );
            }
        }
        "Video" => {
            if supplier {
                for field in ["w", "h", "mimes"] {
                    contract::required(object, field, path, "InMobi supplier video", issues);
                }
                typed_ext(object, &[f!("rewarded", Kind::Flag)], path, issues);
            } else {
                typed_ext(
                    object,
                    &[
                        f!("videotype", Kind::String),
                        f!("experiences", Kind::Array(&Kind::Integer)),
                    ],
                    path,
                    issues,
                );
                if let Some(value) =
                    value_at(object, "ext.rewarded").filter(|value| !value.is_null())
                {
                    if !value.is_boolean() {
                        contract::error(
                            "openrtb.profile.field_type",
                            "InMobi outgoing video.ext.rewarded is a JSON boolean.",
                            join_instance_path(path, "ext.rewarded"),
                            issues,
                        );
                    }
                }
                if let Some(values) = value_at(object, "ext.experiences").and_then(Value::as_array)
                {
                    for (index, value) in values.iter().enumerate() {
                        if !value.as_i64().is_some_and(|v| matches!(v, 1..=3)) {
                            contract::error(
                                "openrtb.profile.inmobi.video_experience",
                                "InMobi video experience IDs are 1, 2 and 3.",
                                format!("{}.ext.experiences[{index}]", path),
                                issues,
                            );
                        }
                    }
                }
                if let Some(value) = object.get("linearity").filter(|v| !v.is_null()) {
                    if value.as_i64() != Some(1) {
                        contract::error(
                            "openrtb.profile.inmobi.video_linearity",
                            "InMobi outgoing video opportunities use linearity 1.",
                            join_instance_path(path, "linearity"),
                            issues,
                        );
                    }
                }
            }
        }
        "Device" => {
            if supplier {
                for field in ["ua", "lmt"] {
                    contract::required(object, field, path, "InMobi supplier device", issues);
                }
                typed_ext(
                    object,
                    &[f!("ifv", Kind::String), f!("atts", Kind::Integer)],
                    path,
                    issues,
                );
                contract::integer_enum(object, "ext.atts", &[0, 1, 2, 3], path, issues);
                if !contract::present(object, "ip") && !contract::present(object, "ipv6") {
                    contract::warning("openrtb.profile.inmobi.device_address", "The supplier guide labels both IP families required but does not define the alternative or privacy redaction behavior. Confirm address availability for this integration.", join_instance_path(path, "ip"), issues);
                }
            } else {
                validate_device(object, path, issues);
            }
        }
        "User" => {
            if !supplier {
                typed_ext(
                    object,
                    &[
                        f!("truesessiondepth", Kind::Integer),
                        f!("gdpr", Kind::Flag),
                        f!(
                            "providersSettings",
                            Kind::Object(&[f!("consented_providers", Kind::Array(&Kind::Integer))])
                        ),
                    ],
                    path,
                    issues,
                );
            }
        }
        "Regs" if !supplier => {
            typed_ext(
                object,
                &[f!("ccpa", Kind::Flag), f!("consent", Kind::Flag)],
                path,
                issues,
            );
        }
        "Source" if !supplier => {
            typed_ext(
                object,
                &[
                    f!("header_bidding", Kind::Flag),
                    f!("omidpn", Kind::String),
                    f!("omidpv", Kind::String),
                    f!("schain", Kind::Object(SUPPLY_CHAIN)),
                ],
                path,
                issues,
            );
            if let Some(chain) = value_at(object, "ext.schain").and_then(Value::as_object) {
                let base = join_instance_path(path, "ext.schain");
                for field in ["ver", "complete", "nodes"] {
                    contract::required(chain, field, &base, "InMobi outgoing supply chain", issues);
                }
                if let Some(nodes) = chain.get("nodes").and_then(Value::as_array) {
                    if nodes.is_empty() {
                        contract::error(
                            "openrtb.profile.inmobi.schain_nodes",
                            "InMobi's declared supply chain requires at least one node.",
                            join_instance_path(&base, "nodes"),
                            issues,
                        );
                    }
                    for (index, node) in nodes.iter().enumerate() {
                        if let Some(node) = node.as_object() {
                            // InMobi explicitly marks asi optional, unlike the IAB chain table.
                            for field in ["sid", "hp"] {
                                contract::required(
                                    node,
                                    field,
                                    &format!("{base}.nodes[{index}]"),
                                    "InMobi outgoing supply-chain node",
                                    issues,
                                );
                            }
                        }
                    }
                }
            }
        }
        "Native" if !supplier => {
            if let Some(request) = object
                .get("request")
                .and_then(Value::as_str)
                .and_then(native::parse_encoded_object)
            {
                contract::required(
                    &request,
                    "context",
                    &join_instance_path(path, "request"),
                    "InMobi outgoing native",
                    issues,
                );
                contract::integer_enum(
                    &request,
                    "context",
                    &[1501],
                    &join_instance_path(path, "request"),
                    issues,
                );
            }
        }
        "SeatBid" if !supplier => {
            contract::required(object, "seat", path, "InMobi bidder response", issues);
        }
        "Segment" if !supplier => {
            contract::fields(object, &[f!("signal", Kind::String)], path, issues)
        }
        "Bid" if !supplier => validate_bid(object, path, issues),
        _ => {}
    }
}

fn validate_request(
    supplier: bool,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    contract::integer_enum(object, "at", &[1, 2], path, issues);
    if supplier {
        contract::required(object, "device", path, "InMobi supplier request", issues);
        if !contract::present(object, "app") && !contract::present(object, "site") {
            contract::error("openrtb.profile.inmobi.inventory_required", "InMobi supplier ingest requires app or site inventory context; the current FAQ supports both directions.", join_instance_path(path, "app"), issues);
        }
    } else {
        typed_ext(
            object,
            &[
                f!("auction_id", Kind::String),
                f!("mediation_name", Kind::String),
            ],
            path,
            issues,
        );
        if let Some(values) = object.get("bapp").and_then(Value::as_array) {
            if values.len() > 30 {
                contract::error(
                    "openrtb.profile.inmobi.bapp_limit",
                    "InMobi sends at most 30 blocked app bundles.",
                    join_instance_path(path, "bapp"),
                    issues,
                );
            }
        }
        // Missing telemetry does not prove a receiving exchange rejects a request.
        // 'Always passed' is a producer expectation, qualified by SDK/platform.
        let sdk = object
            .get("imp")
            .and_then(Value::as_array)
            .is_some_and(|values| {
                values
                    .iter()
                    .any(|imp| imp.get("displaymanager").and_then(Value::as_str) == Some("inmobi"))
            });
        if sdk {
            for field in [
                "ext.mediation_name",
                "source.ext.header_bidding",
                "device.ext.locale",
                "device.ext.batterylevel",
                "device.ext.batterysaver",
                "device.ext.volumelevel",
            ] {
                if !contract::present(object, field) {
                    contract::warning("openrtb.profile.inmobi.sdk_expected", "The outgoing guide says this field is passed by the InMobi SDK; verify SDK version and producer configuration.", join_instance_path(path, field), issues);
                }
            }
        }
    }
}

fn validate_device(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    typed_ext(object, DEVICE_EXT, path, issues);
    for field in ["ext.batterylevel", "ext.volumelevel"] {
        if let Some(value) = value_at(object, field).filter(|v| !v.is_null()) {
            if !value.as_i64().is_some_and(|v| (0..=100).contains(&v)) {
                contract::error(
                    "openrtb.profile.inmobi.percentage_range",
                    "InMobi percentage telemetry must be an integer from 0 to 100.",
                    join_instance_path(path, field),
                    issues,
                );
            }
        }
    }
    contract::integer_enum(object, "ext.atts", &[0, 1, 2, 3], path, issues);
    contract::integer_enum(object, "ext.appsetscope", &[1, 2], path, issues);
    for field in [
        "boottime",
        "iap",
        "advertiserrelevancy",
        "lastimpressioncpm",
    ] {
        if let Some(values) = value_at(object, &format!("ext.{field}")).and_then(Value::as_array) {
            if values.len() > 5
                || (values.is_empty() && matches!(field, "iap" | "advertiserrelevancy"))
            {
                contract::error("openrtb.profile.inmobi.history_count", "InMobi histories contain at most five entries; purchase and advertiser histories are omitted when empty.", join_instance_path(path, &format!("ext.{field}")), issues);
            }
        }
    }
    // The advertiser example is itself out of order, so sorting is advisory.
    for (field, time) in [
        ("iap", "recenttransactiontime"),
        ("advertiserrelevancy", "lastbidtime"),
    ] {
        if let Some(values) = value_at(object, &format!("ext.{field}")).and_then(Value::as_array) {
            for window in values.windows(2) {
                if let (Some(a), Some(b)) = (
                    window[0].get(time).and_then(Value::as_i64),
                    window[1].get(time).and_then(Value::as_i64),
                ) {
                    if a < b {
                        contract::warning("openrtb.profile.inmobi.history_order", "InMobi documents recent history first; its advertiser example conflicts with this ordering.", join_instance_path(path, &format!("ext.{field}")), issues);
                        break;
                    }
                }
            }
        }
    }
    if value_at(object, "ext.psdisable").and_then(Value::as_i64) == Some(1)
        && contract::present(object, "ext.psv")
    {
        contract::error(
            "openrtb.profile.inmobi.play_store_state",
            "InMobi omits Play Store version when psdisable is 1.",
            join_instance_path(path, "ext.psv"),
            issues,
        );
    }
}

fn validate_bid(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    for field in ["burl", "cat"] {
        contract::required(object, field, path, "InMobi bidder response", issues);
    }
    if object
        .get("cat")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        contract::error(
            "openrtb.profile.inmobi.category_required",
            "InMobi requires the creative's IAB categories.",
            join_instance_path(path, "cat"),
            issues,
        );
    }
    // Current response table calls these recommended, while Getting Started
    // labels them required. Keep this conflict visible without rejecting bids.
    for field in ["crid", "adomain"] {
        if !contract::present(object, field) {
            contract::warning("openrtb.profile.inmobi.quality_declaration", "InMobi Getting Started requires this quality declaration; the current response table labels it recommended. Confirm the active integration contract.", join_instance_path(path, field), issues);
        }
    }
    if object
        .get("crid")
        .and_then(Value::as_str)
        .is_some_and(|v| v.chars().count() > 64)
    {
        contract::warning(
            "openrtb.profile.inmobi.creative_id_truncated",
            "InMobi truncates creative IDs longer than 64 characters.",
            join_instance_path(path, "crid"),
            issues,
        );
    }
    if object
        .get("mtype")
        .and_then(Value::as_i64)
        .is_some_and(|m| matches!(m, 1..=3))
    {
        contract::required(object, "adm", path, "InMobi non-native response", issues);
    }
    if object.get("mtype").and_then(Value::as_i64) == Some(1) {
        for field in ["w", "h"] {
            contract::required(object, field, path, "InMobi banner response", issues);
        }
    }
    typed_ext(object, BID_EXT, path, issues);
    contract::integer_enum(object, "ext.openbrowsermode", &[1, 3], path, issues);
    contract::integer_enum(object, "ext.campaigntype", &[0, 1, 2, 3], path, issues);
    contract::integer_enum(object, "ext.video.experience", &[1, 2, 3], path, issues);
    if let Some(markup) = object.get("admobject").filter(|v| !v.is_null()) {
        if let Some(markup) = markup.as_object() {
            native::validate_markup_response(
                markup,
                &join_instance_path(path, "admobject"),
                issues,
            );
        } else {
            contract::error(
                "openrtb.profile.field_type",
                "InMobi admobject must be a native markup object.",
                join_instance_path(path, "admobject"),
                issues,
            );
        }
    }
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    for (path, bid) in contract::bids(response) {
        let Some(imp) = contract::matching_imp(request, bid) else {
            continue;
        };
        let media: Vec<&str> = ["banner", "video", "audio", "native"]
            .into_iter()
            .filter(|name| imp.get(*name).is_some_and(Value::is_object))
            .collect();
        if media.len() == 1 {
            if media[0] != "native" {
                contract::required(bid, "adm", &path, "InMobi non-native response", issues);
            }
            if media[0] == "banner" {
                for field in ["w", "h"] {
                    contract::required(bid, field, &path, "InMobi banner response", issues);
                }
            }
        }
        if let Some(experience @ 1..=3) =
            value_at(bid, "ext.video.experience").and_then(Value::as_i64)
        {
            let offered = value_at(imp, "video.ext.experiences");
            let accepted = match offered {
                None | Some(Value::Null) => Some(experience == 1),
                Some(Value::Array(values))
                    if values
                        .iter()
                        .all(|v| v.as_i64().is_some_and(|v| matches!(v, 1..=3))) =>
                {
                    Some(values.iter().any(|v| v.as_i64() == Some(experience)))
                }
                _ => None,
            };
            if accepted == Some(false) {
                contract::error("openrtb.profile.inmobi.video_experience_unoffered", "InMobi portrait and vertical experiences must be explicitly offered in the request.", join_instance_path(&path, "ext.video.experience"), issues);
            }
        }
        if let (Some(network), Some(Value::Array(networks))) = (
            value_at(bid, "ext.skadn.network").and_then(Value::as_str),
            value_at(imp, "ext.skadn.skadnetids"),
        ) {
            if !network.is_empty()
                && networks
                    .iter()
                    .all(|v| v.as_str().is_some_and(|s| !s.is_empty()))
                && !networks.iter().any(|v| v.as_str() == Some(network))
            {
                contract::error(
                    "openrtb.profile.inmobi.skadnetwork_unoffered",
                    "The response SKAdNetwork must match a requested skadnetids entry.",
                    join_instance_path(&path, "ext.skadn.network"),
                    issues,
                );
            }
        }
        if let (Some(markup), Some(encoded)) = (
            bid.get("admobject").and_then(Value::as_object),
            value_at(imp, "native.request").and_then(Value::as_str),
        ) {
            if let Some(native_request) = native::parse_encoded_object(encoded) {
                native::validate_response_against_request(
                    &native::index_markup_request(&native_request),
                    markup,
                    &join_instance_path(&path, "admobject"),
                    issues,
                );
            }
        }
    }
}

fn typed_ext(object: &Map<String, Value>, fields: &[Field], path: &str, issues: &mut Vec<Issue>) {
    if let Some(value) = object.get("ext").filter(|v| !v.is_null()) {
        if let Some(ext) = value.as_object() {
            contract::fields(ext, fields, &join_instance_path(path, "ext"), issues);
        }
        // The ordinary object validator handles malformed extension containers.
    }
}

fn positive_integer(value: Option<&Value>) -> bool {
    value.and_then(Value::as_u64).is_some_and(|v| v > 0)
}
