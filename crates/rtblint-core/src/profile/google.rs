//! Authorized Buyers JSON extensions from the reviewed Google protocol v210.
//! Sources and generator receipts live in docs/exchange-profiles.
use super::{join_instance_path, profile_issue, value_at};
use crate::{Issue, Severity};
use serde_json::{Map, Value};

#[derive(Clone, Copy)]
enum WireType {
    String,
    Int32,
    Int64String,
    Uint64String,
    Number,
    Flag,
    Enum(&'static [i64]),
    Message(&'static str),
}

struct WireField {
    name: &'static str,
    kind: WireType,
    repeated: bool,
    oneof: Option<usize>,
}

include!("google_schema.rs");

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(fields) = extension_schema(object_name) else {
        return;
    };
    if let Some(ext) = object.get("ext").and_then(Value::as_object) {
        validate_fields(ext, fields, &join_instance_path(path, "ext"), issues);
    }
    if object_name == "Bid" {
        validate_bid(object, path, issues);
    }
}

fn validate_fields(
    object: &Map<String, Value>,
    fields: &[WireField],
    path: &str,
    issues: &mut Vec<Issue>,
) {
    for field in fields {
        let Some(value) = object.get(field.name) else {
            continue;
        };
        let field_path = join_instance_path(path, field.name);
        if field.repeated {
            let Some(values) = value.as_array() else {
                invalid(
                    &field_path,
                    "must be an array in Authorized Buyers JSON",
                    issues,
                );
                continue;
            };
            for (index, item) in values.iter().enumerate() {
                validate_value(item, field.kind, &format!("{field_path}[{index}]"), issues);
            }
        } else {
            validate_value(value, field.kind, &field_path, issues);
        }
        if let Some(group) = field.oneof {
            // Report once, at the later member, and count actual presence rather
            // than truthiness. A oneof field holding an empty string is set.
            let conflict = fields
                .iter()
                .take_while(|other| other.name != field.name)
                .any(|other| other.oneof == Some(group) && object.contains_key(other.name));
            if conflict {
                issues.push(profile_issue(
                    "openrtb.profile.google.oneof_conflict",
                    String::from("Google's protobuf oneof permits at most one of these fields."),
                    field_path,
                ));
            }
        }
    }
}

fn validate_value(value: &Value, kind: WireType, path: &str, issues: &mut Vec<Issue>) {
    if let WireType::Message(name) = kind {
        match value.as_object() {
            Some(object) => {
                if let Some(fields) = message_schema(name) {
                    validate_fields(object, fields, path, issues);
                }
            }
            None => invalid(path, "must be an object in Authorized Buyers JSON", issues),
        }
        return;
    }
    let (valid, requirement) = match kind {
        WireType::String => (value.is_string(), "must be a string"),
        WireType::Int32 => (
            value.as_i64().is_some_and(|v| i32::try_from(v).is_ok()),
            "must be a signed 32-bit integer JSON number",
        ),
        WireType::Int64String => (
            signed_id(value).is_some(),
            "must be a decimal string within the signed 64-bit range",
        ),
        WireType::Uint64String => (
            value.as_str().is_some_and(|v| {
                !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()) && v.parse::<u64>().is_ok()
            }),
            "must be a decimal string within the unsigned 64-bit range",
        ),
        WireType::Number => (value.is_number(), "must be a JSON number"),
        WireType::Flag => (
            matches!(value.as_i64(), Some(0 | 1)),
            "must be the integer flag 0 or 1 in Authorized Buyers JSON",
        ),
        WireType::Enum(allowed) => (
            value.as_i64().is_some_and(|v| allowed.contains(&v)),
            "must be an integer from the enum in the reviewed Google protocol",
        ),
        WireType::Message(_) => unreachable!(),
    };
    if !valid {
        invalid(path, requirement, issues);
    }
}

fn invalid(path: &str, requirement: &str, issues: &mut Vec<Issue>) {
    issues.push(profile_issue(
        "openrtb.profile.google.value_invalid",
        format!("{path} {requirement}."),
        String::from(path),
    ));
}

fn signed_id(value: &Value) -> Option<i64> {
    let text = value.as_str()?;
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

fn validate_bid(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    if let Some(sdk) = value_at(object, "ext.sdk_rendered_ad").and_then(Value::as_object) {
        validate_selected_sdk(object, sdk, path, issues);
    }
    if let Some(payload) =
        value_at(object, "ext.event_notification_token.payload").and_then(Value::as_str)
    {
        // The protocol declares string, not bytes. Count decoded UTF-8 bytes,
        // without interpreting the opaque string as base64 or JSON escapes.
        if payload.len() > 128 {
            let mut issue = profile_issue(
                "openrtb.profile.google.event_token_ignored",
                String::from(
                    "Google ignores event notification token payloads longer than 128 UTF-8 bytes.",
                ),
                join_instance_path(path, "ext.event_notification_token.payload"),
            );
            issue.severity = Severity::Warning;
            issue.section = Some(String::from("https://developers.google.com/static/authorized-buyers/rtb/downloads/openrtb-adx-proto.txt"));
            issues.push(issue);
        }
    }
    if let Some(markup) = object.get("adm").and_then(Value::as_str) {
        if let Ok(value) = serde_json::from_str::<Value>(markup) {
            if let Some(envelope) = value.as_object() {
                let (native, native_path) =
                    if let Some(native) = envelope.get("native").and_then(Value::as_object) {
                        (native, join_instance_path(path, "adm.native"))
                    } else {
                        (envelope, join_instance_path(path, "adm"))
                    };
                if let Some(trackers) = native.get("eventtrackers").and_then(Value::as_array) {
                    for (index, tracker) in trackers.iter().enumerate() {
                        if let Some(tracker) = tracker.as_object() {
                            validate(
                                "EventTracker",
                                tracker,
                                &format!("{native_path}.eventtrackers[{index}]"),
                                issues,
                            );
                        }
                    }
                }
            }
        }
    }
    // v210 BidExt.clickurl requires click URL or advertiser-domain declaration.
    // Do not require creative markup: publisher-hosted PG deals can omit it.
    let declared = |value: Option<&Value>| {
        value.and_then(Value::as_array).is_some_and(|values| {
            values
                .iter()
                .any(|v| v.as_str().is_some_and(|text| !text.is_empty()))
        })
    };
    if !declared(object.get("adomain")) && !declared(value_at(object, "ext.clickurl")) {
        issues.push(profile_issue(
            "openrtb.profile.google.landing_page_required",
            String::from("Authorized Buyers bids must declare a click URL in ext.clickurl, an advertiser domain in adomain, or both."),
            join_instance_path(path, "adomain"),
        ));
    }
    if let Some(caps) = value_at(object, "ext.fcap").and_then(Value::as_array) {
        if caps.len() > 10 {
            invalid(
                &join_instance_path(path, "ext.fcap"),
                "must not contain more than 10 frequency caps",
                issues,
            );
        }
        for (index, cap) in caps.iter().enumerate() {
            let Some(cap) = cap.as_object() else { continue };
            let cap_path = join_instance_path(path, &format!("ext.fcap[{index}]"));
            if !cap
                .get("fcap_id")
                .and_then(Value::as_str)
                .is_some_and(|id| !id.is_empty() && id.chars().count() <= 64)
            {
                invalid(
                    &join_instance_path(&cap_path, "fcap_id"),
                    "must be a non-empty frequency-cap identifier of at most 64 characters",
                    issues,
                );
            }
            if !cap
                .get("time_unit")
                .and_then(Value::as_i64)
                .is_some_and(|unit| (1..=5).contains(&unit))
            {
                invalid(
                    &join_instance_path(&cap_path, "time_unit"),
                    "must specify a supported frequency-cap time unit from 1 through 5",
                    issues,
                );
            }
            if cap.get("time_unit").and_then(Value::as_i64) != Some(5) {
                if let Some(range) = cap.get("time_range") {
                    if !range.as_i64().is_some_and(|range| range > 0) {
                        invalid(
                            &join_instance_path(&cap_path, "time_range"),
                            "must be positive unless time_unit is INDEFINITE (5)",
                            issues,
                        );
                    }
                }
            }
            if !cap
                .get("max_imp")
                .and_then(Value::as_i64)
                .is_some_and(|max| max > 0)
            {
                invalid(
                    &join_instance_path(&cap_path, "max_imp"),
                    "must be a positive frequency-cap impression limit",
                    issues,
                );
            }
        }
    }
    if let Some(dsa) = value_at(object, "ext.dsa").and_then(Value::as_object) {
        for field in ["behalf", "paid"] {
            if dsa
                .get(field)
                .and_then(Value::as_str)
                .is_some_and(|text| text.chars().count() > 100)
            {
                invalid(
                    &join_instance_path(path, &format!("ext.dsa.{field}")),
                    "must not exceed Google's 100-character DSA declaration limit",
                    issues,
                );
            }
        }
        if !dsa
            .get("paid")
            .and_then(Value::as_str)
            .is_some_and(|text| !text.is_empty())
        {
            issues.push(profile_issue("openrtb.profile.google.dsa_paid_required", String::from("Google's DSA declaration requires paid, including when the payer and advertiser are the same."), join_instance_path(path, "ext.dsa.paid")));
        }
    }
}

fn sdk_issue(id: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    let mut issue = profile_issue(id, message.to_owned(), path);
    issue.section = Some(String::from(
        if id == "openrtb.profile.google.sdk_native_field_required" {
            "https://developers.google.com/static/authorized-buyers/rtb/downloads/openrtb-proto.txt"
        } else if matches!(
            id,
            "openrtb.profile.google.sdk_params_not_offered"
                | "openrtb.profile.google.sdk_native_image_type_required"
        ) {
            "https://developers.google.com/static/authorized-buyers/rtb/downloads/openrtb-adx-proto.txt"
        } else {
            "https://developers.google.com/authorized-buyers/rtb/buyer-sdk-ads"
        },
    ));
    issues.push(issue);
}

fn validate_selected_sdk(
    bid: &Map<String, Value>,
    sdk: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let selected_path = join_instance_path(path, "ext.sdk_rendered_ad");
    if ["adm", "ext.amp_ad_url"].iter().any(|field| {
        value_at(bid, field)
            .and_then(Value::as_str)
            .is_some_and(|content| !content.is_empty())
    }) {
        sdk_issue("openrtb.profile.google.sdk_renderer_conflict", "Google filters bids that select a buyer SDK creative and a separate ordinary or AMP creative.", selected_path.clone(), issues);
    }
    for field in ["adomain", "ext.billing_id", "crid", "w", "h"] {
        let present = value_at(bid, field).is_some_and(|v| match v {
            Value::Null => false,
            Value::String(text) => !text.is_empty(),
            Value::Array(values) => values
                .iter()
                .any(|v| v.as_str().is_some_and(|v| !v.is_empty())),
            _ => true,
        });
        if !present {
            sdk_issue("openrtb.profile.google.sdk_field_required", "Google's selected buyer SDK ad contract requires this bid field, even when the general protocol marks it optional.", join_instance_path(path, field), issues);
        }
    }
    if !sdk
        .get("id")
        .is_some_and(|v| !v.is_null() && v.as_str() != Some(""))
    {
        sdk_issue(
            "openrtb.profile.google.sdk_field_required",
            "A selected Google buyer SDK ad must specify its SDK identifier.",
            join_instance_path(&selected_path, "id"),
            issues,
        );
    }
    if !sdk.get("rendering_data").is_some_and(|v| !v.is_null()) {
        sdk_issue(
            "openrtb.profile.google.sdk_field_required",
            "A selected Google buyer SDK ad must supply rendering_data. Its content is opaque.",
            join_instance_path(&selected_path, "rendering_data"),
            issues,
        );
    }
    let declared = sdk.get("declared_ad").filter(|v| !v.is_null());
    if declared.is_none() {
        sdk_issue("openrtb.profile.google.sdk_field_required", "Google requires a declared ad for selected buyer SDK creatives and filters bids without it.", join_instance_path(&selected_path, "declared_ad"), issues);
    } else if let Some(ad) = declared.and_then(Value::as_object) {
        if ![
            "html_snippet",
            "video_url",
            "video_vast_xml",
            "native_response",
        ]
        .iter()
        .any(|field| ad.get(*field).is_some_and(|v| !v.is_null()))
        {
            sdk_issue(
                "openrtb.profile.google.sdk_declared_creative_required",
                "The SDK declared ad must specify one supported creative content field.",
                join_instance_path(&selected_path, "declared_ad"),
                issues,
            );
        }
        if let Some(native) = ad.get("native_response").and_then(Value::as_object) {
            let native_path = join_instance_path(&selected_path, "declared_ad.native_response");
            sdk_native_member(native, "link", "url", &native_path, issues);
            if let Some(assets) = native.get("assets").and_then(Value::as_array) {
                for (index, asset) in assets.iter().enumerate() {
                    if let Some(asset) = asset.as_object() {
                        let asset_path =
                            join_instance_path(&native_path, &format!("assets[{index}]"));
                        for (container, field) in [
                            ("title", "text"),
                            ("img", "url"),
                            ("video", "vasttag"),
                            ("data", "value"),
                            ("link", "url"),
                        ] {
                            sdk_native_member(asset, container, field, &asset_path, issues);
                        }
                    }
                    if let Some(image) = asset.get("img").and_then(Value::as_object) {
                        if !image.get("type").is_some_and(|v| !v.is_null()) {
                            sdk_issue("openrtb.profile.google.sdk_native_image_type_required", "Declared SDK Native image assets require img.type; their asset IDs do not have to match the request.", join_instance_path(&selected_path, &format!("declared_ad.native_response.assets[{index}].img.type")), issues);
                        }
                    }
                }
            }
        }
    }
}

fn sdk_native_member(
    object: &Map<String, Value>,
    container: &str,
    field: &str,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    if let Some(member) = object.get(container).and_then(Value::as_object) {
        if !member.get(field).is_some_and(|v| !v.is_null()) {
            sdk_issue("openrtb.profile.google.sdk_native_field_required", "The declared SDK Native member requires this field in Google's published NativeResponse protocol. Asset IDs remain optional in this SDK context.", join_instance_path(path, &format!("{container}.{field}")), issues);
        }
    }
}

fn billing_ids(value: Option<&Value>) -> Option<Vec<i64>> {
    value?.as_array()?.iter().map(signed_id).collect()
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    let Some(imps) = request.get("imp").and_then(Value::as_array) else {
        return;
    };
    for (seat_index, seat) in response
        .get("seatbid")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        for (bid_index, bid) in seat
            .get("bid")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let Some(bid) = bid.as_object() else { continue };
            let Some(impid) = bid.get("impid").and_then(Value::as_str) else {
                continue;
            };
            let mut matches = imps
                .iter()
                .filter_map(Value::as_object)
                .filter(|imp| imp.get("id").and_then(Value::as_str) == Some(impid));
            let Some(imp) = matches.next() else { continue };
            if matches.next().is_some() {
                continue;
            } // Ambiguous duplicate IDs.
            let path = format!("seatbid[{seat_index}].bid[{bid_index}]");
            if value_at(request, "ext.fcap_scope").and_then(Value::as_i64) == Some(1)
                && value_at(bid, "ext.fcap")
                    .and_then(Value::as_array)
                    .is_some_and(|caps| !caps.is_empty())
            {
                issues.push(profile_issue("openrtb.profile.google.frequency_caps_unsupported", String::from("Google's request explicitly declares fcap_scope=NONE (1), so frequency-capped bids are filtered."), join_instance_path(&path, "ext.fcap")));
            }
            // Null cannot select an offered billing ID. The wire validator
            // reports its type separately from multiple-ID requiredness.
            let supplied = value_at(bid, "ext.billing_id").filter(|value| !value.is_null());
            let selected = supplied.and_then(signed_id);
            if let Some(eligible) = billing_ids(value_at(imp, "ext.billing_id")) {
                if eligible.len() > 1 && supplied.is_none() {
                    issues.push(profile_issue("openrtb.profile.google.billing_id_required", String::from("A Google bid must select ext.billing_id when its impression offers multiple billing IDs."), join_instance_path(&path, "ext.billing_id")));
                }
                if let Some(selected) = selected {
                    if !eligible.is_empty() && !eligible.contains(&selected) {
                        let mut issue = profile_issue("openrtb.profile.google.billing_id_not_offered", String::from("The selected Google billing ID is not offered by the referenced impression. With a sole ID and no active child seats Google may ignore this field; account configuration is unavailable."), join_instance_path(&path, "ext.billing_id"));
                        if eligible.len() == 1 {
                            issue.severity = Severity::Warning;
                        }
                        issues.push(issue);
                    }
                }
            }
            if let Some(dealid) = bid.get("dealid").and_then(Value::as_str) {
                let deals = value_at(imp, "pmp.deals").and_then(Value::as_array);
                let mut matching = deals
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_object)
                    .filter(|deal| deal.get("id").and_then(Value::as_str) == Some(dealid));
                if let Some(deal) = matching.next() {
                    if matching.next().is_none() {
                        if let (Some(selected), Some(eligible)) =
                            (selected, billing_ids(value_at(deal, "ext.billing_id")))
                        {
                            if !eligible.is_empty() && !eligible.contains(&selected) {
                                issues.push(profile_issue("openrtb.profile.google.deal_billing_id_not_offered", String::from("The Google deal's non-empty billing_id list does not offer the selected billing ID."), join_instance_path(&path, "ext.billing_id")));
                            }
                        }
                        if let (Some(mtype), Some(values)) = (
                            bid.get("mtype").and_then(Value::as_i64),
                            value_at(deal, "ext.creative_constraints.mtypes")
                                .and_then(Value::as_array),
                        ) {
                            let allowed: Option<Vec<i64>> =
                                values.iter().map(Value::as_i64).collect();
                            if let Some(allowed) = allowed {
                                if !allowed.is_empty()
                                    && allowed.iter().all(|v| (1..=4).contains(v))
                                    && (1..=4).contains(&mtype)
                                    && !allowed.contains(&mtype)
                                {
                                    issues.push(profile_issue("openrtb.profile.google.deal_media_not_offered", String::from("The bid's mtype is excluded by the Google deal's creative_constraints.mtypes."), join_instance_path(&path, "mtype")));
                                }
                            }
                        }
                    }
                }
            }
            if let Some(sdk) = value_at(bid, "ext.sdk_rendered_ad").and_then(Value::as_object) {
                validate_sdk_pair(request, sdk, &path, issues);
                validate_sdk_mapping_pair(imp, sdk, &path, issues);
            }
            for (field, limit) in [("wexp", "wmax"), ("hexp", "hmax")] {
                if let (Some(expanded), Some(maximum)) = (
                    value_at(bid, &format!("ext.clpad.{field}")).and_then(Value::as_i64),
                    value_at(imp, &format!("banner.ext.clpadslot.{limit}")).and_then(Value::as_i64),
                ) {
                    if expanded > maximum {
                        issues.push(profile_issue("openrtb.profile.google.collapsible_size_exceeded", format!("The expanded Google collapsible-ad {field} exceeds the impression's {limit}."), join_instance_path(&path, &format!("ext.clpad.{field}"))));
                    }
                }
            }
        }
    }
}

fn complete_sdk_params(value: &Value, allow_empty: bool) -> Option<Vec<(&str, &str)>> {
    let mut pairs = Vec::new();
    for value in value.as_array()? {
        let object = value.as_object()?;
        let key = object.get("key")?.as_str()?;
        let value = object.get("value")?.as_str()?;
        if (!allow_empty && (key.is_empty() || value.is_empty()))
            || pairs.iter().any(|(other, _)| *other == key)
        {
            return None;
        }
        pairs.push((key, value));
    }
    pairs.sort_unstable();
    Some(pairs)
}

fn validate_sdk_mapping_pair(
    imp: &Map<String, Value>,
    sdk: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(mappings) = value_at(imp, "ext.ad_unit_mapping").and_then(Value::as_array) else {
        return;
    };
    if mappings.is_empty() {
        return;
    }
    // Any incomplete mapping could contain the selected parameters. Duplicate
    // keys cannot establish an unambiguous chosen set, so comparison defers.
    let offered: Option<Vec<_>> = mappings
        .iter()
        .map(|mapping| {
            let mapping = mapping.as_object()?;
            if mapping
                .get("format")
                .is_some_and(|v| !v.as_i64().is_some_and(|format| (0..=7).contains(&format)))
            {
                return None;
            }
            let params = complete_sdk_params(mapping.get("keyvals")?, false)?;
            if params.is_empty() {
                None
            } else {
                Some(params)
            }
        })
        .collect();
    let Some(offered) = offered else {
        return;
    };
    let selected = match sdk.get("sdk_params") {
        Some(value) => complete_sdk_params(value, true),
        None => Some(Vec::new()),
    };
    if let Some(selected) = selected {
        if !selected.is_empty() && !offered.contains(&selected) {
            sdk_issue("openrtb.profile.google.sdk_params_not_offered", "The chosen SDK parameters must exactly match one complete offered ad-unit mapping, independent of pair order.", join_instance_path(path, "ext.sdk_rendered_ad.sdk_params"), issues);
        }
    }
}

fn validate_sdk_pair(
    request: &Map<String, Value>,
    sdk: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    if request.get("app").is_some_and(|value| !value.is_object())
        || request
            .get("app")
            .and_then(Value::as_object)
            .and_then(|app| app.get("ext"))
            .is_some_and(|value| !value.is_object())
    {
        return;
    }
    let installed = value_at(request, "app.ext.installed_sdk");
    let Some(values) = installed.and_then(Value::as_array) else {
        if installed.is_none() {
            issues.push(profile_issue("openrtb.profile.google.sdk_not_offered", String::from("Google SDK-rendered ads require an installed_sdk submessage in the corresponding app request."), join_instance_path(path, "ext.sdk_rendered_ad")));
        }
        return;
    };
    let ids: Option<Vec<&str>> = values
        .iter()
        .map(|v| {
            v.as_object()?
                .get("id")?
                .as_str()
                .filter(|id| !id.is_empty())
        })
        .collect();
    if let (Some(ids), Some(id)) = (ids, sdk.get("id").and_then(Value::as_str)) {
        if !ids.contains(&id) {
            issues.push(profile_issue("openrtb.profile.google.sdk_not_offered", String::from("The SDK-rendered ad's id must match an app.ext.installed_sdk.id in the Google request."), join_instance_path(path, "ext.sdk_rendered_ad.id")));
        }
    }
}
