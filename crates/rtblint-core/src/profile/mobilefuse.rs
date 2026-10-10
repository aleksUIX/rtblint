//! MobileFuse app supply and the narrower SDK token contract.
use super::contract::{self, Field, Kind};
use super::{join_instance_path, value_at};
use crate::Issue;
use serde_json::{Map, Value};
use std::net::{Ipv4Addr, Ipv6Addr};

pub(super) fn validate(
    sdk: bool,
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    match object_name {
        "BidRequest" => {
            contract::integer_enum(object, "at", &[1], path, issues);
            if !sdk {
                for field in ["app", "device"] {
                    contract::required(
                        object,
                        field,
                        path,
                        "MobileFuse app supplier request",
                        issues,
                    );
                }
            }
            if let Some(values) = object.get("imp").and_then(Value::as_array) {
                if values.len() > 1 {
                    contract::warning("openrtb.profile.mobilefuse.impressions_ignored", "MobileFuse processes only the first impression; later impressions are ignored.", join_instance_path(path, "imp"), issues);
                }
            }
            if let Some(value) = object.get("tmax").and_then(Value::as_i64) {
                if value < 100 {
                    contract::error(
                        "openrtb.profile.mobilefuse.tmax_minimum",
                        "MobileFuse tmax must be at least 100 milliseconds.",
                        join_instance_path(path, "tmax"),
                        issues,
                    );
                }
            }
            if object
                .get("badv")
                .and_then(Value::as_array)
                .is_some_and(|v| v.len() > 50)
            {
                contract::error(
                    "openrtb.profile.mobilefuse.badv_limit",
                    "MobileFuse supports at most 50 blocked advertiser domains.",
                    join_instance_path(path, "badv"),
                    issues,
                );
            }
            if sdk {
                validate_token(object, path, issues);
            }
        }
        "Imp" if !sdk || path == "imp[0]" || path.ends_with(".imp[0]") => {
            contract::required(object, "tagid", path, "MobileFuse placement", issues);
        }
        "App" if !sdk => {
            contract::required(object, "bundle", path, "MobileFuse app supplier", issues);
        }
        "Device" => {
            typed_ext(
                object,
                &[
                    Field {
                        name: "ifv",
                        kind: Kind::String,
                    },
                    Field {
                        name: "atts",
                        kind: Kind::Integer,
                    },
                ],
                path,
                issues,
            );
            contract::integer_enum(object, "ext.atts", &[0, 1, 2, 3], path, issues);
            if !sdk {
                contract::required(object, "ua", path, "MobileFuse app supplier device", issues);
                if !contract::present(object, "ip") && !contract::present(object, "ipv6") {
                    contract::error(
                        "openrtb.profile.mobilefuse.device_address_required",
                        "MobileFuse requires ip or ipv6.",
                        join_instance_path(path, "ip"),
                        issues,
                    );
                }
            }
            for (field, valid) in [
                (
                    "ip",
                    object
                        .get("ip")
                        .and_then(Value::as_str)
                        .map(|v| v.parse::<Ipv4Addr>().is_ok()),
                ),
                (
                    "ipv6",
                    object
                        .get("ipv6")
                        .and_then(Value::as_str)
                        .map(|v| v.parse::<Ipv6Addr>().is_ok()),
                ),
            ] {
                if valid == Some(false) {
                    contract::error(
                        "openrtb.profile.mobilefuse.device_address_invalid",
                        "The address must use the documented IP family.",
                        join_instance_path(path, field),
                        issues,
                    );
                }
            }
        }
        "Native" => {
            if let Some(ver) = object.get("ver").and_then(Value::as_str) {
                if ver != "1.2" {
                    contract::warning(
                        "openrtb.profile.mobilefuse.native_version",
                        "MobileFuse expects Native Ads 1.2.",
                        join_instance_path(path, "ver"),
                        issues,
                    );
                }
            }
        }
        "Source" => typed_ext(
            object,
            &[
                Field {
                    name: "omidpn",
                    kind: Kind::String,
                },
                Field {
                    name: "omidpv",
                    kind: Kind::String,
                },
            ],
            path,
            issues,
        ),
        "Regs" => typed_ext(
            object,
            &[
                Field {
                    name: "gpp",
                    kind: Kind::String,
                },
                Field {
                    name: "gpp_sid",
                    kind: Kind::Array(&Kind::Integer),
                },
                Field {
                    name: "us_privacy",
                    kind: Kind::String,
                },
            ],
            path,
            issues,
        ),
        "User" => typed_ext(
            object,
            &[Field {
                name: "hems",
                kind: Kind::Array(&Kind::Object(&[
                    Field {
                        name: "hem",
                        kind: Kind::String,
                    },
                    Field {
                        name: "hem_type",
                        kind: Kind::String,
                    },
                ])),
            }],
            path,
            issues,
        ),
        "Segment" if sdk => contract::fields(
            object,
            &[Field {
                name: "signal",
                kind: Kind::String,
            }],
            path,
            issues,
        ),
        "BidResponse" => {
            if let Some(cur) = object.get("cur").and_then(Value::as_str) {
                if cur != "USD" {
                    contract::error(
                        "openrtb.profile.mobilefuse.currency",
                        "MobileFuse auctions use USD.",
                        join_instance_path(path, "cur"),
                        issues,
                    );
                }
            }
        }
        "Bid" => {
            typed_ext(
                object,
                &[
                    Field {
                        name: "signaldata",
                        kind: Kind::String,
                    },
                    Field {
                        name: "mf",
                        kind: Kind::Object(&[
                            Field {
                                name: "render_event",
                                kind: Kind::String,
                            },
                            Field {
                                name: "cache_event",
                                kind: Kind::String,
                            },
                            Field {
                                name: "sdk_click_event",
                                kind: Kind::String,
                            },
                        ]),
                    },
                ],
                path,
                issues,
            );
            if sdk {
                contract::required(
                    object,
                    "ext.signaldata",
                    path,
                    "MobileFuse SDK rendering",
                    issues,
                );
            }
        }
        _ => {}
    }
}

fn validate_token(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let signal = object
        .get("user")
        .and_then(|v| v.get("data"))
        .and_then(Value::as_array)
        .and_then(|v| v.first())
        .and_then(|v| v.get("segment"))
        .and_then(Value::as_array)
        .and_then(|v| v.first())
        .and_then(|v| v.get("signal"));
    let signal_path = join_instance_path(path, "user.data[0].segment[0].signal");
    match signal {
        None | Some(Value::Null) => contract::error(
            "openrtb.profile.mobilefuse.sdk_signal_required",
            "MobileFuse SDK bidding requires the SDK token in the first data segment.",
            signal_path,
            issues,
        ),
        Some(Value::String(value)) if value.is_empty() => contract::error(
            "openrtb.profile.mobilefuse.sdk_signal_required",
            "The SDK token must not be empty.",
            signal_path,
            issues,
        ),
        Some(Value::String(value)) if value.len() > 4000 => contract::error(
            "openrtb.profile.mobilefuse.sdk_signal_size",
            "The SDK token may be at most 4000 UTF-8 bytes.",
            signal_path,
            issues,
        ),
        Some(Value::String(_)) => {}
        Some(_) => contract::error(
            "openrtb.profile.field_type",
            "The MobileFuse SDK token is an opaque string.",
            signal_path,
            issues,
        ),
    }
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    let Some(imps) = request.get("imp").and_then(Value::as_array) else {
        return;
    };
    let Some(first_id) = imps
        .first()
        .and_then(|imp| imp.get("id"))
        .and_then(Value::as_str)
    else {
        return;
    };
    for (path, bid) in contract::bids(response) {
        if contract::matching_imp(request, bid).is_some()
            && bid
                .get("impid")
                .and_then(Value::as_str)
                .is_some_and(|id| id != first_id)
        {
            contract::warning("openrtb.profile.mobilefuse.bid_for_ignored_imp", "MobileFuse documents processing only the first impression; this bid references a later impression.", join_instance_path(&path, "impid"), issues);
        }
    }
}

fn typed_ext(object: &Map<String, Value>, fields: &[Field], path: &str, issues: &mut Vec<Issue>) {
    if let Some(ext) = value_at(object, "ext").and_then(Value::as_object) {
        contract::fields(ext, fields, &join_instance_path(path, "ext"), issues);
    }
}
