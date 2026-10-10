//! Explicit vendor wire alternatives. Every exception has a scoped source receipt.
use super::{join_instance_path, profile_issue, Profile};
use crate::{native, Issue};
use serde_json::{Map, Value};

pub(crate) fn native_object(profile: Profile) -> bool {
    matches!(
        profile.as_str(),
        "bidswitch" | "bidswitch-supplier" | "mobilefuse" | "mobilefuse-sdk" | "commerce-grid"
    )
}

pub(crate) fn native_alternative(profile: Profile) -> bool {
    matches!(profile.as_str(), "bidswitch-supplier" | "commerce-grid")
}

pub(crate) fn adm_native(profile: Profile) -> bool {
    matches!(profile.as_str(), "bidswitch-supplier" | "applovin-alx")
}

pub(crate) fn seat_precedence(profile: Profile) -> bool {
    matches!(profile.as_str(), "bidswitch" | "bidswitch-supplier")
}

pub(crate) fn required_exception(
    profile: Profile,
    name: &str,
    field: &str,
    object: &Map<String, Value>,
) -> bool {
    (name == "Video"
        && field == "mimes"
        && matches!(profile.as_str(), "mobilefuse" | "mobilefuse-sdk"))
        || (name == "Native"
            && field == "request"
            && native_alternative(profile)
            && object.contains_key("request_native"))
}

pub(crate) fn field_shape(profile: Profile, name: &str, field: &str, value: &Value) -> bool {
    (name == "Native" && field == "request" && value.is_object() && native_object(profile))
        || (profile.as_str() == "equativ-supplier"
            && name == "Publisher"
            && field == "id"
            && (value.is_i64() || value.is_u64()))
}

pub(crate) fn extra_field(profile: Profile, name: &str, field: &str) -> bool {
    (profile == Profile::DigitalTurbine
        && ((name == "Geo" && field == "dma")
            || (name == "Imp" && field == "rwdd")
            || (name == "Video" && field == "plcmt")))
        || (name == "User" && field == "eids" && profile.as_str() == "adform-handler")
        || (name == "Bid" && field == "crtype" && profile.as_str() == "applovin-alx")
        || (profile.as_str() == "adform-handler"
            && ((name == "Regs" && matches!(field, "gpp" | "gpp_sid"))
                || (name == "Bid" && field == "dur")))
        || (profile.as_str() == "yandex-sdk-bidding" && name == "Regs" && field == "gdpr")
        || (name == "Bid" && field == "admobject" && profile.as_str() == "inmobi")
        || (name == "Segment"
            && field == "signal"
            && matches!(
                profile.as_str(),
                "mobilefuse-sdk" | "inmobi" | "yandex-sdk-bidding"
            ))
        || (name == "Native" && field == "request_native" && native_alternative(profile))
        || (name == "Bid" && field == "adm_native" && adm_native(profile))
}

pub(crate) fn native_request(profile: Profile, value: &Value) -> Option<Map<String, Value>> {
    if profile == Profile::DigitalTurbine
        && value
            .as_object()
            .is_some_and(|map| !map.contains_key("request"))
    {
        return native::parse_object_value(value);
    }
    let source = value.as_object().and_then(|object| {
        object.get("request").or_else(|| {
            if native_alternative(profile) {
                object.get("request_native")
            } else {
                None
            }
        })
    });
    let source = source.or_else(|| {
        if profile.as_str() == "commerce-grid" {
            Some(value)
        } else {
            None
        }
    })?;
    if source.is_string() || native_object(profile) {
        native::parse_object_value(source)
    } else {
        None
    }
}

pub(crate) fn direct_native(profile: Profile, value: &Value, path: &str, issues: &mut Vec<Issue>) {
    let source = value
        .as_object()
        .filter(|_| profile != Profile::DigitalTurbine)
        .and_then(|object| {
            object
                .get("request")
                .map(|value| (value, "request"))
                .or_else(|| {
                    object
                        .get("request_native")
                        .map(|value| (value, "request_native"))
                })
        });
    let (source, suffix) = source.unwrap_or((value, ""));
    let markup_path = if suffix.is_empty() {
        path.to_owned()
    } else {
        join_instance_path(path, suffix)
    };
    if let Some(root) = native::parse_object_value(source) {
        let start = issues.len();
        if profile == Profile::DigitalTurbine {
            super::digital_turbine::validate_native(&root, &markup_path, issues);
        } else {
            super::commerce_grid::validate_native(&root, &markup_path, issues);
        }
        for issue in &mut issues[start..] {
            if issue.section.is_none() {
                issue.section = profile.source_url().map(String::from);
            }
        }
        native::validate_markup_request(
            &root,
            &markup_path,
            root.get("ver").and_then(Value::as_str),
            true,
            issues,
        );
        if profile == Profile::CommerceGrid
            && root.get("privacy").and_then(Value::as_i64) != Some(1)
        {
            let mut issue = profile_issue(
                "openrtb.profile.commerce_grid.native_privacy",
                "Commerce Grid native requests require privacy 1.".to_owned(),
                join_instance_path(&markup_path, "privacy"),
            );
            issue.section = profile.source_url().map(String::from);
            issues.push(issue);
        }
    } else {
        let mut issue = profile_issue(
            "openrtb.profile.native_encoding",
            "This exchange requires a Native request encoded as an object or a JSON object string."
                .to_owned(),
            markup_path,
        );
        issue.section = profile.source_url().map(String::from);
        issues.push(issue);
    }
}

pub(crate) fn native_bid(profile: Profile, bid: &Map<String, Value>) -> Option<Map<String, Value>> {
    if let Some(adm) = bid
        .get("adm")
        .and_then(Value::as_str)
        .filter(|markup| !markup.trim().is_empty())
    {
        return native::parse_encoded_object(adm);
    }
    if adm_native(profile) {
        bid.get("adm_native").and_then(native::parse_object_value)
    } else {
        None
    }
}

pub(crate) fn interest_group_response(profile: Profile, response: &Map<String, Value>) -> bool {
    matches!(profile.as_str(), "bidswitch" | "bidswitch-supplier")
        && super::value_at(response, "ext.igbid")
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty())
}

pub(crate) fn raw_response(profile: Profile, input: &str, issues: &mut Vec<Issue>) {
    // The source says 4 KB without defining decimal vs binary units.
    // Error only above 4096 bytes, outside both interpretations.
    if profile.as_str() == "applovin-alx" && input.len() > 4096 {
        let mut issue = profile_issue(
            "openrtb.profile.applovin.response_size",
            "ALX documents a 4 KB maximum bid response; this JSON body exceeds 4096 UTF-8 bytes."
                .to_owned(),
            String::new(),
        );
        issue.path = None;
        issue.section = profile.source_url().map(String::from);
        issues.push(issue);
    }
}

/// SDK bidding negotiates the format through its token and ad_type.
pub(crate) fn token_impression(profile: Profile, imp: &Map<String, Value>) -> bool {
    profile.as_str() == "yandex-sdk-bidding"
        && super::value_at(imp, "ext.ad_type")
            .and_then(Value::as_str)
            .is_some_and(|kind| {
                matches!(
                    kind,
                    "banner" | "native" | "interstitial" | "rewarded" | "appopenad"
                )
            })
}

/// Both BidSwitch contracts explicitly permit an empty seatbid no-bid response.
pub(crate) fn empty_no_bid(profile: Profile, response: &Map<String, Value>) -> bool {
    matches!(profile.as_str(), "bidswitch" | "bidswitch-supplier")
        && response
            .get("seatbid")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty)
}
