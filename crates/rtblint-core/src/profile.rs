//! Exchange profiles: documented protocol requirements on top of the spec.
//!
//! Orthogonal to [`Dialect`]. Dialect is how flag fields are serialised
//! (integer vs bool). A profile is the extra constraints an exchange
//! publishes. Google Authorized Buyers JSON still uses integer flags, and
//! still has to be declared, because `at: 3` is not in the spec's {1, 2} set
//! and sits below the vendor range (500+).
//!
//! Business policy (floors, blocklists, deal terms) stays out. Only what the
//! exchange documents as protocol.

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
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spec => "spec",
            Self::GoogleAuthorizedBuyers => "google-ab",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Spec => "the OpenRTB specification",
            Self::GoogleAuthorizedBuyers => "Google Authorized Buyers",
        }
    }

    /// Parses the profile id used by the CLI, the MCP tools, and the npm
    /// bindings. Accepts a few spellings for the Google profile, since the
    /// product has been renamed more than once.
    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "spec" | "none" | "openrtb" => Some(Self::Spec),
            "google-ab"
            | "google_ab"
            | "google"
            | "adx"
            | "google-authorized-buyers"
            | "authorized-buyers" => Some(Self::GoogleAuthorizedBuyers),
            _ => None,
        }
    }

    /// Canonical ids, for error messages that list what is available.
    pub fn ids() -> &'static [&'static str] {
        &["spec", "google-ab"]
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
            Self::Spec => &[],
            Self::GoogleAuthorizedBuyers => GOOGLE_AB_REQUIRED,
        }
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

fn extra_enum_values(profile: Profile) -> &'static [ExtraEnum] {
    match profile {
        Profile::Spec => &[],
        Profile::GoogleAuthorizedBuyers => GOOGLE_AB_ENUMS,
    }
}

/// Whether a dotted path on `object` is present and non-empty.
pub(crate) fn path_populated(
    object: &serde_json::Map<String, serde_json::Value>,
    path: &str,
) -> bool {
    let mut current = object;
    let mut parts = path.split('.');
    let Some(mut part) = parts.next() else {
        return false;
    };
    loop {
        let Some(value) = current.get(part) else {
            return false;
        };
        match parts.next() {
            None => {
                return match value {
                    serde_json::Value::Null => false,
                    serde_json::Value::Array(items) => !items.is_empty(),
                    serde_json::Value::String(text) => !text.is_empty(),
                    _ => true,
                };
            }
            Some(next) => match value.as_object() {
                Some(child) => {
                    current = child;
                    part = next;
                }
                None => return false,
            },
        }
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
        assert_eq!(Profile::from_id("magnite"), None);
        assert_eq!(Profile::default(), Profile::Spec);
    }

    #[test]
    fn google_ab_allows_fixed_price_auction_type() {
        assert!(Profile::GoogleAuthorizedBuyers.allows_enum_value("BidRequest", "at", 3));
        assert!(Profile::GoogleAuthorizedBuyers.allows_enum_value("Deal", "at", 3));
        assert!(!Profile::Spec.allows_enum_value("BidRequest", "at", 3));
        assert!(!Profile::GoogleAuthorizedBuyers.allows_enum_value("BidRequest", "at", 4));
    }
}
