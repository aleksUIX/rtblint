//! AppLovin ALX standard DSP responses. SDK bidders use a different contract.

use serde_json::{Map, Value};

use super::{
    contract::{self, Field, Kind},
    join_instance_path, require_integer_in_range, value_at,
};
use crate::Issue;

const STRINGS: Kind = Kind::Array(&Kind::String);
const SKOVERLAY: &[Field] = &[
    Field {
        name: "position",
        kind: Kind::Flag,
    },
    Field {
        name: "dismissable",
        kind: Kind::Flag,
    },
    Field {
        name: "video_delay",
        kind: Kind::Integer,
    },
    Field {
        name: "companion_delay",
        kind: Kind::Integer,
    },
    Field {
        name: "sk_dismiss_delay",
        kind: Kind::Integer,
    },
];
const FIDELITY: &[Field] = &[
    Field {
        name: "fidelity",
        kind: Kind::Integer,
    },
    Field {
        name: "nonce",
        kind: Kind::String,
    },
    Field {
        name: "signature",
        kind: Kind::String,
    },
    Field {
        name: "timestamp",
        kind: Kind::String,
    },
];
const SKADN: &[Field] = &[
    Field {
        name: "version",
        kind: Kind::String,
    },
    Field {
        name: "network",
        kind: Kind::String,
    },
    Field {
        name: "campaign",
        kind: Kind::String,
    },
    Field {
        name: "itunesitem",
        kind: Kind::String,
    },
    Field {
        name: "sourceapp",
        kind: Kind::String,
    },
    Field {
        name: "productpage",
        kind: Kind::String,
    },
    Field {
        name: "nonce",
        kind: Kind::String,
    },
    Field {
        name: "signature",
        kind: Kind::String,
    },
    Field {
        name: "timestamp",
        kind: Kind::String,
    },
    Field {
        name: "skoverlay",
        kind: Kind::Object(SKOVERLAY),
    },
    Field {
        name: "fidelities",
        kind: Kind::Array(&Kind::Object(FIDELITY)),
    },
];
const EXT: &[Field] = &[
    Field {
        name: "crtype",
        kind: Kind::String,
    },
    Field {
        name: "clicktrackers",
        kind: STRINGS,
    },
    Field {
        name: "imptrackers",
        kind: STRINGS,
    },
    Field {
        name: "vendor",
        kind: STRINGS,
    },
    Field {
        name: "duration",
        kind: Kind::Integer,
    },
    Field {
        name: "skadn",
        kind: Kind::Object(SKADN),
    },
];

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    match object_name {
        "BidResponse" => {
            if object
                .get("cur")
                .is_some_and(|cur| !cur.is_null() && cur.as_str() != Some("USD"))
            {
                error(
                    "currency",
                    "AppLovin ALX accepts only USD; omitting currency defaults to USD.",
                    join_instance_path(path, "cur"),
                    issues,
                );
            }
        }
        "SeatBid" => {
            if let Some(seat) = object.get("seat").and_then(Value::as_str) {
                if seat.is_empty()
                    || seat.chars().count() > 40
                    || !seat.chars().all(|c| c.is_ascii_alphanumeric())
                {
                    error("seat_format", "A supplied AppLovin seat must be an alphanumeric string of at most 40 characters.", join_instance_path(path, "seat"), issues);
                }
            }
        }
        "Bid" => validate_bid(object, path, issues),
        _ => {}
    }
}

fn validate_bid(bid: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    if !contract::present(bid, "adm_native") {
        required(bid, "adm", path, issues);
    }
    if let Some(native) = bid.get("adm_native").filter(|value| !value.is_null()) {
        // ALX specifies Native 1.2 content but leaves this carrier's wire type open.
        // Preserve both direct objects and JSON strings containing objects.
        let object_content = native.is_object()
            || native.as_str().is_some_and(|encoded| {
                serde_json::from_str::<Value>(encoded).is_ok_and(|value| value.is_object())
            });
        if !object_content {
            contract::error(
                "openrtb.profile.native_encoding",
                "AppLovin ALX native markup must contain a Native 1.2 JSON object.",
                join_instance_path(path, "adm_native"),
                issues,
            );
            if let Some(issue) = issues.last_mut() {
                issue.section = Some("https://support.applovin.com/en/max/demand-partners/demand-side-platforms/applovin-ortb-specification/creative-types".to_owned());
            }
        }
    }
    for field in ["adomain", "burl", "cat", "crid"] {
        required(bid, field, path, issues);
    }
    contract::fields(
        bid,
        &[
            Field {
                name: "ext",
                kind: Kind::Object(EXT),
            },
            Field {
                name: "crtype",
                kind: Kind::String,
            },
        ],
        path,
        issues,
    );
    if let Some(domains) = bid.get("adomain").and_then(Value::as_array) {
        if domains.is_empty() {
            error(
                "advertiser_domain",
                "AppLovin requires an advertiser domain.",
                join_instance_path(path, "adomain"),
                issues,
            );
        }
        if let Some(domain) = domains.first().and_then(Value::as_str) {
            if domain.is_empty() || domain.contains('/') {
                error(
                    "advertiser_domain",
                    "AppLovin advertiser domains must omit URL schemes and paths.",
                    join_instance_path(path, "adomain[0]"),
                    issues,
                );
            }
        }
    }
    let crtype = value_at(bid, "ext.crtype")
        .or_else(|| bid.get("crtype"))
        .and_then(Value::as_str);
    if let Some(crtype) = crtype {
        const ALLOWED: &[&str] = &[
            "HTML",
            "MRAID 1.0",
            "MRAID 2.0",
            "MRAID 3.0",
            "native",
            "VAST 2.0",
            "VAST 3.0",
            "VAST 4.0",
            "VAST 4.1",
            "VAST 4.2",
        ];
        if !ALLOWED
            .iter()
            .any(|allowed| allowed.eq_ignore_ascii_case(crtype))
        {
            error("creative_type", "AppLovin creative type must identify a supported HTML, MRAID, Native or VAST format.", join_instance_path(path, if bid.contains_key("crtype") && value_at(bid, "ext.crtype").is_none() { "crtype" } else { "ext.crtype" }), issues);
        }
    }
    // VPAID and ORMMA are explicitly marked unsupported, rather than merely omitted.
    if let Some(apis) = bid.get("apis").and_then(Value::as_array) {
        for (index, api) in apis.iter().enumerate() {
            if matches!(api.as_i64(), Some(1 | 2 | 4)) {
                error(
                    "api_unsupported",
                    "AppLovin ALX does not support VPAID or ORMMA.",
                    format!("{path}.apis[{index}]"),
                    issues,
                );
            }
        }
    }
    if matches!(bid.get("api").and_then(Value::as_i64), Some(1 | 2 | 4)) {
        error(
            "api_unsupported",
            "AppLovin ALX does not support VPAID or ORMMA.",
            join_instance_path(path, "api"),
            issues,
        );
    }
    for field in ["nurl", "burl"] {
        if bid
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(|url| url.contains("${AUCTION_LOSS}"))
        {
            error(
                "macro_context",
                "AppLovin substitutes AUCTION_LOSS only in lurl.",
                join_instance_path(path, field),
                issues,
            );
        }
    }
    require_integer_in_range(
        bid,
        "ext.duration",
        1,
        i64::MAX,
        "AppLovin video duration must be a positive integer number of seconds.",
        path,
        issues,
    );
    if let Some(adm) = bid.get("adm").and_then(Value::as_str) {
        native_video(adm, &join_instance_path(path, "adm.native"), issues);
    }
    if let Some(native) = bid.get("adm_native") {
        if let Some(encoded) = native.as_str() {
            native_video(encoded, &join_instance_path(path, "adm_native"), issues);
        } else if let Some(native) = native.as_object() {
            native_video_object(native, &join_instance_path(path, "adm_native"), issues);
        }
    }
    let Some(skadn) = value_at(bid, "ext.skadn").and_then(Value::as_object) else {
        return;
    };
    required(bid, "bundle", path, issues);
    if crtype.is_some_and(|crtype| crtype.eq_ignore_ascii_case("HTML")) {
        required(bid, "ext.clicktrackers", path, issues);
        if value_at(bid, "ext.clicktrackers")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            error(
                "clicktrackers",
                "AppLovin HTML with SKAdNetwork requires click trackers.",
                join_instance_path(path, "ext.clicktrackers"),
                issues,
            );
        }
    }
    let base = join_instance_path(path, "ext.skadn");
    for field in ["version", "itunesitem", "sourceapp", "network", "campaign"] {
        required(skadn, field, &base, issues);
    }
    let major = skadn
        .get("version")
        .and_then(Value::as_str)
        .and_then(|version| {
            let mut parts = version.split('.');
            let major = parts.next()?.parse::<u32>().ok()?;
            parts.next().map(str::parse::<u32>).transpose().ok()?;
            if parts.next().is_some() || major < 2 {
                None
            } else {
                Some(major)
            }
        });
    if skadn
        .get("version")
        .is_some_and(|version| version.is_string())
        && major.is_none()
    {
        error(
            "skadn_version",
            "AppLovin SKAdNetwork version must be 2.0 or above.",
            join_instance_path(&base, "version"),
            issues,
        );
    }
    if let Some(campaign) = skadn.get("campaign").and_then(Value::as_str) {
        let (min, max) = if major.is_some_and(|major| major >= 4) {
            (0, 9999)
        } else {
            (1, 100)
        };
        if campaign.is_empty()
            || !campaign.bytes().all(|c| c.is_ascii_digit())
            || !campaign
                .parse::<u32>()
                .is_ok_and(|value| (min..=max).contains(&value))
        {
            error("skadn_campaign", "AppLovin SKAdNetwork campaign must be an integer string in its version-specific range.", join_instance_path(&base, "campaign"), issues);
        }
    }
    if let (Some(item), Some(bundle)) = (
        skadn.get("itunesitem").and_then(Value::as_str),
        bid.get("bundle").and_then(Value::as_str),
    ) {
        if item != bundle {
            error(
                "skadn_mismatch",
                "AppLovin SKAdNetwork itunesitem must match advertised bid.bundle.",
                join_instance_path(&base, "itunesitem"),
                issues,
            );
        }
    }
    let version = skadn.get("version").and_then(Value::as_str).unwrap_or("");
    if !matches!(version, "2.0" | "2.1") {
        required(skadn, "fidelities", &base, issues);
        if skadn
            .get("fidelities")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            error(
                "skadn_fidelity",
                "AppLovin SKAdNetwork fidelities must not be empty.",
                join_instance_path(&base, "fidelities"),
                issues,
            );
        }
    }
    if let Some(fidelities) = skadn.get("fidelities").and_then(Value::as_array) {
        for (index, fidelity) in fidelities.iter().enumerate() {
            let Some(fidelity) = fidelity.as_object() else {
                continue;
            };
            let path = format!("{base}.fidelities[{index}]");
            for field in ["fidelity", "nonce", "signature", "timestamp"] {
                required(fidelity, field, &path, issues);
            }
            attribution_formats(fidelity, &path, issues);
        }
    }
    attribution_formats(skadn, &base, issues);
    if let Some(overlay) = skadn.get("skoverlay").and_then(Value::as_object) {
        let base = join_instance_path(&base, "skoverlay");
        for field in ["video_delay", "companion_delay", "sk_dismiss_delay"] {
            required(overlay, field, &base, issues);
            require_integer_in_range(
                overlay,
                field,
                -1,
                i64::MAX,
                "AppLovin SKOverlay delay must be -1 (disabled) or non-negative seconds.",
                &base,
                issues,
            );
        }
    }
}

fn attribution_formats(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    if let Some(nonce) = object.get("nonce").and_then(Value::as_str) {
        let valid = nonce.len() == 36
            && nonce.bytes().enumerate().all(|(index, byte)| {
                if matches!(index, 8 | 13 | 18 | 23) {
                    byte == b'-'
                } else {
                    byte.is_ascii_hexdigit()
                }
            });
        if !valid {
            error(
                "skadn_nonce",
                "AppLovin SKAdNetwork nonce must use UUID format.",
                join_instance_path(path, "nonce"),
                issues,
            );
        }
    }
    if let Some(timestamp) = object.get("timestamp").and_then(Value::as_str) {
        if timestamp.is_empty() || !timestamp.bytes().all(|byte| byte.is_ascii_digit()) {
            error("skadn_timestamp", "AppLovin SKAdNetwork timestamp must be a decimal Unix timestamp in milliseconds, expressed as a string.", join_instance_path(path, "timestamp"), issues);
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
        if value_at(request, "device.devicetype").and_then(Value::as_i64) == Some(3) {
            required(bid, "ext.duration", &path, issues);
        }
        if let (Some(publisher_bundle), Some(advertised_bundle)) = (
            value_at(request, "app.bundle").and_then(Value::as_str),
            bid.get("bundle").and_then(Value::as_str),
        ) {
            if publisher_bundle == advertised_bundle && !advertised_bundle.is_empty() {
                error(
                    "publisher_bundle",
                    "AppLovin rejects publisher app.bundle used as the advertised app bundle.",
                    join_instance_path(&path, "bundle"),
                    issues,
                );
            }
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
                error(
                    "skadn_mismatch",
                    "AppLovin SKAdNetwork sourceapp must match the offered publisher sourceapp.",
                    join_instance_path(&base, "sourceapp"),
                    issues,
                );
            }
        }
        if let (Some(network), Some(ids)) = (
            skadn.get("network").and_then(Value::as_str),
            offered.get("skadnetids").and_then(Value::as_array),
        ) {
            if ids.iter().all(Value::is_string)
                && !ids.iter().any(|id| id.as_str() == Some(network))
            {
                error("skadn_mismatch", "AppLovin SKAdNetwork network must be advertised in the impression's skadnetids.", join_instance_path(&base, "network"), issues);
            }
        }
    }
}

fn native_video(encoded: &str, path: &str, issues: &mut Vec<Issue>) {
    if let Ok(value) = serde_json::from_str::<Value>(encoded) {
        if let Some(native) = value.as_object() {
            native_video_object(native, path, issues);
        }
    }
}

fn native_video_object(native: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let native = native
        .get("native")
        .and_then(Value::as_object)
        .unwrap_or(native);
    if let Some(assets) = native.get("assets").and_then(Value::as_array) {
        for (index, asset) in assets.iter().enumerate() {
            if asset.get("video").is_some_and(|value| !value.is_null()) {
                error(
                    "native_video",
                    "AppLovin ALX Native does not support video assets.",
                    format!("{path}.assets[{index}].video"),
                    issues,
                );
            }
        }
    }
}

fn required(object: &Map<String, Value>, field: &str, path: &str, issues: &mut Vec<Issue>) {
    contract::required(object, field, path, "AppLovin ALX standard DSP", issues);
}

fn error(suffix: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    contract::error(
        &format!("openrtb.profile.applovin.{suffix}"),
        message,
        path,
        issues,
    );
    if suffix == "native_video" {
        if let Some(issue) = issues.last_mut() {
            issue.section = Some("https://support.applovin.com/en/max/demand-partners/demand-side-platforms/applovin-ortb-specification/creative-types".to_owned());
        }
    }
}
