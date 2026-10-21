use std::{collections::HashSet, fs, path::Path};

use rtblint_core::{
    validate_bid_request_with_dialect, validate_bid_response_against_request_with_dialect,
    validate_bid_response_with_dialect, Dialect, OpenRtbVersion,
};
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
struct ExpectedIssue {
    id: String,
    severity: String,
    path: Option<String>,
}

#[derive(Deserialize)]
struct Case {
    name: String,
    mode: String,
    version: String,
    dialect: String,
    payload: String,
    request: Option<String>,
    valid: bool,
    issues: Vec<ExpectedIssue>,
}

#[test]
fn stored_dialect_regressions_match_exact_findings() {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/dialects");
    let cases: Vec<Case> = serde_json::from_str(include_str!("fixtures/dialects/cases.json"))
        .expect("valid fixture manifest");
    let mut names = HashSet::new();
    let mut referenced = HashSet::new();
    for mut case in cases {
        assert!(names.insert(case.name.clone()), "duplicate fixture name");
        referenced.insert(case.payload.clone());
        let payload = fs::read_to_string(directory.join(&case.payload)).expect("fixture payload");
        let version = OpenRtbVersion::from_id(&case.version).expect("supported fixture version");
        let dialect = Dialect::from_id(&case.dialect).expect("supported fixture dialect");
        let result = match case.mode.as_str() {
            "request" => validate_bid_request_with_dialect(version, dialect, &payload),
            "response" => validate_bid_response_with_dialect(version, dialect, &payload),
            "pair" => {
                let file = case.request.as_ref().expect("paired request file");
                referenced.insert(file.clone());
                let request = fs::read_to_string(directory.join(file)).expect("paired request");
                validate_bid_response_against_request_with_dialect(
                    version, dialect, &request, &payload,
                )
            }
            mode => panic!("unsupported fixture mode: {mode}"),
        };
        assert_eq!(
            result.valid, case.valid,
            "{}: {:?}",
            case.name, result.issues
        );
        let mut actual: Vec<_> = result
            .issues
            .into_iter()
            .map(|issue| ExpectedIssue {
                id: issue.id,
                severity: issue.severity.as_str().to_owned(),
                path: issue.path,
            })
            .collect();
        actual.sort();
        case.issues.sort();
        assert_eq!(actual, case.issues, "{}", case.name);
    }
    for entry in fs::read_dir(directory).expect("fixture directory") {
        let name = entry
            .expect("fixture entry")
            .file_name()
            .into_string()
            .expect("UTF-8 file name");
        if name.ends_with(".json") && name != "cases.json" || name.ends_with(".txt") {
            assert!(
                referenced.contains(&name),
                "fixture is not exercised: {name}"
            );
        }
    }
}
