pub(crate) struct AdcomListValueSet {
    pub name: &'static str,
    pub references: &'static [&'static str],
    pub allowed_values: &'static [i64],
    pub minimum_inclusive: Option<i64>,
}

pub(crate) fn adcom_list_value_set(description: &str) -> Option<&'static AdcomListValueSet> {
    let normalized = description.to_ascii_lowercase();
    ADCOM_LISTS.iter().find(|candidate| {
        candidate
            .references
            .iter()
            .any(|reference| normalized.contains(reference))
    })
}

pub(crate) fn adcom_list_by_name(name: &str) -> Option<&'static AdcomListValueSet> {
    ADCOM_LISTS.iter().find(|candidate| candidate.name == name)
}

const AGENT_TYPES: &[i64] = &[1, 2, 3];
const API_FRAMEWORKS: &[i64] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
const AUDIT_STATUS_CODES: &[i64] = &[1, 2, 3, 4, 5, 6];
const AUTO_REFRESH_TRIGGERS: &[i64] = &[0, 1, 2, 3];
const CATEGORY_TAXONOMIES: &[i64] = &[1, 2, 3, 4, 5, 6, 7, 8, 9];
const CLICK_TYPES: &[i64] = &[0, 1, 2, 3];
const COMPANION_TYPES: &[i64] = &[1, 2, 3];
const CONNECTION_TYPES: &[i64] = &[1, 2, 3, 4, 5, 6, 7];
const CONTENT_CONTEXTS: &[i64] = &[1, 2, 3, 4, 5, 6, 7];
const CREATIVE_ATTRIBUTES: &[i64] = &[
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
];
const CREATIVE_SUBTYPES_AUDIO_VIDEO: &[i64] =
    &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
const CREATIVE_SUBTYPES_DISPLAY: &[i64] = &[1, 2, 3, 4];
const DELIVERY_METHODS: &[i64] = &[1, 2, 3];
const DEVICE_TYPES: &[i64] = &[1, 2, 3, 4, 5, 6, 7, 8];
const DISPLAY_CONTEXT_TYPES: &[i64] = &[10, 11, 12, 13, 14, 15, 20, 21, 22, 30, 31, 32];
const DISPLAY_PLACEMENT_TYPES: &[i64] = &[1, 2, 3, 4];
const DOOH_MULTIPLIER_MEASUREMENT_SOURCE_TYPES: &[i64] = &[0, 1, 2, 3];
const DOOH_VENUE_TAXONOMIES: &[i64] = &[0, 1, 2, 3, 4, 5];
const EVENT_TRACKING_METHODS: &[i64] = &[1, 2];
const EVENT_TYPES: &[i64] = &[1, 2, 3, 4, 5];
const EXPANDABLE_DIRECTIONS: &[i64] = &[1, 2, 3, 4, 5, 6];
const FEED_TYPES: &[i64] = &[1, 2, 3, 4, 5, 6, 7];
const ID_MATCH_METHODS: &[i64] = &[0, 1, 2, 3, 4, 5];
const IP_LOCATION_SERVICES: &[i64] = &[1, 2, 3, 4, 511, 512];
const LINEARITY_MODES: &[i64] = &[1, 2];
const LOCATION_TYPES: &[i64] = &[1, 2, 3];
const MEDIA_RATINGS: &[i64] = &[1, 2, 3];
const NATIVE_DATA_ASSET_TYPES: &[i64] = &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
const NATIVE_IMAGE_ASSET_TYPES: &[i64] = &[1, 3];
const NO_BID_REASON_CODES: &[i64] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17];
const OPERATING_SYSTEMS: &[i64] = &[
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
    26, 27, 28,
];
const PLACEMENT_POSITIONS: &[i64] = &[0, 1, 2, 3, 4, 5, 6, 7];
const PLAYBACK_CESSATION_MODES: &[i64] = &[1, 2, 3];
const PLAYBACK_METHODS: &[i64] = &[1, 2, 3, 4, 5, 6, 7];
const PLCMT_SUBTYPES_VIDEO: &[i64] = &[1, 2, 3, 4];
const POD_DEDUPLICATION: &[i64] = &[1, 2, 3, 4, 5];
const POD_SEQUENCE: &[i64] = &[-1, 0, 1];
const PRODUCTION_QUALITIES: &[i64] = &[0, 1, 2, 3];
const SIZE_UNITS: &[i64] = &[1, 2, 3];
const SLOT_POSITION_IN_POD: &[i64] = &[-1, 0, 1, 2];
const START_DELAY_MODES: &[i64] = &[-2, -1, 0];
const USER_AGENT_SOURCE: &[i64] = &[0, 1, 2, 3];
const VOLUME_NORMALIZATION_MODES: &[i64] = &[0, 1, 2, 3, 4];

const ADCOM_LISTS: &[AdcomListValueSet] = &[
    AdcomListValueSet {
        name: "List: Agent Types",
        references: &["list: agent types"],
        allowed_values: AGENT_TYPES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: API Frameworks",
        references: &["list: api frameworks", "list: api framworks"],
        allowed_values: API_FRAMEWORKS,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Audit Status Codes",
        references: &["list: audit status codes"],
        allowed_values: AUDIT_STATUS_CODES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Auto Refresh Triggers",
        references: &["list: auto refresh triggers"],
        allowed_values: AUTO_REFRESH_TRIGGERS,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Category Taxonomies",
        references: &["list: category taxonomies"],
        allowed_values: CATEGORY_TAXONOMIES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Click Types",
        references: &["list: click types"],
        allowed_values: CLICK_TYPES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Companion Types",
        references: &["list: companion types"],
        allowed_values: COMPANION_TYPES,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Connection Types",
        references: &["list: connection types"],
        allowed_values: CONNECTION_TYPES,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Content Contexts",
        references: &["list: content contexts"],
        allowed_values: CONTENT_CONTEXTS,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Creative Attributes",
        references: &["list: creative attributes"],
        allowed_values: CREATIVE_ATTRIBUTES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Creative Subtypes - Audio/Video",
        references: &[
            "list: creative subtypes - audio/video",
            "list: creative substypes - audio/video",
        ],
        allowed_values: CREATIVE_SUBTYPES_AUDIO_VIDEO,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Creative Subtypes - Display",
        references: &["list: creative subtypes - display"],
        allowed_values: CREATIVE_SUBTYPES_DISPLAY,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Delivery Methods",
        references: &["list: delivery methods"],
        allowed_values: DELIVERY_METHODS,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Device Types",
        references: &["list: device types"],
        allowed_values: DEVICE_TYPES,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Display Context Types",
        references: &["list: display context types"],
        allowed_values: DISPLAY_CONTEXT_TYPES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Display Placement Types",
        references: &["list: display placement types"],
        allowed_values: DISPLAY_PLACEMENT_TYPES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: DOOH Multiplier Measurement Source Types",
        references: &[
            "list: dooh multiplier measurement source types",
            "#list-multiplier-measurement-source-types-",
        ],
        allowed_values: DOOH_MULTIPLIER_MEASUREMENT_SOURCE_TYPES,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: DOOH Venue Taxonomies",
        references: &["list: dooh venue taxonomies", "#list-venue-taxonomies-"],
        allowed_values: DOOH_VENUE_TAXONOMIES,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Event Tracking Methods",
        references: &["list: event tracking methods"],
        allowed_values: EVENT_TRACKING_METHODS,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Event Types",
        references: &["list: event types"],
        allowed_values: EVENT_TYPES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Expandable Directions",
        references: &["list: expandable directions"],
        allowed_values: EXPANDABLE_DIRECTIONS,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Feed Types",
        references: &["list: feed types"],
        allowed_values: FEED_TYPES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: ID Match Methods",
        references: &["list: id match methods"],
        allowed_values: ID_MATCH_METHODS,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: IP Location Services",
        references: &["list: ip location services"],
        allowed_values: IP_LOCATION_SERVICES,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Linearity Modes",
        references: &["list: linearity modes"],
        allowed_values: LINEARITY_MODES,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Location Types",
        references: &["list: location types"],
        allowed_values: LOCATION_TYPES,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Media Ratings",
        references: &["list: media ratings"],
        allowed_values: MEDIA_RATINGS,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Native Data Asset Types",
        references: &["list: native data asset types"],
        allowed_values: NATIVE_DATA_ASSET_TYPES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Native Image Asset Types",
        references: &["list: native image asset types"],
        allowed_values: NATIVE_IMAGE_ASSET_TYPES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: No-Bid Reason Codes",
        references: &["list: no-bid reason codes"],
        allowed_values: NO_BID_REASON_CODES,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Operating Systems",
        references: &["list: operating systems"],
        allowed_values: OPERATING_SYSTEMS,
        minimum_inclusive: Some(500),
    },
    AdcomListValueSet {
        name: "List: Placement Positions",
        references: &["list: placement positions"],
        allowed_values: PLACEMENT_POSITIONS,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Playback Cessation Modes",
        references: &["list: playback cessation modes"],
        allowed_values: PLAYBACK_CESSATION_MODES,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Playback Methods",
        references: &["list: playback methods"],
        allowed_values: PLAYBACK_METHODS,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Plcmt Subtypes - Video",
        references: &["list: plcmt subtypes - video"],
        allowed_values: PLCMT_SUBTYPES_VIDEO,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Pod Deduplication",
        references: &["list: pod deduplication"],
        allowed_values: POD_DEDUPLICATION,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Pod Sequence",
        references: &["list: pod sequence"],
        allowed_values: POD_SEQUENCE,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Production Qualities",
        references: &["list: production qualities"],
        allowed_values: PRODUCTION_QUALITIES,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Size Units",
        references: &["list: size units"],
        allowed_values: SIZE_UNITS,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Slot Position in Pod",
        references: &["list: slot position in pod"],
        allowed_values: SLOT_POSITION_IN_POD,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Start Delay Modes",
        references: &["list: start delay modes"],
        allowed_values: START_DELAY_MODES,
        minimum_inclusive: Some(1),
    },
    AdcomListValueSet {
        name: "List: User-Agent Source",
        references: &["list: user-agent source"],
        allowed_values: USER_AGENT_SOURCE,
        minimum_inclusive: None,
    },
    AdcomListValueSet {
        name: "List: Volume Normalization Modes",
        references: &["list: volume normalization modes"],
        allowed_values: VOLUME_NORMALIZATION_MODES,
        minimum_inclusive: None,
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_values_are_strictly_ascending_for_binary_search() {
        for list in ADCOM_LISTS {
            assert!(
                list.allowed_values.windows(2).all(|pair| pair[0] < pair[1]),
                "{} allowed_values must be strictly ascending",
                list.name
            );
        }
    }

    #[test]
    fn matches_api_framework_reference_with_catalog_typo() {
        let description =
            "List of supported API frameworks. Refer to List: API Framworks in AdCOM 1.0.";

        let matched = adcom_list_value_set(description).expect("API frameworks list should match");

        assert_eq!(matched.name, "List: API Frameworks");
        assert_eq!(matched.minimum_inclusive, Some(500));
    }

    #[test]
    fn start_delay_modes_allow_positive_values_via_range() {
        let matched = adcom_list_value_set(
            "Indicates the start delay in seconds. Refer to List: Start Delay Modes in AdCOM 1.0.",
        )
        .expect("start delay modes list should match");

        assert_eq!(matched.allowed_values, &[-2, -1, 0]);
        assert_eq!(matched.minimum_inclusive, Some(1));
    }

    #[test]
    fn matches_multiplier_measurement_source_url_reference() {
        let description = "The source type of the quantity measurement. Refer to the list https://github.com/InteractiveAdvertisingBureau/AdCOM/blob/master/AdCOM%20v1.0%20FINAL.md#list-multiplier-measurement-source-types-";

        let matched =
            adcom_list_value_set(description).expect("measurement source list should match");

        assert_eq!(
            matched.name,
            "List: DOOH Multiplier Measurement Source Types"
        );
        assert_eq!(matched.allowed_values, &[0, 1, 2, 3]);
    }

    #[test]
    fn matches_dooh_venue_taxonomy_url_reference() {
        let description = "The venue taxonomy in use. Refer to list https://github.com/InteractiveAdvertisingBureau/AdCOM/blob/master/AdCOM%20v1.0%20FINAL.md#list-venue-taxonomies-";

        let matched = adcom_list_value_set(description).expect("venue taxonomy list should match");

        assert_eq!(matched.name, "List: DOOH Venue Taxonomies");
        assert_eq!(matched.allowed_values, &[0, 1, 2, 3, 4, 5]);
    }
}
