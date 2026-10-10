//! Microsoft Monetize requests sent to bidders and responses returned by bidders.
//! Supplier-ingest fields and account configuration are a different protocol.

use super::{
    join_instance_path, path_populated, profile_issue, require_integer_in_range, value_at,
};
use crate::Issue;
use serde_json::{Map, Value};

#[derive(Clone, Copy)]
enum Kind {
    Integer,
    Number,
    String,
    Object,
    Integers,
    Flag,
}

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    // `ext` remains open. Only documented members are checked when supplied.
    let fields: &[(&str, Kind)] = match object_name {
        "BidRequest" => &[
            ("ext.appnexus", Kind::Object),
            ("ext.appnexus.seller_member_id", Kind::Integer),
            ("ext.appnexus.ext_inv_code", Kind::Integer),
            ("ext.appnexus.publisher_integration", Kind::Object),
            ("ext.appnexus.publisher_integration.is_header", Kind::Flag),
        ],
        "Imp" => &[
            ("ext.appnexus", Kind::Object),
            ("ext.appnexus.estimated_clear_price", Kind::Number),
            ("ext.appnexus.predicted_view_rate", Kind::Number),
            ("ext.appnexus.predicted_view_rate_over_total", Kind::Number),
            ("ext.appnexus.predicted_video_view_rate", Kind::Number),
            (
                "ext.appnexus.predicted_video_view_rate_over_total",
                Kind::Number,
            ),
            ("ext.appnexus.predicted_video_completion_rate", Kind::Number),
            ("ext.appnexus.member_ad_profile_id", Kind::Integer),
            ("ext.appnexus.traffic_source_code", Kind::String),
            ("ext.appnexus.gpid", Kind::String),
            ("ext.tid", Kind::String),
        ],
        "Video" => &[("ext.appnexus", Kind::Object)],
        "Deal" => &[
            ("ext.appnexus", Kind::Object),
            ("ext.appnexus.allowed_media_types", Kind::Integers),
            ("ext.appnexus.allowed_media_subtypes", Kind::Integers),
        ],
        "App" => &[("ext.inventorypartnerdomain", Kind::String)],
        "Content" => &[("ext.network", Kind::String)],
        "Device" => &[("ext.ifa_type", Kind::String)],
        "Geo" => &[
            ("ext.appnexus", Kind::Object),
            ("ext.appnexus.timezone", Kind::String),
        ],
        "Bid" => &[
            ("ext.appnexus", Kind::Object),
            ("ext.appnexus.min_price", Kind::Number),
            ("ext.appnexus.custom_notify_data", Kind::String),
            ("ext.appnexus.click_url", Kind::String),
            ("ext.dsa", Kind::Object),
            ("ext.dsa.behalf", Kind::String),
            ("ext.dsa.paid", Kind::String),
        ],
        _ => &[],
    };
    check_fields(object, fields, path, issues);
    match object_name {
        "BidRequest" => {
            if object.get("badv").and_then(Value::as_array).is_some_and(|v| v.len() > 64) {
                error("openrtb.profile.xandr.array_limit", "Xandr sends at most 64 blocked advertiser domains.", join_instance_path(path, "badv"), issues);
            }
        }
        "Video" => require_integer_in_range(object, "ext.appnexus.context", 0, 10,
            "Xandr video context must be 0 through 10, including accompanying-content contexts 8, 9 and 10.", path, issues),
        "Source" | "Imp" => {
            require_integer_in_range(object, "ext.tidt", 1, 2,
                "Xandr transaction ID type must be 1 (globally unique) or 2 (not unique across paths).", path, issues);
            if object_name == "Imp" { validate_payments(object, "ext.appnexus.allowed_payment_types", path, true, issues); }
        }
        "Deal" => {
            require_integer_in_range(object, "ext.appnexus.ad_quality_override", 1, 3,
                "Xandr deal ad_quality_override must be 1, 2 or 3.", path, issues);
            for field in ["ext.appnexus.sc", "ext.appnexus.gtd"] {
                require_integer_in_range(object, field, 1, 1,
                    "Xandr sends this deal flag only when its value is 1.", path, issues);
            }
            object_array(object, "ext.appnexus.sizes", path, issues, |size, base, issues| {
                check_fields(size, &[("w", Kind::Integer), ("h", Kind::Integer)], base, issues);
            });
        }
        "SeatBid" => {
            if !path_populated(object, "seat") {
                error("openrtb.profile.field_required", "Xandr bidder responses require SeatBid.seat; registered buyer seat codes can be nonnumeric.", join_instance_path(path, "seat"), issues);
            }
        }
        "Bid" => validate_bid(object, path, issues),
        "BidResponse" => validate_payment_currency(object, path, issues),
        _ => {}
    }
}

fn check_fields(
    object: &Map<String, Value>,
    fields: &[(&str, Kind)],
    path: &str,
    issues: &mut Vec<Issue>,
) {
    for (field, kind) in fields {
        let Some(value) = value_at(object, field) else {
            continue;
        };
        let valid = match kind {
            Kind::Integer => value.is_i64() || value.is_u64(),
            Kind::Number => value.is_number(),
            Kind::String => value.is_string(),
            Kind::Object => value.is_object(),
            Kind::Integers => value
                .as_array()
                .is_some_and(|v| v.iter().all(|x| x.is_i64() || x.is_u64())),
            // The table calls this Boolean and explicitly documents 0 and 1.
            Kind::Flag => value.is_boolean() || matches!(value.as_i64(), Some(0 | 1)),
        };
        if !valid {
            error(
                "openrtb.profile.xandr.type_invalid",
                &format!("Xandr {field} has an invalid documented JSON type."),
                join_instance_path(path, field),
                issues,
            );
        }
    }
}

fn object_array<F>(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    issues: &mut Vec<Issue>,
    mut check: F,
) where
    F: FnMut(&Map<String, Value>, &str, &mut Vec<Issue>),
{
    let Some(value) = value_at(object, field) else {
        return;
    };
    let base = join_instance_path(path, field);
    let Some(values) = value.as_array() else {
        error(
            "openrtb.profile.xandr.type_invalid",
            "Xandr extension field must be an array of objects.",
            base,
            issues,
        );
        return;
    };
    for (index, item) in values.iter().enumerate() {
        let item_path = format!("{base}[{index}]");
        if let Some(item) = item.as_object() {
            check(item, &item_path, issues);
        } else {
            error(
                "openrtb.profile.xandr.type_invalid",
                "Xandr extension array entries must be objects.",
                item_path,
                issues,
            );
        }
    }
}

fn validate_payments(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    request: bool,
    issues: &mut Vec<Issue>,
) {
    object_array(object, field, path, issues, |payment, base, issues| {
        let fields: &[(&str, Kind)] = if request {
            &[
                ("payment_type", Kind::Integer),
                ("conversion_rate", Kind::Number),
            ]
        } else {
            &[("payment_type", Kind::Integer), ("price", Kind::Number)]
        };
        check_fields(payment, fields, base, issues);
        if let Some(value) = payment
            .get("payment_type")
            .filter(|value| value.is_i64() || value.is_u64())
        {
            if !value
                .as_i64()
                .is_some_and(|value| [1, 2, 6, 8, 9].contains(&value))
            {
                error(
                    "openrtb.profile.value_invalid",
                    "Xandr payment_type must be 1, 2, 6, 8 or 9.",
                    join_instance_path(base, "payment_type"),
                    issues,
                );
            }
        }
        if request {
            require_integer_in_range(
                payment,
                "imp_count_method",
                0,
                3,
                "Xandr imp_count_method must be 0 through 3.",
                base,
                issues,
            );
        } else if payment
            .get("payment_type")
            .and_then(Value::as_i64)
            .is_some_and(|v| [2, 6, 8, 9].contains(&v))
            && !path_populated(object, "burl")
        {
            error(
                "openrtb.profile.xandr.billing_url_required",
                "Xandr non-impression payment bids require a billing notify URL in bid.burl.",
                join_instance_path(path, "burl"),
                issues,
            );
        }
    });
}

fn validate_bid(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    object_array(
        object,
        "ext.appnexus.custom_macros",
        path,
        issues,
        |item, base, issues| {
            check_fields(
                item,
                &[("name", Kind::String), ("value", Kind::String)],
                base,
                issues,
            );
            if item
                .get("value")
                .and_then(Value::as_str)
                .is_some_and(|v| v.chars().count() > 550)
            {
                error(
                    "openrtb.profile.xandr.string_limit",
                    "Xandr custom macro values have a maximum length of 550 characters.",
                    join_instance_path(base, "value"),
                    issues,
                );
            }
        },
    );
    validate_payments(object, "ext.appnexus.bid_payment_type", path, false, issues);
    for field in ["nurl", "burl", "lurl"] {
        let Some(url) = object.get(field).and_then(Value::as_str) else {
            continue;
        };
        let mut tail = url;
        while let Some(start) = tail.find("${") {
            tail = &tail[start + 2..];
            let Some(end) = tail.find('}') else { break };
            let name = &tail[..end];
            let allowed = [
                "AUCTION_ID",
                "AUCTION_BID_ID",
                "AUCTION_IMP_ID",
                "AUCTION_SEAT_ID",
                "AUCTION_AD_ID",
                "AUCTION_CURRENCY",
                "CREATIVE_CODE",
            ]
            .contains(&name)
                || (field == "lurl" && ["AUCTION_LOSS", "AUCTION_MIN_TO_WIN"].contains(&name))
                || (field != "lurl" && ["AUCTION_PRICE", "AN_PAYMENT_TYPE"].contains(&name));
            if !allowed {
                error(
                    "openrtb.profile.xandr.notify_macro",
                    &format!("Xandr does not document ${{{name}}} in bid.{field}."),
                    join_instance_path(path, field),
                    issues,
                );
            }
            tail = &tail[end + 1..];
        }
        // The bound applies after expansion. With a macro, expansion length is unknown.
        if field != "lurl" && !url.contains("${") && url.chars().count() > 2000 {
            error(
                "openrtb.profile.xandr.string_limit",
                "Xandr notify URLs may contain at most 2000 characters after macro expansion.",
                join_instance_path(path, field),
                issues,
            );
        }
    }
    for field in ["ext.dsa.behalf", "ext.dsa.paid"] {
        if value_at(object, field)
            .and_then(Value::as_str)
            .is_some_and(|v| v.chars().count() > 100)
        {
            error(
                "openrtb.profile.xandr.string_limit",
                "Xandr DSA names may contain at most 100 Unicode characters.",
                join_instance_path(path, field),
                issues,
            );
        }
    }
    require_integer_in_range(
        object,
        "ext.dsa.adrender",
        0,
        1,
        "Xandr DSA adrender must be 0 or 1.",
        path,
        issues,
    );
    object_array(
        object,
        "ext.dsa.transparency",
        path,
        issues,
        |item, base, issues| {
            check_fields(
                item,
                &[("domain", Kind::String), ("params", Kind::Integers)],
                base,
                issues,
            );
        },
    );
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    let Some(seats) = response.get("seatbid").and_then(Value::as_array) else {
        return;
    };
    let imps = request.get("imp").and_then(Value::as_array);
    for (seat_index, seat) in seats.iter().enumerate() {
        let Some(bids) = seat.get("bid").and_then(Value::as_array) else {
            continue;
        };
        for (bid_index, bid) in bids.iter().enumerate() {
            let Some(bid) = bid.as_object() else { continue };
            let base = format!("seatbid[{seat_index}].bid[{bid_index}]");
            let payments = value_at(bid, "ext.appnexus.bid_payment_type").and_then(Value::as_array);
            let Some(impid) = bid.get("impid").and_then(Value::as_str) else {
                continue;
            };
            let matches: Vec<_> = imps
                .map(|values| {
                    values
                        .iter()
                        .filter(|imp| imp.get("id").and_then(Value::as_str) == Some(impid))
                        .collect()
                })
                .unwrap_or_default();
            if matches.len() != 1 {
                continue;
            }
            let Some(imp) = matches[0].as_object() else {
                continue;
            };
            let offered = value_at(imp, "ext.appnexus.allowed_payment_types");
            if let Some(payments) = payments {
                for (index, payment) in payments.iter().enumerate() {
                    let Some(kind) = payment.get("payment_type").and_then(Value::as_i64) else {
                        continue;
                    };
                    // Missing offer explicitly means impression only. Malformed offers defer.
                    let allowed = match offered {
                        None => Some(kind == 1),
                        Some(Value::Array(values))
                            if values.iter().all(|v| {
                                v.get("payment_type").and_then(Value::as_i64).is_some()
                            }) =>
                        {
                            Some(values.iter().any(|v| {
                                v.get("payment_type").and_then(Value::as_i64) == Some(kind)
                            }))
                        }
                        _ => None,
                    };
                    if allowed == Some(false) {
                        error("openrtb.profile.xandr.payment_not_offered", "The Xandr request does not offer this payment type for the referenced impression.", format!("{base}.ext.appnexus.bid_payment_type[{index}].payment_type"), issues);
                    }
                }
            }
            if let Some(video) = imp.get("video").and_then(Value::as_object) {
                if bid
                    .get("slotinpod")
                    .and_then(Value::as_i64)
                    .is_some_and(|slot| slot != 0)
                    && match video.get("slotinpod") {
                        None => true,
                        Some(value) => value.as_i64() == Some(0),
                    }
                {
                    error("openrtb.profile.xandr.slot_not_offered", "Xandr bid.slotinpod may identify a guaranteed position only when the seller offered one.", format!("{base}.slotinpod"), issues);
                }
                // Candidate bids are alternatives. Never sum every offer as a winning pod.
                if let (Some(duration), Some(pod_duration)) = (
                    bid.get("dur").and_then(Value::as_i64),
                    video.get("poddur").and_then(Value::as_i64),
                ) {
                    if duration > pod_duration {
                        error(
                            "openrtb.profile.xandr.pod_duration",
                            "This Xandr creative is longer than the entire offered dynamic pod.",
                            format!("{base}.dur"),
                            issues,
                        );
                    }
                }
            }
        }
    }
}

fn validate_payment_currency(response: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(currency) = response.get("cur").and_then(Value::as_str) else {
        return;
    };
    if currency == "USD" {
        return;
    }
    let Some(seats) = response.get("seatbid").and_then(Value::as_array) else {
        return;
    };
    if seats
        .iter()
        .filter_map(|seat| seat.get("bid").and_then(Value::as_array))
        .flatten()
        .filter_map(Value::as_object)
        .filter_map(|bid| value_at(bid, "ext.appnexus.bid_payment_type").and_then(Value::as_array))
        .flatten()
        .any(|payment| {
            payment
                .get("payment_type")
                .and_then(Value::as_i64)
                .is_some_and(|value| [2, 6, 8, 9].contains(&value))
        })
    {
        error(
            "openrtb.profile.xandr.payment_currency",
            "Xandr viewable-impression payment bids support USD only.",
            join_instance_path(path, "cur"),
            issues,
        );
    }
}

fn error(id: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    issues.push(profile_issue(id, String::from(message), path));
}
