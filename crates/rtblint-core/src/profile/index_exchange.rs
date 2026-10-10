//! Separate Index supplier-ingest and Index-to-DSP contracts.
use super::{
    contract::{self, Field, Kind},
    join_instance_path, value_at,
};
use crate::Issue;
use serde_json::{Map, Value};

include!("index_schema.rs");

pub(super) fn validate(
    seller: bool,
    name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let start = issues.len();
    let schema = if matches!(name, "Bid" | "SeatBid" | "BidResponse") {
        if seller {
            &[]
        } else {
            index_response_schema(name)
        }
    } else if seller {
        index_seller_schema(name)
    } else {
        index_dsp_schema(name)
    };
    if let Some(ext) = object.get("ext").and_then(Value::as_object) {
        contract::fields(ext, schema, &join_instance_path(path, "ext"), issues);
    }
    let label = if seller {
        "Index supplier ingest"
    } else {
        "Index DSP contract"
    };
    match name {
        "BidRequest" => {
            for field in ["device", "tmax"] {
                contract::required(object, field, path, label, issues);
            }
            if seller {
                validate_structured_pod(object, path, issues);
                if object
                    .get("tmax")
                    .and_then(Value::as_i64)
                    .is_some_and(|time| time < 100)
                {
                    contract::error(
                        "openrtb.profile.index.tmax_minimum",
                        "Index supplier requests allow at least 100 milliseconds.",
                        join_instance_path(path, "tmax"),
                        issues,
                    );
                }
            } else {
                contract::required(object, "ext", path, label, issues);
                if let Some(currencies) = object.get("cur").and_then(Value::as_array) {
                    for (index, currency) in currencies.iter().enumerate() {
                        if currency.as_str().is_some_and(|v| v != "USD") {
                            contract::error(
                                "openrtb.profile.index.currency",
                                "Index's published DSP requests use USD.",
                                format!("{}.cur[{index}]", path)
                                    .trim_start_matches('.')
                                    .to_owned(),
                                issues,
                            );
                        }
                    }
                }
            }
            contract::integer_enum(object, "at", &[1], path, issues);
        }
        "Device" => {
            contract::integer_enum(object, "ext.atts", &[0, 1, 2, 3], path, issues);
            if seller && !contract::present(object, "ip") && !contract::present(object, "ipv6") {
                contract::error(
                    "openrtb.profile.index.ip_required",
                    "Index supplier requests require device.ip or device.ipv6.",
                    join_instance_path(path, "ip"),
                    issues,
                );
            }
        }
        "Source" if !seller => {
            if let Some(ext) = object.get("ext").and_then(Value::as_object) {
                for field in ["sourceType", "sourceOrigin"] {
                    contract::required(ext, field, &join_instance_path(path, "ext"), label, issues);
                }
            }
            contract::integer_enum(object, "ext.sourceType", &[1, 2, 3], path, issues);
            contract::integer_enum(
                object,
                "ext.sourceOrigin",
                &[1, 2, 3, 4, 5, 6, 7],
                path,
                issues,
            );
        }
        "Site" if seller => {
            if !contract::present(object, "domain") && !contract::present(object, "page") {
                contract::error(
                    "openrtb.profile.index.site_identity",
                    "Index supplier site requests require domain or page.",
                    join_instance_path(path, "domain"),
                    issues,
                );
            }
        }
        "Site" | "App" | "Publisher" if !seller => {
            contract::required(object, "id", path, label, issues)
        }
        "Imp" => {
            if !seller {
                contract::required(object, "secure", path, label, issues);
                // July 2026 explicitly labels bcrid temporarily unavailable.
                // Other impression extensions do not prove it must be sent.
            }
            contract::integer_enum(object, "ext.ae", &[0, 1], path, issues);
        }
        "Banner" => {
            for field in ["w", "h"] {
                contract::required(object, field, path, label, issues);
            }
            if !seller {
                contract::required(object, "topframe", path, label, issues);
            }
        }
        "Video" => {
            for field in ["minduration", "maxduration", "protocols"] {
                contract::required(object, field, path, label, issues);
            }
            if seller {
                for field in ["w", "h"] {
                    contract::required(object, field, path, label, issues);
                }
            }
            if let Some(protocols) = object.get("protocols").and_then(Value::as_array) {
                if protocols.is_empty() {
                    contract::error(
                        "openrtb.profile.index.video_protocol",
                        "Index requires at least one supported video protocol.",
                        join_instance_path(path, "protocols"),
                        issues,
                    );
                }
                for (index, protocol) in protocols.iter().enumerate() {
                    if protocol
                        .as_i64()
                        .is_some_and(|v| ![2, 3, 5, 6, 7, 8, 11, 12, 13, 14].contains(&v))
                    {
                        contract::error(
                            "openrtb.profile.index.video_protocol",
                            "This video protocol is not supported by Index's published contract.",
                            format!("{path}.protocols[{index}]"),
                            issues,
                        );
                    }
                }
            }
            if seller
                && object
                    .get("mimes")
                    .and_then(Value::as_array)
                    .is_some_and(|mimes| {
                        mimes
                            .iter()
                            .any(|mime| mime.as_str() == Some("application/javascript"))
                    })
            {
                let vpaid = object
                    .get("api")
                    .and_then(Value::as_array)
                    .is_some_and(|api| api.iter().any(|v| matches!(v.as_i64(), Some(1 | 2))));
                if !vpaid {
                    contract::error(
                        "openrtb.profile.index.vpaid_api",
                        "Index requires VPAID API 1 or 2 with application/javascript video.",
                        join_instance_path(path, "api"),
                        issues,
                    );
                }
            }
            // poddur signals a duration-based dynamic pod; structured rqddurs does not.
            if seller && contract::present(object, "poddur") {
                contract::required(object, "maxseq", path, label, issues);
                contract::required(object, "podid", path, label, issues);
            }
            contract::integer_enum(object, "ext.rewarded", &[0, 1], path, issues);
            if !seller {
                contract::integer_enum(object, "ext.playertype", &[1, 2], path, issues);
                contract::integer_enum(object, "ext.renderedBy", &[1, 2], path, issues);
            }
        }
        "Native" if !seller => {
            contract::integer_enum(object, "ext.renderedBy", &[1, 2], path, issues)
        }
        "Deal" => {
            // The guide explicitly leaves exchange-defined auction types open.
            if !seller
                || !object
                    .get("at")
                    .and_then(Value::as_i64)
                    .is_some_and(|at| at >= 500)
            {
                contract::integer_enum(object, "at", &[1, 3], path, issues);
            }
            contract::integer_enum(object, "ext.guaranteed", &[0, 1], path, issues);
            if !seller
                && value_at(object, "ext.disc")
                    .and_then(Value::as_f64)
                    .is_some_and(|discount| !(0.01..=1.0).contains(&discount))
            {
                contract::error(
                    "openrtb.profile.index.deal_discount",
                    "Index deal discounts are fractions from 0.01 through 1.0.",
                    join_instance_path(path, "ext.disc"),
                    issues,
                );
            }
        }
        "Regs" => {
            for (field, values) in [
                ("ext.gdpr", &[0, 1][..]),
                ("ext.dsa.dsarequired", &[0, 1, 2, 3][..]),
                ("ext.dsa.pubrender", &[0, 1, 2][..]),
                ("ext.dsa.datatopub", &[0, 1, 2][..]),
            ] {
                contract::integer_enum(object, field, values, path, issues);
            }
        }
        "Metric" => {
            if matches!(
                object.get("type").and_then(Value::as_str),
                Some("click_through_rate" | "video_completion_rate" | "viewability")
            ) && object
                .get("value")
                .and_then(Value::as_f64)
                .is_some_and(|value| !(0.0..=1.0).contains(&value))
            {
                contract::error(
                    "openrtb.profile.index.metric_probability",
                    "Index probability metrics must be from 0.0 through 1.0.",
                    join_instance_path(path, "value"),
                    issues,
                );
            }
        }
        "Bid" if !seller => {
            contract::required(object, "adm", path, label, issues);
            // The publisher's pass-on state is not observable in a standalone bid.
            if !contract::present(object, "adomain")
                || object
                    .get("adomain")
                    .and_then(Value::as_array)
                    .is_some_and(Vec::is_empty)
            {
                contract::warning("openrtb.profile.index.adomain_expected", "Index expects adomain unless the impression is passed on; that exception needs integration context.",join_instance_path(path,"adomain"),issues);
            }
            if let Some(domains) = object.get("adomain").and_then(Value::as_array) {
                for (index, domain) in domains.iter().enumerate() {
                    if domain.as_str().is_some_and(|v| v.chars().count() > 128) {
                        contract::error(
                            "openrtb.profile.index.adomain_length",
                            "Index advertiser domains allow at most 128 characters.",
                            format!("{path}.adomain[{index}]"),
                            issues,
                        );
                    }
                }
            }
            if let Some(dsa) = value_at(object, "ext.dsa").and_then(Value::as_object) {
                contract::required(
                    dsa,
                    "paid",
                    &join_instance_path(path, "ext.dsa"),
                    label,
                    issues,
                );
                for field in ["behalf", "paid"] {
                    if dsa
                        .get(field)
                        .and_then(Value::as_str)
                        .is_some_and(|v| v.chars().count() > 100)
                    {
                        contract::error(
                            "openrtb.profile.index.dsa_length",
                            "Index DSA names allow at most 100 characters.",
                            join_instance_path(path, &format!("ext.dsa.{field}")),
                            issues,
                        );
                    }
                }
                contract::integer_enum(
                    dsa,
                    "adrender",
                    &[0, 1],
                    &join_instance_path(path, "ext.dsa"),
                    issues,
                );
            }
            if object.get("mtype").and_then(Value::as_i64) == Some(4) {
                native_wrapper(object, path, issues);
            }
        }
        "SeatBid" if !seller && !contract::present(object, "seat") => {
            contract::warning("openrtb.profile.index.seat_expected", "Index normally requires seat, but can disable it per DSP; account configuration is unavailable.",join_instance_path(path,"seat"),issues);
        }
        _ => {}
    }
    let source = if matches!(name, "Bid" | "SeatBid" | "BidResponse") {
        "https://kb.indexexchange.com/dsps/open-rtb/list_of_supported_openrtb_bid_response_fields_dsp.htm"
    } else if seller {
        "https://kb.indexexchange.com/publishers/openrtb_integration/list_of_supported_openrtb_bid_request_fields_for_sellers.htm"
    } else {
        "https://kb.indexexchange.com/dsps/open-rtb/list_of_supported_openrtb_bid_request_fields_dsp.htm"
    };
    for issue in &mut issues[start..] {
        if issue.section.is_none() {
            issue.section = Some(source.to_owned());
        }
    }
}

/// Supplier requests with multiple video impressions describe a single pod.
fn validate_structured_pod(request: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(imps) = request.get("imp").and_then(Value::as_array) else {
        return;
    };
    let mut videos = Vec::new();
    for (index, imp) in imps.iter().enumerate() {
        let Some(imp) = imp.as_object() else { return };
        if let Some(video) = imp.get("video").filter(|video| !video.is_null()) {
            let Some(video) = video.as_object() else {
                return;
            };
            videos.push((index, video));
        }
    }
    if videos.len() < 2 {
        return;
    }
    let mut ids = Vec::new();
    for (index, video) in &videos {
        let video_path = join_instance_path(path, &format!("imp[{index}].video"));
        contract::required(
            video,
            "podid",
            &video_path,
            "Index structured supplier pod",
            issues,
        );
        if let Some(id) = video
            .get("podid")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
        {
            ids.push((*index, id));
        }
    }
    // Standalone type/required checks own incomplete references.
    if ids.len() != videos.len() {
        return;
    }
    let expected = ids[0].1;
    for (index, id) in ids.into_iter().skip(1) {
        if id != expected {
            contract::error(
                "openrtb.profile.index.pod_id_mismatch",
                "Index supplier requests with multiple video impressions require the same podid for a single ad pod.",
                join_instance_path(path, &format!("imp[{index}].video.podid")),
                issues,
            );
        }
    }
}

fn native_wrapper(bid: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    if let Some(adm) = bid
        .get("adm")
        .and_then(Value::as_str)
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
    {
        if adm.is_object() && !adm.get("native").is_some_and(Value::is_object) {
            contract::error(
                "openrtb.profile.index.native_wrapper",
                "Index native bid markup requires an outer native object.",
                join_instance_path(path, "adm"),
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
        let banner_bid = match bid.get("mtype").and_then(Value::as_i64) {
            Some(media) => media == 1,
            None if bid.get("mtype").map_or(true, Value::is_null) => {
                imp.get("banner").is_some_and(Value::is_object)
                    && ["video", "audio", "native"]
                        .iter()
                        .all(|media| imp.get(*media).map_or(true, Value::is_null))
            }
            None => false,
        };
        if banner_bid {
            if let Some(banner) = imp.get("banner").and_then(Value::as_object) {
                if let Some(formats) =
                    banner
                        .get("format")
                        .and_then(Value::as_array)
                        .filter(|formats| {
                            !formats.is_empty()
                                && formats.iter().all(|format| {
                                    let positive = |field| {
                                        format
                                            .get(field)
                                            .and_then(Value::as_u64)
                                            .is_some_and(|v| v > 0)
                                    };
                                    format.is_object()
                                        && ((positive("w") && positive("h"))
                                            || (positive("wratio")
                                                && positive("hratio")
                                                && positive("wmin")))
                                })
                        })
                {
                    for field in ["w", "h"] {
                        contract::required(
                            bid,
                            field,
                            &path,
                            "Index multi-size banner response",
                            issues,
                        );
                    }
                    let fixed = !formats.is_empty()
                        && formats.iter().all(|format| {
                            format
                                .get("w")
                                .and_then(Value::as_u64)
                                .is_some_and(|v| v > 0)
                                && format
                                    .get("h")
                                    .and_then(Value::as_u64)
                                    .is_some_and(|v| v > 0)
                                && ["wratio", "hratio", "wmin"]
                                    .iter()
                                    .all(|field| format.get(*field).map_or(true, Value::is_null))
                        })
                        && ["wmin", "wmax", "hmin", "hmax"]
                            .iter()
                            .all(|field| !contract::present(banner, field));
                    if let (true, Some(width), Some(height)) = (
                        fixed,
                        bid.get("w").and_then(Value::as_u64),
                        bid.get("h").and_then(Value::as_u64),
                    ) {
                        if width > 0
                            && height > 0
                            && !formats.iter().any(|format| {
                                format.get("w").and_then(Value::as_u64) == Some(width)
                                    && format.get("h").and_then(Value::as_u64) == Some(height)
                            })
                        {
                            contract::error("openrtb.profile.index.banner_size_unoffered", "The banner creative dimensions must match an offered fixed format.", join_instance_path(&path, "w"), issues);
                        }
                    }
                }
            }
        }
        if bid.get("mtype").and_then(Value::as_i64) != Some(4)
            && imp.contains_key("native")
            && !imp.contains_key("banner")
            && !imp.contains_key("video")
        {
            native_wrapper(bid, &path, issues);
        }
        if matches!(
            value_at(request, "regs.ext.dsa.dsarequired").and_then(Value::as_i64),
            Some(2 | 3)
        ) {
            contract::required(bid, "ext.dsa", &path, "Index DSA response", issues);
        }
        if let Some(skad) = value_at(bid, "ext.skadn").and_then(Value::as_object) {
            let offered = value_at(imp, "ext.skadn.skadnetids").and_then(Value::as_array);
            if let (Some(network), Some(offered)) =
                (skad.get("network").and_then(Value::as_str), offered)
            {
                if !network.is_empty()
                    && !offered.is_empty()
                    && offered
                        .iter()
                        .all(|value| value.as_str().is_some_and(|value| !value.is_empty()))
                    && !offered.iter().any(|v| v.as_str() == Some(network))
                {
                    contract::error(
                        "openrtb.profile.index.skad_network",
                        "The SKAdNetwork response network must match an offered request ID.",
                        join_instance_path(&path, "ext.skadn.network"),
                        issues,
                    );
                }
            }
            if let (Some(item), Some(bundle)) = (
                skad.get("itunesitem").and_then(Value::as_str),
                bid.get("bundle").and_then(Value::as_str),
            ) {
                if !item.is_empty() && !bundle.is_empty() && item != bundle {
                    contract::error(
                        "openrtb.profile.index.skad_bundle",
                        "SKAdNetwork itunesitem must match bid.bundle.",
                        join_instance_path(&path, "ext.skadn.itunesitem"),
                        issues,
                    );
                }
            }
        }
    }
}
