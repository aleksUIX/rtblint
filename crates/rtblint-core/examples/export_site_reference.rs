//! Exports the structured reference data behind the rtblint.org field and
//! enumerated-list pages:
//!
//! - `field-reference.json`: every OpenRTB 2.x object and field across the
//!   tracked versions, with type, required and deprecated markers, AdCOM list
//!   references, documented value sets, spec sections, the paths each object
//!   is reachable at, and the versioned rules that name each path.
//! - `adcom-lists.json`: every AdCOM list in the validator registry with its
//!   allowed values.
//!
//! The data comes from the same static catalogs and version profiles the
//! validator uses. It carries no spec prose.
//!
//! Usage: cargo run -p rtblint-core --example export_site_reference [out_dir]

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use rtblint_core::{
    adcom_list_names, adcom_list_values, canonical_object_catalog, version_profiles, OpenRtbFamily,
    OpenRtbVersion, StaticField,
};

/// Objects that start a payload. Every other object is reached from these.
const ROOTS: &[(&str, &str)] = &[("BidRequest", ""), ("BidResponse", "")];
/// Parent links the catalogs do not resolve to a child object (the type_spec
/// names the object only in prose, or not at all). Used for paths only; the
/// validator's own nesting is unchanged.
const SUPPLEMENTAL_EDGES: &[(&str, &str, &str)] = &[
    ("Device", "sua", "UserAgent"),
    ("UserAgent", "browsers", "BrandVersion"),
    ("BidRequest", "regs", "Regulations"),
    ("Pmp", "deals", "DirectDeals"),
];
/// Upper bound on paths kept per object; nesting such as Banner under Video
/// companions makes the count grow quickly without adding information.
const MAX_PATHS: usize = 12;

fn main() -> Result<(), Box<dyn Error>> {
    let out_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("site-reference"));
    fs::create_dir_all(&out_dir)?;

    let versions: Vec<OpenRtbVersion> = OpenRtbVersion::all()
        .iter()
        .copied()
        .filter(|v| v.family() == OpenRtbFamily::TwoX)
        .filter(|v| canonical_object_catalog(*v).is_some_and(|c| !c.objects.is_empty()))
        .collect();
    let version_ids: Vec<&str> = versions.iter().map(|v| v.id()).collect();

    // object -> field -> version -> field attributes
    let mut objects: BTreeMap<&str, ObjectAcc> = BTreeMap::new();
    for version in &versions {
        let catalog = canonical_object_catalog(*version).expect("catalog");
        for object in catalog.objects {
            let acc = objects.entry(object.name).or_default();
            acc.versions.push(version.id());
            acc.section.insert(version.id(), object.section);
            for field in object.fields {
                acc.fields
                    .entry(field.name)
                    .or_default()
                    .push((version.id(), field_attrs(field)));
            }
        }
    }

    let paths = object_paths(&versions);
    let rules = rule_index();

    let mut object_out = Vec::new();
    for (name, acc) in &objects {
        let object_paths: Vec<String> = paths.get(*name).cloned().unwrap_or_default();
        let mut fields_out = Vec::new();
        for (field_name, per_version) in &acc.fields {
            let present: Vec<&str> = per_version.iter().map(|(v, _)| *v).collect();
            let full_paths: Vec<String> = object_paths
                .iter()
                .map(|p| {
                    if p.is_empty() {
                        field_name.to_string()
                    } else {
                        format!("{p}.{field_name}")
                    }
                })
                .collect();
            let mut matched: BTreeSet<&str> = BTreeSet::new();
            for p in &full_paths {
                if let Some(codes) = rules.get(p.as_str()) {
                    matched.extend(codes.iter().copied());
                }
            }
            fields_out.push(json!({
                "name": field_name,
                "versions": present,
                "variants": variants(per_version),
                "paths": full_paths,
                "rules": matched.into_iter().collect::<Vec<_>>(),
            }));
        }
        object_out.push(json!({
            "name": name,
            "versions": acc.versions,
            "section": acc.section,
            "paths": object_paths,
            "fields": fields_out,
        }));
    }

    let field_reference = json!({
        "kind": "rtblint-field-reference",
        "generated_by": format!("rtblint-core {} export_site_reference", env!("CARGO_PKG_VERSION")),
        "versions": version_ids,
        "objects": object_out,
    });
    fs::write(
        out_dir.join("field-reference.json"),
        serde_json::to_string_pretty(&field_reference)? + "\n",
    )?;

    let lists: Vec<Value> = adcom_list_names()
        .map(|name| {
            let values = adcom_list_values(name).expect("registered list");
            json!({
                "name": name,
                "values": values.allowed_values,
                "minimum_inclusive": values.minimum_inclusive,
            })
        })
        .collect();
    let adcom = json!({
        "kind": "rtblint-adcom-lists",
        "generated_by": format!("rtblint-core {} export_site_reference", env!("CARGO_PKG_VERSION")),
        "lists": lists,
    });
    fs::write(
        out_dir.join("adcom-lists.json"),
        serde_json::to_string_pretty(&adcom)? + "\n",
    )?;

    println!(
        "wrote {} objects across {} versions and {} AdCOM lists to {}",
        objects.len(),
        versions.len(),
        adcom_list_names().count(),
        out_dir.display()
    );
    Ok(())
}

#[derive(Default)]
struct ObjectAcc {
    versions: Vec<&'static str>,
    section: BTreeMap<&'static str, &'static str>,
    fields: BTreeMap<&'static str, Vec<(&'static str, Value)>>,
}

fn field_attrs(field: &StaticField) -> Value {
    let mut attrs = Map::new();
    attrs.insert("type_spec".into(), json!(field.type_spec));
    attrs.insert("shape".into(), json!(format!("{:?}", field.shape)));
    attrs.insert("required".into(), json!(field.required));
    attrs.insert("deprecated".into(), json!(field.deprecated));
    attrs.insert("section".into(), json!(field.citation.section));
    if let Some(child) = field.child_object {
        attrs.insert("child_object".into(), json!(child));
    }
    if let Some(list) = field.adcom_list {
        attrs.insert("adcom_list".into(), json!(list));
    }
    if let Some(set) = field.value_set {
        attrs.insert(
            "value_set".into(),
            json!({ "values": set.values, "minimum_inclusive": set.minimum_inclusive }),
        );
    }
    Value::Object(attrs)
}

/// Groups consecutive versions with identical attributes, so a field that never
/// changed exports one variant instead of one entry per version.
fn variants(per_version: &[(&'static str, Value)]) -> Vec<Value> {
    let mut out: Vec<(Vec<&str>, &Value)> = Vec::new();
    for (version, attrs) in per_version {
        match out.last_mut() {
            Some((versions, last)) if *last == attrs => versions.push(version),
            _ => out.push((vec![version], attrs)),
        }
    }
    out.into_iter()
        .map(|(versions, attrs)| {
            let mut v = attrs.as_object().cloned().unwrap_or_default();
            v.insert("versions".into(), json!(versions));
            Value::Object(v)
        })
        .collect()
}

/// Dotted paths from the payload roots to every object, using the nesting of
/// every tracked version so objects that disappeared still resolve.
fn object_paths(versions: &[OpenRtbVersion]) -> BTreeMap<String, Vec<String>> {
    let mut edges: BTreeMap<&str, BTreeSet<(&str, &str)>> = BTreeMap::new();
    for version in versions {
        let catalog = canonical_object_catalog(*version).expect("catalog");
        for object in catalog.objects {
            for field in object.fields {
                if let Some(child) = field.child_object {
                    edges
                        .entry(object.name)
                        .or_default()
                        .insert((field.name, child));
                }
            }
        }
    }
    for (parent, field, child) in SUPPLEMENTAL_EDGES {
        edges.entry(*parent).or_default().insert((*field, *child));
    }
    let mut paths: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut queue: VecDeque<(&str, String, Vec<&str>)> = ROOTS
        .iter()
        .map(|(name, path)| (*name, path.to_string(), vec![*name]))
        .collect();
    while let Some((object, path, seen)) = queue.pop_front() {
        let entry = paths.entry(object.to_string()).or_default();
        if entry.len() >= MAX_PATHS || entry.contains(&path) {
            continue;
        }
        entry.push(path.clone());
        if let Some(children) = edges.get(object) {
            for (field, child) in children {
                if seen.contains(child) {
                    continue;
                }
                let next = if path.is_empty() {
                    field.to_string()
                } else {
                    format!("{path}.{field}")
                };
                let mut seen_next = seen.clone();
                seen_next.push(child);
                queue.push_back((child, next, seen_next));
            }
        }
    }
    paths
}

/// Versioned rule codes keyed by every path they name, primary or replacement.
fn rule_index() -> BTreeMap<&'static str, Vec<&'static str>> {
    let mut index: BTreeMap<&'static str, Vec<&'static str>> = BTreeMap::new();
    for profile in version_profiles() {
        for rule in profile.rules {
            for path in rule.paths.iter().chain(rule.replacement_paths) {
                let codes = index.entry(*path).or_default();
                if !codes.contains(&rule.code) {
                    codes.push(rule.code);
                }
            }
        }
    }
    index
}
