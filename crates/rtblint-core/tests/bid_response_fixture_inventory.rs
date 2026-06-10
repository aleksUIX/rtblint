use serde_json::Value;

struct ResponseFixtureCase {
    name: &'static str,
    version: &'static str,
    family: ResponseFixtureFamily,
    input: &'static str,
}

#[derive(Clone, Copy)]
enum ResponseFixtureFamily {
    TwoX,
    ThreeZero,
}

const RESPONSE_FIXTURES: &[ResponseFixtureCase] = &[
    ResponseFixtureCase {
        name: "valid-openrtb-2.5-win-notice",
        version: "2.5",
        family: ResponseFixtureFamily::TwoX,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.5-win-notice.json"),
    },
    ResponseFixtureCase {
        name: "valid-openrtb-2.6-202204-apis-markup",
        version: "2.6-202204",
        family: ResponseFixtureFamily::TwoX,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202204-apis-markup.json"),
    },
    ResponseFixtureCase {
        name: "valid-openrtb-2.6-202211-multi-seat",
        version: "2.6-202211",
        family: ResponseFixtureFamily::TwoX,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202211-multi-seat.json"),
    },
    ResponseFixtureCase {
        name: "valid-openrtb-2.6-202309-pod-package",
        version: "2.6-202309",
        family: ResponseFixtureFamily::TwoX,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202309-pod-package.json"),
    },
    ResponseFixtureCase {
        name: "valid-minimal-2.6-202505",
        version: "2.6-202505",
        family: ResponseFixtureFamily::TwoX,
        input: include_str!("fixtures/bid-responses/2.6-202505/valid-minimal.json"),
    },
    ResponseFixtureCase {
        name: "invalid-empty-seatbid-2.6-202505",
        version: "2.6-202505",
        family: ResponseFixtureFamily::TwoX,
        input: include_str!("fixtures/bid-responses/2.6-202505/invalid-empty-seatbid.json"),
    },
    ResponseFixtureCase {
        name: "valid-openrtb-2.6-202505-no-bid",
        version: "2.6-202505",
        family: ResponseFixtureFamily::TwoX,
        input: include_str!("fixtures/bid-responses/valid-openrtb-2.6-202505-no-bid.json"),
    },
    ResponseFixtureCase {
        name: "valid-openrtb-3.0-layered-response",
        version: "3.0",
        family: ResponseFixtureFamily::ThreeZero,
        input: include_str!("fixtures/bid-responses/valid-openrtb-3.0-layered-response.json"),
    },
];

#[test]
fn bid_response_fixtures_are_parseable_json_objects() {
    for fixture in RESPONSE_FIXTURES {
        let value: Value = serde_json::from_str(fixture.input).unwrap_or_else(|error| {
            panic!(
                "response fixture {} for {} is not valid JSON: {}",
                fixture.name, fixture.version, error
            )
        });

        let object = value.as_object().unwrap_or_else(|| {
            panic!(
                "response fixture {} for {} must be a JSON object",
                fixture.name, fixture.version
            )
        });

        match fixture.family {
            ResponseFixtureFamily::TwoX => assert_two_x_response_shape(fixture, object),
            ResponseFixtureFamily::ThreeZero => assert_three_zero_response_shape(fixture, object),
        }
    }
}

fn assert_two_x_response_shape(
    fixture: &ResponseFixtureCase,
    object: &serde_json::Map<String, Value>,
) {
    assert!(
        object.contains_key("id"),
        "response fixture {} for {} should include a top-level id",
        fixture.name,
        fixture.version
    );

    assert!(
        object.contains_key("seatbid") || object.contains_key("nbr"),
        "response fixture {} for {} should include seatbid or nbr",
        fixture.name,
        fixture.version
    );

    if let Some(seatbid) = object.get("seatbid") {
        let seatbids = seatbid.as_array().unwrap_or_else(|| {
            panic!(
                "response fixture {} for {} should encode seatbid as an array",
                fixture.name,
                fixture.version
            )
        });

        for seat in seatbids {
            let seat_object = seat.as_object().unwrap_or_else(|| {
                panic!(
                    "response fixture {} for {} should encode each seatbid as an object",
                    fixture.name,
                    fixture.version
                )
            });

            if let Some(bids) = seat_object.get("bid") {
                assert!(
                    bids.is_array(),
                    "response fixture {} for {} should encode bid as an array",
                    fixture.name,
                    fixture.version
                );
            }
        }
    }
}

fn assert_three_zero_response_shape(
    fixture: &ResponseFixtureCase,
    object: &serde_json::Map<String, Value>,
) {
    let openrtb = object
        .get("openrtb")
        .and_then(Value::as_object)
        .unwrap_or_else(|| {
            panic!(
                "response fixture {} for {} should include an openrtb object",
                fixture.name,
                fixture.version
            )
        });

    let response = openrtb
        .get("response")
        .and_then(Value::as_object)
        .unwrap_or_else(|| {
            panic!(
                "response fixture {} for {} should include an openrtb.response object",
                fixture.name,
                fixture.version
            )
        });

    assert!(
        response.contains_key("id"),
        "response fixture {} for {} should include response.id",
        fixture.name,
        fixture.version
    );
}