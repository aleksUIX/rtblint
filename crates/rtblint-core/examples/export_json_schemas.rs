//! Generates a JSON Schema (draft 2020-12) per tracked OpenRTB version, for
//! both payload types, from the shipped object catalogs.
//!
//! The IAB publishes no JSON Schema for 2.6 or 3.0, so these are derived from
//! the same structured catalogs the validator uses: object nesting, field
//! types, required fields, documented value sets, and AdCOM list references.
//!
//! What a schema cannot express is exactly what the linter adds on top:
//! deprecated and moved paths, version-specific removals, and the semantic
//! rules (SupplyChain hygiene, GPP coherence, pod duration sanity, markup
//! coherence). Schemas are therefore permissive about unknown members, and
//! validating against one is not the same as linting.
//!
//! Usage: cargo run -p rtblint-core --example export_json_schemas [out_dir]

use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use serde_json::{json, Map, Value};

use rtblint_core::{
    adcom_list_values, canonical_object_catalog, ExpectedShape, OpenRtbVersion, StaticCatalog,
    StaticField, StaticObject,
};

const SCHEMA_BASE_URL: &str = "https://rtblint.org/schemas";

struct PayloadKind {
    /// Root object in a 2.x catalog.
    root: &'static str,
    /// Member this payload occupies inside a 3.0 envelope.
    layered_member: &'static str,
    /// Slug used in the file name and `$id`.
    slug: &'static str,
    title: &'static str,
}

const PAYLOAD_KINDS: &[PayloadKind] = &[
    PayloadKind {
        root: "BidRequest",
        layered_member: "request",
        slug: "bid-request",
        title: "OpenRTB bid request",
    },
    PayloadKind {
        root: "BidResponse",
        layered_member: "response",
        slug: "bid-response",
        title: "OpenRTB bid response",
    },
];

/// 3.0 wraps both payloads in one envelope object, so its schemas are rooted
/// there and pin the member the payload belongs in.
const LAYERED_ROOT: &str = "Openrtb";

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("schemas"));
    fs::create_dir_all(&output_dir)?;

    let mut written = Vec::new();
    for version in OpenRtbVersion::all() {
        let Some(catalog) = canonical_object_catalog(*version) else {
            continue;
        };

        let layered = matches!(version.family(), rtblint_core::OpenRtbFamily::ThreeZero);

        for payload in PAYLOAD_KINDS {
            // A version whose catalog has no usable root (the 2.6-202204 stub)
            // gets no schema rather than an empty one that would validate
            // anything.
            let root_name = if layered { LAYERED_ROOT } else { payload.root };
            let Some(root) = find_object(catalog, root_name) else {
                continue;
            };
            if root.fields.is_empty() {
                continue;
            }

            let file_name = format!("openrtb-{}-{}.schema.json", version.id(), payload.slug);
            let excluded_member = layered.then_some(match payload.layered_member {
                "request" => "response",
                _ => "request",
            });
            let mut schema = build_schema(catalog, root, payload, &file_name, excluded_member);
            if layered {
                pin_layered_payload(&mut schema, payload);
            }
            fs::write(
                output_dir.join(&file_name),
                format!("{}\n", serde_json::to_string_pretty(&schema)?),
            )?;
            written.push(file_name);
        }
    }

    println!(
        "wrote {} schemas to {}",
        written.len(),
        output_dir.display()
    );
    Ok(())
}

fn find_object<'a>(catalog: &'a StaticCatalog, name: &str) -> Option<&'a StaticObject> {
    catalog.objects.iter().find(|object| object.name == name)
}

/// The 3.0 envelope holds a request or a response, never both, and which one
/// is the whole difference between the two schemas. The catalog cannot say so
/// (the spec marks both "required *"), so it is pinned here.
fn pin_layered_payload(schema: &mut Value, payload: &PayloadKind) {
    let Some(root) = schema.as_object_mut() else {
        return;
    };

    // The envelope object is the schema root; nest it under the wrapper member
    // so the document describes a whole payload, not just its inside.
    let mut envelope: Map<String, Value> = root
        .iter()
        .filter(|(key, _)| {
            !key.starts_with('$') && key.as_str() != "title" && key.as_str() != "description"
        })
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    let mut required: Vec<Value> = envelope
        .get("required")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let member = Value::String(String::from(payload.layered_member));
    if !required.contains(&member) {
        required.push(member);
    }
    envelope.insert(String::from("required"), Value::Array(required));

    for key in ["type", "properties", "required", "additionalProperties"] {
        root.remove(key);
    }
    root.insert(String::from("type"), json!("object"));
    root.insert(
        String::from("properties"),
        json!({ "openrtb": Value::Object(envelope) }),
    );
    root.insert(String::from("required"), json!(["openrtb"]));
    root.insert(String::from("additionalProperties"), json!(false));
}

fn build_schema(
    catalog: &StaticCatalog,
    root: &StaticObject,
    payload: &PayloadKind,
    file_name: &str,
    excluded_member: Option<&str>,
) -> Value {
    // Only the objects the root actually reaches, so a request schema does not
    // carry the response tree and vice versa.
    let reachable = reachable_objects(catalog, root, excluded_member);
    let mut defs = Map::new();
    for object_name in &reachable {
        if object_name == root.name {
            continue;
        }
        if let Some(object) = find_object(catalog, object_name) {
            defs.insert(String::from(object.name), object_schema(object));
        }
    }

    let mut schema = object_schema(root);
    let schema_object = schema.as_object_mut().expect("object schema");
    if let Some(excluded) = excluded_member {
        if let Some(properties) = schema_object
            .get_mut("properties")
            .and_then(Value::as_object_mut)
        {
            properties.remove(excluded);
        }
    }
    schema_object.insert(
        String::from("$schema"),
        json!("https://json-schema.org/draft/2020-12/schema"),
    );
    schema_object.insert(
        String::from("$id"),
        json!(format!("{SCHEMA_BASE_URL}/{file_name}")),
    );
    schema_object.insert(
        String::from("title"),
        json!(format!("{} ({})", payload.title, catalog.version)),
    );
    schema_object.insert(
        String::from("description"),
        json!(format!(
            "Generated by rtblint from the OpenRTB {} object catalog (released {}). \
             Structure, types, required fields, and documented value sets only: \
             deprecations, moved paths, and semantic rules are what the linter adds.",
            catalog.version, catalog.release_date
        )),
    );
    if !defs.is_empty() {
        schema_object.insert(String::from("$defs"), Value::Object(defs));
    }

    schema
}

/// Walks `child_object` edges from the root, so each schema carries only the
/// definitions its payload can reach.
fn reachable_objects(
    catalog: &StaticCatalog,
    root: &StaticObject,
    excluded_member: Option<&str>,
) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    let mut queue = vec![root.name];

    while let Some(name) = queue.pop() {
        if !seen.insert(String::from(name)) {
            continue;
        }

        let Some(object) = find_object(catalog, name) else {
            continue;
        };
        for field in object.fields {
            if name == root.name && Some(field.name) == excluded_member {
                continue;
            }
            if let Some(child) = field.child_object {
                if !seen.contains(child) {
                    queue.push(child);
                }
            }
        }
    }

    seen
}

fn object_schema(object: &StaticObject) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();

    for field in object.fields {
        properties.insert(String::from(field.name), field_schema(field));
        if field.required {
            required.push(Value::String(String::from(field.name)));
        }
    }

    let mut schema = Map::new();
    schema.insert(String::from("type"), json!("object"));
    schema.insert(
        String::from("description"),
        json!(format!("Section {}", object.section)),
    );
    schema.insert(String::from("properties"), Value::Object(properties));
    if !required.is_empty() {
        schema.insert(String::from("required"), Value::Array(required));
    }
    // Members outside the catalog are the linter's business
    // (openrtb.field.undefined), not the schema's: exchanges legitimately
    // carry ext members, and a stricter schema would reject them.
    schema.insert(String::from("additionalProperties"), json!(true));

    Value::Object(schema)
}

fn field_schema(field: &StaticField) -> Value {
    let mut schema = Map::new();
    if field.deprecated {
        schema.insert(String::from("deprecated"), json!(true));
    }

    let constraint = value_constraint(field);
    match field.shape {
        ExpectedShape::Object | ExpectedShape::ObjectArray => {
            let item = match field.child_object {
                Some(child) => json!({ "$ref": format!("#/$defs/{child}") }),
                None => json!({ "type": "object" }),
            };
            if matches!(field.shape, ExpectedShape::ObjectArray) {
                schema.insert(String::from("type"), json!("array"));
                schema.insert(String::from("items"), item);
            } else {
                return merge(item, schema);
            }
        }
        ExpectedShape::String => {
            schema.insert(String::from("type"), json!("string"));
        }
        ExpectedShape::StringArray => {
            schema.insert(String::from("type"), json!("array"));
            schema.insert(String::from("items"), json!({ "type": "string" }));
        }
        ExpectedShape::Integer => {
            schema.insert(String::from("type"), json!("integer"));
            if let Some(constraint) = constraint.clone() {
                return merge(Value::Object(schema), constraint);
            }
        }
        ExpectedShape::IntegerArray => {
            schema.insert(String::from("type"), json!("array"));
            let mut item = Map::new();
            item.insert(String::from("type"), json!("integer"));
            let item = match constraint.clone() {
                Some(constraint) => merge(Value::Object(item), constraint),
                None => Value::Object(item),
            };
            schema.insert(String::from("items"), item);
        }
        ExpectedShape::Float => {
            schema.insert(String::from("type"), json!("number"));
        }
        ExpectedShape::FloatArray => {
            schema.insert(String::from("type"), json!("array"));
            schema.insert(String::from("items"), json!({ "type": "number" }));
        }
        ExpectedShape::Boolean => {
            // OpenRTB spells booleans as 0/1 integers.
            schema.insert(String::from("type"), json!("integer"));
            schema.insert(String::from("enum"), json!([0, 1]));
        }
        ExpectedShape::BooleanArray => {
            schema.insert(String::from("type"), json!("array"));
            schema.insert(
                String::from("items"),
                json!({ "type": "integer", "enum": [0, 1] }),
            );
        }
        ExpectedShape::AnyArray => {
            schema.insert(String::from("type"), json!("array"));
        }
        ExpectedShape::Unknown => {}
    }

    Value::Object(schema)
}

/// Documented values, plus the vendor-extension floor when the spec defines
/// one ("values 500 and greater are exchange-specific").
fn value_constraint(field: &StaticField) -> Option<Map<String, Value>> {
    let (values, minimum) = match (field.value_set, field.adcom_list) {
        (Some(value_set), _) => (value_set.values, value_set.minimum_inclusive),
        (None, Some(list_name)) => {
            let list = adcom_list_values(list_name)?;
            (list.allowed_values, list.minimum_inclusive)
        }
        (None, None) => return None,
    };

    if values.is_empty() {
        return None;
    }

    let mut constraint = Map::new();
    match minimum {
        Some(minimum) => {
            constraint.insert(
                String::from("anyOf"),
                json!([
                    { "enum": values },
                    { "minimum": minimum },
                ]),
            );
        }
        None => {
            constraint.insert(String::from("enum"), json!(values));
        }
    }

    Some(constraint)
}

fn merge(base: Value, extra: impl Into<Value>) -> Value {
    let mut merged = match base {
        Value::Object(map) => map,
        other => return other,
    };

    if let Value::Object(extra) = extra.into() {
        for (key, value) in extra {
            merged.insert(key, value);
        }
    }

    Value::Object(merged)
}
