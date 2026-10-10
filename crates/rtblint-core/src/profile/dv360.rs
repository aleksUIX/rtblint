//! Display & Video 360's exchange integration. Account allowlists remain external.
use super::{
    contract::{self, Field, Kind},
    join_instance_path,
};
use crate::{Issue, Severity};
use serde_json::{Map, Value};

const UIDS: &[Field] = &[
    Field {
        name: "id",
        kind: Kind::String,
    },
    Field {
        name: "atype",
        kind: Kind::Integer,
    },
];
const EIDS: &[Field] = &[
    Field {
        name: "source",
        kind: Kind::String,
    },
    Field {
        name: "uids",
        kind: Kind::Array(&Kind::Object(UIDS)),
    },
];
const PROVIDERS: &[Field] = &[Field {
    name: "consented_providers",
    kind: Kind::Array(&Kind::Integer),
}];
const REQUEST: &[Field] = &[
    Field {
        name: "schain",
        kind: Kind::Object(&[]),
    },
    Field {
        name: "purch",
        kind: Kind::Integer,
    },
    Field {
        name: "gdemsignals",
        kind: Kind::String,
    },
    Field {
        name: "disable_gma_format",
        kind: Kind::Flag,
    },
];
const SOURCE: &[Field] = &[
    Field {
        name: "schain",
        kind: Kind::Object(&[]),
    },
    Field {
        name: "omidpn",
        kind: Kind::String,
    },
    Field {
        name: "omidpv",
        kind: Kind::String,
    },
];
const REGS: &[Field] = &[
    Field {
        name: "gdpr",
        kind: Kind::Flag,
    },
    Field {
        name: "us_privacy",
        kind: Kind::String,
    },
];
const REWARDED: &[Field] = &[Field {
    name: "rewarded",
    kind: Kind::Flag,
}];
const GUARANTEED: &[Field] = &[Field {
    name: "guaranteed",
    kind: Kind::Flag,
}];
const INVENTORY: &[Field] = &[Field {
    name: "inventorypartnerdomain",
    kind: Kind::String,
}];
const DATA: &[Field] = &[
    Field {
        name: "segtax",
        kind: Kind::Integer,
    },
    Field {
        name: "segclass",
        kind: Kind::String,
    },
];
const DEVICE: &[Field] = &[
    Field {
        name: "truncated_ip",
        kind: Kind::Flag,
    },
    Field {
        name: "ifa_type",
        kind: Kind::String,
    },
    Field {
        name: "attestation_token",
        kind: Kind::String,
    },
    Field {
        name: "atts",
        kind: Kind::Integer,
    },
    Field {
        name: "cdep",
        kind: Kind::String,
    },
];
const USER: &[Field] = &[
    Field {
        name: "consent",
        kind: Kind::String,
    },
    Field {
        name: "us_privacy",
        kind: Kind::String,
    },
    Field {
        name: "eids",
        kind: Kind::Array(&Kind::Object(EIDS)),
    },
    Field {
        name: "consented_providers_settings",
        kind: Kind::Object(PROVIDERS),
    },
];
const RESPONSE: &[Field] = &[
    Field {
        name: "err",
        kind: Kind::String,
    },
    Field {
        name: "errHelp",
        kind: Kind::String,
    },
];
const BID: &[Field] = &[Field {
    name: "apis",
    kind: Kind::Array(&Kind::Integer),
}];

pub(super) fn validate(
    name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let schema = match name {
        "BidRequest" => REQUEST,
        "Source" => SOURCE,
        "Regs" => REGS,
        "Video" => REWARDED,
        "Deal" => GUARANTEED,
        "Site" | "App" => INVENTORY,
        "Data" => DATA,
        "Device" => DEVICE,
        "User" => USER,
        "BidResponse" => RESPONSE,
        "Bid" => BID,
        _ => &[],
    };
    if let Some(ext) = object.get("ext").and_then(Value::as_object) {
        contract::fields(ext, schema, &join_instance_path(path, "ext"), issues);
    }
    match name {
        "BidRequest" => {
            for field in ["device", "user"] {
                contract::required(object, field, path, "DV360", issues);
            }
            if !object.get("site").is_some_and(Value::is_object)
                && !object.get("app").is_some_and(Value::is_object)
            {
                contract::error(
                    "openrtb.profile.dv360.inventory_required",
                    "DV360 requests require site or app inventory.",
                    join_instance_path(path, "site"),
                    issues,
                );
            }
            contract::integer_enum(object, "ext.purch", &[0, 1, 2], path, issues);
            if super::value_at(object, "device.os")
                .and_then(Value::as_str)
                .is_some_and(|os| os.eq_ignore_ascii_case("ios"))
                && super::value_at(object, "app.bundle")
                    .and_then(Value::as_str)
                    .is_some_and(|bundle| {
                        !bundle.is_empty() && !bundle.bytes().all(|byte| byte.is_ascii_digit())
                    })
            {
                contract::error(
                    "openrtb.profile.dv360.ios_bundle",
                    "DV360 iOS app bundles must be numeric App Store IDs.",
                    join_instance_path(path, "app.bundle"),
                    issues,
                );
            }
            if contract::present(object, "ext.disable_gma_format")
                && !contract::present(object, "ext.gdemsignals")
            {
                contract::error(
                    "openrtb.profile.dv360.gma_dependency",
                    "disable_gma_format is permitted only with gdemsignals.",
                    join_instance_path(path, "ext.disable_gma_format"),
                    issues,
                );
            }
            for imp in object
                .get("imp")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
            {
                for (deal_index, deal) in imp
                    .1
                    .get("pmp")
                    .and_then(|pmp| pmp.get("deals"))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .enumerate()
                {
                    let Some(deal) = deal.as_object() else {
                        continue;
                    };
                    let auction = deal
                        .get("at")
                        .or_else(|| object.get("at"))
                        .and_then(Value::as_i64);
                    if auction == Some(3) {
                        contract::required(
                            deal,
                            "bidfloor",
                            &format!(
                                "{}imp[{}].pmp.deals[{deal_index}]",
                                super::prefix(path),
                                imp.0
                            ),
                            "DV360 fixed-price deal",
                            issues,
                        );
                    }
                }
            }
            let privacy = [
                "regs.us_privacy",
                "regs.ext.us_privacy",
                "user.ext.us_privacy",
            ]
            .iter()
            .filter(|field| contract::present(object, field))
            .count();
            if privacy > 1 {
                contract::warning(
                    "openrtb.profile.dv360.privacy_locations",
                    "DV360 expects one US Privacy signal location.",
                    join_instance_path(path, "regs"),
                    issues,
                );
            }
            if !["source.schain", "source.ext.schain", "ext.schain"]
                .iter()
                .any(|field| contract::present(object, field))
            {
                contract::warning("openrtb.profile.dv360.schain_expected","DV360 requires SupplyChain in almost all integrations; an exemption needs account context.",join_instance_path(path,"source.schain"),issues);
            }
        }
        "Device" => {
            contract::required(object, "ua", path, "DV360", issues);
            if !contract::present(object, "ip") && !contract::present(object, "ipv6") {
                contract::error(
                    "openrtb.profile.dv360.ip_required",
                    "DV360 requires the device IPv4 or IPv6 address.",
                    join_instance_path(path, "ip"),
                    issues,
                );
            }
            contract::integer_enum(object, "ext.atts", &[0, 1, 2, 3], path, issues);
        }
        "App" => contract::required(object, "bundle", path, "DV360", issues),
        "Publisher" => contract::required(object, "id", path, "DV360 publisher", issues),
        "Native"
            if object
                .get("ver")
                .and_then(Value::as_str)
                .is_some_and(|ver| matches!(ver, "1.0" | "1.1")) =>
        {
            contract::error(
                "openrtb.profile.dv360.native_version",
                "DV360 supports Native Ads 1.2; versions 1.0 and 1.1 are unsupported.",
                join_instance_path(path, "ver"),
                issues,
            );
        }
        "Video" => {
            contract::required(object, "protocols", path, "DV360 video", issues);
            if object
                .get("protocols")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty)
            {
                contract::error(
                    "openrtb.profile.dv360.protocols_required",
                    "DV360 video requires a nonempty supported protocol list.",
                    join_instance_path(path, "protocols"),
                    issues,
                );
            }
            let classification = object
                .get("plcmt")
                .and_then(Value::as_i64)
                .or_else(|| object.get("placement").and_then(Value::as_i64));
            let sound_off = object
                .get("playbackmethod")
                .and_then(Value::as_array)
                .is_some_and(|methods| {
                    !methods.is_empty()
                        && methods
                            .iter()
                            .all(|v| matches!(v.as_i64(), Some(0 | 2 | 6)))
                });
            if sound_off && matches!(classification, None | Some(0 | 1)) {
                contract::error(
                    "openrtb.profile.dv360.instream_sound",
                    "DV360 sound-off playback must be classified as outstream.",
                    join_instance_path(path, "playbackmethod"),
                    issues,
                );
            }
        }
        "Pmp" => {
            let mut seen = std::collections::HashSet::new();
            for (index, deal) in object
                .get("deals")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
            {
                if let Some(id) = deal.get("id").and_then(Value::as_str) {
                    if !seen.insert(id) {
                        contract::error(
                            "openrtb.profile.dv360.duplicate_deal",
                            "DV360 deal IDs must be unique within an impression.",
                            format!("{path}.deals[{index}].id"),
                            issues,
                        );
                    }
                }
            }
        }
        "Regs" => {
            if contract::present(object, "gdpr") && contract::present(object, "ext.gdpr") {
                contract::warning(
                    "openrtb.profile.dv360.privacy_locations",
                    "DV360 expects one GDPR applicability location.",
                    join_instance_path(path, "ext.gdpr"),
                    issues,
                );
            }
        }
        "User" => {
            if contract::present(object, "consent") && contract::present(object, "ext.consent") {
                contract::warning(
                    "openrtb.profile.dv360.privacy_locations",
                    "DV360 expects one TCF consent location.",
                    join_instance_path(path, "ext.consent"),
                    issues,
                );
            }
        }
        "Bid"
            if object.get("mtype").and_then(Value::as_i64) == Some(1)
                && !contract::present(object, "adm") =>
        {
            contract::error(
                "openrtb.profile.dv360.banner_markup",
                "DV360 always includes adm for banner bids.",
                join_instance_path(path, "adm"),
                issues,
            );
        }
        _ => {}
    }
    // Unsupported fields are still parsed by DV360. Report warnings rather than rejection errors.
    let ignored: &[&str] = match name {
        "BidRequest" => &["test", "allimps", "wlang", "bapp"],
        "Source" => &["tid", "pchain"],
        "Imp" => &["metric"],
        _ => &[],
    };
    for field in ignored {
        if contract::present(object, field) {
            let mut issue = super::profile_issue(
                "openrtb.profile.dv360.ignored_field",
                format!("DV360 parses {field} but does not use it for bidding."),
                join_instance_path(path, field),
            );
            issue.severity = Severity::Warning;
            issues.push(issue);
        }
    }
}

pub(super) fn invalid_request_response(response: &Map<String, Value>) -> bool {
    response.get("id").and_then(Value::as_str) == Some("0")
        && response.get("nbr").and_then(Value::as_i64) == Some(2)
        && response
            .get("seatbid")
            .map_or(true, |seats| seats.as_array().is_some_and(Vec::is_empty))
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
        let banner = imp.get("banner").is_some_and(Value::is_object)
            && ["video", "audio", "native"]
                .iter()
                .all(|field| !imp.get(*field).is_some_and(Value::is_object));
        if banner && bid.get("mtype").and_then(Value::as_i64) != Some(1) {
            contract::required(bid, "adm", &path, "DV360 banner response", issues);
        }
    }
}
