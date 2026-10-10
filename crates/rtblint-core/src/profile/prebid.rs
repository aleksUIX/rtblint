use super::{join_instance_path, path_populated, prefix, profile_issue, value_at};
use crate::{Issue, Severity};
use serde_json::{Map, Value};

/// `imp.ext` keys Prebid Server does not treat as bidder codes. From
/// `openrtb_ext.IsPotentialBidder` / reserved bidder names.
const PREBID_RESERVED_IMP_EXT: &[&str] = &[
    "prebid", "data", "context", "general", "gpid", "skadn", "tid", "ae", "igs", "all",
];

const PREBID_TRACE_VALUES: &[&str] = &["verbose", "basic"];
const PREBID_BID_TYPES: &[&str] = &["banner", "video", "native", "audio"];

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    if value_at(object, "ext.prebid").is_some_and(|value| !value.is_object()) {
        issues.push(profile_issue(
            "openrtb.profile.value_invalid",
            String::from("ext.prebid must be an object for the Prebid Server contract."),
            join_instance_path(path, "ext.prebid"),
        ));
        return;
    }
    match object_name {
        "BidRequest" => validate_prebid_bid_request(object, path, issues),
        "App" => validate_prebid_app(object, path, issues),
        "Bid" => validate_prebid_bid(object, path, issues),
        "User" => require_string_map(object, "ext.prebid.buyeruids", path, issues),
        _ => {}
    }
}

fn validate_prebid_bid_request(
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    validate_request_extension_types(object, instance_path, issues);
    validate_targeting(object, instance_path, issues);
    validate_multibid(object, instance_path, issues);
    validate_cache(object, instance_path, issues);
    validate_aliases(object, instance_path, issues);
    validate_currency(object, instance_path, issues);
    if object
        .get("cur")
        .and_then(Value::as_array)
        .is_some_and(|currencies| currencies.len() > 1)
    {
        warning(
            "openrtb.profile.prebid.currency_truncated",
            "PBS-Go uses only the first request currency and drops subsequent entries.",
            join_instance_path(instance_path, "cur"),
            issues,
        );
    }
    for field in ["wseat", "bseat"] {
        if object.contains_key(field) {
            issues.push(profile_issue(
                "openrtb.profile.field_forbidden",
                format!(
                    "BidRequest.{field} is refused by Prebid Server; impressions are offered to \
                     a bidder only when imp.ext.prebid.bidder.{{bidder}} (or the legacy \
                     imp.ext.{{bidder}}) is present."
                ),
                join_instance_path(instance_path, field),
            ));
        }
    }

    require_stored_id(object, "ext.prebid.storedrequest", instance_path, issues);
    require_stored_id(
        object,
        "ext.prebid.storedauctionresponse",
        instance_path,
        issues,
    );

    if let Some(channel) = value_at(object, "ext.prebid.channel") {
        match channel.as_object() {
            Some(fields) => {
                require_string_if_present(
                    fields,
                    "name",
                    &join_instance_path(instance_path, "ext.prebid.channel"),
                    issues,
                );
                require_string_if_present(
                    fields,
                    "version",
                    &join_instance_path(instance_path, "ext.prebid.channel"),
                    issues,
                );
                if !path_populated(fields, "name") {
                    issues.push(profile_issue(
                        "openrtb.profile.field_required",
                        String::from(
                            "ext.prebid.channel.name is required by Prebid Server when channel \
                             is present.",
                        ),
                        join_instance_path(instance_path, "ext.prebid.channel.name"),
                    ));
                }
            }
            None => issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                String::from(
                    "ext.prebid.channel must be an object (typically {\"name\": \"pbjs\", \
                     \"version\": \"...\"}).",
                ),
                join_instance_path(instance_path, "ext.prebid.channel"),
            )),
        }
    }

    if let Some(trace) = value_at(object, "ext.prebid.trace") {
        let allowed = trace
            .as_str()
            .is_some_and(|value| PREBID_TRACE_VALUES.contains(&value));
        if !allowed {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                String::from("ext.prebid.trace must be \"verbose\" or \"basic\"."),
                join_instance_path(instance_path, "ext.prebid.trace"),
            ));
        }
    }

    let request_has_stored = path_populated(object, "ext.prebid.storedrequest.id");
    let Some(imps) = object.get("imp").and_then(Value::as_array) else {
        return;
    };
    for (index, imp) in imps.iter().enumerate() {
        let Some(imp) = imp.as_object() else {
            continue;
        };
        let imp_path = format!("{}imp[{index}]", prefix(instance_path));
        validate_prebid_imp(imp, &imp_path, request_has_stored, issues);
    }
}

fn validate_prebid_imp(
    imp: &Map<String, Value>,
    imp_path: &str,
    request_has_stored: bool,
    issues: &mut Vec<Issue>,
) {
    if value_at(imp, "ext.prebid").is_some_and(|value| !value.is_object()) {
        issues.push(profile_issue(
            "openrtb.profile.value_invalid",
            String::from("imp.ext.prebid must be an object."),
            join_instance_path(imp_path, "ext.prebid"),
        ));
    }
    require_bool_if_present(imp, "ext.prebid.options.echovideoattrs", imp_path, issues);
    require_object_if_present(imp, "ext.prebid.options", imp_path, issues);
    require_string_if_present(imp, "ext.prebid.adunitcode", imp_path, issues);
    if let Some(value) = value_at(imp, "ext.prebid.is_rewarded_inventory") {
        if !matches!(value.as_i64(), Some(0 | 1)) {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                String::from("imp.ext.prebid.is_rewarded_inventory must be integer 0 or 1."),
                join_instance_path(imp_path, "ext.prebid.is_rewarded_inventory"),
            ));
        }
    }
    require_stored_id(imp, "ext.prebid.storedrequest", imp_path, issues);
    require_stored_id(imp, "ext.prebid.storedauctionresponse", imp_path, issues);
    require_stored_bid_responses(imp, imp_path, issues);

    if let Some(bidder) = value_at(imp, "ext.prebid.bidder") {
        if !bidder.is_object() {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                String::from("imp.ext.prebid.bidder must be an object keyed by bidder code."),
                join_instance_path(imp_path, "ext.prebid.bidder"),
            ));
        }
    }

    if request_has_stored
        || value_at(imp, "ext.prebid.storedrequest").is_some()
        || value_at(imp, "ext.prebid.storedauctionresponse").is_some()
        || value_at(imp, "ext.prebid.storedbidresponse").is_some()
        || imp_has_bidder_targeting(imp)
    {
        return;
    }
    issues.push(profile_issue(
        "openrtb.profile.prebid.bidder_required",
        String::from(
            "Prebid Server requires each Imp to name at least one bidder \
             (imp.ext.prebid.bidder.{bidder}), a legacy imp.ext.{bidder} object, or a stored \
             request / stored auction response id that supplies them after merge.",
        ),
        join_instance_path(imp_path, "ext"),
    ));
}

fn imp_has_bidder_targeting(imp: &Map<String, Value>) -> bool {
    if path_populated(imp, "ext.prebid.storedrequest.id")
        || path_populated(imp, "ext.prebid.storedauctionresponse.id")
    {
        return true;
    }
    if let Some(Value::Array(items)) = value_at(imp, "ext.prebid.storedbidresponse") {
        if items.iter().any(|item| {
            item.as_object()
                .is_some_and(|entry| path_populated(entry, "id") && path_populated(entry, "bidder"))
        }) {
            return true;
        }
    }
    let Some(ext) = imp.get("ext").and_then(Value::as_object) else {
        return false;
    };
    if let Some(bidder) = value_at(imp, "ext.prebid.bidder").and_then(Value::as_object) {
        if !bidder.is_empty() {
            return true;
        }
    }
    ext.iter()
        .any(|(key, value)| !is_reserved_imp_ext(key) && value.is_object())
}

fn is_reserved_imp_ext(key: &str) -> bool {
    PREBID_RESERVED_IMP_EXT
        .iter()
        .any(|reserved| reserved.eq_ignore_ascii_case(key))
}

fn require_stored_id(
    object: &Map<String, Value>,
    object_path: &str,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(value) = value_at(object, object_path) else {
        return;
    };
    if !value.is_object() {
        issues.push(profile_issue(
            "openrtb.profile.value_invalid",
            format!("{object_path} must be an object with a non-empty id."),
            join_instance_path(instance_path, object_path),
        ));
        return;
    }
    let id_path = format!("{object_path}.id");
    require_string_if_present(object, &id_path, instance_path, issues);
    if path_populated(object, &id_path) {
        return;
    }
    issues.push(profile_issue(
        "openrtb.profile.field_required",
        format!("{object_path}.id is required by Prebid Server when {object_path} is present."),
        join_instance_path(instance_path, &id_path),
    ));
}

fn require_stored_bid_responses(imp: &Map<String, Value>, imp_path: &str, issues: &mut Vec<Issue>) {
    let Some(value) = value_at(imp, "ext.prebid.storedbidresponse") else {
        return;
    };
    let Some(items) = value.as_array() else {
        issues.push(profile_issue(
            "openrtb.profile.value_invalid",
            String::from("imp.ext.prebid.storedbidresponse must be an array of objects."),
            join_instance_path(imp_path, "ext.prebid.storedbidresponse"),
        ));
        return;
    };
    for (index, item) in items.iter().enumerate() {
        let Some(entry) = item.as_object() else {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                String::from("Each stored bid response must be an object."),
                format!("{imp_path}.ext.prebid.storedbidresponse[{index}]"),
            ));
            continue;
        };
        let entry_path = format!("{imp_path}.ext.prebid.storedbidresponse[{index}]");
        for field in ["id", "bidder"] {
            require_string_if_present(entry, field, &entry_path, issues);
        }
        require_bool_if_present(entry, "replaceimpid", &entry_path, issues);
        if !path_populated(entry, "id") {
            issues.push(profile_issue(
                "openrtb.profile.field_required",
                String::from(
                    "imp.ext.prebid.storedbidresponse.id is required by Prebid Server when a \
                     stored bid response entry is present.",
                ),
                format!("{entry_path}.id"),
            ));
        }
        if !path_populated(entry, "bidder") {
            issues.push(profile_issue(
                "openrtb.profile.field_required",
                String::from(
                    "imp.ext.prebid.storedbidresponse.bidder is required by Prebid Server when a \
                     stored bid response entry is present.",
                ),
                format!("{entry_path}.bidder"),
            ));
        }
    }
}

fn validate_prebid_app(object: &Map<String, Value>, instance_path: &str, issues: &mut Vec<Issue>) {
    require_string_if_present(object, "ext.prebid.source", instance_path, issues);
    require_string_if_present(object, "ext.prebid.version", instance_path, issues);
}

fn validate_prebid_bid(object: &Map<String, Value>, instance_path: &str, issues: &mut Vec<Issue>) {
    for field in ["targetbiddercode", "bidid"] {
        require_string_if_present(
            object,
            &format!("ext.prebid.{field}"),
            instance_path,
            issues,
        );
    }
    for field in ["cache", "meta", "video", "events"] {
        require_object_if_present(
            object,
            &format!("ext.prebid.{field}"),
            instance_path,
            issues,
        );
    }
    require_string_map(object, "ext.prebid.targeting", instance_path, issues);
    require_bool_if_present(
        object,
        "ext.prebid.dealtiersatisfied",
        instance_path,
        issues,
    );
    require_integer_if_present(object, "ext.prebid.dealpriority", instance_path, issues);
    require_integer_if_present(object, "ext.prebid.video.duration", instance_path, issues);
    for field in [
        "ext.prebid.video.primary_category",
        "ext.prebid.events.win",
        "ext.prebid.events.imp",
        "ext.prebid.cache.key",
        "ext.prebid.cache.url",
        "ext.prebid.cache.bids.url",
        "ext.prebid.cache.bids.cacheId",
    ] {
        require_string_if_present(object, field, instance_path, issues);
    }
    require_object_if_present(object, "ext.prebid.cache.bids", instance_path, issues);
    let Some(value) = value_at(object, "ext.prebid.type") else {
        return;
    };
    let allowed = value
        .as_str()
        .is_some_and(|text| PREBID_BID_TYPES.contains(&text));
    if allowed {
        if let (Some(media), Some(mtype)) =
            (value.as_str(), object.get("mtype").and_then(Value::as_i64))
        {
            if (1..=4).contains(&mtype) && media_type_id(media) != Some(mtype) {
                issues.push(profile_issue(
                    "openrtb.profile.prebid.media_type_mismatch",
                    String::from("The PBS ext.prebid.type marker contradicts the bid's mtype."),
                    join_instance_path(instance_path, "ext.prebid.type"),
                ));
            }
        }
        return;
    }
    issues.push(profile_issue(
        "openrtb.profile.value_invalid",
        String::from(
            "bid.ext.prebid.type must be \"banner\", \"video\", \"native\", or \"audio\".",
        ),
        join_instance_path(instance_path, "ext.prebid.type"),
    ));
}

fn warning(id: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    let mut issue = profile_issue(id, String::from(message), path);
    issue.severity = Severity::Warning;
    issues.push(issue);
}

fn require_object_if_present(
    object: &Map<String, Value>,
    path: &str,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    require_type(
        object,
        path,
        instance_path,
        "an object",
        Value::is_object,
        issues,
    );
}

fn require_bool_if_present(
    object: &Map<String, Value>,
    path: &str,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    require_type(
        object,
        path,
        instance_path,
        "a JSON boolean",
        Value::is_boolean,
        issues,
    );
}

fn require_integer_if_present(
    object: &Map<String, Value>,
    path: &str,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    require_type(
        object,
        path,
        instance_path,
        "an integer JSON number",
        |v| v.as_i64().is_some(),
        issues,
    );
}

fn require_type(
    object: &Map<String, Value>,
    path: &str,
    instance_path: &str,
    expected: &str,
    predicate: impl Fn(&Value) -> bool,
    issues: &mut Vec<Issue>,
) {
    if value_at(object, path).is_some_and(|value| !predicate(value)) {
        issues.push(profile_issue(
            "openrtb.profile.value_invalid",
            format!("{path} must be {expected} when supplied to Prebid Server."),
            join_instance_path(instance_path, path),
        ));
    }
}

fn require_string_map(
    object: &Map<String, Value>,
    path: &str,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(value) = value_at(object, path) else {
        return;
    };
    let Some(entries) = value.as_object() else {
        issues.push(profile_issue(
            "openrtb.profile.value_invalid",
            format!("{path} must be an object of string values."),
            join_instance_path(instance_path, path),
        ));
        return;
    };
    for (key, value) in entries {
        if !value.is_string() {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                format!("{path}.{key} must be a string."),
                join_instance_path(instance_path, &format!("{path}.{key}")),
            ));
        }
    }
}

fn validate_request_extension_types(
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    for field in ["debug", "supportdeals", "returnallbidstatus"] {
        require_bool_if_present(object, &format!("ext.prebid.{field}"), path, issues);
    }
    for field in [
        "integration",
        "targeting.prefix",
        "server.externalurl",
        "server.datacenter",
    ] {
        require_string_if_present(object, &format!("ext.prebid.{field}"), path, issues);
    }
    for field in [
        "targeting",
        "cache",
        "currency",
        "data",
        "sdk",
        "server",
        "alternatebiddercodes",
        "biddercontrols",
    ] {
        require_object_if_present(object, &format!("ext.prebid.{field}"), path, issues);
    }
    require_integer_if_present(object, "ext.prebid.server.gvlid", path, issues);
    require_string_map(object, "ext.prebid.macros", path, issues);
    for field in ["adservertargeting", "bidderconfig", "schains", "nosale"] {
        require_type(
            object,
            &format!("ext.prebid.{field}"),
            path,
            "an array",
            Value::is_array,
            issues,
        );
    }
    for field in ["nosale", "data.bidders"] {
        require_string_array(object, &format!("ext.prebid.{field}"), path, issues);
    }
    if let Some(renderers) = value_at(object, "ext.prebid.sdk.renderers") {
        if let Some(renderers) = renderers.as_array() {
            for (index, renderer) in renderers.iter().enumerate() {
                let renderer_path =
                    join_instance_path(path, &format!("ext.prebid.sdk.renderers[{index}]"));
                if let Some(renderer) = renderer.as_object() {
                    require_string_if_present(renderer, "name", &renderer_path, issues);
                    require_string_if_present(renderer, "version", &renderer_path, issues);
                } else {
                    issues.push(profile_issue(
                        "openrtb.profile.value_invalid",
                        String::from("Each PBS SDK renderer must be an object."),
                        renderer_path,
                    ));
                }
            }
        } else {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                String::from("ext.prebid.sdk.renderers must be an array."),
                join_instance_path(path, "ext.prebid.sdk.renderers"),
            ));
        }
    }
}

fn require_string_array(
    object: &Map<String, Value>,
    field: &str,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(value) = value_at(object, field) else {
        return;
    };
    let Some(entries) = value.as_array() else {
        issues.push(profile_issue(
            "openrtb.profile.value_invalid",
            format!("{field} must be an array of strings."),
            join_instance_path(path, field),
        ));
        return;
    };
    for (index, entry) in entries.iter().enumerate() {
        if !entry.is_string() {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                format!("{field} entries must be strings."),
                join_instance_path(path, &format!("{field}[{index}]")),
            ));
        }
    }
}

fn validate_targeting(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    for field in [
        "includewinners",
        "includebidderkeys",
        "includeformat",
        "preferdeals",
        "alwaysincludedeals",
        "appendbiddernames",
    ] {
        require_bool_if_present(
            object,
            &format!("ext.prebid.targeting.{field}"),
            path,
            issues,
        );
    }
    for field in [
        "ext.prebid.targeting.pricegranularity",
        "ext.prebid.targeting.mediatypepricegranularity.banner",
        "ext.prebid.targeting.mediatypepricegranularity.video",
        "ext.prebid.targeting.mediatypepricegranularity.native",
    ] {
        if let Some(value) = value_at(object, field) {
            validate_price_granularity(value, &join_instance_path(path, field), issues);
        }
    }
    require_object_if_present(
        object,
        "ext.prebid.targeting.mediatypepricegranularity",
        path,
        issues,
    );
}

fn validate_price_granularity(value: &Value, path: &str, issues: &mut Vec<Issue>) {
    if value
        .as_str()
        .is_some_and(|v| ["low", "med", "medium", "high", "auto", "dense"].contains(&v))
    {
        return;
    }
    let Some(object) = value.as_object() else {
        issues.push(profile_issue("openrtb.profile.value_invalid", String::from("PBS price granularity must be a custom object or a supported legacy name (low, med, medium, high, auto, dense)."), String::from(path)));
        return;
    };
    if let Some(precision) = object.get("precision") {
        if !precision.as_i64().is_some_and(|p| (0..=15).contains(&p)) {
            issues.push(profile_issue("openrtb.profile.value_invalid", String::from("The reviewed PBS-Go implementation accepts price precision integers from 0 through 15."), join_instance_path(path, "precision")));
        }
    }
    let Some(ranges) = object.get("ranges") else {
        return;
    };
    let Some(ranges) = ranges.as_array() else {
        issues.push(profile_issue(
            "openrtb.profile.value_invalid",
            String::from("PBS price granularity ranges must be an array."),
            join_instance_path(path, "ranges"),
        ));
        return;
    };
    let mut previous = Some(0.0);
    for (index, range) in ranges.iter().enumerate() {
        let range_path = format!("{path}.ranges[{index}]");
        let Some(range) = range.as_object() else {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                String::from("Each PBS price range must be an object."),
                range_path,
            ));
            previous = None;
            continue;
        };
        let max = range.get("max").and_then(Value::as_f64);
        if max.is_none() || previous.zip(max).is_some_and(|(prior, max)| max <= prior) {
            issues.push(profile_issue("openrtb.profile.value_invalid", String::from("PBS price ranges require numeric max values in strictly increasing order, starting above zero."), join_instance_path(&range_path, "max")));
        }
        previous = max;
        if !range
            .get("increment")
            .and_then(Value::as_f64)
            .is_some_and(|increment| increment > 0.0)
        {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                String::from("PBS price range increment must be a positive number."),
                join_instance_path(&range_path, "increment"),
            ));
        }
        require_type(
            range,
            "min",
            &range_path,
            "a number",
            Value::is_number,
            issues,
        );
    }
}

fn validate_cache(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(cache) = value_at(object, "ext.prebid.cache").and_then(Value::as_object) else {
        return;
    };
    // A stored request can supply bids/vastxml to this partial object.
    if !cache.contains_key("bids")
        && !cache.contains_key("vastxml")
        && !path_populated(object, "ext.prebid.storedrequest.id")
    {
        issues.push(profile_issue(
            "openrtb.profile.prebid.cache_type_required",
            String::from("PBS cache configuration requires bids or vastxml."),
            join_instance_path(path, "ext.prebid.cache"),
        ));
    }
    for field in ["bids", "vastxml"] {
        require_object_if_present(
            cache,
            field,
            &join_instance_path(path, "ext.prebid.cache"),
            issues,
        );
        require_bool_if_present(
            cache,
            &format!("{field}.returnCreative"),
            &join_instance_path(path, "ext.prebid.cache"),
            issues,
        );
    }
}

fn validate_aliases(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    require_string_map(object, "ext.prebid.aliases", path, issues);
    let aliases = value_at(object, "ext.prebid.aliases").and_then(Value::as_object);
    if let Some(aliases) = aliases {
        for (alias, bidder) in aliases {
            if bidder.as_str() == Some(alias) {
                issues.push(profile_issue(
                    "openrtb.profile.prebid.alias_noop",
                    String::from("PBS aliases must not map a bidder code to itself."),
                    join_instance_path(path, &format!("ext.prebid.aliases.{alias}")),
                ));
            }
        }
    }
    let Some(ids) = value_at(object, "ext.prebid.aliasgvlids") else {
        return;
    };
    let Some(ids) = ids.as_object() else {
        issues.push(profile_issue(
            "openrtb.profile.value_invalid",
            String::from("PBS aliasgvlids must be an object keyed by alias."),
            join_instance_path(path, "ext.prebid.aliasgvlids"),
        ));
        return;
    };
    for (alias, id) in ids {
        let field_path = join_instance_path(path, &format!("ext.prebid.aliasgvlids.{alias}"));
        if !id.as_u64().is_some_and(|id| (1..=65535).contains(&id)) {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                String::from("PBS alias GVL IDs must be positive uint16 integers."),
                field_path.clone(),
            ));
        }
        if !path_populated(object, "ext.prebid.storedrequest.id")
            && aliases.map_or(true, |aliases| !aliases.contains_key(alias))
        {
            issues.push(profile_issue(
                "openrtb.profile.prebid.alias_not_declared",
                String::from("A PBS alias GVL ID must reference a declared request alias."),
                field_path,
            ));
        }
    }
}

fn validate_currency(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    require_bool_if_present(object, "ext.prebid.currency.usepbsrates", path, issues);
    require_object_if_present(object, "ext.prebid.currency.rates", path, issues);
    let Some(rates) = value_at(object, "ext.prebid.currency.rates").and_then(Value::as_object)
    else {
        return;
    };
    for (from, rates) in rates {
        let rate_path = join_instance_path(path, &format!("ext.prebid.currency.rates.{from}"));
        let Some(rates) = rates.as_object() else {
            issues.push(profile_issue("openrtb.profile.value_invalid", String::from("PBS currency rates require an object of numeric destination rates per source currency."), rate_path));
            continue;
        };
        for (to, rate) in rates {
            if !rate.is_number() {
                issues.push(profile_issue(
                    "openrtb.profile.value_invalid",
                    String::from("PBS custom currency conversion rates must be numbers."),
                    join_instance_path(&rate_path, to),
                ));
            }
        }
    }
}

fn validate_multibid(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(value) = value_at(object, "ext.prebid.multibid") else {
        return;
    };
    let Some(entries) = value.as_array() else {
        issues.push(profile_issue(
            "openrtb.profile.value_invalid",
            String::from("PBS multibid must be an array of selector objects."),
            join_instance_path(path, "ext.prebid.multibid"),
        ));
        return;
    };
    let mut selected = std::collections::HashSet::new();
    for (index, entry) in entries.iter().enumerate() {
        let entry_path = join_instance_path(path, &format!("ext.prebid.multibid[{index}]"));
        let Some(entry) = entry.as_object() else {
            issues.push(profile_issue(
                "openrtb.profile.value_invalid",
                String::from("Each PBS multibid entry must be an object."),
                entry_path,
            ));
            continue;
        };
        require_string_if_present(entry, "bidder", &entry_path, issues);
        require_string_array(entry, "bidders", &entry_path, issues);
        require_string_if_present(entry, "targetbiddercodeprefix", &entry_path, issues);
        if let Some(max) = entry.get("maxbids") {
            if max.as_i64().is_none() {
                issues.push(profile_issue(
                    "openrtb.profile.value_invalid",
                    String::from("PBS maxbids must be an integer."),
                    join_instance_path(&entry_path, "maxbids"),
                ));
            } else if !max.as_i64().is_some_and(|max| (1..=9).contains(&max)) {
                warning(
                    "openrtb.profile.prebid.multibid_normalized",
                    "PBS-Go clamps maxbids to the supported range 1 through 9.",
                    join_instance_path(&entry_path, "maxbids"),
                    issues,
                );
            }
        } else {
            warning(
                "openrtb.profile.prebid.multibid_normalized",
                "PBS-Go ignores multibid entries without maxbids.",
                join_instance_path(&entry_path, "maxbids"),
                issues,
            );
        }
        let bidder = entry
            .get("bidder")
            .and_then(Value::as_str)
            .filter(|v| !v.is_empty());
        let bidders = entry.get("bidders").and_then(Value::as_array);
        if bidder.is_some() && entry.contains_key("bidders") {
            warning("openrtb.profile.prebid.multibid_normalized", "PBS-Go gives bidder precedence and ignores the bidders selector when both are supplied.", join_instance_path(&entry_path, "bidders"), issues);
        }
        let names: Vec<&str> = if let Some(bidder) = bidder {
            vec![bidder]
        } else {
            bidders
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect()
        };
        if names.is_empty() {
            warning(
                "openrtb.profile.prebid.multibid_normalized",
                "PBS-Go ignores multibid entries that do not select bidders.",
                entry_path.clone(),
                issues,
            );
        }
        if bidder.is_none()
            && entry
                .get("targetbiddercodeprefix")
                .and_then(Value::as_str)
                .is_some_and(|prefix| !prefix.is_empty())
        {
            warning(
                "openrtb.profile.prebid.multibid_normalized",
                "PBS-Go ignores targetbiddercodeprefix for a grouped bidders selector.",
                join_instance_path(&entry_path, "targetbiddercodeprefix"),
                issues,
            );
        }
        for name in names {
            if !selected.insert(name) {
                warning(
                    "openrtb.profile.prebid.multibid_normalized",
                    "PBS-Go ignores repeated multibid selections for a bidder.",
                    entry_path.clone(),
                    issues,
                );
            }
        }
    }
}

fn media_type_id(media: &str) -> Option<i64> {
    match media {
        "banner" => Some(1),
        "video" => Some(2),
        "audio" => Some(3),
        "native" => Some(4),
        _ => None,
    }
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    if path_populated(request, "ext.prebid.storedrequest.id") {
        return;
    }
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
            let Some(media) = value_at(bid, "ext.prebid.type").and_then(Value::as_str) else {
                continue;
            };
            if media_type_id(media).is_none() {
                continue;
            }
            let Some(impid) = bid.get("impid").and_then(Value::as_str) else {
                continue;
            };
            let mut matching = imps
                .iter()
                .filter_map(Value::as_object)
                .filter(|imp| imp.get("id").and_then(Value::as_str) == Some(impid));
            let Some(imp) = matching.next() else { continue };
            if matching.next().is_some() || path_populated(imp, "ext.prebid.storedrequest.id") {
                continue;
            }
            // A supplied malformed media container is handled by request validation.
            if imp.contains_key(media) {
                continue;
            }
            issues.push(profile_issue("openrtb.profile.prebid.media_not_offered", format!("The PBS bid marker ext.prebid.type={media} references an impression without that media object. Stored-request media cannot be resolved locally."), format!("seatbid[{seat_index}].bid[{bid_index}].ext.prebid.type")));
        }
    }
}

fn require_string_if_present(
    object: &Map<String, Value>,
    path: &str,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(value) = value_at(object, path) else {
        return;
    };
    if value.is_string() {
        return;
    }
    issues.push(profile_issue(
        "openrtb.profile.value_invalid",
        format!("{path} must be a string when present."),
        join_instance_path(instance_path, path),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reserved_imp_ext_is_case_insensitive() {
        assert!(is_reserved_imp_ext("prebid"));
        assert!(is_reserved_imp_ext("SKAdN"));
        assert!(!is_reserved_imp_ext("appnexus"));
    }
}
