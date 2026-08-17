//! Opt-in sellers.json / ads.txt resolution against a local cache.
//!
//! The OpenRTB core stays a pure function of the payload. This crate reads a
//! directory of already-fetched sellers.json, ads.txt, and app-ads.txt files
//! and reports whether each SupplyChain hop and the publisher's authorization
//! file agree with the request.

mod parse;

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use rtblint_core::{Issue, Severity, ValidationResult};

use parse::{parse_ads_txt, parse_sellers_json, AdsTxt, SellersJson};

/// On-disk reference database. Layout:
///
/// ```text
/// <root>/sellers/<asi>/sellers.json
/// <root>/ads/<site.domain>/ads.txt
/// <root>/app-ads/<app.bundle>/app-ads.txt
/// ```
///
/// Files are read on demand. Nothing is fetched from the network.
#[derive(Debug, Clone)]
pub struct Cache {
    root: PathBuf,
}

impl Cache {
    /// Opens an existing cache directory. Does not read files until a lookup.
    pub fn open(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        if !root.is_dir() {
            return Err(format!(
                "supply-chain cache {} is not a directory",
                root.display()
            ));
        }
        Ok(Self { root })
    }

    fn sellers_json(&self, asi: &str) -> Lookup<SellersJson> {
        let Some(key) = cache_key(asi) else {
            return Lookup::Missing;
        };
        read_parsed(
            self.root.join("sellers").join(key).join("sellers.json"),
            |raw| parse_sellers_json(raw).map_err(|_| ()),
        )
    }

    fn ads_txt(&self, domain: &str) -> Lookup<AdsTxt> {
        lookup_ads_txt(&self.root.join("ads"), domain, "ads.txt")
    }

    fn app_ads_txt(&self, bundle: &str) -> Lookup<AdsTxt> {
        let Some(key) = cache_key(bundle) else {
            return Lookup::Missing;
        };
        read_parsed(
            self.root.join("app-ads").join(key).join("app-ads.txt"),
            |raw| Ok(parse_ads_txt(raw)),
        )
    }
}

enum Lookup<T> {
    Missing,
    Unparseable,
    Present(T),
}

fn lookup_ads_txt(dir: &Path, domain: &str, file_name: &str) -> Lookup<AdsTxt> {
    let mut keys = Vec::new();
    if let Some(exact) = cache_key(domain) {
        keys.push(exact.clone());
        if let Some(stripped) = exact.strip_prefix("www.") {
            if !stripped.is_empty() {
                keys.push(String::from(stripped));
            }
        }
    }
    let mut saw_unparseable = false;
    for key in keys {
        match read_parsed(dir.join(key).join(file_name), |raw| Ok(parse_ads_txt(raw))) {
            Lookup::Present(parsed) => return Lookup::Present(parsed),
            Lookup::Unparseable => saw_unparseable = true,
            Lookup::Missing => {}
        }
    }
    if saw_unparseable {
        Lookup::Unparseable
    } else {
        Lookup::Missing
    }
}

fn read_parsed<T>(path: PathBuf, parse: impl FnOnce(&str) -> Result<T, ()>) -> Lookup<T> {
    match fs::read_to_string(&path) {
        Err(_) => Lookup::Missing,
        Ok(raw) => match parse(&raw) {
            Ok(parsed) => Lookup::Present(parsed),
            Err(()) => Lookup::Unparseable,
        },
    }
}

/// Directory names are the lookup key (asi, site.domain, app.bundle). Reject
/// anything that could walk out of the cache root.
fn cache_key(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == ".." {
        return None;
    }
    if trimmed.contains('/') || trimmed.contains('\\') || trimmed.contains('\0') {
        return None;
    }
    Some(trimmed.to_ascii_lowercase())
}

/// Appends resolution findings onto a core result and clears `valid` when any
/// of them are errors.
pub fn merge_into(result: &mut ValidationResult, extra: Vec<Issue>) {
    if extra.iter().any(|issue| issue.severity == Severity::Error) {
        result.valid = false;
    }
    result.issues.extend(extra);
}

/// Walks SupplyChain hops and the publisher's ads.txt / app-ads.txt against
/// `cache`. Unknown JSON yields no findings; the core already reported that.
pub fn resolve_bid_request(input: &str, cache: &Cache) -> Vec<Issue> {
    let Ok(value) = serde_json::from_str::<Value>(input) else {
        return Vec::new();
    };
    let (request, path_prefix) = request_view(&value);
    let Some(request) = request.as_object() else {
        return Vec::new();
    };

    let mut issues = Vec::new();
    let nodes = supply_chain_nodes(request, path_prefix);
    for node in &nodes {
        check_sellers_json(cache, node, &mut issues);
    }

    if let Some((domain, domain_path)) = publisher_domain(request, path_prefix) {
        check_authorization_file(
            cache.ads_txt(&domain),
            &nodes,
            &domain_path,
            "openrtb.resolve.ads_txt_unavailable",
            "openrtb.resolve.ads_txt_unauthorized",
            "ads.txt",
            &mut issues,
        );
    }

    if let Some((bundle, bundle_path)) = app_bundle(request, path_prefix) {
        check_authorization_file(
            cache.app_ads_txt(&bundle),
            &nodes,
            &bundle_path,
            "openrtb.resolve.app_ads_txt_unavailable",
            "openrtb.resolve.app_ads_txt_unauthorized",
            "app-ads.txt",
            &mut issues,
        );
    }

    issues
}

struct ChainNode {
    asi: String,
    sid: String,
    payment: bool,
    sid_path: String,
    asi_path: String,
}

fn request_view(value: &Value) -> (&Value, &str) {
    if let Some(request) = value
        .get("openrtb")
        .and_then(Value::as_object)
        .and_then(|envelope| envelope.get("request"))
    {
        // concat so the finding-id scraper does not treat this as a rule id.
        (request, concat!("openrtb", ".request"))
    } else {
        (value, "")
    }
}

fn json_path(prefix: &str, tail: &str) -> String {
    if prefix.is_empty() {
        String::from(tail)
    } else {
        format!("{prefix}.{tail}")
    }
}

fn supply_chain_nodes(request: &Map<String, Value>, prefix: &str) -> Vec<ChainNode> {
    let Some(source) = request.get("source").and_then(Value::as_object) else {
        return Vec::new();
    };

    let (schain, schain_path) =
        if let Some(schain) = source.get("schain").and_then(Value::as_object) {
            (schain, json_path(prefix, "source.schain"))
        } else if let Some(schain) = source
            .get("ext")
            .and_then(Value::as_object)
            .and_then(|ext| ext.get("schain"))
            .and_then(Value::as_object)
        {
            (schain, json_path(prefix, "source.ext.schain"))
        } else {
            return Vec::new();
        };

    let Some(nodes) = schain.get("nodes").and_then(Value::as_array) else {
        return Vec::new();
    };

    nodes
        .iter()
        .enumerate()
        .filter_map(|(index, node)| {
            let object = node.as_object()?;
            let asi = string_field(object, "asi")?;
            let sid = string_field(object, "sid")?;
            if asi.is_empty() || sid.is_empty() {
                return None;
            }
            let payment = !matches!(object.get("hp").and_then(integer_value), Some(0));
            Some(ChainNode {
                asi,
                sid,
                payment,
                sid_path: format!("{schain_path}.nodes[{index}].sid"),
                asi_path: format!("{schain_path}.nodes[{index}].asi"),
            })
        })
        .collect()
}

fn publisher_domain(request: &Map<String, Value>, prefix: &str) -> Option<(String, String)> {
    let site = request.get("site").and_then(Value::as_object).or_else(|| {
        request
            .get("context")
            .and_then(Value::as_object)
            .and_then(|context| context.get("site"))
            .and_then(Value::as_object)
    })?;
    let domain = string_field(site, "domain")?;
    if domain.is_empty() {
        return None;
    }
    let path = if request.get("context").is_some() && request.get("site").is_none() {
        json_path(prefix, "context.site.domain")
    } else {
        json_path(prefix, "site.domain")
    };
    Some((domain, path))
}

fn app_bundle(request: &Map<String, Value>, prefix: &str) -> Option<(String, String)> {
    let app = request.get("app").and_then(Value::as_object).or_else(|| {
        request
            .get("context")
            .and_then(Value::as_object)
            .and_then(|context| context.get("app"))
            .and_then(Value::as_object)
    })?;
    let bundle = string_field(app, "bundle")?;
    if bundle.is_empty() {
        return None;
    }
    let path = if request.get("context").is_some() && request.get("app").is_none() {
        json_path(prefix, "context.app.bundle")
    } else {
        json_path(prefix, "app.bundle")
    };
    Some((bundle, path))
}

fn check_sellers_json(cache: &Cache, node: &ChainNode, issues: &mut Vec<Issue>) {
    if !node.payment {
        return;
    }
    match cache.sellers_json(&node.asi) {
        Lookup::Missing => issues.push(issue(
            "openrtb.resolve.sellers_json_unavailable",
            Severity::Warning,
            format!(
                "No sellers.json for {} is in the cache; the hop cannot be authorised.",
                node.asi
            ),
            node.asi_path.clone(),
        )),
        Lookup::Unparseable => issues.push(issue(
            "openrtb.resolve.sellers_json_unparseable",
            Severity::Warning,
            format!(
                "The cached sellers.json for {} is not usable, so sid {} was not checked.",
                node.asi, node.sid
            ),
            node.asi_path.clone(),
        )),
        Lookup::Present(sellers) => {
            if !sellers.contains_sid(&node.sid) {
                issues.push(issue(
                    "openrtb.resolve.sid_not_in_sellers",
                    Severity::Error,
                    format!(
                        "sid \"{}\" is not a seller_id in {}'s sellers.json.",
                        node.sid, node.asi
                    ),
                    node.sid_path.clone(),
                ));
            }
        }
    }
}

fn check_authorization_file(
    lookup: Lookup<AdsTxt>,
    nodes: &[ChainNode],
    publisher_path: &str,
    unavailable_id: &'static str,
    unauthorized_id: &'static str,
    file_label: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(first) = nodes.iter().find(|node| node.payment) else {
        return;
    };

    match lookup {
        Lookup::Missing => issues.push(issue(
            unavailable_id,
            Severity::Warning,
            format!("No {file_label} for this publisher is in the cache."),
            String::from(publisher_path),
        )),
        Lookup::Unparseable => issues.push(issue(
            unavailable_id,
            Severity::Warning,
            format!("The cached {file_label} for this publisher is not usable."),
            String::from(publisher_path),
        )),
        Lookup::Present(file) => {
            if !file.authorizes(&first.asi, &first.sid) {
                issues.push(issue(
                    unauthorized_id,
                    Severity::Error,
                    format!(
                        "The first payment hop ({} / {}) is not listed as DIRECT or RESELLER in this publisher's {file_label}.",
                        first.asi, first.sid
                    ),
                    first.sid_path.clone(),
                ));
            }
        }
    }
}

fn string_field(object: &Map<String, Value>, name: &str) -> Option<String> {
    match object.get(name)? {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        _ => None,
    }
}

fn integer_value(value: &Value) -> Option<i64> {
    value.as_i64().or_else(|| {
        value
            .as_u64()
            .and_then(|integer| i64::try_from(integer).ok())
    })
}

fn issue(id: &'static str, severity: Severity, message: String, path: String) -> Issue {
    Issue::new(id, severity, message, Some(path))
}
