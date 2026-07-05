//! One-shot migration: enriches the committed object catalogs with structured
//! validation fields (child_object, adcom_list, value_set) computed from the
//! legacy description prose, then drops the prose entirely.
//!
//! Run once from the repo root:
//! cargo run -p rtblint-core --example migrate_catalog_prose

use std::error::Error;
use std::fs;
use std::path::PathBuf;

use rtblint_core::catalog_extract::{extract_value_set, resolve_child_object};
use serde_json::{json, Map, Value};

fn main() -> Result<(), Box<dyn Error>> {
    let specs_dir = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("crates/rtblint-core/specs"));

    let mut migrated = 0usize;
    for entry in fs::read_dir(&specs_dir)? {
        let path = entry?.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if !file_name.ends_with("-object-catalog.json") {
            continue;
        }

        let raw = fs::read_to_string(&path)?;
        let mut catalog: Value = serde_json::from_str(&raw)?;
        migrate_catalog(&mut catalog).map_err(|error| format!("{file_name}: {error}"))?;
        fs::write(&path, format!("{}\n", serde_json::to_string_pretty(&catalog)?))?;
        migrated += 1;
        println!("migrated {file_name}");
    }

    println!("done: {migrated} catalogs migrated");
    Ok(())
}

fn migrate_catalog(catalog: &mut Value) -> Result<(), String> {
    let objects = catalog
        .get_mut("objects")
        .and_then(Value::as_array_mut)
        .ok_or("catalog has no objects array")?;

    let object_names: Vec<String> = objects
        .iter()
        .filter_map(|object| object.get("name").and_then(Value::as_str))
        .map(String::from)
        .collect();

    for object in objects.iter_mut() {
        let object_map = object.as_object_mut().ok_or("object entry is not a map")?;
        object_map.remove("description");

        let Some(fields) = object_map.get_mut("fields").and_then(Value::as_array_mut) else {
            continue;
        };

        for field in fields.iter_mut() {
            let field_map = field.as_object_mut().ok_or("field entry is not a map")?;
            let description = field_map
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let field_name = field_map
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            let mut enriched = Map::new();
            enriched.insert("name".into(), field_map.get("name").cloned().unwrap_or(Value::Null));
            enriched.insert(
                "type_spec".into(),
                field_map.get("type_spec").cloned().unwrap_or(Value::Null),
            );

            let type_spec = field_map
                .get("type_spec")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_ascii_lowercase();
            if type_spec.contains("object") {
                if let Some(child) = resolve_child_object(&description, &field_name, &object_names) {
                    enriched.insert("child_object".into(), Value::String(child));
                }
            }

            if let Some(value_set) = extract_value_set(&description) {
                if let Some(list_name) = value_set.adcom_list {
                    enriched.insert("adcom_list".into(), Value::String(list_name.into()));
                } else {
                    let mut set = Map::new();
                    set.insert("values".into(), json!(value_set.values));
                    if let Some(minimum) = value_set.minimum_inclusive {
                        set.insert("minimum_inclusive".into(), json!(minimum));
                    }
                    enriched.insert("value_set".into(), Value::Object(set));
                }
            }

            enriched.insert(
                "citation".into(),
                field_map.get("citation").cloned().unwrap_or(Value::Null),
            );

            *field_map = enriched;
        }
    }

    Ok(())
}
