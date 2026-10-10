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

mod adform;
mod applovin;
mod bidswitch;
mod commerce_grid;
pub(crate) mod compat;
mod contract;
mod digital_turbine;
mod dv360;
mod equativ;
mod google;
mod index_exchange;
mod inmobi;
mod magnite;
mod mobilefuse;
mod prebid;
mod pubmatic_openwrap;
mod sovrn;
mod triplelift;
mod unity;
mod vungle;
mod xandr;
mod yandex;

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
    /// Microsoft Monetize (Xandr) outgoing bid request to bidders, as
    /// documented at
    /// <https://learn.microsoft.com/en-us/xandr/bidders/outgoing-bid-request-to-bidders>
    /// and the OpenRTB 2.6 video context note at
    /// <https://learn.microsoft.com/en-us/xandr/supply-partners/integration-with-openrtb-2-6>.
    Xandr,
    /// Magnite DV+ Exchange API (xAPI) JSON, as documented by the public
    /// protobuf extensions at
    /// <https://github.com/MagniteEngineering/xapi-proto>.
    Magnite,
    /// Display & Video 360.
    Dv360,
    /// Index Exchange DSP.
    IndexExchange,
    /// Index Exchange supplier ingest.
    IndexExchangeSeller,
    /// Unity Exchange.
    Unity,
    /// Vungle Exchange.
    Vungle,
    /// BidSwitch buyer 5.7.
    BidSwitch,
    /// BidSwitch supplier 1.1.
    BidSwitchSupplier,
    /// InMobi DSP.
    InMobi,
    /// InMobi supplier ingest.
    InMobiSupplier,
    /// MobileFuse in-app bidding.
    MobileFuse,
    /// MobileFuse SDK bidding.
    MobileFuseSdk,
    /// AppLovin ALX DSP response.
    AppLovinAlx,
    /// Commerce Grid supplier ingest.
    CommerceGrid,
    /// Digital Turbine Exchange's published request and response contract.
    DigitalTurbine,
    /// Sovrn request.
    Sovrn,
    /// Equativ bidder contract.
    Equativ,
    /// Equativ supplier ingest.
    EquativSupplier,
    /// TripleLift supplier ingest.
    TripleLiftSupplier,
    /// Adform OpenRTB handler.
    AdformHandler,
    /// Yandex Android and iOS SDK bidding.
    YandexSdkBidding,
    /// PubMatic OpenWrap custom wire fields.
    PubMaticOpenWrap,
    /// PubMatic OpenWrap CTV ad-pod middleware.
    PubMaticOpenWrapCtv,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Spec => "spec",
            Self::PubMaticOpenWrap => "pubmatic-openwrap",
            Self::PubMaticOpenWrapCtv => "pubmatic-openwrap-ctv",
            Self::TripleLiftSupplier => "triplelift-supplier",
            Self::AdformHandler => "adform-handler",
            Self::YandexSdkBidding => "yandex-sdk-bidding",
            Self::Equativ => "equativ",
            Self::EquativSupplier => "equativ-supplier",

            Self::AppLovinAlx => "applovin-alx",
            Self::CommerceGrid => "commerce-grid",
            Self::DigitalTurbine => "digital-turbine",
            Self::Sovrn => "sovrn",

            Self::InMobi => "inmobi",
            Self::InMobiSupplier => "inmobi-supplier",
            Self::MobileFuse => "mobilefuse",
            Self::MobileFuseSdk => "mobilefuse-sdk",

            Self::GoogleAuthorizedBuyers => "google-ab",
            Self::PrebidServer => "prebid-server",
            Self::Xandr => "xandr",
            Self::Magnite => "magnite",
            Self::Dv360 => "dv360",
            Self::IndexExchange => "index-exchange",
            Self::IndexExchangeSeller => "index-exchange-seller",
            Self::Unity => "unity",
            Self::Vungle => "vungle",
            Self::BidSwitch => "bidswitch",
            Self::BidSwitchSupplier => "bidswitch-supplier",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Spec => "the OpenRTB specification",
            Self::PubMaticOpenWrap => "PubMatic OpenWrap wire contract",
            Self::PubMaticOpenWrapCtv => "PubMatic OpenWrap CTV middleware",
            Self::TripleLiftSupplier => "TripleLift supplier ingest",
            Self::AdformHandler => "Adform OpenRTB handler",
            Self::YandexSdkBidding => "Yandex SDK bidding",
            Self::Equativ => "Equativ bidder contract",
            Self::EquativSupplier => "Equativ supplier ingest",

            Self::AppLovinAlx => "AppLovin ALX DSP response",
            Self::CommerceGrid => "Commerce Grid supplier ingest",
            Self::DigitalTurbine => "Digital Turbine Exchange",
            Self::Sovrn => "Sovrn request",

            Self::InMobi => "InMobi DSP",
            Self::InMobiSupplier => "InMobi supplier ingest",
            Self::MobileFuse => "MobileFuse in-app bidding",
            Self::MobileFuseSdk => "MobileFuse SDK bidding",

            Self::GoogleAuthorizedBuyers => "Google Authorized Buyers",
            Self::PrebidServer => "Prebid Server",
            Self::Xandr => "Xandr",
            Self::Magnite => "Magnite",
            Self::Dv360 => "Display & Video 360",
            Self::IndexExchange => "Index Exchange DSP",
            Self::IndexExchangeSeller => "Index Exchange supplier ingest",
            Self::Unity => "Unity Exchange",
            Self::Vungle => "Vungle Exchange",
            Self::BidSwitch => "BidSwitch buyer 5.7",
            Self::BidSwitchSupplier => "BidSwitch supplier 1.1",
        }
    }

    /// Whether public evidence supports this profile's request contract.
    pub fn supports_request(self) -> bool {
        !matches!(self, Self::AppLovinAlx)
    }

    /// Whether public evidence supports this profile's response contract.
    pub fn supports_response(self) -> bool {
        !matches!(
            self,
            Self::IndexExchangeSeller | Self::CommerceGrid | Self::Sovrn
        )
    }

    /// Primary protocol source for this exchange contract.
    pub fn source_url(self) -> Option<&'static str> {
        match self {
            Self::Spec => None,
            Self::PubMaticOpenWrap => Some("https://github.com/PubMatic-OpenWrap/prebid-server/tree/24ff50ca1f00ca1e80ab80bcfa1c812c35f87797"),
            Self::PubMaticOpenWrapCtv => Some("https://github.com/PubMatic-OpenWrap/prebid-server/tree/24ff50ca1f00ca1e80ab80bcfa1c812c35f87797"),
            Self::TripleLiftSupplier => Some("https://support.triplelift.com/en_US/openrtb/openrtb-2x-bid-request-objects"),
            Self::AdformHandler => Some("https://www.adformhelp.com/hc/en-us/articles/10431570694033"),
            Self::YandexSdkBidding => Some("https://ads.yandex.com/helpcenter/en/support/open-bidding/open-bidding-integration-android"),
            Self::Equativ => Some("https://help.equativ.com/connect-bidder-to-equativ-ssp-supported-fields"),
            Self::EquativSupplier => Some("https://help.equativ.com/open-rtb-api-integration-bid-request-specification"),

            Self::AppLovinAlx => Some("https://support.applovin.com/en/max/demand-partners/demand-side-platforms/applovin-ortb-specification/bid-responses"),
            Self::CommerceGrid => Some("https://docs.commercegrid.criteo.com/kb/guide/en/custom-server-to-server-openrtb-Vy9QrGVwyl/Steps/2366154"),
            Self::DigitalTurbine => Some("https://docs.digitalturbine.com/dt-ads-demand/dt-exchange-openrtb-2.5-specs"),
            Self::Sovrn => Some("https://knowledge.sovrn.com/kb/sovrn-ortb-specs"),

            Self::InMobi => Some("https://support.inmobi.com/advertise/integration/ortb-specs/bid-requet-dsp"),
            Self::InMobiSupplier => Some("https://support.inmobi.com/monetize/ortb-integrations/bid-request-overview"),
            Self::MobileFuse => Some("https://docs.mobilefuse.com/docs/bid-requests"),
            Self::MobileFuseSdk => Some("https://docs.mobilefuse.com/docs/sdk-bidding"),

            Self::BidSwitch => Some("https://protocol.bidswitch.com/standards-v57/bidrequest.html"),
            Self::BidSwitchSupplier => Some("https://protocol.bidswitch.com/ssp-protocol-v11/ssp-request.html"),

            Self::GoogleAuthorizedBuyers => Some("https://developers.google.com/authorized-buyers/rtb/openrtb-guide"),
            Self::PrebidServer => Some("https://docs.prebid.org/prebid-server/endpoints/openrtb2/pbs-endpoint-auction.html"),
            Self::Xandr => Some("https://learn.microsoft.com/en-us/xandr/bidders/outgoing-bid-request-to-bidders"),
            Self::Magnite => Some("https://github.com/MagniteEngineering/xapi-proto"),
            Self::Dv360 => Some("https://developers.google.com/display-video/ortb-spec"),
            Self::IndexExchange => Some("https://kb.indexexchange.com/dsps/open-rtb/list_of_supported_openrtb_bid_request_fields_dsp.htm"),
            Self::IndexExchangeSeller => Some("https://kb.indexexchange.com/publishers/openrtb_integration/list_of_supported_openrtb_bid_request_fields_for_sellers.htm"),
            Self::Unity => Some("https://docs.unity.com/en-us/grow/programmatic/unity-exchange/bid-requests"),
            Self::Vungle => Some("https://support.vungle.com/hc/en-us/articles/360045953431-Vungle-Exchange-OpenRTB-2-5-Integration-Guide"),

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
            "xandr" | "appnexus" | "microsoft" | "microsoft-monetize" | "monetize" => {
                Some(Self::Xandr)
            }
            "magnite" | "rubicon" | "dvplus" | "dv+" | "xapi" => Some(Self::Magnite),
            "dv360" => Some(Self::Dv360),
            "index-exchange" => Some(Self::IndexExchange),
            "index-exchange-seller" => Some(Self::IndexExchangeSeller),
            "unity" => Some(Self::Unity),
            "vungle" => Some(Self::Vungle),
            "display-video-360" => Some(Self::Dv360),
            "liftoff" => Some(Self::Vungle),
            "bidswitch" => Some(Self::BidSwitch),
            "bidswitch-supplier" => Some(Self::BidSwitchSupplier),
            "inmobi" => Some(Self::InMobi),
            "inmobi-supplier" => Some(Self::InMobiSupplier),
            "mobilefuse" => Some(Self::MobileFuse),
            "mobilefuse-sdk" => Some(Self::MobileFuseSdk),
            "applovin-alx" => Some(Self::AppLovinAlx),
            "commerce-grid" => Some(Self::CommerceGrid),
            "digital-turbine" => Some(Self::DigitalTurbine),
            "sovrn" => Some(Self::Sovrn),
            "equativ" => Some(Self::Equativ),
            "equativ-supplier" => Some(Self::EquativSupplier),
            "triplelift-supplier" => Some(Self::TripleLiftSupplier),
            "adform-handler" => Some(Self::AdformHandler),
            "yandex-sdk-bidding" => Some(Self::YandexSdkBidding),
            "pubmatic-openwrap" => Some(Self::PubMaticOpenWrap),
            "pubmatic-openwrap-ctv" => Some(Self::PubMaticOpenWrapCtv),
            _ => None,
        }
    }

    /// Canonical ids, for error messages that list what is available.
    pub fn ids() -> &'static [&'static str] {
        &[
            "spec",
            "google-ab",
            "prebid-server",
            "xandr",
            "magnite",
            "dv360",
            "index-exchange",
            "index-exchange-seller",
            "unity",
            "vungle",
            "bidswitch",
            "bidswitch-supplier",
            "inmobi",
            "inmobi-supplier",
            "mobilefuse",
            "mobilefuse-sdk",
            "applovin-alx",
            "commerce-grid",
            "digital-turbine",
            "sovrn",
            "equativ",
            "equativ-supplier",
            "triplelift-supplier",
            "adform-handler",
            "yandex-sdk-bidding",
            "pubmatic-openwrap",
            "pubmatic-openwrap-ctv",
        ]
    }

    /// Whether this profile documents `object.field = value` as a valid enum
    /// member the specification does not list.
    pub fn allows_enum_value(self, object_name: &str, field_name: &str, value: i64) -> bool {
        if matches!(self, Self::Equativ | Self::EquativSupplier)
            && object_name == "Video"
            && field_name == "plcmt"
            && (5..=9).contains(&value)
        {
            return true;
        }
        if self == Self::Dv360
            && ((matches!(object_name, "BidRequest" | "Deal") && field_name == "at" && value == 3)
                || (object_name == "Video"
                    && matches!(field_name, "placement" | "plcmt" | "playbackmethod")
                    && value == 0))
        {
            return true;
        }
        if matches!(self, Self::IndexExchange | Self::IndexExchangeSeller)
            && object_name == "Deal"
            && field_name == "at"
            && (value == 3 || (self == Self::IndexExchangeSeller && value >= 500))
        {
            return true;
        }
        extra_enum_values(self).iter().any(|entry| {
            entry.object == object_name && entry.field == field_name && entry.value == value
        })
    }

    /// Extra required fields this profile documents, as `(object, dotted path)`
    /// pairs relative to that object (`ext.billing_id` lives on Imp).
    pub fn extra_required(self) -> &'static [RequiredField] {
        if self == Self::GoogleAuthorizedBuyers {
            GOOGLE_AB_REQUIRED
        } else {
            &[]
        }
    }

    /// Vendor sources that explicitly retain a community extension location.
    pub(crate) fn allows_legacy_path(self, path: &str) -> bool {
        match self {
            Self::DigitalTurbine => matches!(
                path,
                "regs.ext.gdpr"
                    | "regs.ext.us_privacy"
                    | "regs.ext.gpp"
                    | "regs.ext.gpp_sid"
                    | "user.ext.consent"
                    | "user.ext.eids"
                    | "source.ext.schain"
            ),
            Self::PubMaticOpenWrap | Self::PubMaticOpenWrapCtv => matches!(
                path,
                "regs.ext.gdpr" | "regs.ext.us_privacy" | "user.ext.consent"
            ),
            Self::TripleLiftSupplier => matches!(
                path,
                "regs.ext.gdpr" | "regs.ext.us_privacy" | "regs.ext.gpp" | "regs.ext.gpp_sid"
            ),
            Self::AdformHandler => matches!(
                path,
                "regs.ext.gdpr" | "source.ext.schain" | "user.ext.consent" | "user.ext.eids"
            ),
            Self::Equativ | Self::EquativSupplier => matches!(
                path,
                "regs.ext.gdpr" | "user.ext.consent" | "source.ext.schain" | "user.ext.eids"
            ),
            Self::CommerceGrid => matches!(
                path,
                "regs.ext.gdpr" | "regs.ext.us_privacy" | "regs.ext.gpp" | "user.ext.consent"
            ),
            Self::Sovrn => path == "regs.ext.gdpr",
            Self::InMobi | Self::InMobiSupplier => {
                matches!(path, "regs.ext.gdpr" | "user.ext.consent")
                    || (self == Self::InMobiSupplier && path == "user.ext.eids")
                    || (self == Self::InMobi && path == "source.ext.schain")
            }
            Self::MobileFuse | Self::MobileFuseSdk => matches!(
                path,
                "regs.ext.gdpr"
                    | "regs.ext.us_privacy"
                    | "regs.ext.gpp"
                    | "regs.ext.gpp_sid"
                    | "user.ext.consent"
            ),
            Self::BidSwitch => matches!(
                path,
                "regs.ext.gdpr"
                    | "regs.ext.us_privacy"
                    | "user.ext.consent"
                    | "user.ext.eids"
                    | "source.ext.schain"
                    | "site.ext.inventorypartnerdomain"
                    | "app.ext.inventorypartnerdomain"
                    | "ext.dooh"
                    | "ext.s2s_nurl"
                    | "imp.video.ext.rewarded"
                    | "imp.ext.ssai"
            ),
            Self::GoogleAuthorizedBuyers => matches!(
                path,
                "app.ext.inventorypartnerdomain"
                    | "site.ext.inventorypartnerdomain"
                    | "regs.ext.gdpr"
                    | "user.ext.consent"
            ),
            Self::Xandr => path == "app.ext.inventorypartnerdomain",
            Self::Unity => matches!(
                path,
                "regs.ext.gdpr" | "regs.ext.us_privacy" | "user.ext.consent"
            ),
            Self::Vungle => matches!(
                path,
                "regs.ext.gdpr"
                    | "regs.ext.us_privacy"
                    | "user.ext.consent"
                    | "user.ext.eids"
                    | "source.ext.schain"
            ),
            Self::Dv360 => matches!(
                path,
                "app.ext.inventorypartnerdomain"
                    | "site.ext.inventorypartnerdomain"
                    | "regs.ext.gdpr"
                    | "regs.ext.us_privacy"
                    | "user.ext.consent"
                    | "user.ext.eids"
                    | "source.ext.schain"
            ),
            Self::IndexExchange | Self::IndexExchangeSeller => matches!(
                path,
                "app.ext.inventorypartnerdomain"
                    | "site.ext.inventorypartnerdomain"
                    | "regs.ext.gdpr"
                    | "regs.ext.us_privacy"
                    | "user.ext.consent"
            ),
            _ => false,
        }
    }

    pub(crate) fn is_error_response(self, response: &Map<String, Value>) -> bool {
        self == Self::Dv360 && dv360::invalid_request_response(response)
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

fn extra_enum_values(profile: Profile) -> &'static [ExtraEnum] {
    if profile == Profile::GoogleAuthorizedBuyers {
        GOOGLE_AB_ENUMS
    } else {
        &[]
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

pub(super) fn value_at<'a>(object: &'a Map<String, Value>, path: &str) -> Option<&'a Value> {
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
/// documented `ext.prebid` value sets. Xandr checks closed `ext.appnexus`
/// value sets the catalog does not walk (ext is open).
pub(crate) fn push_profile_semantics(
    profile: Profile,
    object_name: &str,
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let start = issues.len();
    match profile {
        Profile::PubMaticOpenWrap | Profile::PubMaticOpenWrapCtv => pubmatic_openwrap::validate(
            profile == Profile::PubMaticOpenWrapCtv,
            object_name,
            object,
            instance_path,
            issues,
        ),
        Profile::TripleLiftSupplier => {
            triplelift::validate(object_name, object, instance_path, issues)
        }
        Profile::AdformHandler => adform::validate(object_name, object, instance_path, issues),
        Profile::YandexSdkBidding => yandex::validate(object_name, object, instance_path, issues),
        Profile::GoogleAuthorizedBuyers => {
            google::validate(object_name, object, instance_path, issues)
        }
        Profile::Magnite => magnite::validate(object_name, object, instance_path, issues),
        Profile::Dv360 => dv360::validate(object_name, object, instance_path, issues),
        Profile::IndexExchange => {
            index_exchange::validate(false, object_name, object, instance_path, issues)
        }
        Profile::IndexExchangeSeller => {
            index_exchange::validate(true, object_name, object, instance_path, issues)
        }
        Profile::Unity => unity::validate(object_name, object, instance_path, issues),
        Profile::Vungle => vungle::validate(object_name, object, instance_path, issues),
        Profile::BidSwitch => {
            bidswitch::validate(object_name, object, instance_path, issues, false)
        }
        Profile::BidSwitchSupplier => {
            bidswitch::validate(object_name, object, instance_path, issues, true)
        }
        Profile::InMobi => inmobi::validate(false, object_name, object, instance_path, issues),
        Profile::InMobiSupplier => {
            inmobi::validate(true, object_name, object, instance_path, issues)
        }
        Profile::MobileFuse => {
            mobilefuse::validate(false, object_name, object, instance_path, issues)
        }
        Profile::MobileFuseSdk => {
            mobilefuse::validate(true, object_name, object, instance_path, issues)
        }
        Profile::AppLovinAlx => applovin::validate(object_name, object, instance_path, issues),
        Profile::CommerceGrid => {
            commerce_grid::validate(object_name, object, instance_path, issues)
        }
        Profile::DigitalTurbine => {
            digital_turbine::validate(object_name, object, instance_path, issues)
        }
        Profile::Sovrn => sovrn::validate(object_name, object, instance_path, issues),
        Profile::Equativ => equativ::validate(object_name, object, instance_path, issues, false),
        Profile::EquativSupplier => {
            equativ::validate(object_name, object, instance_path, issues, true)
        }
        Profile::PrebidServer => prebid::validate(object_name, object, instance_path, issues),
        Profile::Xandr => xandr::validate(object_name, object, instance_path, issues),
        _ => {}
    }
    cite_profile_issues(profile, &mut issues[start..]);
}

pub(crate) fn push_profile_pair(
    profile: Profile,
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    let start = issues.len();
    match profile {
        Profile::TripleLiftSupplier => triplelift::validate_pair(request, response, issues),
        Profile::AdformHandler => adform::validate_pair(request, response, issues),
        Profile::YandexSdkBidding => yandex::validate_pair(request, response, issues),
        Profile::GoogleAuthorizedBuyers => google::validate_pair(request, response, issues),
        Profile::Dv360 => dv360::validate_pair(request, response, issues),
        Profile::IndexExchange => index_exchange::validate_pair(request, response, issues),
        Profile::Unity => unity::validate_pair(request, response, issues),
        Profile::Vungle => vungle::validate_pair(request, response, issues),
        Profile::BidSwitch => bidswitch::validate_pair(request, response, issues, false),
        Profile::BidSwitchSupplier => bidswitch::validate_pair(request, response, issues, true),
        Profile::InMobi => inmobi::validate_pair(request, response, issues),
        Profile::MobileFuse | Profile::MobileFuseSdk => {
            mobilefuse::validate_pair(request, response, issues)
        }
        Profile::AppLovinAlx => applovin::validate_pair(request, response, issues),
        Profile::DigitalTurbine => digital_turbine::validate_pair(request, response, issues),
        Profile::Equativ => equativ::validate_pair(request, response, issues, false),
        Profile::EquativSupplier => equativ::validate_pair(request, response, issues, true),
        Profile::PrebidServer => prebid::validate_pair(request, response, issues),
        Profile::Xandr => xandr::validate_pair(request, response, issues),
        Profile::Magnite => magnite::validate_pair(request, response, issues),
        _ => {}
    }
    cite_profile_issues(profile, &mut issues[start..]);
}

fn cite_profile_issues(profile: Profile, issues: &mut [Issue]) {
    for issue in issues {
        if issue.section.is_none() {
            issue.section = profile.source_url().map(String::from);
        }
    }
}

pub(super) fn require_integer_in_range(
    object: &Map<String, Value>,
    path: &str,
    min: i64,
    max: i64,
    message: &str,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(value) = value_at(object, path) else {
        return;
    };
    if value
        .as_i64()
        .is_some_and(|number| (min..=max).contains(&number))
    {
        return;
    }
    issues.push(profile_issue(
        "openrtb.profile.value_invalid",
        String::from(message),
        join_instance_path(instance_path, path),
    ));
}

pub(super) fn prefix(instance_path: &str) -> String {
    if instance_path.is_empty() {
        String::new()
    } else {
        format!("{instance_path}.")
    }
}

pub(super) fn join_instance_path(base: &str, segment: &str) -> String {
    if base.is_empty() {
        return String::from(segment);
    }
    format!("{base}.{segment}")
}

pub(super) fn profile_issue(id: &str, message: String, path: String) -> Issue {
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
        assert_eq!(Profile::from_id("xandr"), Some(Profile::Xandr));
        assert_eq!(Profile::from_id("appnexus"), Some(Profile::Xandr));
        assert_eq!(Profile::from_id("magnite"), Some(Profile::Magnite));
        assert_eq!(Profile::from_id("rubicon"), Some(Profile::Magnite));
        assert_eq!(Profile::from_id("amazon-tam"), None);
        assert_eq!(Profile::default(), Profile::Spec);
        assert_eq!(
            &Profile::ids()[..5],
            &["spec", "google-ab", "prebid-server", "xandr", "magnite"]
        );
        let mut seen = std::collections::HashSet::new();
        for id in Profile::ids() {
            assert!(seen.insert(id), "duplicate canonical profile {id}");
            let profile = Profile::from_id(id).expect("listed profile is parseable");
            assert_eq!(profile.as_str(), *id, "canonical profile round trip");
            assert!(!profile.display_name().is_empty());
            if profile != Profile::Spec {
                assert!(profile.source_url().is_some());
            }
        }
    }

    #[test]
    fn google_ab_allows_fixed_price_auction_type() {
        assert!(Profile::GoogleAuthorizedBuyers.allows_enum_value("BidRequest", "at", 3));
        assert!(Profile::GoogleAuthorizedBuyers.allows_enum_value("Deal", "at", 3));
        assert!(!Profile::Spec.allows_enum_value("BidRequest", "at", 3));
        assert!(!Profile::PrebidServer.allows_enum_value("BidRequest", "at", 3));
        assert!(!Profile::GoogleAuthorizedBuyers.allows_enum_value("BidRequest", "at", 4));
    }
}
