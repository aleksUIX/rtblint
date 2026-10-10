//! Unity Exchange requests emitted to DSPs and the DSP responses they consume.
//! Sources are recorded in docs/exchange-profiles/unity-vungle.md.

use serde_json::{Map, Value};

use super::{
    join_instance_path, path_populated, profile_issue, require_integer_in_range, value_at,
};
use crate::Issue;

#[derive(Clone, Copy)]
enum Kind {
    String,
    Object,
    Strings,
    Integer,
}

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    match object_name {
        "BidRequest" => {
            required_integer(object, "at", 1, 1, path, issues);
            if object
                .get("imp")
                .and_then(Value::as_array)
                .is_some_and(|imps| imps.len() != 1)
            {
                error(
                    "impression_count",
                    "Unity emits exactly one impression per request.",
                    join_instance_path(path, "imp"),
                    issues,
                );
            }
            if let (Some(bundle), Some(imps)) = (
                value_at(object, "app.bundle").and_then(Value::as_str),
                object.get("imp").and_then(Value::as_array),
            ) {
                for (index, imp) in imps.iter().enumerate() {
                    let Some(imp) = imp.as_object() else { continue };
                    if value_at(imp, "ext.skadn.sourceapp")
                        .and_then(Value::as_str)
                        .is_some_and(|sourceapp| sourceapp != bundle)
                    {
                        error(
                            "skadn_mismatch",
                            "Unity SKAdNetwork sourceapp must identify the publisher app.bundle.",
                            format!("{}imp[{index}].ext.skadn.sourceapp", super::prefix(path)),
                            issues,
                        );
                    }
                }
            }
        }
        "Source" => {
            fields(
                object,
                &[("ext.omidpn", Kind::String), ("ext.omidpv", Kind::String)],
                path,
                issues,
            );
            integer(object, "ext.header_bidding", 0, 1, path, issues);
        }
        "Imp" => {
            if object.get("id").and_then(Value::as_str) != Some("1") {
                error(
                    "impression_id",
                    "Unity's sole impression has the string id 1.",
                    join_instance_path(path, "id"),
                    issues,
                );
            }
            required_integer(object, "secure", 1, 1, path, issues);
            fields(
                object,
                &[
                    ("ext.skadn", Kind::Object),
                    ("ext.skadn.ext", Kind::Object),
                    ("ext.skadn.sourceapp", Kind::String),
                    ("ext.skadn.skadnetids", Kind::Strings),
                    ("ext.skadn.versions", Kind::Strings),
                ],
                path,
                issues,
            );
            for field in [
                "ext.skadn.skoverlay",
                "ext.skadn.productpage",
                "ext.skadn.ext.ask",
            ] {
                integer(object, field, 0, 1, path, issues);
            }
            // Mixed or malformed media captures do not establish a banner placement.
            let media_known = media_containers_well_formed(object);
            let banner_only = media_known
                && object.get("banner").is_some_and(Value::is_object)
                && ["video", "audio", "native"]
                    .iter()
                    .all(|field| !object.get(*field).is_some_and(Value::is_object));
            if banner_only {
                integer(object, "instl", 0, 0, path, issues);
            } else if media_known && object.get("video").is_some_and(Value::is_object) {
                integer(object, "instl", 1, 1, path, issues);
            }
        }
        "Video" => {
            required_integer(object, "minduration", 5, 5, path, issues);
            required_integer(object, "maxduration", 30, 30, path, issues);
            required_integer(object, "pos", 7, 7, path, issues);
            if let Some(placement) = object.get("placement") {
                if !matches!(placement.as_i64(), Some(1 | 5)) {
                    error(
                        "value_invalid",
                        "Unity video placement is 1 or 5.",
                        join_instance_path(path, "placement"),
                        issues,
                    );
                }
            }
        }
        "Device" => {
            fields(object, &[("ext.ifv", Kind::String)], path, issues);
            integer(object, "ext.atts", 0, 3, path, issues);
        }
        "Regs" => {
            integer(object, "ext.gdpr", 0, 1, path, issues);
            fields(object, &[("ext.us_privacy", Kind::String)], path, issues);
            // Unity's LGPD type column conflicts with its Boolean example.
            // Account-specific LGPD handling is not a closed JSON type contract.
        }
        "User" => fields(object, &[("ext.consent", Kind::String)], path, issues),
        "BidResponse" => {
            // The documented empty-object and no-bid responses do not carry currency.
            let bidding = object
                .get("seatbid")
                .and_then(Value::as_array)
                .is_some_and(|seats| {
                    seats.iter().any(|seat| {
                        seat.get("bid")
                            .and_then(Value::as_array)
                            .is_some_and(|bids| !bids.is_empty())
                    })
                });
            if bidding {
                required_string(object, "cur", path, issues);
            }
        }
        "SeatBid" => required_string(object, "seat", path, issues),
        "Bid" => validate_bid(object, path, issues),
        _ => {}
    }
}

fn validate_bid(bid: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    required_string(bid, "adm", path, issues);
    required_string(bid, "ext.crtype", path, issues);
    required_array(bid, "cat", path, issues);
    fields(
        bid,
        &[
            ("cat", Kind::Strings),
            ("ext", Kind::Object),
            ("ext.crtype", Kind::String),
            ("ext.appid", Kind::String),
            ("ext.appname", Kind::String),
            ("ext.storeurl", Kind::String),
            ("ext.imptrackers", Kind::Strings),
            ("ext.length", Kind::Integer),
            ("ext.skadn", Kind::Object),
            ("ext.skadn.skoverlay", Kind::Object),
            ("ext.skadn.ext", Kind::Object),
            ("ext.skadn.productpageid", Kind::String),
        ],
        path,
        issues,
    );
    integer(bid, "ext.length", 0, i64::MAX, path, issues);
    let price = bid.get("price");
    if price
        .and_then(Value::as_f64)
        .is_some_and(|price| price <= 0.0)
    {
        error(
            "price_invalid",
            "Unity bid price must be greater than zero.",
            join_instance_path(path, "price"),
            issues,
        );
    }
    if price.is_some_and(|price| price.is_number() && decimal_places(price) > 3) {
        error(
            "price_precision",
            "Unity accepts at most three decimal places in bid price.",
            join_instance_path(path, "price"),
            issues,
        );
    }
    validate_domains(bid, path, issues);
    if let Some(crtype) = value_at(bid, "ext.crtype").and_then(Value::as_str) {
        // PLAYABLE appears in Unity's official response example, although its table omits it.
        const TYPES: &[&str] = &[
            "VAST",
            "VAST 2.0",
            "VAST 3.0",
            "VAST 4.0",
            "VAST VPAID",
            "MRAID playable",
            "MRAID 2.0",
            "BANNER",
            "HTML",
            "HTML5",
            "JS",
            "PLAYABLE",
        ];
        if !TYPES
            .iter()
            .any(|allowed| crtype.eq_ignore_ascii_case(allowed))
        {
            error(
                "creative_type",
                "Unity ext.crtype must be a documented creative type (case-insensitive).",
                join_instance_path(path, "ext.crtype"),
                issues,
            );
        }
        if crtype.eq_ignore_ascii_case("BANNER") {
            dimensions(bid, path, issues);
        }
    }
    if !nonempty_string(value_at(bid, "burl")) {
        required_string(bid, "nurl", path, issues);
        required_array(bid, "ext.imptrackers", path, issues);
    }
    // These fields identify the advertised app. App inventory alone does not.
    let app_ad = [
        "bundle",
        "ext.appid",
        "ext.storeurl",
        "ext.skadn.itunesitem",
    ]
    .iter()
    .any(|field| nonempty_string(value_at(bid, field)));
    if app_ad {
        required_string(bid, "bundle", path, issues);
        required_string(bid, "ext.storeurl", path, issues);
    }
    if bid.get("mtype").and_then(Value::as_i64) == Some(1) {
        dimensions(bid, path, issues);
    }
    for field in [
        "ext.clickablevideo",
        "ext.skadn.skoverlay.show",
        "ext.skadn.skoverlay.position",
        "ext.skadn.skoverlay.dismissable",
    ] {
        integer(bid, field, 0, 1, path, issues);
    }
    integer(bid, "ext.skadn.skoverlay.video_delay", 0, 60, path, issues);
    integer(bid, "ext.skadn.ext.ask", 1, 4, path, issues);
    validate_skadn(bid, path, issues);
}

fn validate_domains(bid: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    required_array(bid, "adomain", path, issues);
    let Some(domains) = bid.get("adomain").and_then(Value::as_array) else {
        return;
    };
    if domains.len() != 1 {
        error(
            "advertiser_domain",
            "Unity accepts exactly one advertiser domain.",
            join_instance_path(path, "adomain"),
            issues,
        );
        return;
    }
    let valid = domains[0].as_str().is_some_and(|domain| {
        !domain.is_empty()
            && !domain.contains('/')
            && !domain.contains("://")
            && !domain.to_ascii_lowercase().starts_with("www.")
    });
    if !valid {
        error("advertiser_domain", "Unity advertiser domain must omit protocol, paths and the www prefix; other subdomains are permitted.", join_instance_path(path, "adomain[0]"), issues);
    }
}

fn validate_skadn(bid: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(skadn) = value_at(bid, "ext.skadn").and_then(Value::as_object) else {
        return;
    };
    let base = join_instance_path(path, "ext.skadn");
    fields(
        skadn,
        &[
            ("version", Kind::String),
            ("signature", Kind::String),
            ("network", Kind::String),
            ("campaign", Kind::String),
            ("sourceidentifier", Kind::String),
            ("itunesitem", Kind::String),
            ("sourceapp", Kind::String),
            ("nonce", Kind::String),
            ("timestamp", Kind::String),
        ],
        &base,
        issues,
    );
    let major = skadn
        .get("version")
        .and_then(Value::as_str)
        .and_then(version_major);
    if let Some(campaign) = skadn.get("campaign").and_then(Value::as_str) {
        if major.is_some_and(|major| major < 4) && !digits_in_range(campaign, 1, 100) {
            error(
                "skadn_campaign",
                "Unity SKAdNetwork 2.x and 3.x campaign is an integer string from 1 through 100.",
                join_instance_path(&base, "campaign"),
                issues,
            );
        }
    }
    if major.is_some_and(|major| major >= 4) {
        required_string(skadn, "sourceidentifier", &base, issues);
    }
    if let Some(sourceid) = skadn.get("sourceidentifier").and_then(Value::as_str) {
        if sourceid.len() != 4 || !sourceid.bytes().all(|byte| byte.is_ascii_digit()) {
            error(
                "skadn_sourceidentifier",
                "Unity SKAdNetwork sourceidentifier is a four-digit string.",
                join_instance_path(&base, "sourceidentifier"),
                issues,
            );
        }
    }
    if let (Some(item), Some(bundle)) = (
        skadn.get("itunesitem").and_then(Value::as_str),
        bid.get("bundle").and_then(Value::as_str),
    ) {
        if item != bundle {
            error(
                "skadn_mismatch",
                "Unity SKAdNetwork itunesitem must match the advertised bid.bundle.",
                join_instance_path(&base, "itunesitem"),
                issues,
            );
        }
    }
    if let Some(fidelities) = skadn.get("fidelities") {
        if let Some(fidelities) = fidelities.as_array() {
            for (index, fidelity) in fidelities.iter().enumerate() {
                let fp = format!("{base}.fidelities[{index}]");
                if let Some(fidelity) = fidelity.as_object() {
                    fields(
                        fidelity,
                        &[
                            ("fidelity", Kind::Integer),
                            ("nonce", Kind::String),
                            ("timestamp", Kind::String),
                            ("signature", Kind::String),
                        ],
                        &fp,
                        issues,
                    );
                    timestamp(fidelity, "timestamp", &fp, issues);
                } else {
                    error(
                        "type_invalid",
                        "Unity SKAdNetwork fidelities entries must be objects.",
                        fp,
                        issues,
                    );
                }
            }
        } else {
            error(
                "type_invalid",
                "Unity SKAdNetwork fidelities must be an array of objects.",
                join_instance_path(&base, "fidelities"),
                issues,
            );
        }
    }
    timestamp(skadn, "timestamp", &base, issues);
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    let Some(imps) = request.get("imp").and_then(Value::as_array) else {
        return;
    };
    let Some(seats) = response.get("seatbid").and_then(Value::as_array) else {
        return;
    };
    for (si, seat) in seats.iter().enumerate() {
        let Some(bids) = seat.get("bid").and_then(Value::as_array) else {
            continue;
        };
        for (bi, bid) in bids.iter().enumerate() {
            let Some(bid) = bid.as_object() else { continue };
            let Some(impid) = bid.get("impid").and_then(Value::as_str) else {
                continue;
            };
            let matches: Vec<_> = imps
                .iter()
                .filter_map(Value::as_object)
                .filter(|imp| imp.get("id").and_then(Value::as_str) == Some(impid))
                .collect();
            if matches.len() != 1 {
                continue;
            }
            let imp = matches[0];
            let path = format!("seatbid[{si}].bid[{bi}]");
            validate_deal_offer(imp, bid, &path, issues);
            if media_containers_well_formed(imp)
                && imp.get("banner").is_some_and(Value::is_object)
                && ["video", "audio", "native"]
                    .iter()
                    .all(|field| !imp.get(*field).is_some_and(Value::is_object))
            {
                dimensions(bid, &path, issues);
            }
            let Some(skadn) = value_at(bid, "ext.skadn").and_then(Value::as_object) else {
                continue;
            };
            let Some(offered) = value_at(imp, "ext.skadn").and_then(Value::as_object) else {
                continue;
            };
            let base = join_instance_path(&path, "ext.skadn");
            if let (Some(actual), Some(expected)) = (
                skadn.get("sourceapp").and_then(Value::as_str),
                offered.get("sourceapp").and_then(Value::as_str),
            ) {
                if actual != expected {
                    error("skadn_mismatch", "Unity SKAdNetwork response sourceapp must match the matched impression's sourceapp.", join_instance_path(&base, "sourceapp"), issues);
                }
            }
            if let (Some(network), Some(networks)) = (
                skadn.get("network").and_then(Value::as_str),
                offered.get("skadnetids").and_then(Value::as_array),
            ) {
                if networks.iter().all(Value::is_string)
                    && !networks.iter().any(|item| item.as_str() == Some(network))
                {
                    error("skadn_mismatch", "Unity SKAdNetwork network must be one of the matched impression's skadnetids.", join_instance_path(&base, "network"), issues);
                }
            }
        }
    }
}

fn media_containers_well_formed(imp: &Map<String, Value>) -> bool {
    ["banner", "video", "audio", "native"]
        .iter()
        .all(|field| imp.get(*field).map_or(true, Value::is_object))
}

fn validate_deal_offer(
    imp: &Map<String, Value>,
    bid: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(chosen) = bid
        .get("dealid")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
    else {
        return;
    };
    let Some(deals) = value_at(imp, "pmp.deals").and_then(Value::as_array) else {
        return;
    };
    let mut offered = Vec::new();
    for deal in deals {
        let Some(id) = deal
            .as_object()
            .and_then(|deal| deal.get("id"))
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        else {
            return;
        };
        if offered.contains(&id) {
            return;
        }
        offered.push(id);
    }
    if !offered.contains(&chosen) {
        error(
            "deal_not_offered",
            "Unity response dealid must match a deal ID offered by the referenced impression.",
            join_instance_path(path, "dealid"),
            issues,
        );
    }
}

fn dimensions(bid: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    for field in ["w", "h"] {
        required_integer(bid, field, 1, i64::MAX, path, issues);
    }
}

fn fields(
    object: &Map<String, Value>,
    entries: &[(&str, Kind)],
    path: &str,
    issues: &mut Vec<Issue>,
) {
    for (field, kind) in entries {
        let Some(value) = value_at(object, field) else {
            continue;
        };
        let valid = match kind {
            Kind::String => value.is_string(),
            Kind::Object => value.is_object(),
            Kind::Strings => value
                .as_array()
                .is_some_and(|items| items.iter().all(Value::is_string)),
            Kind::Integer => value.is_i64() || value.is_u64(),
        };
        if !valid {
            error(
                "type_invalid",
                &format!("Unity {field} has an invalid documented JSON type."),
                join_instance_path(path, field),
                issues,
            );
        }
    }
}

fn required_string(object: &Map<String, Value>, field: &str, path: &str, issues: &mut Vec<Issue>) {
    if !nonempty_string(value_at(object, field)) {
        issues.push(profile_issue(
            "openrtb.profile.field_required",
            format!("Unity requires a non-empty string for {field}."),
            join_instance_path(path, field),
        ));
    }
}

fn required_array(object: &Map<String, Value>, field: &str, path: &str, issues: &mut Vec<Issue>) {
    if !value_at(object, field)
        .and_then(Value::as_array)
        .is_some_and(|items| !items.is_empty())
    {
        issues.push(profile_issue(
            "openrtb.profile.field_required",
            format!("Unity requires a non-empty array for {field}."),
            join_instance_path(path, field),
        ));
    }
}

fn required_integer(
    object: &Map<String, Value>,
    field: &str,
    min: i64,
    max: i64,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    if !path_populated(object, field) {
        issues.push(profile_issue(
            "openrtb.profile.field_required",
            format!("Unity requires {field}."),
            join_instance_path(path, field),
        ));
    } else {
        integer(object, field, min, max, path, issues);
    }
}

fn integer(
    object: &Map<String, Value>,
    field: &str,
    min: i64,
    max: i64,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    require_integer_in_range(
        object,
        field,
        min,
        max,
        &format!("Unity {field} must be an integer from {min} through {max}."),
        path,
        issues,
    );
}

fn nonempty_string(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty())
}

fn version_major(version: &str) -> Option<u32> {
    let (major, minor) = version.split_once('.')?;
    if minor.is_empty() || !minor.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    major.parse().ok()
}

fn digits_in_range(value: &str, min: u32, max: u32) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| byte.is_ascii_digit())
        && value
            .parse::<u32>()
            .is_ok_and(|value| (min..=max).contains(&value))
}

fn timestamp(object: &Map<String, Value>, field: &str, path: &str, issues: &mut Vec<Issue>) {
    if let Some(timestamp) = object.get(field).and_then(Value::as_str) {
        if timestamp.is_empty() || !timestamp.bytes().all(|byte| byte.is_ascii_digit()) {
            error(
                "skadn_timestamp",
                "Unity SKAdNetwork timestamp must be a Unix-milliseconds integer string.",
                join_instance_path(path, field),
                issues,
            );
        }
    }
}

fn decimal_places(number: &Value) -> usize {
    let text = number.to_string();
    let (mantissa, exponent) = match text.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().unwrap_or(0)),
        None => (text.as_str(), 0),
    };
    let fractional = mantissa
        .split_once('.')
        .map_or("", |(_, fractional)| fractional);
    let digits = fractional.trim_end_matches('0').len() as i32 - exponent;
    digits.max(0) as usize
}

fn error(suffix: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    issues.push(profile_issue(
        &format!("openrtb.profile.unity.{suffix}"),
        message.to_owned(),
        path,
    ));
}
