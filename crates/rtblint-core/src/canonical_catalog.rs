use serde::{Deserialize, Serialize};

use crate::OpenRtbVersion;

// ── static catalog data (code-generated, zero runtime parsing) ──────────
//
// The validator reads these `&'static` structures, transcribed from the
// JSON catalogs in `specs/` by build.rs at compile time. The serde types
// further down remain the canonical JSON interchange format used by the
// export pipeline.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticCatalog {
    pub version: &'static str,
    pub family: &'static str,
    pub release_date: &'static str,
    pub archive_path: &'static str,
    pub objects: &'static [StaticObject],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticObject {
    pub name: &'static str,
    pub section: &'static str,
    pub citation: StaticCitation,
    pub fields: &'static [StaticField],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticField {
    pub name: &'static str,
    pub type_spec: &'static str,
    /// JSON shape implied by `type_spec`, resolved at build time so the
    /// validator never re-parses the type string at runtime.
    pub shape: ExpectedShape,
    /// Whether `type_spec` marks the field unconditionally required.
    pub required: bool,
    /// Whether `type_spec` marks the field deprecated.
    pub deprecated: bool,
    /// Catalog object this field nests into, resolved at generation time.
    pub child_object: Option<&'static str>,
    /// Name of the AdCOM list constraining this field's values, when one
    /// applies. Values are resolved from the validator's list registry.
    pub adcom_list: Option<&'static str>,
    /// Inline documented value set, when the spec enumerates values directly.
    pub value_set: Option<StaticValueSet>,
    pub citation: StaticCitation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticValueSet {
    pub values: &'static [i64],
    pub minimum_inclusive: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticCitation {
    pub section: &'static str,
    pub canonical_source_file: &'static str,
    pub helper_source_file: &'static str,
    pub start_line: usize,
    pub end_line: usize,
}

/// JSON shape a catalog `type_spec` maps to. Derived once at build time by
/// build.rs; the mapping logic lives there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpectedShape {
    Unknown,
    Object,
    ObjectArray,
    String,
    StringArray,
    Integer,
    IntegerArray,
    Float,
    FloatArray,
    Boolean,
    BooleanArray,
    AnyArray,
}

impl ExpectedShape {
    pub fn label(self) -> &'static str {
        match self {
            Self::Unknown => "a supported type",
            Self::Object => "object",
            Self::ObjectArray => "array of objects",
            Self::String => "string",
            Self::StringArray => "array of strings",
            Self::Integer => "integer",
            Self::IntegerArray => "array of integers",
            Self::Float => "number",
            Self::FloatArray => "array of numbers",
            Self::Boolean => "boolean",
            Self::BooleanArray => "array of booleans",
            Self::AnyArray => "array",
        }
    }
}

include!(concat!(env!("OUT_DIR"), "/static_catalogs.rs"));

// ── serde types for the JSON catalog interchange format ─────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalObjectCatalog {
    pub kind: String,
    pub version: String,
    pub family: String,
    pub release_date: String,
    pub archive_path: String,
    pub canonical_source_file: String,
    pub helper_source_file: String,
    pub source_of_truth: String,
    pub objects: Vec<CanonicalObject>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalObject {
    pub name: String,
    pub section: String,
    pub citation: CatalogCitation,
    pub fields: Vec<CanonicalField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalField {
    pub name: String,
    pub type_spec: String,
    /// Catalog object this field nests into, resolved at generation time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub child_object: Option<String>,
    /// Name of the AdCOM list constraining this field's values, when one
    /// applies. Values are resolved from the validator's list registry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adcom_list: Option<String>,
    /// Inline documented value set, when the spec enumerates values directly.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_set: Option<CatalogValueSet>,
    pub citation: CatalogCitation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogValueSet {
    pub values: Vec<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum_inclusive: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogCitation {
    pub section: String,
    pub canonical_source_file: String,
    pub helper_source_file: String,
    pub start_line: usize,
    pub end_line: usize,
}

pub fn canonical_object_catalog_versions() -> &'static [OpenRtbVersion] {
    OpenRtbVersion::all()
}

pub fn canonical_object_catalog(version: OpenRtbVersion) -> Option<&'static StaticCatalog> {
    let id = version.id();
    GENERATED_CATALOGS
        .iter()
        .find(|(candidate, _)| *candidate == id)
        .map(|(_, catalog)| *catalog)
}

pub fn canonical_adcom_catalog() -> Option<&'static StaticCatalog> {
    GENERATED_ADCOM_CATALOGS
        .iter()
        .find(|(candidate, _)| *candidate == "1.0")
        .map(|(_, catalog)| *catalog)
}

pub fn canonical_adcom_object(object_name: &str) -> Option<&'static StaticObject> {
    canonical_adcom_catalog().and_then(|catalog| {
        catalog
            .objects
            .iter()
            .find(|object| object.name == object_name)
    })
}

pub fn canonical_object(
    version: OpenRtbVersion,
    object_name: &str,
) -> Option<&'static StaticObject> {
    canonical_object_catalog(version).and_then(|catalog| {
        catalog
            .objects
            .iter()
            .find(|object| object.name == object_name)
    })
}

pub fn canonical_field(
    version: OpenRtbVersion,
    object_name: &str,
    field_name: &str,
) -> Option<&'static StaticField> {
    canonical_object(version, object_name)
        .and_then(|object| object.fields.iter().find(|field| field.name == field_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_cover_all_known_versions() {
        assert_eq!(canonical_object_catalog_versions(), OpenRtbVersion::all());
    }

    #[test]
    fn every_version_has_generated_static_data() {
        for version in OpenRtbVersion::all() {
            assert!(
                canonical_object_catalog(*version).is_some(),
                "missing generated static catalog for {}",
                version.id()
            );
        }
    }

    #[test]
    fn catalog_2_0_contains_bidrequest_imp_field() {
        let field = canonical_field(OpenRtbVersion::V2_0, "BidRequest", "imp")
            .expect("2.0 canonical catalog should include BidRequest.imp");

        assert_eq!(field.citation.section, "3.3.1");
        assert_eq!(field.citation.canonical_source_file, "source.pdf");
        assert_eq!(field.citation.helper_source_file, "source-layout.txt");
    }

    #[test]
    fn catalog_2_5_contains_bidrequest_source_field() {
        let field = canonical_field(OpenRtbVersion::V2_5, "BidRequest", "source")
            .expect("2.5 canonical catalog should include BidRequest.source");
        let catalog = canonical_object_catalog(OpenRtbVersion::V2_5)
            .expect("2.5 canonical catalog should exist");

        assert!(catalog.objects.len() > 20);
        assert_eq!(field.citation.section, "3.2.1");
        assert_eq!(field.citation.canonical_source_file, "source.pdf");
        assert_eq!(field.citation.helper_source_file, "source-layout.txt");
    }

    #[test]
    fn catalog_2_6_contains_imp_plcmt_and_dooh() {
        let field = canonical_field(OpenRtbVersion::V2_6_202505, "Video", "plcmt")
            .expect("latest 2.6 canonical catalog should include Video.plcmt");
        let object = canonical_object(OpenRtbVersion::V2_6_202505, "DOOH")
            .expect("latest 2.6 canonical catalog should include DOOH");

        assert_eq!(field.citation.section, "3.2.7");
        assert_eq!(field.citation.canonical_source_file, "source.md");
        assert_eq!(field.citation.helper_source_file, "source.md");
        assert_eq!(object.citation.canonical_source_file, "source.md");
        assert!(!object.section.is_empty());
    }

    #[test]
    fn generated_value_sets_are_strictly_ascending_for_binary_search() {
        for (version, catalog) in GENERATED_CATALOGS {
            for object in catalog.objects {
                for field in object.fields {
                    if let Some(value_set) = field.value_set {
                        assert!(
                            value_set.values.windows(2).all(|pair| pair[0] < pair[1]),
                            "{version} {}.{} value_set must be strictly ascending",
                            object.name,
                            field.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn catalog_3_0_contains_request_item_field() {
        let field = canonical_field(OpenRtbVersion::V3_0, "Request", "item")
            .expect("3.0 canonical catalog should include Request.item");
        let root = canonical_object(OpenRtbVersion::V3_0, "Openrtb")
            .expect("3.0 canonical catalog should include Openrtb");

        assert_eq!(field.citation.canonical_source_file, "source.md");
        assert_eq!(field.citation.helper_source_file, "source.md");
        assert_eq!(root.citation.section, "Object: Openrtb");
    }

    #[test]
    fn catalog_3_0_walks_adcom_placement_through_item_spec() {
        let spec = canonical_field(OpenRtbVersion::V3_0, "Item", "spec")
            .expect("3.0 Item.spec should exist");
        assert_eq!(spec.child_object, Some("Spec"));
        let placement = canonical_field(OpenRtbVersion::V3_0, "Spec", "placement")
            .expect("AdCOM Spec.placement should exist");
        assert_eq!(placement.child_object, Some("Placement"));
        assert!(canonical_object(OpenRtbVersion::V3_0, "Placement").is_some());
        assert!(canonical_adcom_object("Placement").is_some());
        assert!(canonical_adcom_object("Ad").is_some());
        assert!(canonical_adcom_object("Context").is_some());
    }
}
