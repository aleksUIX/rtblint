//! Vungle Exchange's DSP-facing OpenRTB 2.5 and Native Ads 1.2 contracts.
//! Sources are recorded in docs/exchange-profiles/unity-vungle.md.

use serde_json::{Map, Value};

use super::{
    join_instance_path, path_populated, profile_issue, require_integer_in_range, value_at,
};
use crate::{Issue, Severity};

#[derive(Clone, Copy)]
enum Kind {
    String,
    Object,
    Objects,
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
            for field in ["app", "device", "source"] {
                required(object, field, Kind::Object, path, issues);
            }
            required_integer(object, "at", 1, 1, path, issues);
            required_integer(object, "tmax", 1, i64::MAX, path, issues);
            string_values(object, "cur", &["USD"], false, path, issues);
            request_skadn_matches_bundle(object, path, issues);
        }
        "Source" => {
            validate_supply_chain(object, path, issues);
            fields(
                object,
                &[("ext.omidpn", Kind::String), ("ext.omidpv", Kind::String)],
                path,
                issues,
            );
            if value_at(object, "ext.omidpn").is_some_and(|value| value.as_str() != Some("vungle"))
            {
                error(
                    "value_invalid",
                    "Vungle source.ext.omidpn is vungle when supplied.",
                    join_instance_path(path, "ext.omidpn"),
                    issues,
                );
            }
        }
        "Imp" => {
            for field in [
                "displaymanager",
                "displaymanagerver",
                "bidfloorcur",
                "tagid",
            ] {
                required(object, field, Kind::String, path, issues);
            }
            if !object.get("bidfloor").is_some_and(Value::is_number) {
                missing("bidfloor", path, issues);
            }
            required_integer(object, "secure", 1, 1, path, issues);
            required_integer(object, "clickbrowser", 0, 1, path, issues);
            // pcta is always emitted and explicitly not nullable, despite ext being optional in the parent table.
            required_integer(object, "ext.pcta", 0, 1, path, issues);
            fields(
                object,
                &[("ext.skadn", Kind::Object), ("ext.gpid", Kind::String)],
                path,
                issues,
            );
            for field in ["ext.deeplink", "ext.skpv", "ext.vxec"] {
                integer(object, field, 0, 1, path, issues);
            }
            if let Some(skadn) = value_at(object, "ext.skadn").and_then(Value::as_object) {
                let base = join_instance_path(path, "ext.skadn");
                for field in ["version", "sourceapp"] {
                    required(skadn, field, Kind::String, &base, issues);
                }
                required(skadn, "versions", Kind::Strings, &base, issues);
                // The spec explicitly permits an empty skadnetids array.
                if !skadn
                    .get("skadnetids")
                    .and_then(Value::as_array)
                    .is_some_and(|items| items.iter().all(Value::is_string))
                {
                    missing("skadnetids", &base, issues);
                }
                fields(skadn, &[("ext", Kind::Object)], &base, issues);
                skadn_version(skadn, "version", &base, issues);
                if let Some(versions) = skadn.get("versions").and_then(Value::as_array) {
                    for (index, version) in versions.iter().enumerate() {
                        if version.as_str().is_some_and(|version| {
                            version_major(version).map_or(true, |major| major < 2)
                                || version == "2.1"
                        }) {
                            error("skadn_version", "Vungle request SKAdNetwork versions must be 2.0 or later; Vungle does not advertise 2.1.", format!("{base}.versions[{index}]"), issues);
                        }
                    }
                }
                integer(skadn, "ext.sko", 0, 1, &base, issues);
            }
        }
        "Banner" => {
            for field in ["w", "h"] {
                required_integer(object, field, 1, i64::MAX, path, issues);
            }
            required(object, "id", Kind::String, path, issues);
            if !object
                .get("format")
                .and_then(Value::as_array)
                .is_some_and(|items| !items.is_empty() && items.iter().all(Value::is_object))
            {
                missing("format", path, issues);
            }
            string_values(
                object,
                "mimes",
                &["image/png", "image/jpg", "image/gif", "text/html"],
                !path.contains("companionad["),
                path,
                issues,
            );
            integer(object, "ext.rewarded", 1, 1, path, issues);
        }
        "Video" => {
            for field in ["w", "h", "maxbitrate"] {
                required_integer(
                    object,
                    field,
                    1,
                    if field == "maxbitrate" {
                        15000
                    } else {
                        i64::MAX
                    },
                    path,
                    issues,
                );
            }
            required_integer(object, "minbitrate", 250, 250, path, issues);
            required_integer(object, "boxingallowed", 1, 1, path, issues);
            integer_values(object, "delivery", &[1, 2], true, path, issues);
            integer_values(object, "protocols", &[2, 3, 5, 6], true, path, issues);
            integer_values(object, "playbackmethod", &[1, 2, 3, 4], true, path, issues);
            integer_values(object, "companiontype", &[1, 2], false, path, issues);
            integer(object, "ext.rewarded", 1, 1, path, issues);
            fields(object, &[("ext.videotype", Kind::String)], path, issues);
        }
        "Native" => {
            required(object, "ver", Kind::String, path, issues);
            if object
                .get("ver")
                .is_some_and(|version| version.as_str() != Some("1.2"))
            {
                error(
                    "native_version",
                    "Vungle supports Native Ads version 1.2.",
                    join_instance_path(path, "ver"),
                    issues,
                );
            }
        }
        "Pmp" | "PMP" => integer(object, "private_auction", 0, 0, path, issues),
        "Deal" => integer(object, "at", 1, 1, path, issues),
        "App" => {
            required(object, "id", Kind::String, path, issues);
            required(object, "publisher", Kind::Object, path, issues);
            required(object, "cat", Kind::Strings, path, issues);
            integer(object, "privacypolicy", 1, 1, path, issues);
        }
        "Publisher" => required(object, "id", Kind::String, path, issues),
        "Regs" => {
            // Regs is optional, while its extension object is always passed.
            // The extension's privacy members remain optional.
            if !object.get("ext").is_some_and(Value::is_object) {
                missing("ext", path, issues);
            }
            fields(
                object,
                &[("ext", Kind::Object), ("ext.us_privacy", Kind::String)],
                path,
                issues,
            );
            integer(object, "ext.gdpr", 0, 1, path, issues);
        }
        "User" => {
            if let Some(consent) = value_at(object, "ext.consent") {
                if !consent.is_string() && !matches!(consent.as_i64(), Some(0 | 1)) {
                    error("type_invalid", "Vungle user.ext.consent is an integer 0 or 1, or a configured TCF consent string.", join_instance_path(path, "ext.consent"), issues);
                }
            }
        }
        "Device" => validate_device(object, path, issues),
        "Geo" => {
            required(object, "country", Kind::String, path, issues);
            // ipservice describes IP-derived location. Other location methods do not imply it.
            if object.get("type").and_then(Value::as_i64) == Some(2) {
                required_integer(object, "ipservice", 3, 3, path, issues);
            } else {
                integer(object, "ipservice", 3, 3, path, issues);
            }
        }
        "BidResponse" => {
            // The exchange consumes all currency declarations as USD, rather than rejecting non-USD.
            if object
                .get("cur")
                .and_then(Value::as_str)
                .is_some_and(|currency| currency != "USD")
            {
                warning("currency_ignored", "Vungle treats the response currency as USD, including a supplied non-USD value.", join_instance_path(path, "cur"), issues);
            }
        }
        "Bid" => validate_bid(object, path, issues),
        _ => {}
    }
}

fn validate_device(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    required(object, "ua", Kind::String, path, issues);
    for field in ["w", "h"] {
        required_integer(object, field, 1, i64::MAX, path, issues);
    }
    if !nonempty_string(value_at(object, "ip")) && !nonempty_string(value_at(object, "ipv6")) {
        missing("ip", path, issues);
    }
    if !path_populated(object, "connectiontype") {
        missing("connectiontype", path, issues);
    }
    fields(
        object,
        &[
            ("ext.ifv", Kind::String),
            ("ext.idfv", Kind::String),
            ("ext.app_set_id", Kind::String),
        ],
        path,
        issues,
    );
    integer(object, "ext.atts", 0, 3, path, issues);
    integer(object, "ext.app_set_id_scope", 1, 2, path, issues);
    if let (Some(ifv), Some(idfv)) = (
        value_at(object, "ext.ifv").and_then(Value::as_str),
        value_at(object, "ext.idfv").and_then(Value::as_str),
    ) {
        if ifv != idfv {
            error(
                "idfv_mismatch",
                "Vungle ext.ifv and ext.idfv must contain the same IDFV.",
                join_instance_path(path, "ext.idfv"),
                issues,
            );
        }
    }
    let has_idfv = ["ext.ifv", "ext.idfv"]
        .iter()
        .any(|field| nonempty_string(value_at(object, field)));
    if has_idfv {
        if object
            .get("os")
            .and_then(Value::as_str)
            .is_some_and(|os| os != "iOS")
        {
            error(
                "idfv_context",
                "Vungle IDFV extensions are emitted only for iOS devices.",
                join_instance_path(path, "ext.ifv"),
                issues,
            );
        }
        if object
            .get("ifa")
            .and_then(Value::as_str)
            .is_some_and(|ifa| {
                !ifa.is_empty() && ifa.bytes().any(|byte| byte != b'0' && byte != b'-')
            })
        {
            error(
                "idfv_context",
                "Vungle emits IDFV only when IDFA is unavailable or all zeros.",
                join_instance_path(path, "ext.ifv"),
                issues,
            );
        }
    }
}

fn validate_bid(bid: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    required(bid, "adm", Kind::String, path, issues);
    required(bid, "adomain", Kind::Strings, path, issues);
    fields(
        bid,
        &[
            ("ext", Kind::Object),
            ("ext.skadn", Kind::Object),
            ("ext.imptrackers", Kind::Strings),
            ("ext.clicktrackers", Kind::Strings),
            ("ext.deeplink", Kind::String),
            ("ext.crtype", Kind::String),
            ("ext.pcta", Kind::Object),
            ("ext.skpv", Kind::Object),
        ],
        path,
        issues,
    );
    // NULL explicitly selects the default for these two fields.
    for field in ["ext.campaigntype", "ext.ai_disclosure"] {
        if !value_at(bid, field).is_some_and(Value::is_null) {
            integer(bid, field, 0, 1, path, issues);
        }
    }
    for field in [
        "ext.pcta.pctapage1",
        "ext.vxec",
        "ext.skpv.page1onclose",
        "ext.skpv.page2onclose",
    ] {
        integer(bid, field, 0, 1, path, issues);
    }
    if let Some(adm) = bid.get("adm").and_then(Value::as_str) {
        let vast = value_at(bid, "ext.crtype")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind.to_ascii_uppercase().starts_with("VAST"))
            || adm.trim_start().starts_with("<VAST")
            || adm.trim_start().starts_with("<?xml");
        if vast && adm.find("<VAST").map_or(true, |offset| offset >= 100) {
            error(
                "vast_prefix",
                "Vungle VAST markup must start its VAST element within the first 100 bytes.",
                join_instance_path(path, "adm"),
                issues,
            );
        }
        validate_native(adm, path, issues);
    }
    for field in ["nurl", "burl"] {
        if bid.get(field).and_then(Value::as_str).is_some_and(|url| {
            url.contains("${MIN_BID_TO_WIN}") || url.contains("${AUCTION_MIN_TO_WIN}")
        }) {
            error(
                "macro_context",
                "Vungle minimum-bid-to-win macros are supported only in lurl.",
                join_instance_path(path, field),
                issues,
            );
        }
    }
    if let Some(skadn) = value_at(bid, "ext.skadn").and_then(Value::as_object) {
        validate_skadn(bid, skadn, path, issues);
    }
}

fn validate_skadn(
    bid: &Map<String, Value>,
    skadn: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let base = join_instance_path(path, "ext.skadn");
    for field in ["version", "itunesitem", "sourceapp", "network"] {
        required(skadn, field, Kind::String, &base, issues);
    }
    fields(
        skadn,
        &[
            ("campaign", Kind::String),
            ("sourceidentifier", Kind::String),
            ("cpp", Kind::String),
            ("ext", Kind::Object),
            ("ext.skoverlay", Kind::Object),
        ],
        &base,
        issues,
    );
    skadn_version(skadn, "version", &base, issues);
    let major = skadn
        .get("version")
        .and_then(Value::as_str)
        .and_then(version_major);
    // The text calls campaign required but explicitly lets sourceidentifier replace it in 4.0.
    if major.is_some_and(|major| major >= 4) {
        if !nonempty_string(skadn.get("campaign"))
            && !nonempty_string(skadn.get("sourceidentifier"))
        {
            missing("sourceidentifier", &base, issues);
        }
    } else {
        required(skadn, "campaign", Kind::String, &base, issues);
    }
    if let Some(campaign) = skadn.get("campaign").and_then(Value::as_str) {
        let (min, max) = if major.is_some_and(|major| major >= 4) {
            (0, 9999)
        } else {
            (1, 100)
        };
        if !digits_in_range(campaign, min, max) {
            error("skadn_campaign", "Vungle SKAdNetwork campaign must be an integer string in the version-specific range (1 through 100 before 4.0, 0 through 9999 in 4.0 and later).", join_instance_path(&base, "campaign"), issues);
        }
    }
    if let Some(sourceid) = skadn.get("sourceidentifier").and_then(Value::as_str) {
        if sourceid.len() != 4
            || !sourceid.bytes().all(|byte| byte.is_ascii_digit())
            || major.is_some_and(|major| major < 4)
        {
            error("skadn_sourceidentifier", "Vungle sourceidentifier is a four-digit integer string for SKAdNetwork 4.0 and later.", join_instance_path(&base, "sourceidentifier"), issues);
        }
    }
    if let (Some(item), Some(bundle)) = (
        skadn.get("itunesitem").and_then(Value::as_str),
        bid.get("bundle").and_then(Value::as_str),
    ) {
        if item != bundle {
            error(
                "skadn_mismatch",
                "Vungle SKAdNetwork itunesitem must match the advertised bid.bundle when supplied.",
                join_instance_path(&base, "itunesitem"),
                issues,
            );
        }
    }
    let Some(fidelities) = skadn
        .get("fidelities")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
    else {
        missing("fidelities", &base, issues);
        validate_overlay(skadn, &base, issues);
        return;
    };
    for (index, fidelity) in fidelities.iter().enumerate() {
        let fp = format!("{base}.fidelities[{index}]");
        let Some(fidelity) = fidelity.as_object() else {
            error(
                "type_invalid",
                "Vungle SKAdNetwork fidelity entries must be objects.",
                fp,
                issues,
            );
            continue;
        };
        for field in ["nonce", "timestamp", "signature"] {
            required(fidelity, field, Kind::String, &fp, issues);
        }
        required(fidelity, "fidelity", Kind::Integer, &fp, issues);
        if let Some(nonce) = fidelity.get("nonce").and_then(Value::as_str) {
            if !uuid_shape(nonce) {
                error(
                    "skadn_nonce",
                    "Vungle SKAdNetwork fidelity nonce must use UUID syntax.",
                    join_instance_path(&fp, "nonce"),
                    issues,
                );
            }
        }
        if let Some(timestamp) = fidelity.get("timestamp").and_then(Value::as_str) {
            if timestamp.is_empty() || !timestamp.bytes().all(|byte| byte.is_ascii_digit()) {
                error(
                    "skadn_timestamp",
                    "Vungle SKAdNetwork timestamp is a Unix-milliseconds integer string.",
                    join_instance_path(&fp, "timestamp"),
                    issues,
                );
            }
        }
    }
    validate_overlay(skadn, &base, issues);
}

fn validate_overlay(skadn: &Map<String, Value>, base: &str, issues: &mut Vec<Issue>) {
    let Some(overlay) = value_at(skadn, "ext.skoverlay").and_then(Value::as_object) else {
        return;
    };
    let op = join_instance_path(base, "ext.skoverlay");
    required_integer(overlay, "show", 0, 1, &op, issues);
    integer(overlay, "pos", 0, 1, &op, issues);
    for field in ["delay", "companion_delay"] {
        integer(overlay, field, 0, i64::MAX, &op, issues);
    }
    if overlay.get("show").and_then(Value::as_i64) == Some(0)
        && overlay.keys().any(|field| field != "show")
    {
        error(
            "skoverlay_disabled",
            "Vungle disallows other SKOverlay attributes when show is 0.",
            op,
            issues,
        );
    }
    if overlay.get("show").and_then(Value::as_i64) == Some(1) {
        required(skadn, "itunesitem", Kind::String, base, issues);
    }
}

fn validate_native(adm: &str, path: &str, issues: &mut Vec<Issue>) {
    let Ok(value) = serde_json::from_str::<Value>(adm) else {
        return;
    };
    let Some(root) = value.as_object() else {
        return;
    };
    let native = root
        .get("native")
        .and_then(Value::as_object)
        .unwrap_or(root);
    if !native.contains_key("assets") && !native.contains_key("assetsurl") {
        return;
    }
    let base = join_instance_path(path, "adm.native");
    for field in ["jstracker", "link.fallback"] {
        if path_populated(native, field) {
            warning(
                "native_unsupported",
                &format!("Vungle does not support Native {field}."),
                join_instance_path(&base, field),
                issues,
            );
        }
    }
    if let Some(trackers) = native.get("eventtrackers").and_then(Value::as_array) {
        for (index, tracker) in trackers.iter().enumerate() {
            if matches!(tracker.get("event").and_then(Value::as_i64), Some(2..=4)) {
                warning(
                    "native_unsupported",
                    "Vungle Native does not support viewability event trackers.",
                    format!("{base}.eventtrackers[{index}].event"),
                    issues,
                );
            }
        }
    }
    let Some(assets) = native.get("assets").and_then(Value::as_array) else {
        return;
    };
    if !assets.iter().any(|asset| {
        asset.get("id").and_then(Value::as_i64) == Some(1)
            && asset.get("img").is_some_and(Value::is_object)
    }) {
        error(
            "native_main_image",
            "Vungle Native requires main-image asset id 1.",
            join_instance_path(&base, "assets"),
            issues,
        );
    }
    for (index, asset) in assets.iter().enumerate() {
        let Some(img) = asset.get("img").and_then(Value::as_object) else {
            continue;
        };
        let ap = format!("{base}.assets[{index}].img");
        if let Some(url) = img.get("url").and_then(Value::as_str) {
            let filename = url
                .split(['?', '#'])
                .next()
                .unwrap_or(url)
                .to_ascii_lowercase();
            // The feature table expressly allows GIF input rendered as a static image.
            if ![".jpg", ".jpeg", ".png", ".gif"]
                .iter()
                .any(|extension| filename.ends_with(extension))
            {
                error("native_image_extension", "Vungle Native image URL must have a supported image file extension before its query or fragment.", join_instance_path(&ap, "url"), issues);
            }
        }
        if asset.get("id").and_then(Value::as_i64) == Some(5) {
            for dimension in ["w", "h"] {
                if img
                    .get(dimension)
                    .and_then(Value::as_i64)
                    .is_some_and(|value| value < 80)
                {
                    error(
                        "native_icon_size",
                        "Vungle Native app icons must be at least 80 by 80 pixels when supplied.",
                        join_instance_path(&ap, dimension),
                        issues,
                    );
                }
            }
        }
    }
}

fn request_skadn_matches_bundle(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(bundle) = value_at(object, "app.bundle").and_then(Value::as_str) else {
        return;
    };
    let Some(imps) = object.get("imp").and_then(Value::as_array) else {
        return;
    };
    for (index, imp) in imps.iter().enumerate() {
        let Some(imp) = imp.as_object() else { continue };
        if value_at(imp, "ext.skadn.sourceapp")
            .and_then(Value::as_str)
            .is_some_and(|sourceapp| sourceapp != bundle)
        {
            error(
                "skadn_mismatch",
                "Vungle SKAdNetwork request sourceapp must match app.bundle.",
                format!("{}imp[{index}].ext.skadn.sourceapp", super::prefix(path)),
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
            let allows_pcta =
                value_at(bid, "ext.pcta.pctapage1").map_or(true, |value| value.as_i64() == Some(1));
            if value_at(imp, "ext.pcta").and_then(Value::as_i64) == Some(1) && allows_pcta {
                required(bid, "bundle", Kind::String, &path, issues);
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
                    error("skadn_mismatch", "Vungle response SKAdNetwork sourceapp must match the originating impression.", join_instance_path(&base, "sourceapp"), issues);
                }
            }
            for (field, offered_field) in [("network", "skadnetids"), ("version", "versions")] {
                if let (Some(actual), Some(allowed)) = (
                    skadn.get(field).and_then(Value::as_str),
                    offered.get(offered_field).and_then(Value::as_array),
                ) {
                    if allowed.iter().all(Value::is_string)
                        && !allowed.iter().any(|value| value.as_str() == Some(actual))
                    {
                        error("skadn_mismatch", &format!("Vungle response SKAdNetwork {field} must match an advertised {offered_field} value."), join_instance_path(&base, field), issues);
                    }
                }
            }
        }
    }
}

fn validate_supply_chain(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    fields(
        object,
        &[
            ("ext.schain", Kind::Object),
            ("ext.schain.ver", Kind::String),
            ("ext.schain.nodes", Kind::Objects),
        ],
        path,
        issues,
    );
    integer(object, "ext.schain.complete", 0, 1, path, issues);
    let Some(nodes) = value_at(object, "ext.schain.nodes").and_then(Value::as_array) else {
        return;
    };
    for (index, node) in nodes.iter().enumerate() {
        let Some(node) = node.as_object() else {
            continue;
        };
        let node_path = format!("{}[{index}]", join_instance_path(path, "ext.schain.nodes"));
        fields(
            node,
            &[
                ("asi", Kind::String),
                ("sid", Kind::String),
                ("rid", Kind::String),
                ("name", Kind::String),
            ],
            &node_path,
            issues,
        );
        integer(node, "hp", 0, 1, &node_path, issues);
    }
}

fn fields(
    object: &Map<String, Value>,
    entries: &[(&str, Kind)],
    path: &str,
    issues: &mut Vec<Issue>,
) {
    for (field, kind) in entries {
        if value_at(object, field).is_some_and(|value| !kind.matches(value)) {
            error(
                "type_invalid",
                &format!("Vungle {field} has an invalid documented JSON type."),
                join_instance_path(path, field),
                issues,
            );
        }
    }
}

impl Kind {
    fn matches(self, value: &Value) -> bool {
        match self {
            Self::String => value.is_string(),
            Self::Object => value.is_object(),
            Self::Objects => value
                .as_array()
                .is_some_and(|items| items.iter().all(Value::is_object)),
            Self::Strings => value
                .as_array()
                .is_some_and(|items| items.iter().all(Value::is_string)),
            Self::Integer => value.is_i64() || value.is_u64(),
        }
    }
}

fn required(
    object: &Map<String, Value>,
    field: &str,
    kind: Kind,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    if !path_populated(object, field)
        || !value_at(object, field).is_some_and(|value| kind.matches(value))
    {
        missing(field, path, issues);
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
        missing(field, path, issues);
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
        &format!("Vungle {field} must be an integer from {min} through {max}."),
        path,
        issues,
    );
}

fn integer_values(
    object: &Map<String, Value>,
    field: &str,
    allowed: &[i64],
    required_field: bool,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(value) = value_at(object, field) else {
        if required_field {
            missing(field, path, issues);
        }
        return;
    };
    let Some(values) = value.as_array() else {
        error(
            "type_invalid",
            "Vungle field must be an array of integers.",
            join_instance_path(path, field),
            issues,
        );
        return;
    };
    if required_field && values.is_empty() {
        missing(field, path, issues);
    }
    for (index, value) in values.iter().enumerate() {
        if !value.as_i64().is_some_and(|value| allowed.contains(&value)) {
            error(
                "value_invalid",
                &format!(
                    "Vungle {field} must contain only documented supported values {allowed:?}."
                ),
                format!("{}[{index}]", join_instance_path(path, field)),
                issues,
            );
        }
    }
}

fn string_values(
    object: &Map<String, Value>,
    field: &str,
    allowed: &[&str],
    required_field: bool,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(value) = value_at(object, field) else {
        if required_field {
            missing(field, path, issues);
        }
        return;
    };
    let Some(values) = value.as_array() else {
        error(
            "type_invalid",
            "Vungle field must be an array of strings.",
            join_instance_path(path, field),
            issues,
        );
        return;
    };
    if required_field && values.is_empty() {
        missing(field, path, issues);
    }
    for (index, value) in values.iter().enumerate() {
        if !value.as_str().is_some_and(|value| allowed.contains(&value)) {
            error(
                "value_invalid",
                &format!("Vungle {field} supports {allowed:?}."),
                format!("{}[{index}]", join_instance_path(path, field)),
                issues,
            );
        }
    }
}

fn skadn_version(object: &Map<String, Value>, field: &str, path: &str, issues: &mut Vec<Issue>) {
    if object
        .get(field)
        .and_then(Value::as_str)
        .is_some_and(|version| version_major(version).map_or(true, |major| major < 2))
    {
        error(
            "skadn_version",
            "Vungle SKAdNetwork version must be a major.minor string for version 2.0 or later.",
            join_instance_path(path, field),
            issues,
        );
    }
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

fn uuid_shape(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| {
            if [8, 13, 18, 23].contains(&index) {
                byte == b'-'
            } else {
                byte.is_ascii_hexdigit()
            }
        })
}

fn nonempty_string(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|value| !value.is_empty())
}

fn missing(field: &str, path: &str, issues: &mut Vec<Issue>) {
    issues.push(profile_issue(
        "openrtb.profile.field_required",
        format!("Vungle requires a populated, correctly typed {field}."),
        join_instance_path(path, field),
    ));
}

fn error(suffix: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    issues.push(profile_issue(
        &format!("openrtb.profile.vungle.{suffix}"),
        message.to_owned(),
        path,
    ));
}

fn warning(suffix: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    let mut issue = profile_issue(
        &format!("openrtb.profile.vungle.{suffix}"),
        message.to_owned(),
        path,
    );
    issue.severity = Severity::Warning;
    issues.push(issue);
}
