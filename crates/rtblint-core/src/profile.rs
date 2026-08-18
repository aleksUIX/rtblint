//! Exchange profiles: documented protocol requirements on top of the spec.
//!
//! Orthogonal to [`Dialect`]. Dialect is how flag fields are serialised
//! (integer vs bool). A profile is the extra constraints an exchange
//! publishes. Google Authorized Buyers JSON still uses integer flags, and
//! still has to be declared, because `at: 3` is not in the spec's {1, 2} set
//! and sits below the vendor range (500+).
//!
//! Business policy (floors, blocklists, deal terms, bid adjustments) stays
//! out. Only what the exchange documents as protocol.

use serde_json::{Map, Value};

use crate::{Issue, Severity};

/// An exchange's documented protocol requirements, applied on top of the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Profile {
    /// Just the specification. The default.
    #[default]
    Spec,
    /// Google Authorized Buyers / AdX OpenRTB, as documented at
    /// <https://developers.google.com/authorized-buyers/rtb/openrtb-guide>.
    GoogleAuthorizedBuyers,
    /// Prebid Server `/openrtb2/auction`, as documented at
    /// <https://docs.prebid.org/prebid-server/endpoints/openrtb2/pbs-endpoint-auction.html>.
    PrebidServer,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spec => "spec",
            Self::GoogleAuthorizedBuyers => "google-ab",
            Self::PrebidServer => "prebid-server",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Spec => "the OpenRTB specification",
            Self::GoogleAuthorizedBuyers => "Google Authorized Buyers",
            Self::PrebidServer => "Prebid Server",
        }
    }

    /// Parses the profile id used by the CLI, the MCP tools, and the npm
    /// bindings. Accepts a few spellings for the Google and Prebid profiles.
    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "spec" | "none" | "openrtb" => Some(Self::Spec),
            "google-ab"
            | "google_ab"
            | "google"
            | "adx"
            | "google-authorized-buyers"
            | "authorized-buyers" => Some(Self::GoogleAuthorizedBuyers),
            "prebid-server" | "prebid_server" | "prebid" | "pbs" => Some(Self::PrebidServer),
            _ => None,
        }
    }

    /// Canonical ids, for error messages that list what is available.
    pub fn ids() -> &'static [&'static str] {
        &["spec", "google-ab", "prebid-server"]
    }

    /// Whether this profile documents `object.field = value` as a valid enum
    /// member the specification does not list.
    pub fn allows_enum_value(self, object_name: &str, field_name: &str, value: i64) -> bool {
        extra_enum_values(self).iter().any(|entry| {
            entry.object == object_name && entry.field == field_name && entry.value == value
        })
    }

    /// Extra required fields this profile documents, as `(object, dotted path)`
    /// pairs relative to that object (`ext.billing_id` lives on Imp).
    pub fn extra_required(self) -> &'static [RequiredField] {
        match self {
            Self::Spec | Self::PrebidServer => &[],
            Self::GoogleAuthorizedBuyers => GOOGLE_AB_REQUIRED,
        }
    }

    /// Native Ads 1.2 requires each request asset to carry an integer `id`.
    /// Prebid Server fills a missing id from the asset's array index, so the
    /// request-side required check is skipped under that profile.
    pub(crate) fn native_request_asset_id_required(self) -> bool {
        !matches!(self, Self::PrebidServer)
    }
}

impl std::fmt::Display for Profile {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// A field the exchange requires that the specification leaves optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequiredField {
    pub object: &'static str,
    pub path: &'static str,
}

struct ExtraEnum {
    object: &'static str,
    field: &'static str,
    value: i64,
}

/// Google's OpenRTB implementation adds `FIXED_PRICE = 3` to AuctionType.
/// The spec's value set is {1, 2} plus the vendor range >= 500, so 3 is
/// rejected unless this profile is declared. Documented in the Authorized
/// Buyers OpenRTB migration guide.
const GOOGLE_AB_ENUMS: &[ExtraEnum] = &[
    ExtraEnum {
        object: "BidRequest",
        field: "at",
        value: 3,
    },
    ExtraEnum {
        object: "Deal",
        field: "at",
        value: 3,
    },
];

/// `Imp.ext.billing_id` is required in Google's bid request: the eligible
/// billing IDs a winning bid may attribute the impression to.
const GOOGLE_AB_REQUIRED: &[RequiredField] = &[RequiredField {
    object: "Imp",
    path: "ext.billing_id",
}];

/// `imp.ext` keys Prebid Server does not treat as bidder codes. From
/// `openrtb_ext.IsPotentialBidder` / reserved bidder names.
const PREBID_RESERVED_IMP_EXT: &[&str] = &[
    "prebid", "data", "context", "general", "gpid", "skadn", "tid", "ae", "igs", "all",
];

const PREBID_TRACE_VALUES: &[&str] = &["verbose", "basic"];
const PREBID_BID_TYPES: &[&str] = &["banner", "video", "native", "audio"];

fn extra_enum_values(profile: Profile) -> &'static [ExtraEnum] {
    match profile {
        Profile::Spec | Profile::PrebidServer => &[],
        Profile::GoogleAuthorizedBuyers => GOOGLE_AB_ENUMS,
    }
}

/// Whether a dotted path on `object` is present and non-empty.
pub(crate) fn path_populated(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> bool {
    match value_at(object, path) {
        None => false,
        Some(value) => match value {
            serde_json::Value::Null => false,
            serde_json::Value::Array(items) => !items.is_empty(),
            serde_json::Value::String(text) => !text.is_empty(),
            _ => true,
        },
    }
}

fn value_at<'a>(object: &'a Map<String, Value>, path: &str) -> Option<&'a Value> {
    let mut parts = path.split('.');
    let first = parts.next()?;
    let mut current = object.get(first)?;
    for part in parts {
        current = current.as_object()?.get(part)?;
    }
    Some(current)
}

/// Profile checks that are not a single extra-required path: Prebid Server
/// bidder targeting, stored-request ids, forbidden `wseat`/`bseat`, and the
/// documented `ext.prebid` value sets.
pub(crate) fn push_profile_semantics(
    profile: Profile,
    object_name: &str,
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    if profile != Profile::PrebidServer {
        return;
    }
    match object_name {
        "BidRequest" => validate_prebid_bid_request(object, instance_path, issues),
        "App" => validate_prebid_app(object, instance_path, issues),
        "Bid" => validate_prebid_bid(object, instance_path, issues),
        _ => {}
    }
}

fn validate_prebid_bid_request(
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
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
            continue;
        };
        let entry_path = format!("{imp_path}.ext.prebid.storedbidresponse[{index}]");
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
    let Some(value) = value_at(object, "ext.prebid.type") else {
        return;
    };
    let allowed = value
        .as_str()
        .is_some_and(|text| PREBID_BID_TYPES.contains(&text));
    if allowed {
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

fn prefix(instance_path: &str) -> String {
    if instance_path.is_empty() {
        String::new()
    } else {
        format!("{instance_path}.")
    }
}

fn join_instance_path(base: &str, segment: &str) -> String {
    if base.is_empty() {
        return String::from(segment);
    }
    format!("{base}.{segment}")
}

fn profile_issue(id: &str, message: String, path: String) -> Issue {
    Issue {
        id: String::from(id),
        severity: Severity::Error,
        message,
        path: Some(path),
        section: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_round_trip() {
        assert_eq!(
            Profile::from_id("google-ab"),
            Some(Profile::GoogleAuthorizedBuyers)
        );
        assert_eq!(
            Profile::from_id("adx"),
            Some(Profile::GoogleAuthorizedBuyers)
        );
        assert_eq!(Profile::from_id("spec"), Some(Profile::Spec));
        assert_eq!(
            Profile::from_id("prebid-server"),
            Some(Profile::PrebidServer)
        );
        assert_eq!(Profile::from_id("prebid"), Some(Profile::PrebidServer));
        assert_eq!(Profile::from_id("pbs"), Some(Profile::PrebidServer));
        assert_eq!(Profile::from_id("magnite"), None);
        assert_eq!(Profile::default(), Profile::Spec);
        assert_eq!(Profile::ids(), &["spec", "google-ab", "prebid-server"]);
    }

    #[test]
    fn google_ab_allows_fixed_price_auction_type() {
        assert!(Profile::GoogleAuthorizedBuyers.allows_enum_value("BidRequest", "at", 3));
        assert!(Profile::GoogleAuthorizedBuyers.allows_enum_value("Deal", "at", 3));
        assert!(!Profile::Spec.allows_enum_value("BidRequest", "at", 3));
        assert!(!Profile::PrebidServer.allows_enum_value("BidRequest", "at", 3));
        assert!(!Profile::GoogleAuthorizedBuyers.allows_enum_value("BidRequest", "at", 4));
    }

    #[test]
    fn reserved_imp_ext_is_case_insensitive() {
        assert!(is_reserved_imp_ext("prebid"));
        assert!(is_reserved_imp_ext("SKAdN"));
        assert!(!is_reserved_imp_ext("appnexus"));
    }
}
