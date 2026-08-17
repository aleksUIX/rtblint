//! Resolution against the fixture cache.

use std::path::PathBuf;

use rtblint_core::{Severity, ValidationResult};
use rtblint_resolve::{merge_into, resolve_bid_request, Cache};

fn cache() -> Cache {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cache");
    Cache::open(root).expect("fixture cache")
}

fn load(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/requests")
        .join(name);
    std::fs::read_to_string(path).expect(name)
}

fn has_issue(issues: &[rtblint_core::Issue], id: &str, path: &str) -> bool {
    issues
        .iter()
        .any(|issue| issue.id == id && issue.path.as_deref() == Some(path))
}

#[test]
fn matching_cache_is_clean() {
    let issues = resolve_bid_request(&load("valid-resolved.json"), &cache());
    assert!(issues.is_empty(), "{issues:?}");
}

#[test]
fn unknown_sid_is_an_error() {
    let issues = resolve_bid_request(&load("unknown-sid.json"), &cache());
    assert!(has_issue(
        &issues,
        "openrtb.resolve.sid_not_in_sellers",
        "source.schain.nodes[0].sid"
    ));
    assert_eq!(
        issues
            .iter()
            .find(|issue| issue.id == "openrtb.resolve.sid_not_in_sellers")
            .map(|issue| issue.severity),
        Some(Severity::Error)
    );
}

#[test]
fn missing_sellers_json_is_a_warning() {
    let issues = resolve_bid_request(&load("missing-sellers.json"), &cache());
    assert!(has_issue(
        &issues,
        "openrtb.resolve.sellers_json_unavailable",
        "source.schain.nodes[0].asi"
    ));
}

#[test]
fn unparseable_sellers_json_is_a_warning() {
    let payload = r#"{
        "id": "req-1",
        "source": {
            "schain": {
                "complete": 1,
                "ver": "1.0",
                "nodes": [{ "asi": "broken.example", "sid": "x", "hp": 1 }]
            }
        },
        "imp": [{ "id": "1", "banner": { "w": 1, "h": 1 } }]
    }"#;
    let issues = resolve_bid_request(payload, &cache());
    assert!(has_issue(
        &issues,
        "openrtb.resolve.sellers_json_unparseable",
        "source.schain.nodes[0].asi"
    ));
}

#[test]
fn ads_txt_must_list_the_first_payment_hop() {
    let payload = r#"{
        "id": "req-1",
        "source": {
            "schain": {
                "complete": 1,
                "ver": "1.0",
                "nodes": [{ "asi": "exchange2.example", "sid": "seller-xyz", "hp": 1 }]
            }
        },
        "site": { "domain": "publisher.example" },
        "imp": [{ "id": "1", "banner": { "w": 1, "h": 1 } }]
    }"#;
    let issues = resolve_bid_request(payload, &cache());
    assert!(has_issue(
        &issues,
        "openrtb.resolve.ads_txt_unauthorized",
        "source.schain.nodes[0].sid"
    ));
}

#[test]
fn missing_ads_txt_is_a_warning() {
    let payload = r#"{
        "id": "req-1",
        "source": {
            "schain": {
                "complete": 1,
                "ver": "1.0",
                "nodes": [{ "asi": "exchange1.example", "sid": "seller-abc", "hp": 1 }]
            }
        },
        "site": { "domain": "other-publisher.example" },
        "imp": [{ "id": "1", "banner": { "w": 1, "h": 1 } }]
    }"#;
    let issues = resolve_bid_request(payload, &cache());
    assert!(has_issue(
        &issues,
        "openrtb.resolve.ads_txt_unavailable",
        "site.domain"
    ));
}

#[test]
fn www_prefix_falls_back_to_the_registrable_domain() {
    let payload = r#"{
        "id": "req-1",
        "source": {
            "schain": {
                "complete": 1,
                "ver": "1.0",
                "nodes": [{ "asi": "exchange1.example", "sid": "seller-abc", "hp": 1 }]
            }
        },
        "site": { "domain": "www.publisher.example" },
        "imp": [{ "id": "1", "banner": { "w": 1, "h": 1 } }]
    }"#;
    let issues = resolve_bid_request(payload, &cache());
    assert!(
        !issues.iter().any(|issue| issue.id.contains("ads_txt")),
        "{issues:?}"
    );
}

#[test]
fn app_ads_txt_authorizes_the_bundle() {
    let issues = resolve_bid_request(&load("valid-app.json"), &cache());
    assert!(issues.is_empty(), "{issues:?}");
}

#[test]
fn missing_app_ads_txt_is_a_warning() {
    let payload = r#"{
        "id": "req-1",
        "source": {
            "schain": {
                "complete": 1,
                "ver": "1.0",
                "nodes": [{ "asi": "exchange1.example", "sid": "seller-abc", "hp": 1 }]
            }
        },
        "app": { "bundle": "com.missing.app" },
        "imp": [{ "id": "1", "banner": { "w": 1, "h": 1 } }]
    }"#;
    let issues = resolve_bid_request(payload, &cache());
    assert!(has_issue(
        &issues,
        "openrtb.resolve.app_ads_txt_unavailable",
        "app.bundle"
    ));
}

#[test]
fn layered_3_0_paths_include_the_envelope() {
    let issues = resolve_bid_request(&load("layered-3-0-unknown-sid.json"), &cache());
    assert!(has_issue(
        &issues,
        "openrtb.resolve.sid_not_in_sellers",
        "openrtb.request.source.schain.nodes[0].sid"
    ));
}

#[test]
fn hp_zero_nodes_are_not_checked() {
    let payload = r#"{
        "id": "req-1",
        "source": {
            "schain": {
                "complete": 1,
                "ver": "1.0",
                "nodes": [
                    { "asi": "unknown-ssp.example", "sid": "nope", "hp": 0 },
                    { "asi": "exchange1.example", "sid": "seller-abc", "hp": 1 }
                ]
            }
        },
        "site": { "domain": "publisher.example" },
        "imp": [{ "id": "1", "banner": { "w": 1, "h": 1 } }]
    }"#;
    let issues = resolve_bid_request(payload, &cache());
    assert!(
        !issues
            .iter()
            .any(|issue| issue.path.as_deref() == Some("source.schain.nodes[0].asi")),
        "{issues:?}"
    );
}

#[test]
fn merge_into_clears_valid_on_resolve_errors() {
    let mut result = ValidationResult::default();
    let extra = resolve_bid_request(&load("unknown-sid.json"), &cache());
    merge_into(&mut result, extra);
    assert!(!result.valid);
}

#[test]
fn app_ads_txt_must_list_the_first_payment_hop() {
    let payload = r#"{
        "id": "req-1",
        "source": {
            "schain": {
                "complete": 1,
                "ver": "1.0",
                "nodes": [{ "asi": "exchange2.example", "sid": "seller-xyz", "hp": 1 }]
            }
        },
        "app": { "bundle": "com.example.app" },
        "imp": [{ "id": "1", "banner": { "w": 1, "h": 1 } }]
    }"#;
    let issues = resolve_bid_request(payload, &cache());
    assert!(has_issue(
        &issues,
        "openrtb.resolve.app_ads_txt_unauthorized",
        "source.schain.nodes[0].sid"
    ));
}

#[test]
fn ext_schain_is_resolved() {
    let payload = r#"{
        "id": "req-1",
        "source": {
            "ext": {
                "schain": {
                    "complete": 1,
                    "ver": "1.0",
                    "nodes": [{ "asi": "exchange1.example", "sid": "unknown-sid", "hp": 1 }]
                }
            }
        },
        "site": { "domain": "publisher.example" },
        "imp": [{ "id": "1", "banner": { "w": 1, "h": 1 } }]
    }"#;
    let issues = resolve_bid_request(payload, &cache());
    assert!(has_issue(
        &issues,
        "openrtb.resolve.sid_not_in_sellers",
        "source.ext.schain.nodes[0].sid"
    ));
}

#[test]
fn cache_open_rejects_a_missing_directory() {
    let err = Cache::open("/no/such/rtblint-cache").unwrap_err();
    assert!(err.contains("not a directory"), "{err}");
}
