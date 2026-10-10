//! Yandex Mobile Ads SDK Open Bidding. This is not the general Yandex DSP protocol.

use super::{
    contract::{self, Field, Kind},
    join_instance_path, value_at,
};
use crate::Issue;
use serde_json::{Map, Value};

const USER_DATA: &[Field] = &[Field {
    name: "segment",
    kind: Kind::Array(&Kind::Object(&[Field {
        name: "signal",
        kind: Kind::String,
    }])),
}];

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    match object_name {
        "BidRequest" => {
            for field in ["app", "device", "user", "tmax", "cur"] {
                required(object, field, path, issues);
            }
            if object
                .get("cur")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
            {
                error(
                    "currency_offer",
                    "Yandex SDK Open Bidding needs a supported request currency.",
                    join_instance_path(path, "cur"),
                    issues,
                );
            }
            if let Some(imps) = object.get("imp").and_then(Value::as_array) {
                if imps.len() > 1 {
                    contract::warning(
                        "openrtb.profile.yandex.ignored_impressions",
                        "Yandex SDK Open Bidding considers only the first impression.",
                        join_instance_path(path, "imp[1]"),
                        issues,
                    );
                }
            }
        }
        "App" => required(object, "bundle", path, issues),
        "Device" => {
            for field in ["ua", "ip"] {
                required(object, field, path, issues);
            }
        }
        "User" => {
            contract::fields(
                object,
                &[Field {
                    name: "data",
                    kind: Kind::Array(&Kind::Object(USER_DATA)),
                }],
                path,
                issues,
            );
            let signal = object
                .get("data")
                .and_then(Value::as_array)
                .is_some_and(|data| {
                    data.iter().any(|entry| {
                        entry
                            .get("segment")
                            .and_then(Value::as_array)
                            .is_some_and(|segments| {
                                segments.iter().any(|segment| {
                                    segment
                                        .get("signal")
                                        .and_then(Value::as_str)
                                        .is_some_and(|signal| !signal.is_empty())
                                })
                            })
                    })
                });
            if !signal {
                error("bidder_token", "Yandex SDK Open Bidding requires the SDK bidder token in user.data[].segment[].signal.", join_instance_path(path, "data"), issues);
            }
        }
        "Imp" if path == "imp[0]" || path.ends_with(".imp[0]") => {
            for field in ["tagid", "ext.ad_type"] {
                required(object, field, path, issues);
            }
            contract::fields(
                object,
                &[Field {
                    name: "ext",
                    kind: Kind::Object(&[Field {
                        name: "ad_type",
                        kind: Kind::String,
                    }]),
                }],
                path,
                issues,
            );
            if let Some(ad_type) = value_at(object, "ext.ad_type").and_then(Value::as_str) {
                if !matches!(
                    ad_type,
                    "banner" | "interstitial" | "rewarded" | "appopenad" | "native"
                ) {
                    error("ad_type", "Yandex SDK ad type must be banner, interstitial, rewarded, appopenad, or native.", join_instance_path(path, "ext.ad_type"), issues);
                }
            }
        }
        "Regs" => {
            contract::integer_enum(object, "gdpr", &[0, 1], path, issues);
            contract::integer_enum(object, "ext.consent", &[0, 1], path, issues);
            contract::fields(
                object,
                &[Field {
                    name: "ext",
                    kind: Kind::Object(&[Field {
                        name: "tcf_consent_string",
                        kind: Kind::String,
                    }]),
                }],
                path,
                issues,
            );
        }
        "BidResponse" => count(object, "seatbid", path, issues),
        "SeatBid" => count(object, "bid", path, issues),
        "Bid" => {
            required(object, "ext.signaldata", path, issues);
            contract::fields(
                object,
                &[Field {
                    name: "ext",
                    kind: Kind::Object(&[Field {
                        name: "signaldata",
                        kind: Kind::String,
                    }]),
                }],
                path,
                issues,
            );
            for field in ["nurl", "burl", "lurl"] {
                let Some(url) = object.get(field).and_then(Value::as_str) else {
                    continue;
                };
                if (field != "lurl" && url.contains("${AUCTION_LOSS}"))
                    || (field == "burl" && url.contains("${AUCTION_MIN_TO_WIN}"))
                {
                    error("macro_context", "Yandex substitutes AUCTION_LOSS only in lurl and AUCTION_MIN_TO_WIN only in nurl or lurl.", join_instance_path(path, field), issues);
                }
            }
        }
        _ => {}
    }
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    let Some(id) = request
        .get("imp")
        .and_then(Value::as_array)
        .and_then(|imps| imps.first())
        .and_then(|imp| imp.get("id"))
        .and_then(Value::as_str)
    else {
        return;
    };
    for (path, bid) in contract::bids(response) {
        if bid
            .get("impid")
            .and_then(Value::as_str)
            .is_some_and(|actual| actual != id)
        {
            error(
                "impression_mismatch",
                "Yandex SDK Open Bidding responses must reference the first offered impression.",
                join_instance_path(&path, "impid"),
                issues,
            );
        }
    }
}

fn count(object: &Map<String, Value>, field: &str, path: &str, issues: &mut Vec<Issue>) {
    if object
        .get(field)
        .and_then(Value::as_array)
        .is_some_and(|array| array.len() > 1)
    {
        error(
            "response_count",
            "A Yandex SDK Open Bidding response contains one SeatBid and one bid.",
            join_instance_path(path, field),
            issues,
        );
    }
}

fn required(object: &Map<String, Value>, field: &str, path: &str, issues: &mut Vec<Issue>) {
    contract::required(object, field, path, "Yandex SDK Open Bidding", issues);
}

fn error(suffix: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    contract::error(
        &format!("openrtb.profile.yandex.{suffix}"),
        message,
        path,
        issues,
    );
}
