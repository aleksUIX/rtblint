//! BidSwitch buyer 5.7 and supplier 1.1 are separate wire contracts.
//! Optional extensions stay open. Account, transport and runtime facts defer.
use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use super::contract::{bids, error, matching_imp, present, required, warning};
use super::{join_instance_path, value_at};
use crate::Issue;

#[derive(Clone, Copy)]
enum Shape {
    String,
    Integer,
    Number,
    Flag,
    Object,
    Strings,
    Integers,
    Objects,
}

struct Descriptor {
    object: &'static str,
    path: &'static str,
    shape: Shape,
    supplier: bool,
}

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
    supplier: bool,
) {
    for descriptor in DESCRIPTORS
        .iter()
        .filter(|d| d.object == object_name && d.supplier == supplier)
    {
        visit(object, descriptor.path, path, &mut |value, field_path| {
            check_shape(value, descriptor.shape, field_path, issues);
        });
    }
    let label = if supplier {
        "BidSwitch supplier 1.1"
    } else {
        "BidSwitch buyer 5.7"
    };
    let needed: &[&str] = match object_name {
        "BidRequest" if supplier => &["device"],
        "BidRequest" => &[
            "device",
            "user",
            "tmax",
            "cur",
            "ext",
            "ext.ssp",
            "ext.media_src",
        ],
        "App" => &["id", "publisher"],
        "Site" if supplier => &["id", "publisher"],
        "Site" => &["id"],
        "DOOH" => &["publisher"],
        "Publisher" => &["id"],
        "Banner" => &["w", "h"],
        "Video" => &["mimes", "protocols"],
        "Audio" => &["mimes"],
        "Device" if supplier => &["geo"],
        "Data" => &["name", "segment"],
        "Segment" if supplier => &["name"],
        "Pmp" | "PMP" => &["deals"],
        "Bid" if supplier => &["burl", "adomain", "crid"],
        "Bid" => &["burl", "adomain"],
        _ => &[],
    };
    for field in needed {
        required(object, field, path, label, issues);
    }
    match object_name {
        "BidRequest" => {
            let context = ["site", "app", "dooh"]
                .iter()
                .any(|field| present(object, field))
                || (!supplier
                    && ["ext.tv", "ext.dooh"]
                        .iter()
                        .any(|field| present(object, field)));
            if !context {
                error(
                    "openrtb.profile.bidswitch.inventory_required",
                    "BidSwitch requires a documented inventory context.",
                    path.to_owned(),
                    issues,
                );
            }
            if present(object, "dooh") && ["site", "app"].iter().any(|field| present(object, field))
            {
                error(
                    "openrtb.profile.bidswitch.inventory_conflict",
                    "A DOOH request cannot also contain site or app.",
                    join_instance_path(path, "dooh"),
                    issues,
                );
            }
            if !supplier {
                for field in ["ext.tv", "ext.dooh"] {
                    if value_at(object, field).is_some_and(Value::is_object) {
                        required(object, &format!("{field}.publisher"), path, label, issues);
                        if value_at(object, &format!("{field}.publisher"))
                            .is_some_and(Value::is_object)
                        {
                            required(
                                object,
                                &format!("{field}.publisher.id"),
                                path,
                                label,
                                issues,
                            );
                        }
                    }
                }
            }
        }
        "Video" => {
            if !present(object, "rqddurs") {
                for field in ["minduration", "maxduration"] {
                    required(object, field, path, label, issues);
                }
            }
            enum_field(object, "ext.vast_url_rq", &[0, 1], path, issues);
            enum_field(object, "ext.rewarded", &[0, 1], path, issues);
        }
        "Audio" => enum_field(object, "ext.format", &[1, 2, 3], path, issues),
        "Native" => {
            let field = if supplier {
                "request_native"
            } else {
                "request"
            };
            required(object, field, path, label, issues);
            if let Some(request) = object.get(field).and_then(Value::as_object) {
                let request_path = join_instance_path(path, field);
                if supplier && request.get("ver").and_then(Value::as_str) != Some("1.2") {
                    error(
                        "openrtb.profile.bidswitch.native_version",
                        "Supplier native markup requires version 1.2.",
                        join_instance_path(&request_path, "ver"),
                        issues,
                    );
                }
            }
            enum_field(object, "ext.adchoicesurl_required", &[0, 1], path, issues);
        }
        "User" => {
            for field in ["id", "buyeruid"] {
                if object
                    .get(field)
                    .and_then(Value::as_str)
                    .is_some_and(|v| v.chars().count() > 50)
                {
                    error(
                        "openrtb.profile.bidswitch.user_id_length",
                        "BidSwitch user identifiers have a maximum of 50 characters.",
                        join_instance_path(path, field),
                        issues,
                    );
                }
            }
            enum_field(
                object,
                "ext.ug",
                &(0..=99).collect::<Vec<_>>(),
                path,
                issues,
            );
        }
        "Imp" => {
            for field in [
                "ext.s2s_nurl",
                "ext.ae",
                "ext.autostore.present",
                "ext.autostore.click",
            ] {
                enum_field(object, field, &[0, 1], path, issues);
            }
            enum_field(object, "ext.notification_type", &[1, 2, 3], path, issues);
        }
        "Deal" => {
            enum_field(object, "ext.deal_type", &[0, 1, 2, 3, 4], path, issues);
            enum_field(object, "ext.guaranteed", &[0, 1], path, issues);
        }
        "Device" => enum_field(object, "ext.truncated_ip", &[0, 1], path, issues),
        "Bid" => validate_bid(object, path, supplier, issues),
        "BidResponse" => validate_response(object, path, supplier, issues),
        _ => {}
    }
}

fn validate_bid(bid: &Map<String, Value>, path: &str, supplier: bool, issues: &mut Vec<Issue>) {
    let click_macro = if supplier {
        "${CLICK_URL_ENC}"
    } else {
        "${CLICK_URL:URLENCODE}"
    };
    if bid
        .get("adm")
        .and_then(Value::as_str)
        .is_some_and(|v| v.matches(click_macro).count() > 1)
    {
        error(
            "openrtb.profile.bidswitch.click_macro",
            "Creative markup may contain the supplier click tracking macro at most once.",
            join_instance_path(path, "adm"),
            issues,
        );
    }
    if !supplier {
        enum_field(bid, "ext.at1", &[1], path, issues);
        if !present(bid, "adid") && !present(bid, "crid") {
            warning(
                "openrtb.profile.bidswitch.creative_id_recommended",
                "BidSwitch recommends adid or crid for creative review.",
                path.to_owned(),
                issues,
            );
        }
        if let Some(burl) = bid.get("burl").and_then(Value::as_str) {
            if !burl.contains("${AUCTION_PRICE}") {
                error(
                    "openrtb.profile.bidswitch.billing_macro",
                    "The buyer billing URL must contain the win price macro.",
                    join_instance_path(path, "burl"),
                    issues,
                );
            }
        }
        let adm_count = bid
            .get("adm")
            .and_then(Value::as_str)
            .map_or(0, |v| v.matches("${AUCTION_PRICE}").count());
        let nurl_macro = bid
            .get("nurl")
            .and_then(Value::as_str)
            .is_some_and(|v| v.contains("${AUCTION_PRICE}"));
        if adm_count > 1 || (adm_count > 0 && nurl_macro) {
            error("openrtb.profile.bidswitch.impression_macro", "The win price macro may appear once in adm and cannot appear in both adm and nurl.", join_instance_path(path, "adm"), issues);
        }
        for field in ["ext.vast_url", "ext.daast_url"] {
            if value_at(bid, field)
                .and_then(Value::as_str)
                .is_some_and(|v| v.contains("${AUCTION_PRICE}"))
            {
                error(
                    "openrtb.profile.bidswitch.markup_macro",
                    "Markup document URLs cannot contain the win price macro.",
                    join_instance_path(path, field),
                    issues,
                );
            }
        }
        if let Some(native) = value_at(bid, "ext.native").and_then(Value::as_object) {
            crate::native::validate_markup_response(
                native,
                &join_instance_path(path, "ext.native"),
                issues,
            );
        }
    } else if let Some(native) = bid.get("adm_native").and_then(Value::as_object) {
        crate::native::validate_markup_response(
            native,
            &join_instance_path(path, "adm_native"),
            issues,
        );
    }
    if let Some(domains) = bid.get("adomain").and_then(Value::as_array) {
        for (index, domain) in domains.iter().enumerate() {
            if domain.as_str().is_some_and(|domain| !domain.is_ascii()) {
                error("openrtb.profile.bidswitch.domain_ascii", "Advertiser domains must use ASCII, including punycode for international domains.", format!("{}.adomain[{index}]", path), issues);
            }
        }
    }
    for field in ["ext.dsa.behalf", "ext.dsa.paid"] {
        if value_at(bid, field)
            .and_then(Value::as_str)
            .is_some_and(|v| v.chars().count() > 100)
        {
            error(
                "openrtb.profile.bidswitch.dsa_name_length",
                "DSA names have a maximum of 100 characters.",
                join_instance_path(path, field),
                issues,
            );
        }
    }
    for field in [
        "ext.dsa.adrender",
        "ext.autostore.present",
        "ext.autostore.click",
        "ext.autostore.show_on_skip",
    ] {
        enum_field(bid, field, &[0, 1], path, issues);
    }
}

fn validate_response(
    response: &Map<String, Value>,
    path: &str,
    supplier: bool,
    issues: &mut Vec<Issue>,
) {
    let yes_bid = bids(response).next().is_some()
        || value_at(response, "ext.igbid")
            .and_then(Value::as_array)
            .is_some_and(|v| !v.is_empty());
    if yes_bid || supplier {
        required(response, "ext", path, "BidSwitch", issues);
        if !supplier {
            required(response, "ext.protocol", path, "BidSwitch buyer", issues);
        }
    }
    let mut seats = BTreeSet::new();
    let mut counts = BTreeMap::<&str, usize>::new();
    if let Some(values) = response.get("seatbid").and_then(Value::as_array) {
        for (index, seat) in values.iter().enumerate() {
            if let Some(id) = seat.get("seat").and_then(Value::as_str) {
                if !seats.insert(id) {
                    error(
                        "openrtb.profile.bidswitch.seat_unique",
                        "All bids from a buyer seat must be grouped into one SeatBid.",
                        join_instance_path(path, &format!("seatbid[{index}].seat")),
                        issues,
                    );
                }
            }
        }
    }
    if !supplier {
        for (bid_path, bid) in bids(response) {
            if let Some(id) = bid.get("impid").and_then(Value::as_str) {
                let count = counts.entry(id).or_default();
                *count += 1;
                if *count > 2 {
                    error(
                        "openrtb.profile.bidswitch.bid_count",
                        "Buyer responses allow at most two bids per ad slot across all seats.",
                        join_instance_path(&bid_path, "impid"),
                        issues,
                    );
                }
            }
            if present(bid, "adm") {
                if let Some(version) = value_at(response, "ext.protocol")
                    .and_then(Value::as_str)
                    .and_then(protocol_major)
                {
                    if version < 4 {
                        error(
                            "openrtb.profile.bidswitch.markup_version",
                            "Buyer adm requires a declared protocol version of at least 4.0.",
                            join_instance_path(&bid_path, "adm"),
                            issues,
                        );
                    } else if version == 4
                        && bid
                            .get("adm")
                            .and_then(Value::as_str)
                            .is_some_and(|v| v.contains("${AUCTION_PRICE}"))
                    {
                        error(
                            "openrtb.profile.bidswitch.markup_version",
                            "Protocol 4.x adm cannot contain the win price macro.",
                            join_instance_path(&bid_path, "adm"),
                            issues,
                        );
                    }
                }
            }
        }
    }
    if let Some(groups) = value_at(response, "ext.igbid").and_then(Value::as_array) {
        for (index, group) in groups.iter().enumerate() {
            let Some(group) = group.as_object() else {
                continue;
            };
            let group_path = join_instance_path(path, &format!("ext.igbid[{index}]"));
            for field in if supplier {
                &["impid", "igbuyer", "ignurl"][..]
            } else {
                &["igbuyer"][..]
            } {
                required(
                    group,
                    field,
                    &group_path,
                    "BidSwitch interest group",
                    issues,
                );
            }
            if let Some(buyers) = group.get("igbuyer").and_then(Value::as_array) {
                for (buyer_index, buyer) in buyers.iter().enumerate() {
                    if let Some(buyer) = buyer.as_object() {
                        required(
                            buyer,
                            "origin",
                            &format!("{group_path}.igbuyer[{buyer_index}]"),
                            "BidSwitch interest group buyer",
                            issues,
                        );
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
    supplier: bool,
) {
    if supplier && present(request, "wseat") {
        if let Some(seats) = response.get("seatbid").and_then(Value::as_array) {
            for (index, seat) in seats.iter().enumerate() {
                if let Some(seat) = seat.as_object() {
                    required(
                        seat,
                        "seat",
                        &format!("seatbid[{index}]"),
                        "BidSwitch supplier with request.wseat",
                        issues,
                    );
                }
            }
        }
    }
    for (path, bid) in bids(response) {
        let Some(imp) = matching_imp(request, bid) else {
            continue;
        };
        let media_fields = ["banner", "video", "audio", "native"];
        // A malformed supplied media container leaves the capture ambiguous.
        // Request validation reports its type without inferring a response format.
        let one_media = media_fields
            .iter()
            .filter(|field| imp.contains_key(**field))
            .count()
            == 1
            && media_fields
                .iter()
                .all(|field| imp.get(*field).map_or(true, Value::is_object));
        if one_media && imp.get("banner").is_some_and(Value::is_object) {
            required(bid, "adm", &path, "BidSwitch banner response", issues);
            required(bid, "iurl", &path, "BidSwitch banner response", issues);
            if !supplier
                && value_at(request, "ext.clktrkrq").and_then(Value::as_i64) == Some(1)
                && !bid
                    .get("adm")
                    .and_then(Value::as_str)
                    .is_some_and(|v| v.contains("${CLICK_URL:URLENCODE}"))
            {
                error("openrtb.profile.bidswitch.click_macro", "Banner responses must include the supplier click macro when ext.clktrkrq requests it.", join_instance_path(&path, "adm"), issues);
            }
        }
        if one_media && imp.get("video").is_some_and(Value::is_object) {
            required(bid, "protocol", &path, "BidSwitch video response", issues);
            if supplier {
                required(
                    bid,
                    "adm",
                    &path,
                    "BidSwitch supplier video response",
                    issues,
                );
            } else if value_at(imp, "video.ext.vast_url_rq").and_then(Value::as_i64) == Some(1) {
                required(
                    bid,
                    "ext.vast_url",
                    &path,
                    "BidSwitch video URL requirement",
                    issues,
                );
            } else if !present(bid, "ext.vast_url") && !present(bid, "nurl") {
                error(
                    "openrtb.profile.bidswitch.video_url",
                    "Video bids need ext.vast_url or the documented nurl fallback.",
                    join_instance_path(&path, "ext.vast_url"),
                    issues,
                );
            }
        }
        if one_media && imp.get("audio").is_some_and(Value::is_object) {
            if supplier {
                required(
                    bid,
                    "adm",
                    &path,
                    "BidSwitch supplier audio response",
                    issues,
                );
            } else {
                match value_at(imp, "audio.ext.format").and_then(Value::as_i64) {
                    Some(1) => required(
                        bid,
                        "ext.daast_url",
                        &path,
                        "BidSwitch audio format 1",
                        issues,
                    ),
                    Some(2) => required(
                        bid,
                        "ext.vast_url",
                        &path,
                        "BidSwitch audio format 2",
                        issues,
                    ),
                    Some(3) if !present(bid, "ext.vast_url") && !present(bid, "ext.daast_url") => {
                        error(
                            "openrtb.profile.bidswitch.audio_url",
                            "Audio format 3 needs either VAST or DAAST URL.",
                            join_instance_path(&path, "ext"),
                            issues,
                        )
                    }
                    None => required(
                        bid,
                        "ext.daast_url",
                        &path,
                        "BidSwitch audio default",
                        issues,
                    ),
                    _ => {}
                }
            }
        }
        if one_media && imp.get("native").is_some_and(Value::is_object) {
            let response_field = if supplier { "adm_native" } else { "ext.native" };
            required(
                bid,
                response_field,
                &path,
                "BidSwitch native response",
                issues,
            );
            let request_field = if supplier {
                "native.request_native"
            } else {
                "native.request"
            };
            if let (Some(native_request), Some(native_response)) = (
                value_at(imp, request_field).and_then(Value::as_object),
                value_at(bid, response_field).and_then(Value::as_object),
            ) {
                if !supplier {
                    crate::native::validate_response_against_request(
                        &crate::native::index_markup_request(native_request),
                        native_response,
                        &join_instance_path(&path, response_field),
                        issues,
                    );
                }
            }
            if !supplier
                && value_at(imp, "native.ext.adchoicesurl_required").and_then(Value::as_i64)
                    == Some(1)
            {
                required(
                    bid,
                    "ext.native.ext.adchoiceurl",
                    &path,
                    "BidSwitch Ad Choices URL requirement",
                    issues,
                );
            }
        }
        if !supplier {
            let s2s = value_at(imp, "ext.s2s_nurl")
                .or_else(|| value_at(request, "ext.s2s_nurl"))
                .and_then(Value::as_i64)
                == Some(1);
            if s2s
                && bid
                    .get("adm")
                    .and_then(Value::as_str)
                    .is_some_and(|v| v.contains("${AUCTION_PRICE}"))
            {
                error(
                    "openrtb.profile.bidswitch.s2s_markup_macro",
                    "Server-to-server notification requests forbid the win price macro in adm.",
                    join_instance_path(&path, "adm"),
                    issues,
                );
            }
            if !s2s
                && (imp.get("secure").and_then(Value::as_i64) == Some(1)
                    || imp.get("secure").and_then(Value::as_bool) == Some(true))
            {
                if let Some(nurl) = bid.get("nurl").and_then(Value::as_str) {
                    if !nurl.to_ascii_lowercase().starts_with("https://") {
                        error(
                            "openrtb.profile.bidswitch.secure_notification",
                            "Browser win notifications on secure inventory require HTTPS.",
                            join_instance_path(&path, "nurl"),
                            issues,
                        );
                    }
                }
            }
            let supplier_id = value_at(request, "ext.ssp")
                .and_then(Value::as_str)
                .map(str::to_ascii_lowercase);
            if let Some(id) = supplier_id.as_deref() {
                let fields: &[&str] = match id {
                    "rubicon" | "nexage" => &["cid"],
                    "smaato" | "mopub" => &["cid", "cat"],
                    "yieldone" if imp.get("banner").is_some_and(Value::is_object) => {
                        &["w", "h", "cat"]
                    }
                    "pubmatic" if imp.get("banner").is_some_and(Value::is_object) => &["w", "h"],
                    "adscale" => &["ext.advertiser_name", "ext.agency_name"],
                    "centro" | "brx" => &["ext.advertiser_name"],
                    "fyber" => &["bundle"],
                    "trustx" => &["nurl"],
                    "yieldone" => &["cat"],
                    "appnexus" if present(request, "app") => &["ext.lpdomain"],
                    _ => &[],
                };
                for field in fields {
                    required(
                        bid,
                        field,
                        &path,
                        "BidSwitch documented supplier requirement",
                        issues,
                    );
                }
            }
        }
    }
}

fn protocol_major(version: &str) -> Option<u32> {
    let (major, minor) = version.split_once('.')?;
    if major.is_empty()
        || minor.is_empty()
        || !major
            .bytes()
            .chain(minor.bytes())
            .all(|c| c.is_ascii_digit())
    {
        return None;
    }
    major.parse().ok()
}

fn enum_field(
    object: &Map<String, Value>,
    field: &str,
    options: &[i64],
    path: &str,
    issues: &mut Vec<Issue>,
) {
    if let Some(value) = value_at(object, field).filter(|value| !value.is_null()) {
        if !value.as_i64().is_some_and(|v| options.contains(&v)) {
            error(
                "openrtb.profile.bidswitch.value_invalid",
                "The extension value is outside the BidSwitch documented set.",
                join_instance_path(path, field),
                issues,
            );
        }
    }
}

fn visit(
    object: &Map<String, Value>,
    fields: &str,
    path: &str,
    emit: &mut impl FnMut(&Value, &str),
) {
    let (head, tail) = fields.split_once('.').unwrap_or((fields, ""));
    let array = head.ends_with("[]");
    let field = head.strip_suffix("[]").unwrap_or(head);
    let Some(value) = object.get(field).filter(|value| !value.is_null()) else {
        return;
    };
    let field_path = join_instance_path(path, field);
    if array {
        if let Some(values) = value.as_array() {
            for (index, value) in values.iter().enumerate() {
                if let Some(object) = value.as_object() {
                    visit(object, tail, &format!("{field_path}[{index}]"), emit);
                }
            }
        }
    } else if tail.is_empty() {
        emit(value, &field_path);
    } else if let Some(object) = value.as_object() {
        visit(object, tail, &field_path, emit);
    }
}

fn check_shape(value: &Value, shape: Shape, path: &str, issues: &mut Vec<Issue>) {
    let scalar = match shape {
        Shape::String => value.is_string(),
        Shape::Integer => value.is_i64() || value.is_u64(),
        Shape::Number => value.is_number(),
        Shape::Flag => value.is_boolean() || value.as_i64().is_some_and(|v| matches!(v, 0 | 1)),
        Shape::Object => value.is_object(),
        Shape::Strings | Shape::Integers | Shape::Objects => {
            if let Some(values) = value.as_array() {
                let item = match shape {
                    Shape::Strings => Shape::String,
                    Shape::Integers => Shape::Integer,
                    _ => Shape::Object,
                };
                for (index, value) in values.iter().enumerate() {
                    check_shape(value, item, &format!("{path}[{index}]"), issues);
                }
                return;
            }
            false
        }
    };
    if !scalar {
        error(
            "openrtb.profile.field_type",
            "The field does not match the documented BidSwitch JSON type.",
            path.to_owned(),
            issues,
        );
    }
}

// Descriptor names and JSON types are extracted facts from the pinned primary tables.
const DESCRIPTORS: &[Descriptor] = &[
    Descriptor {
        object: "BidRequest",
        path: "ext.ssp",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.media_src",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.ads_txt",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.google",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.google_query_id",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.gumgum",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.rubicon",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.adtruth",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.tv",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.dooh",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.clktrkrq",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.s2s_nurl",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.is_secure",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.wt",
        shape: Shape::Number,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.tgroup",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.ads_txt.status",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.ads_txt.pub_id",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.ads_txt.auth_id",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.ads_txt.supplier_domain",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "refresh.refsettings",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "refresh.count",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "refresh.refsettings[].reftype",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "refresh.refsettings[].minint",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.wopv",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.google",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.yieldone",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.inventory_class",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.notification_type",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.s2s_nurl",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.gpid",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.ae",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.ssai",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.autostore",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "qty.multiplier",
        shape: Shape::Number,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "qty.sourcetype",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "qty.vendor",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.version",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.versions",
        shape: Shape::Strings,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.sourceapp",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.skadnetids",
        shape: Shape::Strings,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.skadnetlist",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.productpage",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.skadnetlist.max",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.skadnetlist.excl",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.skadnetlist.addl",
        shape: Shape::Strings,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.autostore.present",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.autostore.click",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.google.excluded_attribute",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.google.allowed_vendor_type",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.yieldone.allowed_creative_types",
        shape: Shape::Strings,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.yieldone.allowed_creative_category_id",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.yieldone.cat",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Imp",
        path: "ext.yieldone.inventory_class",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.dooh.publisher",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.dooh.audience",
        shape: Shape::Number,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.dooh.impmultiply",
        shape: Shape::Number,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.google.detected_vertical",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.google.detected_vertical[].id",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.google.detected_vertical[].weight",
        shape: Shape::Number,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.adtruth.tdl_millis",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.gumgum.cat",
        shape: Shape::Strings,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.rubicon.ast",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Banner",
        path: "ext.rewarded",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Video",
        path: "ext.rewarded",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Audio",
        path: "ext.format",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Device",
        path: "ext.atts",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Device",
        path: "ext.truncated_ip",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Device",
        path: "ext.ifa_type",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Device",
        path: "ext.idv",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Device",
        path: "ext.cdep",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "User",
        path: "ext.ug",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "User",
        path: "ext.cookie_age",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "User",
        path: "ext.google_consent",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "User",
        path: "ext.impdepth",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "User",
        path: "ext.sessionduration",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "User",
        path: "ext.consented_providers_settings",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "User",
        path: "ext.eids",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "User",
        path: "ext.consent",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "User",
        path: "ext.consented_providers_settings.consented_providers",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Site",
        path: "ext.amp",
        shape: Shape::Flag,
        supplier: false,
    },
    Descriptor {
        object: "Site",
        path: "ext.inventorypartnerdomain",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "App",
        path: "ext.inventorypartnerdomain",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.omidpn",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.omidpv",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.schain",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.schain.complete",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.schain.nodes",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.schain.ver",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.schain.nodes[].asi",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.schain.nodes[].sid",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.schain.nodes[].hp",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.schain.nodes[].rid",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.schain.nodes[].name",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Source",
        path: "ext.schain.nodes[].domain",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Data",
        path: "ext.segtax",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Data",
        path: "ext.segclass",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Deal",
        path: "ext.buyer_wseat",
        shape: Shape::Strings,
        supplier: false,
    },
    Descriptor {
        object: "Deal",
        path: "ext.deal_type",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Deal",
        path: "ext.guaranteed",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Regs",
        path: "ext.gdpr",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Regs",
        path: "ext.us_privacy",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.dsarequired",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.pubrender",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.datatopub",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.transparency",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.transparency[].domain",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.transparency[].dsaparams",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.at1",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.asid",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.country",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.advertiser_name",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.agency_name",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.agency_id",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.lpdomain",
        shape: Shape::Strings,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.data",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.language",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.google",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.yieldone",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.vast_url",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.autostore",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.daast_url",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dur",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.deal",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.img_url",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.click_url",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.js_url",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.version",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.network",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.campaign",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.fidelities",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.itunesitem",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.nonce",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.sourceapp",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.timestamp",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.signature",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.productpageid",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.sourceidentifier",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.fidelities[].fidelity",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.fidelities[].nonce",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.fidelities[].timestamp",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.fidelities[].signature",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.behalf",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.paid",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.transparency",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.adrender",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.transparency[].domain",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.transparency[].dsaparams",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.autostore.present",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.autostore.click",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.autostore.show_on_skip",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.data[].name",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.data[].value",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.protocol",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.true_price_opt_out",
        shape: Shape::Flag,
        supplier: false,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid[].id",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid[].igbuyer",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid[].igbuyer[].origin",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Native",
        path: "request",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Native",
        path: "battr",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Native",
        path: "api",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Native",
        path: "ext",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Native",
        path: "ext.triplelift",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Native",
        path: "ext.adchoicesurl_required",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Native",
        path: "ext.triplelift.formats",
        shape: Shape::Integers,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.link",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.ext",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.ver",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.privacy",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.eventtrackers",
        shape: Shape::Objects,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.imptrackers",
        shape: Shape::Strings,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.jstracker",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.ext.viewtracker",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.ext.adchoiceurl",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].id",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].required",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].title",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].img",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].video",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].data",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].link",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].title.text",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].img.url",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].img.h",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].img.w",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].video.vasttag",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].video.ext",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].video.ext.playbackmethod",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.assets[].data.value",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.link.url",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.link.clicktrackers",
        shape: Shape::Strings,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.eventtrackers[].event",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.eventtrackers[].method",
        shape: Shape::Integer,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.eventtrackers[].url",
        shape: Shape::String,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.eventtrackers[].customdata",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "Bid",
        path: "ext.native.eventtrackers[].ext",
        shape: Shape::Object,
        supplier: false,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.dsp_uuids",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.google_query_id",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.google_audiences",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.purch",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.ads_txt",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.ads_txt.status",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.ads_txt.pub_id",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.ads_txt.auth_id",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "BidRequest",
        path: "ext.ads_txt.supplier_domain",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "refresh.refsettings",
        shape: Shape::Objects,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "refresh.count",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "refresh.refsettings[].reftype",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "refresh.refsettings[].minint",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.wopv",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.wseat",
        shape: Shape::Objects,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.gpid",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.ae",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.crawlid",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.autostore",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "qty.multiplier",
        shape: Shape::Number,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "qty.sourcetype",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "qty.vendor",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.version",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.versions",
        shape: Shape::Strings,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.sourceapp",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.skadnetids",
        shape: Shape::Strings,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.skadnetlist",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.productpage",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.skadnetlist.max",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.skadnetlist.excl",
        shape: Shape::Integers,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.skadn.skadnetlist.addl",
        shape: Shape::Strings,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.autostore.present",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Imp",
        path: "ext.autostore.click",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Banner",
        path: "ext.rewarded",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Video",
        path: "ext.rewarded",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Device",
        path: "ext.atts",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Device",
        path: "ext.dooh",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Device",
        path: "ext.truncated_ip",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Device",
        path: "ext.ifa_type",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Device",
        path: "ext.idfv",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Device",
        path: "ext.cdep",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "User",
        path: "ext.impdepth",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "User",
        path: "ext.sessionduration",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "User",
        path: "ext.consented_providers_settings",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "User",
        path: "ext.consented_providers_settings.consented_providers",
        shape: Shape::Integers,
        supplier: true,
    },
    Descriptor {
        object: "Site",
        path: "ext.amp",
        shape: Shape::Flag,
        supplier: true,
    },
    Descriptor {
        object: "Source",
        path: "ext.omidpn",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Source",
        path: "ext.omidpv",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Data",
        path: "ext.segtax",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Data",
        path: "ext.segclass",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Deal",
        path: "ext.buyer_wseat",
        shape: Shape::Strings,
        supplier: true,
    },
    Descriptor {
        object: "Deal",
        path: "ext.deal_type",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Deal",
        path: "ext.guaranteed",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.dsarequired",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.pubrender",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.datatopub",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.transparency",
        shape: Shape::Objects,
        supplier: true,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.transparency[].domain",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Regs",
        path: "ext.dsa.transparency[].dsaparams",
        shape: Shape::Integers,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.advertiser_name",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.agency_name",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.agency_id",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.third_party_buyer_token",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.data",
        shape: Shape::Objects,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.autostore",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.version",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.network",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.campaign",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.fidelities",
        shape: Shape::Objects,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.itunesitem",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.nonce",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.sourceapp",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.timestamp",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.signature",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.productpageid",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.sourceidentifier",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.fidelities[].fidelity",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.fidelities[].nonce",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.fidelities[].timestamp",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.skadn.fidelities[].signature",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.behalf",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.paid",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.transparency",
        shape: Shape::Objects,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.adrender",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.transparency[].domain",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.dsa.transparency[].dsaparams",
        shape: Shape::Integers,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.autostore.present",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.autostore.click",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.autostore.show_on_skip",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.data[].name",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "ext.data[].value",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid",
        shape: Shape::Objects,
        supplier: true,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid[].impid",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid[].igbuyer",
        shape: Shape::Objects,
        supplier: true,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid[].ignurl",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid[].igbuyer[].origin",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid[].igbuyer[].seat",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid[].igbuyer[].cur",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "BidResponse",
        path: "ext.igbid[].igbuyer[].ps",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Native",
        path: "request_native",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Native",
        path: "battr",
        shape: Shape::Integers,
        supplier: true,
    },
    Descriptor {
        object: "Native",
        path: "api",
        shape: Shape::Integers,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets",
        shape: Shape::Objects,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.link",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.imptrackers",
        shape: Shape::Strings,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.ver",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.jstracker",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.eventtrackers",
        shape: Shape::Objects,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.privacy",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].id",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].required",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].title",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].img",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].video",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].data",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].link",
        shape: Shape::Object,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].title.text",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].img.url",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].img.h",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].img.w",
        shape: Shape::Integer,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].video.vasttag",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.assets[].data.value",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.link.url",
        shape: Shape::String,
        supplier: true,
    },
    Descriptor {
        object: "Bid",
        path: "adm_native.link.clicktrackers",
        shape: Shape::Strings,
        supplier: true,
    },
];
