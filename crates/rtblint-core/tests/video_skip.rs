use rtblint_core::{canonical_field, validate_bid_request_for_version, OpenRtbVersion};

#[test]
fn video_skip_uses_flag_values_across_supported_catalogs() {
    for version in OpenRtbVersion::all() {
        let Some(field) = canonical_field(*version, "Video", "skip") else {
            continue;
        };
        assert_eq!(field.adcom_list, None, "{}", version.id());
        assert_eq!(field.value_set.expect("skip flag values").values, &[0, 1]);
        for skip in [0, 1, 16, 500] {
            let document = format!(
                r#"{{"id":"req-1","app":{{"id":"app-1"}},"imp":[{{"id":"imp-1","video":{{"mimes":["video/mp4"],"minduration":15,"maxduration":30,"protocols":[2],"skip":{skip}}}}}]}}"#
            );
            let result = validate_bid_request_for_version(*version, &document);
            if skip <= 1 {
                assert!(
                    result.valid,
                    "{} skip={}: {:?}",
                    version.id(),
                    skip,
                    result.issues
                );
            } else {
                assert!(
                    result
                        .issues
                        .iter()
                        .any(|issue| issue.id == "openrtb.value.invalid"
                            && issue.path.as_deref() == Some("imp[0].video.skip")),
                    "{} skip={} must be rejected",
                    version.id(),
                    skip
                );
            }
        }
    }
}
