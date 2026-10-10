//! Digital Turbine's published producer requests and DSP responses.
use super::{
    contract::{self, Field, Kind},
    join_instance_path, require_integer_in_range, value_at,
};
use crate::Issue;
use serde_json::{Map, Value};

const SOURCE: &str = "https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs";
const MACROS: &str = "https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs/supported-auction-macros";
const SKAD: &str =
    "https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs/skadnetwork";
const OVERLAY_SOURCE: &str =
    "https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs/skoverlay";
const STORE: &str =
    "https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs/auto-store";
const PLAYABLE: &str =
    "https://docs.digitalturbine.com/dt-ads-demand/ad-creative-types/rewarded-playables";
const DUAL: &str =
    "https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs/dual-end-card";
macro_rules! f {
    ($name:literal, $kind:expr) => {
        Field {
            name: $name,
            kind: $kind,
        }
    };
}
const STRINGS: Kind = Kind::Array(&Kind::String);
const INTS: Kind = Kind::Array(&Kind::Integer);
const NODE: &[Field] = &[
    f!("asi", Kind::String),
    f!("sid", Kind::String),
    f!("hp", Kind::Flag),
    f!("rid", Kind::String),
    f!("name", Kind::String),
    f!("domain", Kind::String),
    f!("ext", Kind::Object(&[])),
];
const CHAIN: &[Field] = &[
    f!("ver", Kind::String),
    f!("complete", Kind::Flag),
    f!("nodes", Kind::Array(&Kind::Object(NODE))),
    f!("ext", Kind::Object(&[])),
];
const UID: &[Field] = &[
    f!("id", Kind::String),
    f!("atype", Kind::Integer),
    f!("ext", Kind::Object(&[])),
];
const EID: &[Field] = &[
    f!("source", Kind::String),
    f!("uids", Kind::Array(&Kind::Object(UID))),
    f!("ext", Kind::Object(&[])),
];
const USER_EXT: &[Field] = &[
    f!("consent", Kind::String),
    f!("impdepth", Kind::Integer),
    f!("sessionduration", Kind::Integer),
    f!("lastbundle", Kind::String),
    f!("lastadomain", Kind::String),
    f!("clickrate", Kind::Number),
    f!("lastclick", Kind::CompatibleFlag),
    f!("lastclicktype", Kind::Integer),
    f!("completionrate", Kind::Number),
    f!("eids", Kind::Array(&Kind::Object(EID))),
];
const DEVICE_EXT: &[Field] = &[
    f!("inputLanguage", STRINGS),
    f!("inputlanguage", STRINGS),
    f!("ifv", Kind::String),
    f!("atts", Kind::Integer),
    f!("diskspace", Kind::Integer),
    f!("totaldisk", Kind::Integer),
    f!("ringmute", Kind::Flag),
    f!("charging", Kind::Flag),
    f!("bluetooth", Kind::CompatibleFlag),
    f!("headset", Kind::CompatibleFlag),
    f!("batterylevel", Kind::Integer),
    f!("batterysaver", Kind::CompatibleFlag),
    f!("darkmode", Kind::CompatibleFlag),
    f!("airplane", Kind::CompatibleFlag),
    f!("dnd", Kind::CompatibleFlag),
];
const REQ_SKAD: &[Field] = &[
    f!("version", Kind::String),
    f!("versions", STRINGS),
    f!("sourceapp", Kind::String),
    f!("skadnetids", STRINGS),
    f!(
        "skadnetlist",
        Kind::Object(&[
            f!("max", Kind::Integer),
            f!("excl", INTS),
            f!("addl", STRINGS)
        ])
    ),
    f!("skoverlay", STRINGS),
    f!("productpage", Kind::Flag),
    f!("ext", Kind::Object(&[])),
];
const OVERLAY: &[Field] = &[
    f!("present", Kind::Flag),
    f!("dismissible", Kind::Flag),
    f!("delay", Kind::Integer),
    f!("pos", Kind::Flag),
    f!("endcarddelay", Kind::Integer),
    f!("autoclose", Kind::Integer),
    f!("autoclick", Kind::Flag),
    f!("landpageclick", Kind::Flag),
    f!("landpagedelay", Kind::Integer),
    f!("ext", Kind::Object(&[])),
];
const FIDELITY: &[Field] = &[
    f!("fidelity", Kind::Integer),
    f!("nonce", Kind::String),
    f!("timestamp", Kind::String),
    f!("signature", Kind::String),
];
const RESP_SKAD: &[Field] = &[
    f!("version", Kind::String),
    f!("network", Kind::String),
    f!("campaign", Kind::String),
    f!("sourceidentifier", Kind::String),
    f!("itunesitem", Kind::String),
    f!("sourceapp", Kind::String),
    f!("nonce", Kind::String),
    f!("timestamp", Kind::String),
    f!("signature", Kind::String),
    f!("fidelities", Kind::Array(&Kind::Object(FIDELITY))),
    f!("skoverlay", Kind::Object(OVERLAY)),
    f!("productpageid", Kind::String),
    f!("ext", Kind::Object(&[])),
];
const BID_EXT: &[Field] = &[
    f!("crtype", Kind::String),
    f!("skadn", Kind::Object(RESP_SKAD)),
    f!("autostore", Kind::Flag),
    f!("autostoreclick", Kind::Flag),
    f!("dualendcard", Kind::Flag),
    f!("clicktrackers", STRINGS),
];
const IMG: &[Field] = &[
    f!("type", Kind::Integer),
    f!("w", Kind::Integer),
    f!("h", Kind::Integer),
    f!("wmin", Kind::Integer),
    f!("hmin", Kind::Integer),
    f!("mimes", STRINGS),
    f!("ext", Kind::Object(&[])),
];
const NATIVE_VIDEO: &[Field] = &[
    f!("mimes", STRINGS),
    f!("minduration", Kind::Integer),
    f!("maxduration", Kind::Integer),
    f!("protocols", INTS),
    f!("ext", Kind::Object(&[])),
];
const ASSET: &[Field] = &[
    f!("id", Kind::Integer),
    f!("required", Kind::Flag),
    f!(
        "title",
        Kind::Object(&[f!("len", Kind::Integer), f!("ext", Kind::Object(&[]))])
    ),
    f!("img", Kind::Object(IMG)),
    f!("video", Kind::Object(NATIVE_VIDEO)),
    f!(
        "data",
        Kind::Object(&[
            f!("type", Kind::Integer),
            f!("len", Kind::Integer),
            f!("ext", Kind::Object(&[]))
        ])
    ),
    f!("ext", Kind::Object(&[])),
];
const NATIVE: &[Field] = &[
    f!("ver", Kind::String),
    f!("context", Kind::Integer),
    f!("contextsubtype", Kind::Integer),
    f!("plcmttype", Kind::Integer),
    f!("plcmtcnt", Kind::Integer),
    f!("seq", Kind::Integer),
    f!("assets", Kind::Array(&Kind::Object(ASSET))),
    f!(
        "eventtrackers",
        Kind::Array(&Kind::Object(&[
            f!("event", Kind::Integer),
            f!("methods", INTS),
            f!("ext", Kind::Object(&[]))
        ]))
    ),
    f!("ext", Kind::Object(&[])),
];

pub(super) fn validate(
    name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    match name {
        "BidRequest" => {
            for field in ["app", "device", "source", "regs", "at", "tmax", "bcat"] {
                required(object, field, path, issues);
            }
            for field in ["user", "cur", "badv", "bapp"] {
                advisory_missing(object, field, path, issues);
            }
            contract::integer_enum(object, "at", &[1, 2], path, issues);
            if let Some(imps) = object.get("imp").and_then(Value::as_array) {
                for (i, imp) in imps.iter().enumerate() {
                    if let (Some(actual), Some(bundle)) = (
                        value_at_value(imp, "ext.skadn.sourceapp").and_then(Value::as_str),
                        value_at(object, "app.bundle").and_then(Value::as_str),
                    ) {
                        if actual != bundle {
                            error(
                                "skadn_mismatch",
                                "DT request SKAdNetwork sourceapp must match app.bundle.",
                                format!("{}imp[{i}].ext.skadn.sourceapp", prefix(path)),
                                SKAD,
                                issues,
                            );
                        }
                    }
                }
            }
        }
        "Source" => {
            for field in ["fd", "ext.schain"] {
                required(object, field, path, issues);
            }
            advisory_missing(object, "tid", path, issues);
            contract::fields(
                object,
                &[f!(
                    "ext",
                    Kind::Object(&[
                        f!("schain", Kind::Object(CHAIN)),
                        f!("omidpn", Kind::String),
                        f!("omidpv", Kind::String)
                    ])
                )],
                path,
                issues,
            );
            if let Some(chain) = value_at(object, "ext.schain").and_then(Value::as_object) {
                let base = join_instance_path(path, "ext.schain");
                for field in ["ver", "complete", "nodes"] {
                    required(chain, field, &base, issues);
                }
                nonempty_array(chain, "nodes", &base, issues);
                if let Some(nodes) = chain.get("nodes").and_then(Value::as_array) {
                    for (i, node) in nodes.iter().enumerate() {
                        if let Some(node) = node.as_object() {
                            for field in ["asi", "sid", "hp"] {
                                required(node, field, &format!("{base}.nodes[{i}]"), issues);
                            }
                        }
                    }
                }
            }
        }
        "Regs" => {
            for field in ["coppa", "ext.gdpr"] {
                required(object, field, path, issues);
            }
            contract::fields(
                object,
                &[f!(
                    "ext",
                    Kind::Object(&[
                        f!("gdpr", Kind::Flag),
                        f!("us_privacy", Kind::String),
                        f!("gpp", Kind::String),
                        f!("gpp_sid", INTS)
                    ])
                )],
                path,
                issues,
            );
            // us_privacy is explicitly conditional on publisher availability.
        }
        "Imp" => {
            contract::fields(object, &[f!("rwdd", Kind::Flag)], path, issues);
            for field in [
                "displaymanager",
                "displaymanagerver",
                "instl",
                "tagid",
                "bidfloor",
                "bidfloorcur",
                "clickbrowser",
                "exp",
                "rwdd",
            ] {
                required(object, field, path, issues);
            }
            advisory_missing(object, "secure", path, issues);
            advisory_missing(object, "ext.brsrclk", path, issues);
            contract::fields(
                object,
                &[f!(
                    "ext",
                    Kind::Object(&[
                        f!("brsrclk", Kind::Flag),
                        f!("dpl", Kind::Flag),
                        f!("skadn", Kind::Object(REQ_SKAD))
                    ])
                )],
                path,
                issues,
            );
            if let Some(skadn) = value_at(object, "ext.skadn").and_then(Value::as_object) {
                request_skad(skadn, &join_instance_path(path, "ext.skadn"), issues);
            }
        }
        "Metric" => {
            for field in ["type", "value", "vendor"] {
                required(object, field, path, issues);
            }
        }
        "Banner" => {
            for field in ["btype", "battr", "pos", "topframe", "mimes", "api", "id"] {
                if path.contains("companionad[") {
                    advisory_missing(object, field, path, issues);
                } else {
                    required(object, field, path, issues);
                }
            }
            advisory_missing(object, "ext.rewarded", path, issues);
            if !object
                .get("format")
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty())
            {
                for field in ["w", "h"] {
                    advisory_missing(object, field, path, issues);
                }
            }
            contract::fields(
                object,
                &[f!(
                    "ext",
                    Kind::Object(&[f!("rewarded", Kind::Flag), f!("autostore", Kind::Flag)])
                )],
                path,
                issues,
            );
            // Eligibility and defaults vary by SDK and traffic; never require autostore.
        }
        "Video" => {
            for field in [
                "minduration",
                "maxduration",
                "w",
                "h",
                "startdelay",
                "placement",
                "plcmt",
                "skip",
                "battr",
                "playbackmethod",
                "playbackend",
                "pos",
                "api",
                "ext.rewarded",
                "ext.mraidendcard",
            ] {
                required(object, field, path, issues);
            }
            contract::integer_enum(object, "startdelay", &[0], path, issues);
            contract::integer_enum(object, "placement", &[5], path, issues);
            contract::integer_enum(object, "plcmt", &[3], path, issues);
            contract::integer_enum(object, "linearity", &[1], path, issues);
            contract::integer_enum(object, "playbackend", &[1], path, issues);
            contract::integer_enum(object, "pos", &[7], path, issues);
            array_subset(object, "protocols", &[2, 3, 5, 6, 7, 8], path, issues);
            array_subset(object, "playbackmethod", &[5], path, issues);
            array_subset(object, "companiontype", &[1, 2, 3], path, issues);
            contract::fields(
                object,
                &[f!(
                    "ext",
                    Kind::Object(&[
                        f!("rewarded", Kind::Flag),
                        f!("mraidendcard", Kind::Flag),
                        f!("autostore", Kind::Flag),
                        f!("dualendcard", Kind::Flag)
                    ])
                )],
                path,
                issues,
            );
            boolean(object, "ext.dspdualendcard", path, issues);
            // Companion fields are conditional on available companions, despite Yes labels.
        }
        "NativeRequest" => validate_native(object, path, issues),
        "Native" => {
            if let Some(ver) = object.get("ver").filter(|v| !v.is_null()) {
                if ver.as_str() != Some("1.2") {
                    error(
                        "native_version",
                        "DT Native requests use version 1.2.",
                        join_instance_path(path, "ver"),
                        SOURCE,
                        issues,
                    );
                }
            }
        }
        "Pmp" | "PMP" => {
            // The table misspells this key; the official examples use private_auction.
            required(object, "private_auction", path, issues);
            required(object, "deals", path, issues);
        }
        "Deal" => {
            required(object, "bidfloor", path, issues);
            for field in ["at", "wseat"] {
                advisory_missing(object, field, path, issues);
            }
        }
        "App" => {
            for field in ["id", "name", "cat", "ver", "privacypolicy", "publisher"] {
                required(object, field, path, issues);
            }
            contract::fields(
                object,
                &[f!(
                    "ext",
                    Kind::Object(&[
                        f!("devuserid", Kind::String),
                        f!("storecat", Kind::String),
                        f!("storesubcat", STRINGS),
                        f!("fmwname", Kind::String),
                        f!("apilevel", Kind::Integer)
                    ])
                )],
                path,
                issues,
            );
            if value_at(object, "ext.storesubcat")
                .and_then(Value::as_array)
                .is_some_and(|a| a.len() > 3)
            {
                error(
                    "store_categories",
                    "DT caps store subcategories at three strings.",
                    join_instance_path(path, "ext.storesubcat"),
                    SOURCE,
                    issues,
                );
            }
            if let Some(fmw) = value_at(object, "ext.fmwname").and_then(Value::as_str) {
                if !matches!(fmw, "unity" | "native") {
                    error(
                        "framework",
                        "DT reports its app framework as unity or native.",
                        join_instance_path(path, "ext.fmwname"),
                        SOURCE,
                        issues,
                    );
                }
            }
        }
        "Publisher" => {
            required(object, "id", path, issues);
            // The published request examples omit name despite its Always Passed label.
            advisory_missing(object, "name", path, issues);
        }
        "Device" => {
            for field in [
                "ua",
                "dnt",
                "ip",
                "devicetype",
                "h",
                "w",
                "js",
                "language",
                "connectiontype",
                "ext.headset",
            ] {
                required(object, field, path, issues);
            }
            contract::fields(object, &[f!("ext", Kind::Object(DEVICE_EXT))], path, issues);
            contract::integer_enum(object, "ext.atts", &[0, 1, 2, 3], path, issues);
            range(object, "ext.batterylevel", 1, 8, path, issues);
            for field in ["ext.diskspace", "ext.totaldisk"] {
                range(object, field, 0, i64::MAX, path, issues);
            }
            // atts has conflicting Always Passed labels; source restricts it to iOS.
        }
        "Geo" => {
            required(object, "country", path, issues);
        }
        "User" => {
            // An empty consent string is present in the official request examples.
            if value_at(object, "ext.consent").map_or(true, Value::is_null) {
                required(object, "ext.consent", path, issues);
            }
            contract::fields(object, &[f!("ext", Kind::Object(USER_EXT))], path, issues);
            contract::integer_enum(object, "ext.lastclicktype", &[0, 1, 2], path, issues);
            for field in ["ext.impdepth", "ext.sessionduration"] {
                range(object, field, 0, i64::MAX, path, issues);
            }
            for field in ["ext.clickrate", "ext.completionrate"] {
                if value_at(object, field)
                    .and_then(Value::as_f64)
                    .is_some_and(|v| !(0.0..=100.0).contains(&v))
                {
                    error(
                        "percentage",
                        "DT user percentages range from 0 through 100.",
                        join_instance_path(path, field),
                        SOURCE,
                        issues,
                    );
                }
            }
        }
        "BidResponse" => {
            if object
                .get("cur")
                .is_some_and(|v| !v.is_null() && v.as_str() != Some("USD"))
            {
                error(
                    "currency",
                    "DT supports only USD.",
                    join_instance_path(path, "cur"),
                    SOURCE,
                    issues,
                );
            }
        }
        "SeatBid" => required(object, "seat", path, issues),
        "Bid" => bid(object, path, issues),
        _ => {}
    }
}

pub(super) fn validate_native(root: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let start = issues.len();
    contract::fields(root, NATIVE, path, issues);
    for field in [
        "ver",
        "context",
        "contextsubtype",
        "plcmttype",
        "plcmtcnt",
        "seq",
        "eventtrackers",
        "assets",
    ] {
        required(root, field, path, issues);
    }
    if root.get("ver").is_some_and(|v| v.as_str() != Some("1.2")) {
        error(
            "native_version",
            "DT Native requests use version 1.2.",
            join_instance_path(path, "ver"),
            SOURCE,
            issues,
        );
    }
    for (field, value) in [
        ("context", 1),
        ("contextsubtype", 10),
        ("plcmttype", 1),
        ("plcmtcnt", 1),
        ("seq", 0),
    ] {
        contract::integer_enum(root, field, &[value], path, issues);
    }
    if let Some(trackers) = root.get("eventtrackers").and_then(Value::as_array) {
        for (i, tracker) in trackers.iter().enumerate() {
            if let Some(tracker) = tracker.as_object() {
                let base = format!("{path}.eventtrackers[{i}]");
                contract::integer_enum(tracker, "event", &[1], &base, issues);
                array_subset(tracker, "methods", &[1, 2], &base, issues);
            }
        }
    }
    if let Some(assets) = root.get("assets").and_then(Value::as_array) {
        for (i, asset) in assets.iter().enumerate() {
            let Some(asset) = asset.as_object() else {
                continue;
            };
            let Some(id) = asset.get("id").and_then(Value::as_i64) else {
                continue;
            };
            let branch = match id {
                1 => Some("title"),
                2 | 4 => Some("img"),
                3 => Some("video"),
                5..=7 => Some("data"),
                _ => None,
            };
            if branch.is_none()
                || !branch.is_some_and(|branch| asset.get(branch).is_some_and(Value::is_object))
            {
                error("native_asset","DT Native producer asset IDs 1 through 7 identify title, main image, video, icon, description, rating and CTA respectively.",format!("{path}.assets[{i}]"),SOURCE,issues);
            }
        }
    }
    for issue in &mut issues[start..] {
        if issue.section.is_none() {
            issue.section = Some(SOURCE.to_owned());
        }
    }
}

fn request_skad(skadn: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    required(skadn, "sourceapp", path, issues);
    if !skadn.contains_key("skadnetids") {
        required(skadn, "skadnetids", path, issues);
    }
    version(skadn, "version", path, issues);
    if let Some(versions) = skadn.get("versions").and_then(Value::as_array) {
        for (i, v) in versions.iter().enumerate() {
            if v.as_str().is_some_and(|v| major(v).map_or(true, |m| m < 2)) {
                error(
                    "skadn_version",
                    "DT supports SKAdNetwork 2.0 and later.",
                    format!("{path}.versions[{i}]"),
                    SKAD,
                    issues,
                );
            }
        }
    }
}
fn bid(bid: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    for field in ["cid", "crid", "adomain"] {
        required(bid, field, path, issues);
    }
    for field in ["attr", "cat", "ext.crtype"] {
        advisory_missing(bid, field, path, issues);
    }
    advisory_missing(bid, "burl", path, issues);
    advisory_missing(bid, "adm", path, issues);
    contract::fields(bid, &[f!("ext", Kind::Object(BID_EXT))], path, issues);
    boolean(bid, "ext.dspdualendcard", path, issues);
    if value_at(bid, "ext.crtype")
        .and_then(Value::as_str)
        .is_some_and(str::is_empty)
    {
        error(
            "creative_type",
            "DT ext.crtype must identify the creative type when supplied.",
            join_instance_path(path, "ext.crtype"),
            SOURCE,
            issues,
        );
    }
    if !["nurl", "burl", "adm"].iter().any(|f| {
        bid.get(*f)
            .and_then(Value::as_str)
            .is_some_and(|v| v.contains("${AUCTION_PRICE}"))
    }) {
        error(
            "price_macro",
            "DT requires AUCTION_PRICE in nurl, burl or an adm tracking pixel.",
            join_instance_path(path, "adm"),
            MACROS,
            issues,
        );
    }
    for field in ["burl", "lurl", "adm"] {
        if bid
            .get(field)
            .and_then(Value::as_str)
            .is_some_and(|v| v.contains("${AUCTION_MIN_TO_WIN}"))
        {
            error(
                "macro_context",
                "DT supports AUCTION_MIN_TO_WIN only in nurl.",
                join_instance_path(path, field),
                MACROS,
                issues,
            );
        }
    }
    if let Some(skadn) = value_at(bid, "ext.skadn").and_then(Value::as_object) {
        response_skad(skadn, &join_instance_path(path, "ext.skadn"), bid, issues);
    }
    if value_at(bid, "ext.autostore").and_then(Value::as_i64) == Some(1)
        && !contract::present(bid, "bundle")
        && !contract::present(bid, "ext.skadn.itunesitem")
    {
        error(
            "store_id",
            "DT Auto Store requires a store listing ID in bundle or SKAdNetwork itunesitem.",
            join_instance_path(path, "bundle"),
            STORE,
            issues,
        );
    }
    if value_at(bid, "ext.dualendcard").and_then(Value::as_i64) == Some(1) {
        required_cited(bid, "bundle", path, DUAL, issues);
    }
    let display = value_at(bid, "ext.crtype")
        .and_then(Value::as_str)
        .is_some_and(|t| {
            matches!(
                t,
                "HTML" | "MRAID 1.0" | "MRAID 2.0" | "MRAID 3.0" | "Playable"
            )
        })
        || bid.get("mtype").and_then(Value::as_i64) == Some(1);
    if display
        && ((value_at(bid, "ext.autostore").and_then(Value::as_i64) == Some(1)
            && value_at(bid, "ext.autostoreclick").and_then(Value::as_i64) == Some(1))
            || (value_at(bid, "ext.skadn.skoverlay.present").and_then(Value::as_i64) == Some(1)
                && value_at(bid, "ext.skadn.skoverlay.autoclick").and_then(Value::as_i64)
                    == Some(1)))
    {
        required_cited(bid, "ext.clicktrackers", path, STORE, issues);
        if value_at(bid, "ext.clicktrackers")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
        {
            error(
                "clicktrackers",
                "DT display Auto Store and SKOverlay click firing needs a nonempty tracker list.",
                join_instance_path(path, "ext.clicktrackers"),
                STORE,
                issues,
            );
        }
    }
}
fn response_skad(
    skadn: &Map<String, Value>,
    path: &str,
    bid: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    version(skadn, "version", path, issues);
    if let Some(campaign) = skadn.get("campaign").and_then(Value::as_str) {
        if major(skadn.get("version").and_then(Value::as_str).unwrap_or("")).is_some_and(|m| m < 4)
            && !digits_range(campaign, 1, 100)
        {
            error(
                "skadn_campaign",
                "DT SKAdNetwork 2 and 3 campaign ranges 1 through 100.",
                join_instance_path(path, "campaign"),
                SKAD,
                issues,
            );
        }
    }
    if let Some(id) = skadn.get("sourceidentifier").and_then(Value::as_str) {
        if id.len() != 4 || !digits_range(id, 1, 9999) {
            error(
                "skadn_sourceidentifier",
                "DT SKAdNetwork 4 sourceidentifier is a four-digit string from 0001 through 9999.",
                join_instance_path(path, "sourceidentifier"),
                SKAD,
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
                "DT SKAdNetwork itunesitem must match advertised bundle.",
                join_instance_path(path, "itunesitem"),
                SKAD,
                issues,
            );
        }
    }
    formats(skadn, path, issues);
    if let Some(fidelities) = skadn.get("fidelities").and_then(Value::as_array) {
        for (i, fidelity) in fidelities.iter().enumerate() {
            if let Some(fidelity) = fidelity.as_object() {
                let base = format!("{path}.fidelities[{i}]");
                for field in ["fidelity", "nonce", "timestamp", "signature"] {
                    required_cited(fidelity, field, &base, SKAD, issues);
                }
                formats(fidelity, &base, issues);
            }
        }
    }
    if let Some(overlay) = skadn.get("skoverlay").and_then(Value::as_object) {
        let base = join_instance_path(path, "skoverlay");
        required_cited(overlay, "present", &base, OVERLAY_SOURCE, issues);
        for field in ["delay", "autoclose"] {
            range(overlay, field, 0, 60, &base, issues);
        }
        for field in ["endcarddelay", "landpagedelay"] {
            range(overlay, field, -1, 60, &base, issues);
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
        if let Some(offered) = value_at(imp, "ext.skadn").and_then(Value::as_object) {
            let known_networks = offered
                .get("skadnetids")
                .and_then(Value::as_array)
                .filter(|a| !a.is_empty() && a.iter().all(Value::is_string));
            if known_networks.is_some() {
                required_cited(bid, "ext.skadn", &path, SKAD, issues);
            }
            if let Some(actual) = value_at(bid, "ext.skadn").and_then(Value::as_object) {
                let base = join_instance_path(&path, "ext.skadn");
                if let (Some(expected), Some(sourceapp)) = (
                    offered.get("sourceapp").and_then(Value::as_str),
                    actual.get("sourceapp").and_then(Value::as_str),
                ) {
                    if expected != sourceapp {
                        error(
                            "skadn_mismatch",
                            "DT response sourceapp must match the uniquely resolved impression.",
                            join_instance_path(&base, "sourceapp"),
                            SKAD,
                            issues,
                        );
                    }
                }
                if let (Some(networks), Some(network)) = (
                    known_networks,
                    actual.get("network").and_then(Value::as_str),
                ) {
                    if !networks.iter().any(|v| v.as_str() == Some(network)) {
                        error(
                            "skadn_mismatch",
                            "DT response network must be offered in matched skadnetids.",
                            join_instance_path(&base, "network"),
                            SKAD,
                            issues,
                        );
                    }
                }
                if let (Some(versions), Some(version)) = (
                    offered.get("versions").and_then(Value::as_array),
                    actual.get("version").and_then(Value::as_str),
                ) {
                    if !versions.is_empty()
                        && versions.iter().all(Value::is_string)
                        && !versions.iter().any(|v| v.as_str() == Some(version))
                    {
                        error("skadn_version","DT response attribution version must be offered by the matched impression.",join_instance_path(&base,"version"),SKAD,issues);
                    }
                }
            }
        }
        let playable = value_at(imp, "banner.ext.rewarded").and_then(Value::as_i64) == Some(1)
            && value_at(imp, "banner.battr")
                .and_then(Value::as_array)
                .is_some_and(|a| a.iter().all(|v| v.as_i64() != Some(13)))
            && imp.get("instl").and_then(Value::as_i64) == Some(1)
            && value_at(bid, "ext.crtype").and_then(Value::as_str) == Some("Playable");
        if playable
            && !bid
                .get("attr")
                .and_then(Value::as_array)
                .is_some_and(|a| a.iter().any(|v| v.as_i64() == Some(13)))
        {
            error(
                "playable_attr",
                "DT Rewarded Playable responses require creative attribute 13.",
                join_instance_path(&path, "attr"),
                PLAYABLE,
                issues,
            );
        }
        if let (Some(dealid), Some(deals)) = (
            bid.get("dealid").and_then(Value::as_str),
            value_at(imp, "pmp.deals").and_then(Value::as_array),
        ) {
            if !deals
                .iter()
                .any(|d| d.get("id").and_then(Value::as_str) == Some(dealid))
            {
                error(
                    "deal_mismatch",
                    "DT response dealid must reference a deal in the matched impression.",
                    join_instance_path(&path, "dealid"),
                    SOURCE,
                    issues,
                );
            }
        }
    }
}
fn required(o: &Map<String, Value>, f: &str, p: &str, i: &mut Vec<Issue>) {
    contract::required(o, f, p, "Digital Turbine", i);
}
fn required_cited(o: &Map<String, Value>, f: &str, p: &str, s: &str, i: &mut Vec<Issue>) {
    let start = i.len();
    required(o, f, p, i);
    for issue in &mut i[start..] {
        issue.section = Some(s.to_owned());
    }
}
fn advisory_missing(o: &Map<String, Value>, f: &str, p: &str, i: &mut Vec<Issue>) {
    if !o.contains_key(f.split('.').next().unwrap_or(f)) || value_at(o, f).is_none() {
        contract::warning("openrtb.profile.digital_turbine.source_conflict","DT sources give conflicting, conditional or optional presence guidance; verify this field against your integration.",join_instance_path(p,f),i);
    }
}
fn range(o: &Map<String, Value>, f: &str, min: i64, max: i64, p: &str, i: &mut Vec<Issue>) {
    if value_at(o, f).is_some_and(|v| !v.is_null()) {
        require_integer_in_range(
            o,
            f,
            min,
            max,
            "DT value is outside its documented integer range.",
            p,
            i,
        );
    }
}
fn boolean(o: &Map<String, Value>, f: &str, p: &str, i: &mut Vec<Issue>) {
    if value_at(o, f).is_some_and(|v| !v.is_null() && !v.is_boolean()) {
        contract::error(
            "openrtb.profile.field_type",
            "DT DSP Dual End Card uses JSON true or false.",
            join_instance_path(p, f),
            i,
        );
    }
}
fn array_subset(o: &Map<String, Value>, f: &str, values: &[i64], p: &str, i: &mut Vec<Issue>) {
    if let Some(a) = value_at(o, f).and_then(Value::as_array) {
        for (index, v) in a.iter().enumerate() {
            if v.as_i64().is_some_and(|v| !values.contains(&v)) {
                error(
                    "producer_value",
                    "DT producer field contains a value outside its documented emitted subset.",
                    format!("{}.{}[{index}]", p, f),
                    SOURCE,
                    i,
                );
            }
        }
    }
}
fn nonempty_array(o: &Map<String, Value>, f: &str, p: &str, i: &mut Vec<Issue>) {
    if value_at(o, f)
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty)
    {
        error(
            "array_empty",
            "DT supply chain needs at least one node.",
            join_instance_path(p, f),
            SOURCE,
            i,
        );
    }
}
fn version(o: &Map<String, Value>, f: &str, p: &str, i: &mut Vec<Issue>) {
    if let Some(v) = o.get(f).and_then(Value::as_str) {
        if major(v).map_or(true, |m| m < 2) {
            error(
                "skadn_version",
                "DT supports SKAdNetwork 2.0 and later.",
                join_instance_path(p, f),
                SKAD,
                i,
            );
        }
    }
}
fn formats(o: &Map<String, Value>, p: &str, i: &mut Vec<Issue>) {
    if let Some(v) = o.get("nonce").and_then(Value::as_str) {
        if v.len() != 36
            || !v.bytes().enumerate().all(|(n, b)| {
                if [8, 13, 18, 23].contains(&n) {
                    b == b'-'
                } else {
                    b.is_ascii_hexdigit()
                }
            })
        {
            error(
                "skadn_nonce",
                "DT attribution nonce must be a UUID string.",
                join_instance_path(p, "nonce"),
                SKAD,
                i,
            );
        }
    }
    if let Some(v) = o.get("timestamp").and_then(Value::as_str) {
        if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
            error(
                "skadn_timestamp",
                "DT attribution timestamp must be a decimal Unix millisecond string.",
                join_instance_path(p, "timestamp"),
                SKAD,
                i,
            );
        }
    }
}
fn major(s: &str) -> Option<u32> {
    s.split('.').next()?.parse().ok()
}
fn digits_range(s: &str, min: u32, max: u32) -> bool {
    !s.is_empty()
        && s.bytes().all(|b| b.is_ascii_digit())
        && s.parse::<u32>().is_ok_and(|v| (min..=max).contains(&v))
}
fn prefix(p: &str) -> String {
    if p.is_empty() {
        String::new()
    } else {
        format!("{p}.")
    }
}
fn value_at_value<'a>(v: &'a Value, p: &str) -> Option<&'a Value> {
    p.split('.').try_fold(v, |v, k| v.get(k))
}
fn error(suffix: &str, message: &str, path: String, source: &str, issues: &mut Vec<Issue>) {
    contract::error(
        &format!("openrtb.profile.digital_turbine.{suffix}"),
        message,
        path,
        issues,
    );
    if let Some(issue) = issues.last_mut() {
        issue.section = Some(source.to_owned());
    }
}
