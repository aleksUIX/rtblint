#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenRtbFamily {
    TwoX,
    ThreeZero,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OpenRtbVersion {
    V2_0,
    V2_1,
    V2_2,
    V2_3,
    V2_3_1,
    V2_4,
    V2_5,
    V2_6_202204,
    V2_6_202210,
    V2_6_202211,
    V2_6_202303,
    V2_6_202309,
    V2_6_202402,
    V2_6_202409,
    V2_6_202501,
    V2_6_202505,
    V2_6_202606,
    V3_0,
}

impl OpenRtbVersion {
    pub const ALL: [Self; 18] = [
        Self::V2_0,
        Self::V2_1,
        Self::V2_2,
        Self::V2_3,
        Self::V2_3_1,
        Self::V2_4,
        Self::V2_5,
        Self::V2_6_202204,
        Self::V2_6_202210,
        Self::V2_6_202211,
        Self::V2_6_202303,
        Self::V2_6_202309,
        Self::V2_6_202402,
        Self::V2_6_202409,
        Self::V2_6_202501,
        Self::V2_6_202505,
        Self::V2_6_202606,
        Self::V3_0,
    ];

    pub fn all() -> &'static [Self] {
        &Self::ALL
    }

    pub fn from_id(id: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|version| version.id() == id)
    }

    pub const fn id(self) -> &'static str {
        match self {
            Self::V2_0 => "2.0",
            Self::V2_1 => "2.1",
            Self::V2_2 => "2.2",
            Self::V2_3 => "2.3",
            Self::V2_3_1 => "2.3.1",
            Self::V2_4 => "2.4",
            Self::V2_5 => "2.5",
            Self::V2_6_202204 => "2.6-202204",
            Self::V2_6_202210 => "2.6-202210",
            Self::V2_6_202211 => "2.6-202211",
            Self::V2_6_202303 => "2.6-202303",
            Self::V2_6_202309 => "2.6-202309",
            Self::V2_6_202402 => "2.6-202402",
            Self::V2_6_202409 => "2.6-202409",
            Self::V2_6_202501 => "2.6-202501",
            Self::V2_6_202505 => "2.6-202505",
            Self::V2_6_202606 => "2.6-202606",
            Self::V3_0 => "3.0",
        }
    }

    pub const fn family(self) -> OpenRtbFamily {
        match self {
            Self::V3_0 => OpenRtbFamily::ThreeZero,
            _ => OpenRtbFamily::TwoX,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VersionRuleKind {
    AddedField,
    AddedObject,
    AddedMacro,
    AddedHeader,
    AddedList,
    AddedGuidance,
    AddedBehavior,
    DeprecatedField,
    RemovedField,
    MovedField,
    CorrectedField,
    StructuralShift,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRule {
    pub code: &'static str,
    pub kind: VersionRuleKind,
    pub paths: &'static [&'static str],
    pub replacement_paths: &'static [&'static str],
    pub summary: &'static str,
    pub section: &'static str,
    pub source: &'static str,
}

impl VersionRule {
    fn role_for_path(&self, path: &str) -> Option<RulePathRole> {
        if self.paths.contains(&path) {
            return Some(RulePathRole::Primary);
        }

        if self.replacement_paths.contains(&path) {
            return Some(RulePathRole::Replacement);
        }

        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionProfile {
    pub version: OpenRtbVersion,
    pub release_date: &'static str,
    pub archive_path: &'static str,
    pub summary: &'static str,
    pub rules: &'static [VersionRule],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RulePathRole {
    Primary,
    Replacement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathRuleMatch {
    pub version: OpenRtbVersion,
    pub role: RulePathRole,
    pub rule: &'static VersionRule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathStateKind {
    Unknown,
    NotYetAvailable,
    Available,
    Deprecated,
    Removed,
    Moved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathStatus {
    pub kind: PathStateKind,
    pub since: Option<OpenRtbVersion>,
    pub replacement_paths: Vec<&'static str>,
    pub matched_rules: Vec<PathRuleMatch>,
}

impl PathStatus {
    fn unknown() -> Self {
        Self {
            kind: PathStateKind::Unknown,
            since: None,
            replacement_paths: Vec::new(),
            matched_rules: Vec::new(),
        }
    }
}

pub fn version_profiles() -> &'static [VersionProfile] {
    &VERSION_PROFILES
}

pub fn version_profile(version: OpenRtbVersion) -> Option<&'static VersionProfile> {
    VERSION_PROFILES.iter().find(|profile| profile.version == version)
}

pub fn rules_for_path(version: OpenRtbVersion, path: &str) -> Vec<PathRuleMatch> {
    VERSION_PROFILES
        .iter()
        .filter(|profile| profile.version.family() == version.family())
        .filter(|profile| profile.version <= version)
        .flat_map(|profile| {
            profile.rules.iter().filter_map(move |rule| {
                rule.role_for_path(path).map(|role| PathRuleMatch {
                    version: profile.version,
                    role,
                    rule,
                })
            })
        })
        .collect()
}

pub fn path_status(version: OpenRtbVersion, path: &str) -> PathStatus {
    let all_matches = VERSION_PROFILES
        .iter()
        .filter(|profile| profile.version.family() == version.family())
        .flat_map(|profile| {
            profile.rules.iter().filter_map(move |rule| {
                rule.role_for_path(path).map(|role| PathRuleMatch {
                    version: profile.version,
                    role,
                    rule,
                })
            })
        })
        .collect::<Vec<_>>();

    if all_matches.is_empty() {
        return PathStatus::unknown();
    }

    let first_match = &all_matches[0];
    if version < first_match.version {
        return pre_change_status(first_match);
    }

    let mut status = PathStatus::unknown();
    for matched in all_matches.into_iter().filter(|matched| matched.version <= version) {
        apply_match(&mut status, matched);
    }

    status
}

fn pre_change_status(first_match: &PathRuleMatch) -> PathStatus {
    let kind = match (first_match.role, first_match.rule.kind) {
        (
            RulePathRole::Primary,
            VersionRuleKind::DeprecatedField
                | VersionRuleKind::RemovedField
                | VersionRuleKind::MovedField
                | VersionRuleKind::CorrectedField,
        ) => PathStateKind::Available,
        _ => PathStateKind::NotYetAvailable,
    };

    PathStatus {
        kind,
        since: (kind == PathStateKind::NotYetAvailable).then_some(first_match.version),
        replacement_paths: Vec::new(),
        matched_rules: Vec::new(),
    }
}

fn apply_match(status: &mut PathStatus, matched: PathRuleMatch) {
    let kind = match matched.rule.kind {
        VersionRuleKind::AddedField
        | VersionRuleKind::AddedObject
        | VersionRuleKind::AddedMacro
        | VersionRuleKind::AddedHeader
        | VersionRuleKind::AddedList
        | VersionRuleKind::AddedGuidance
        | VersionRuleKind::AddedBehavior
        | VersionRuleKind::StructuralShift => PathStateKind::Available,
        VersionRuleKind::DeprecatedField => PathStateKind::Deprecated,
        VersionRuleKind::RemovedField => PathStateKind::Removed,
        VersionRuleKind::MovedField | VersionRuleKind::CorrectedField => match matched.role {
            RulePathRole::Primary => PathStateKind::Moved,
            RulePathRole::Replacement => PathStateKind::Available,
        },
    };

    status.kind = kind;
    status.since = Some(matched.version);
    status.replacement_paths = match kind {
        PathStateKind::Moved => matched.rule.replacement_paths.to_vec(),
        _ => Vec::new(),
    };
    status.matched_rules.push(matched);
}

const V2_0_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.0.imp.video",
        kind: VersionRuleKind::AddedObject,
        paths: &["imp.video"],
        replacement_paths: &[],
        summary: "OpenRTB 2.0 unified display, mobile, and video bidding around the shared Video object and VAST ad-unit support.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.0.device.ifa",
        kind: VersionRuleKind::AddedField,
        paths: &["device.ifa"],
        replacement_paths: &[],
        summary: "OpenRTB 2.0 added mobile device ID support for attribution and audience handling.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.0.device.geo",
        kind: VersionRuleKind::AddedBehavior,
        paths: &["device.geo"],
        replacement_paths: &[],
        summary: "OpenRTB 2.0 improved geographic data handling across mobile and desktop transactions.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.0.user.data",
        kind: VersionRuleKind::AddedField,
        paths: &["user.data"],
        replacement_paths: &[],
        summary: "OpenRTB 2.0 expanded support for third-party audience data segments.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
];

const V2_1_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.1.enum.content_category.tier2",
        kind: VersionRuleKind::AddedList,
        paths: &["enum.content_category.tier2"],
        replacement_paths: &[],
        summary: "OpenRTB 2.1 added support for IAB Tier-2 content categories.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.1.geo.type",
        kind: VersionRuleKind::AddedField,
        paths: &["geo.type"],
        replacement_paths: &[],
        summary: "OpenRTB 2.1 differentiated location-source provenance, such as GPS versus ZIP-derived targeting.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.1.imp.video.behavior",
        kind: VersionRuleKind::AddedBehavior,
        paths: &["imp.video"],
        replacement_paths: &[],
        summary: "OpenRTB 2.1 clarified VAST video support in RTB transactions, including mixed banner-or-video impressions.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
];

const V2_2_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.2.imp.secure",
        kind: VersionRuleKind::AddedField,
        paths: &["imp.secure"],
        replacement_paths: &[],
        summary: "OpenRTB 2.2 added secure versus non-secure inventory signaling.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.2.pmp.deals",
        kind: VersionRuleKind::AddedBehavior,
        paths: &["imp.pmp.deals"],
        replacement_paths: &[],
        summary: "OpenRTB 2.2 expanded private-marketplace deal handling.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.2.regs.coppa",
        kind: VersionRuleKind::AddedField,
        paths: &["regs.coppa"],
        replacement_paths: &[],
        summary: "OpenRTB 2.2 added COPPA regulation signaling.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.2.traffic.feedback.bot",
        kind: VersionRuleKind::AddedGuidance,
        paths: &["traffic.feedback.bot"],
        replacement_paths: &[],
        summary: "OpenRTB 2.2 introduced a path for real-time feedback on suspected bot traffic between buyers and sellers.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
];

const V2_3_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.3.imp.native",
        kind: VersionRuleKind::AddedObject,
        paths: &["imp.native"],
        replacement_paths: &[],
        summary: "OpenRTB 2.3 introduced native ad placements directly on the impression object.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.3.imp.native.request",
        kind: VersionRuleKind::AddedField,
        paths: &["imp.native.request"],
        replacement_paths: &[],
        summary: "OpenRTB 2.3 added the native request payload for declaring required assets and fields.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.3.native.assets",
        kind: VersionRuleKind::AddedBehavior,
        paths: &["native.assets"],
        replacement_paths: &[],
        summary: "OpenRTB 2.3 allowed supply and demand to negotiate native asset availability and requirements in the bidstream.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
];

const V2_3_1_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.3.1.user.buyeruid",
        kind: VersionRuleKind::CorrectedField,
        paths: &["user.buyuid"],
        replacement_paths: &["user.buyeruid"],
        summary: "OpenRTB 2.3.1 corrected the buyer-specific user ID attribute spelling to buyeruid.",
        section: "Section 3.2.13 typo fix",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.3.1.macro.auction_bid_id",
        kind: VersionRuleKind::AddedBehavior,
        paths: &["macro.${AUCTION_BID_ID}"],
        replacement_paths: &[],
        summary: "OpenRTB 2.3.1 corrected the ${AUCTION_BID_ID} substitution target to BidResponse.bidid.",
        section: "Section 4.4 typo fix",
        source: "IAB Tech Lab OpenRTB standards page",
    },
];

const V2_4_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.4.imp.video.skip",
        kind: VersionRuleKind::AddedField,
        paths: &["imp.video.skip"],
        replacement_paths: &[],
        summary: "OpenRTB 2.4 added video skippability support.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.4.imp.audio",
        kind: VersionRuleKind::AddedObject,
        paths: &["imp.audio"],
        replacement_paths: &[],
        summary: "OpenRTB 2.4 introduced the Audio object for audio inventory.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.4.imp.secure.ssl",
        kind: VersionRuleKind::AddedBehavior,
        paths: &["imp.secure"],
        replacement_paths: &[],
        summary: "OpenRTB 2.4 broadened SSL and secure-asset expectations for creatives.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
    VersionRule {
        code: "openrtb.2.4.geo.location",
        kind: VersionRuleKind::AddedBehavior,
        paths: &["geo"],
        replacement_paths: &[],
        summary: "OpenRTB 2.4 increased location support and geographic detail.",
        section: "Release highlights",
        source: "IAB Tech Lab OpenRTB standards page",
    },
];

const V2_5_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.5.bidrequest.source",
        kind: VersionRuleKind::AddedField,
        paths: &["bidrequest.source"],
        replacement_paths: &[],
        summary: "OpenRTB 2.5 added the Source object to model upstream decisioning and header bidding context.",
        section: "Section 3.2.1 / 3.2.2",
        source: "OpenRTB 2.6 Appendix B: Version 2.4 to 2.5",
    },
    VersionRule {
        code: "openrtb.2.5.bidrequest.bseat_wlang",
        kind: VersionRuleKind::AddedField,
        paths: &["bidrequest.bseat", "bidrequest.wlang"],
        replacement_paths: &[],
        summary: "OpenRTB 2.5 added buyer seat blocking and creative language allow-listing on the bid request.",
        section: "Section 3.2.1",
        source: "OpenRTB 2.6 Appendix B: Version 2.4 to 2.5",
    },
    VersionRule {
        code: "openrtb.2.5.imp.metric",
        kind: VersionRuleKind::AddedField,
        paths: &["imp.metric", "metric"],
        replacement_paths: &[],
        summary: "OpenRTB 2.5 introduced the Metric object for historical viewability, CTR, and similar signals.",
        section: "Section 3.2.4 / 3.2.5",
        source: "OpenRTB 2.6 Appendix B: Version 2.4 to 2.5",
    },
    VersionRule {
        code: "openrtb.2.5.imp.video.placement",
        kind: VersionRuleKind::AddedField,
        paths: &["imp.video.placement", "imp.video.playbackend"],
        replacement_paths: &[],
        summary: "OpenRTB 2.5 added video placement-type and playback-cessation signaling.",
        section: "Section 3.2.7",
        source: "OpenRTB 2.6 Appendix B: Version 2.4 to 2.5",
    },
    VersionRule {
        code: "openrtb.2.5.bid.response_urls",
        kind: VersionRuleKind::AddedField,
        paths: &[
            "bid.burl",
            "bid.lurl",
            "bid.tactic",
            "bid.language",
            "bid.wratio",
            "bid.hratio",
        ],
        replacement_paths: &[],
        summary: "OpenRTB 2.5 expanded bid response notice URLs and creative metadata fields.",
        section: "Section 4.2.3",
        source: "OpenRTB 2.6 Appendix B: Version 2.4 to 2.5",
    },
    VersionRule {
        code: "openrtb.2.5.macro.loss",
        kind: VersionRuleKind::AddedMacro,
        paths: &["macro.${AUCTION_MBR}", "macro.${AUCTION_LOSS}"],
        replacement_paths: &[],
        summary: "OpenRTB 2.5 added auction minimum-bid and loss-notice macros.",
        section: "Section 4.4",
        source: "OpenRTB 2.6 Appendix B: Version 2.4 to 2.5",
    },
];

const V2_6_202204_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.6.language.bcp47",
        kind: VersionRuleKind::AddedBehavior,
        paths: &["language.bcp47"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 added BCP 47-oriented language signaling, including wlangb-style fields for more precise locale handling.",
        section: "Sections 3.2.1, 3.2.16, 3.2.18, 4.2.3",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.regs.gdpr",
        kind: VersionRuleKind::MovedField,
        paths: &["regs.ext.gdpr"],
        replacement_paths: &["regs.gdpr"],
        summary: "OpenRTB 2.6 moved GDPR signaling out of regs.ext into the core regs object.",
        section: "Section 3.2.3",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.user.consent",
        kind: VersionRuleKind::MovedField,
        paths: &["user.ext.consent"],
        replacement_paths: &["user.consent"],
        summary: "OpenRTB 2.6 moved the TCF consent string from user.ext into the core user object.",
        section: "Section 3.2.20",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.cattax",
        kind: VersionRuleKind::AddedField,
        paths: &[
            "bidrequest.cattax",
            "site.cattax",
            "app.cattax",
            "publisher.cattax",
            "content.cattax",
            "producer.cattax",
            "bid.cattax",
        ],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 standardized taxonomy selection with cattax across request and bid objects.",
        section: "Sections 3.2.1, 3.2.13, 3.2.14, 3.2.15, 3.2.16, 3.2.17, 4.2.3",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.imp.video_audio.rqddurs",
        kind: VersionRuleKind::AddedField,
        paths: &["imp.video.rqddurs", "imp.audio.rqddurs"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 added exact-duration requirements for video and audio creatives.",
        section: "Sections 3.2.7, 3.2.8",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.imp.video_audio.pod_bidding",
        kind: VersionRuleKind::AddedField,
        paths: &[
            "imp.video.maxseq",
            "imp.video.poddur",
            "imp.video.podid",
            "imp.video.podseq",
            "imp.video.mincpmpersec",
            "imp.video.slotinpod",
            "imp.audio.maxseq",
            "imp.audio.poddur",
            "imp.audio.podid",
            "imp.audio.podseq",
            "imp.audio.mincpmpersec",
            "imp.audio.slotinpod",
        ],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 added pod-bidding controls for video and audio inventory.",
        section: "Sections 3.2.7, 3.2.8",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.macro.min_to_win",
        kind: VersionRuleKind::AddedMacro,
        paths: &["macro.${AUCTION_MIN_TO_WIN}"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 added the AUCTION_MIN_TO_WIN substitution macro.",
        section: "Sections 4.4, 4.4.1",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.bid.apis",
        kind: VersionRuleKind::AddedField,
        paths: &["bid.apis"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 added the multi-valued bid.apis field.",
        section: "Section 4.2.3",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.imp.rwdd_ssai",
        kind: VersionRuleKind::AddedField,
        paths: &["imp.rwdd", "imp.ssai"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 added rewarded-inventory and SSAI signaling on impressions.",
        section: "Section 3.2.4",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.network_channel_supplychain_eids",
        kind: VersionRuleKind::AddedObject,
        paths: &[
            "network",
            "channel",
            "source.schain",
            "source.schain.nodes",
            "user.eids",
            "eid.uids",
        ],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 added network, channel, supply-chain, and extended-ID object families.",
        section: "Sections 3.2.23, 3.2.24, 3.2.25, 3.2.26, 3.2.27, 3.2.28",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.bid.mtype",
        kind: VersionRuleKind::AddedField,
        paths: &["bid.mtype"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 added bid-level markup-type signaling.",
        section: "Section 4.2.3",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.removed_deprecated_fields",
        kind: VersionRuleKind::RemovedField,
        paths: &[
            "banner.wmax",
            "banner.hmax",
            "banner.wmin",
            "banner.hmin",
            "video.protocol",
            "content.videoquality",
        ],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 removed fields that had already been deprecated in earlier revisions.",
        section: "Sections 3.2.6, 3.2.7, 3.2.16",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.deprecated_fields",
        kind: VersionRuleKind::DeprecatedField,
        paths: &[
            "imp.video.sequence",
            "imp.audio.sequence",
            "device.didsha1",
            "device.didmd5",
            "device.dpidsha1",
            "device.dpidmd5",
            "device.macsha1",
            "device.macmd5",
            "user.yob",
            "user.gender",
            "bid.api",
        ],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 deprecated legacy sequence, device-ID hash, demographic, and single-api fields.",
        section: "Sections 3.2.7, 3.2.8, 3.2.18, 3.2.20, 4.2.3",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
    VersionRule {
        code: "openrtb.2.6.user_agent_brandversion",
        kind: VersionRuleKind::AddedObject,
        paths: &["device.sua", "brandversion"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6 added structured user-agent and brand-version objects.",
        section: "Sections 3.2.29, 3.2.30",
        source: "OpenRTB 2.6 Appendix B: Version 2.5 to 2.6",
    },
];

const V2_6_202210_RULES: &[VersionRule] = &[];

const V2_6_202211_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.6-202211.regs.gpp",
        kind: VersionRuleKind::AddedField,
        paths: &["regs.gpp", "regs.gpp_sid"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202211 added GPP consent signaling to the regs object.",
        section: "Section 3.2.3",
        source: "OpenRTB 2.6 Appendix B: Version 2.6-202210 to 2.6-202211",
    },
    VersionRule {
        code: "openrtb.2.6-202211.inventorypartnerdomain",
        kind: VersionRuleKind::MovedField,
        paths: &["site.ext.inventorypartnerdomain", "app.ext.inventorypartnerdomain"],
        replacement_paths: &["site.inventorypartnerdomain", "app.inventorypartnerdomain"],
        summary: "OpenRTB 2.6-202211 moved inventorypartnerdomain from ext into the Site and App objects.",
        section: "Sections 3.2.13, 3.2.14",
        source: "OpenRTB 2.6 Appendix B: Version 2.6-202210 to 2.6-202211",
    },
    VersionRule {
        code: "openrtb.2.6-202211.imp_qty_bidrequest_dooh",
        kind: VersionRuleKind::AddedObject,
        paths: &["imp.qty", "bidrequest.dooh"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202211 added Qty and DOOH support for digital out-of-home inventory.",
        section: "Sections 3.2.31, 3.2.32",
        source: "OpenRTB 2.6 Appendix B: Version 2.6-202210 to 2.6-202211",
    },
];

const V2_6_202303_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.6-202303.imp.refresh",
        kind: VersionRuleKind::AddedObject,
        paths: &["imp.refresh", "refresh.refsettings"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202303 added refresh signaling and refsettings guidance.",
        section: "Sections 3.2.33, 3.2.34",
        source: "GitHub release notes: 2.6-202303",
    },
    VersionRule {
        code: "openrtb.2.6-202303.macro.auction_imp_ts",
        kind: VersionRuleKind::AddedMacro,
        paths: &["macro.${AUCTION_IMP_TS}"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202303 added an impression-fulfillment timestamp substitution macro.",
        section: "Section 4.4",
        source: "GitHub release notes: 2.6-202303",
    },
    VersionRule {
        code: "openrtb.2.6-202303.imp.video.plcmt",
        kind: VersionRuleKind::AddedField,
        paths: &["imp.video.plcmt"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202303 added plcmt for modern video placement semantics.",
        section: "Section 3.2.7",
        source: "GitHub release notes: 2.6-202303",
    },
    VersionRule {
        code: "openrtb.2.6-202303.imp.video.placement",
        kind: VersionRuleKind::DeprecatedField,
        paths: &["imp.video.placement"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202303 deprecated the legacy placement field in favor of plcmt.",
        section: "Section 3.2.7",
        source: "GitHub release notes: 2.6-202303",
    },
];

const V2_6_202309_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.6-202309.bidrequest.acat",
        kind: VersionRuleKind::AddedField,
        paths: &["bidrequest.acat"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202309 added allowed advertiser categories on the bid request.",
        section: "Section 3.2.1",
        source: "GitHub release notes: 2.6-202309",
    },
    VersionRule {
        code: "openrtb.2.6-202309.durfloors",
        kind: VersionRuleKind::AddedObject,
        paths: &["imp.video.durfloors", "imp.audio.durfloors", "deal.durfloors"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202309 added duration-aware floor objects for video, audio, and deals.",
        section: "Sections 3.2.7, 3.2.8, 3.2.12",
        source: "GitHub release notes: 2.6-202309",
    },
    VersionRule {
        code: "openrtb.2.6-202309.deal.floor_fields",
        kind: VersionRuleKind::AddedField,
        paths: &["deal.guaranteed", "deal.mincpmpersec"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202309 added guaranteed and CPM-per-second deal fields.",
        section: "Section 3.2.12",
        source: "GitHub release notes: 2.6-202309",
    },
];

const V2_6_202402_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.6-202402.imp.video.poddedupe",
        kind: VersionRuleKind::AddedField,
        paths: &["imp.video.poddedupe"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202402 added pod deduplication signaling for video pods.",
        section: "Section 3.2.7",
        source: "GitHub release notes: 2.6-202402",
    },
];

const V2_6_202409_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.6-202409.eid.matching",
        kind: VersionRuleKind::AddedField,
        paths: &["eid.inserter", "eid.matcher", "eid.mm"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202409 added EID insertion, matcher, and match-method fields.",
        section: "Section 3.2.27",
        source: "GitHub release notes: 2.6-202409",
    },
    VersionRule {
        code: "openrtb.2.6-202409.cookie_sync_guidance",
        kind: VersionRuleKind::AddedGuidance,
        paths: &["appendix.cookie_sync"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202409 added cookie-based ID syncing guidance to the specification.",
        section: "Appendix C",
        source: "GitHub release notes: 2.6-202409",
    },
];

const V2_6_202501_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.6-202501.content.gtax_genres",
        kind: VersionRuleKind::AddedField,
        paths: &["content.gtax", "content.genres"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202501 added genre taxonomy and genres fields on content.",
        section: "Section 3.2.16",
        source: "GitHub release notes: 2.6-202501",
    },
];

const V2_6_202505_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.6-202505.data.cids",
        kind: VersionRuleKind::AddedField,
        paths: &["data.cids"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202505 added extended content identifiers on Data objects.",
        section: "Section 3.2.21",
        source: "GitHub release notes: 2.6-202505",
    },
];

const V2_6_202606_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.2.6-202606.content.liveness",
        kind: VersionRuleKind::AddedField,
        paths: &["content.realtime", "content.firstbroadcast"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202606 added realtime and firstbroadcast attributes for signaling the liveness of programming on the Content object.",
        section: "Section 3.2.16",
        source: "GitHub release notes: 2.6-202606",
    },
    VersionRule {
        code: "openrtb.2.6-202606.macro.auction_discount",
        kind: VersionRuleKind::AddedMacro,
        paths: &["macro.${AUCTION_DISCOUNT_PCT}", "macro.${AUCTION_DISCOUNT_CPM}"],
        replacement_paths: &[],
        summary: "OpenRTB 2.6-202606 added seller discount substitution macros and revised the ${AUCTION_PRICE} definition.",
        section: "Section 4.4",
        source: "GitHub release notes: 2.6-202606",
    },
];

const V3_0_RULES: &[VersionRule] = &[
    VersionRule {
        code: "openrtb.3.0.openrtb_root",
        kind: VersionRuleKind::StructuralShift,
        paths: &["openrtb"],
        replacement_paths: &[],
        summary: "OpenRTB 3.0 introduced the Openrtb root object as the container for transport and payload metadata.",
        section: "Object Model",
        source: "OpenRTB 3.0 final specification",
    },
    VersionRule {
        code: "openrtb.3.0.request_response",
        kind: VersionRuleKind::StructuralShift,
        paths: &["request", "response"],
        replacement_paths: &[],
        summary: "OpenRTB 3.0 split the transaction into Request and Response payload objects under the layered architecture.",
        section: "Bid Request Payload / Bid Response Payload",
        source: "OpenRTB 3.0 final specification",
    },
    VersionRule {
        code: "openrtb.3.0.item",
        kind: VersionRuleKind::StructuralShift,
        paths: &["request.item"],
        replacement_paths: &[],
        summary: "OpenRTB 3.0 replaced the 2.x impression-centric payload shape with Item objects inside the request.",
        section: "Object: Item",
        source: "OpenRTB 3.0 final specification",
    },
    VersionRule {
        code: "openrtb.3.0.macro_object",
        kind: VersionRuleKind::AddedObject,
        paths: &["response.macro"],
        replacement_paths: &[],
        summary: "OpenRTB 3.0 introduced a Macro object within the response model.",
        section: "Object: Macro",
        source: "OpenRTB 3.0 final specification",
    },
    VersionRule {
        code: "openrtb.3.0.events",
        kind: VersionRuleKind::AddedObject,
        paths: &["event.pending", "event.billing", "event.loss"],
        replacement_paths: &[],
        summary: "OpenRTB 3.0 formalized pending, billing, and loss event notifications.",
        section: "Event Notification",
        source: "OpenRTB 3.0 final specification",
    },
    VersionRule {
        code: "openrtb.3.0.ads_cert",
        kind: VersionRuleKind::AddedBehavior,
        paths: &["inventory_authentication.ads.cert"],
        replacement_paths: &[],
        summary: "OpenRTB 3.0 added signed bid request and inventory-authentication guidance via ads.cert.",
        section: "Inventory Authentication",
        source: "OpenRTB 3.0 final specification",
    },
    VersionRule {
        code: "openrtb.3.0.version_header",
        kind: VersionRuleKind::AddedHeader,
        paths: &["header.x-openrtb-version"],
        replacement_paths: &[],
        summary: "OpenRTB 3.0 retained explicit version headers as part of the transport layer.",
        section: "Layer-1: Transport / Version Headers",
        source: "OpenRTB 3.0 final specification",
    },
];

const VERSION_PROFILES: [VersionProfile; 18] = [
    VersionProfile {
        version: OpenRtbVersion::V2_0,
        release_date: "2012-01",
        archive_path: ".openrtb-specs/2.x/openrtb-2.0-final.pdf",
        summary: "Unified display, mobile, and video protocol baseline.",
        rules: V2_0_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_1,
        release_date: "2012-10",
        archive_path: ".openrtb-specs/2.x/openrtb-2.1-final.pdf",
        summary: "Tablet, VAST video, and location-source improvements.",
        rules: V2_1_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_2,
        release_date: "2014-04",
        archive_path: ".openrtb-specs/2.x/openrtb-2.2-final.pdf",
        summary: "Secure inventory, PMP, and COPPA-era expansion.",
        rules: V2_2_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_3,
        release_date: "2015-01",
        archive_path: ".openrtb-specs/2.x/openrtb-2.3-final.pdf",
        summary: "Native ads arrive in the bidstream.",
        rules: V2_3_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_3_1,
        release_date: "2015-06",
        archive_path: ".openrtb-specs/2.x/openrtb-2.3.1-final.pdf",
        summary: "Targeted typo and macro-correction release.",
        rules: V2_3_1_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_4,
        release_date: "2016-03",
        archive_path: ".openrtb-specs/2.x/openrtb-2.4-final.pdf",
        summary: "Skippable video, SSL, audio, and location expansion.",
        rules: V2_4_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_5,
        release_date: "2016-12",
        archive_path: ".openrtb-specs/2.x/openrtb-2.5-final.pdf",
        summary: "Header-bidding and metric-aware 2.x baseline.",
        rules: V2_5_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_6_202204,
        release_date: "2022-04",
        archive_path: ".openrtb-specs/2.x/openrtb-2.6-202204-final.pdf",
        summary: "CTV-focused 2.6 baseline with AdCOM-backed enums and pod bidding.",
        rules: V2_6_202204_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_6_202210,
        release_date: "2022-10",
        archive_path: ".openrtb-specs/2.x/openrtb-2.6-202210.md",
        summary: "First repo-tagged 2.6 baseline with no substantive delta from April 2022.",
        rules: V2_6_202210_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_6_202211,
        release_date: "2022-11",
        archive_path: ".openrtb-specs/2.x/openrtb-2.6-202211.md",
        summary: "GPP, DOOH, and inventorypartnerdomain promotion into core objects.",
        rules: V2_6_202211_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_6_202303,
        release_date: "2023-04",
        archive_path: ".openrtb-specs/2.x/openrtb-2.6-202303.md",
        summary: "Refresh modeling and plcmt-based video definitions.",
        rules: V2_6_202303_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_6_202309,
        release_date: "2023-09",
        archive_path: ".openrtb-specs/2.x/openrtb-2.6-202309.md",
        summary: "Allowed advertiser categories and duration-aware pricing controls.",
        rules: V2_6_202309_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_6_202402,
        release_date: "2024-02",
        archive_path: ".openrtb-specs/2.x/openrtb-2.6-202402.md",
        summary: "Pod dedupe update.",
        rules: V2_6_202402_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_6_202409,
        release_date: "2024-09",
        archive_path: ".openrtb-specs/2.x/openrtb-2.6-202409.md",
        summary: "Extended-ID provenance and cookie-sync guidance.",
        rules: V2_6_202409_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_6_202501,
        release_date: "2025-01",
        archive_path: ".openrtb-specs/2.x/openrtb-2.6-202501.md",
        summary: "Genres and genre-taxonomy signaling for content.",
        rules: V2_6_202501_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_6_202505,
        release_date: "2025-05",
        archive_path: ".openrtb-specs/2.x/openrtb-2.6-202505.md",
        summary: "Extended content IDs on Data objects.",
        rules: V2_6_202505_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V2_6_202606,
        release_date: "2026-06",
        archive_path: ".openrtb-specs/2.x/openrtb-2.6-202606.md",
        summary: "Programming liveness signaling and seller discount macros.",
        rules: V2_6_202606_RULES,
    },
    VersionProfile {
        version: OpenRtbVersion::V3_0,
        release_date: "2022-03",
        archive_path: ".openrtb-specs/3.0/openrtb-3.0-final.md",
        summary: "Layered OpenMedia transaction model with request/item/response payloads.",
        rules: V3_0_RULES,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_known_version_has_a_profile() {
        let profile_versions = version_profiles()
            .iter()
            .map(|profile| profile.version)
            .collect::<Vec<_>>();

        assert_eq!(profile_versions, OpenRtbVersion::all().to_vec());
    }

    #[test]
    fn source_is_not_available_before_2_5() {
        let pre_25 = path_status(OpenRtbVersion::V2_4, "bidrequest.source");
        let at_25 = path_status(OpenRtbVersion::V2_5, "bidrequest.source");

        assert_eq!(pre_25.kind, PathStateKind::NotYetAvailable);
        assert_eq!(pre_25.since, Some(OpenRtbVersion::V2_5));
        assert_eq!(at_25.kind, PathStateKind::Available);
        assert_eq!(at_25.since, Some(OpenRtbVersion::V2_5));
    }

    #[test]
    fn regs_gpp_starts_in_2_6_202211() {
        let before = path_status(OpenRtbVersion::V2_6_202210, "regs.gpp");
        let after = path_status(OpenRtbVersion::V2_6_202211, "regs.gpp");

        assert_eq!(before.kind, PathStateKind::NotYetAvailable);
        assert_eq!(before.since, Some(OpenRtbVersion::V2_6_202211));
        assert_eq!(after.kind, PathStateKind::Available);
        assert_eq!(after.since, Some(OpenRtbVersion::V2_6_202211));
    }

    #[test]
    fn legacy_video_placement_becomes_deprecated_when_plcmt_arrives() {
        let at_25 = path_status(OpenRtbVersion::V2_5, "imp.video.placement");
        let at_202303 = path_status(OpenRtbVersion::V2_6_202303, "imp.video.placement");

        assert_eq!(at_25.kind, PathStateKind::Available);
        assert_eq!(at_202303.kind, PathStateKind::Deprecated);
        assert_eq!(at_202303.since, Some(OpenRtbVersion::V2_6_202303));
        assert_eq!(at_202303.matched_rules.len(), 2);
    }

    #[test]
    fn moved_fields_report_their_replacement_paths() {
        let before_move = path_status(OpenRtbVersion::V2_5, "regs.ext.gdpr");
        let after_move = path_status(OpenRtbVersion::V2_6_202204, "regs.ext.gdpr");
        let replacement = path_status(OpenRtbVersion::V2_6_202204, "regs.gdpr");

        assert_eq!(before_move.kind, PathStateKind::Available);
        assert_eq!(after_move.kind, PathStateKind::Moved);
        assert_eq!(after_move.replacement_paths, vec!["regs.gdpr"]);
        assert_eq!(replacement.kind, PathStateKind::Available);
    }

    #[test]
    fn item_is_only_known_in_the_3_0_family() {
        let two_x = path_status(OpenRtbVersion::V2_6_202505, "request.item");
        let three_zero = path_status(OpenRtbVersion::V3_0, "request.item");

        assert_eq!(two_x.kind, PathStateKind::Unknown);
        assert_eq!(three_zero.kind, PathStateKind::Available);
        assert_eq!(three_zero.since, Some(OpenRtbVersion::V3_0));
    }
}