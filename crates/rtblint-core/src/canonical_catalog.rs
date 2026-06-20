use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::OpenRtbVersion;

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
    pub description: String,
    pub citation: CatalogCitation,
    pub fields: Vec<CanonicalField>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalField {
    pub name: String,
    pub type_spec: String,
    pub description: String,
    pub citation: CatalogCitation,
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

pub fn canonical_object_catalog(version: OpenRtbVersion) -> Option<&'static CanonicalObjectCatalog> {
    load_all_catalogs()
        .iter()
        .find(|(candidate_version, _)| *candidate_version == version)
        .map(|(_, catalog)| catalog)
}

pub fn canonical_object(
    version: OpenRtbVersion,
    object_name: &str,
) -> Option<&'static CanonicalObject> {
    canonical_object_catalog(version)
        .and_then(|catalog| catalog.objects.iter().find(|object| object.name == object_name))
}

pub fn canonical_field(
    version: OpenRtbVersion,
    object_name: &str,
    field_name: &str,
) -> Option<&'static CanonicalField> {
    canonical_object(version, object_name)
        .and_then(|object| object.fields.iter().find(|field| field.name == field_name))
}

fn parse_catalog(version: OpenRtbVersion, raw: &str) -> CanonicalObjectCatalog {
    serde_json::from_str(raw)
        .unwrap_or_else(|error| panic!("failed to parse canonical catalog for {}: {error}", version.id()))
}

/// Catalogs are embedded at compile time so the validator carries no runtime
/// filesystem dependency and runs unchanged in WASM and other no-fs targets.
fn embedded_catalog(version: OpenRtbVersion) -> &'static str {
    match version {
        OpenRtbVersion::V2_0 => include_str!("../specs/openrtb-2.0-object-catalog.json"),
        OpenRtbVersion::V2_1 => include_str!("../specs/openrtb-2.1-object-catalog.json"),
        OpenRtbVersion::V2_2 => include_str!("../specs/openrtb-2.2-object-catalog.json"),
        OpenRtbVersion::V2_3 => include_str!("../specs/openrtb-2.3-object-catalog.json"),
        OpenRtbVersion::V2_3_1 => include_str!("../specs/openrtb-2.3.1-object-catalog.json"),
        OpenRtbVersion::V2_4 => include_str!("../specs/openrtb-2.4-object-catalog.json"),
        OpenRtbVersion::V2_5 => include_str!("../specs/openrtb-2.5-object-catalog.json"),
        OpenRtbVersion::V2_6_202204 => include_str!("../specs/openrtb-2.6-202204-object-catalog.json"),
        OpenRtbVersion::V2_6_202210 => include_str!("../specs/openrtb-2.6-202210-object-catalog.json"),
        OpenRtbVersion::V2_6_202211 => include_str!("../specs/openrtb-2.6-202211-object-catalog.json"),
        OpenRtbVersion::V2_6_202303 => include_str!("../specs/openrtb-2.6-202303-object-catalog.json"),
        OpenRtbVersion::V2_6_202309 => include_str!("../specs/openrtb-2.6-202309-object-catalog.json"),
        OpenRtbVersion::V2_6_202402 => include_str!("../specs/openrtb-2.6-202402-object-catalog.json"),
        OpenRtbVersion::V2_6_202409 => include_str!("../specs/openrtb-2.6-202409-object-catalog.json"),
        OpenRtbVersion::V2_6_202501 => include_str!("../specs/openrtb-2.6-202501-object-catalog.json"),
        OpenRtbVersion::V2_6_202505 => include_str!("../specs/openrtb-2.6-202505-object-catalog.json"),
        OpenRtbVersion::V3_0 => include_str!("../specs/openrtb-3.0-object-catalog.json"),
    }
}

fn load_all_catalogs() -> &'static Vec<(OpenRtbVersion, CanonicalObjectCatalog)> {
    static CATALOGS: OnceLock<Vec<(OpenRtbVersion, CanonicalObjectCatalog)>> = OnceLock::new();
    CATALOGS.get_or_init(|| {
        OpenRtbVersion::all()
            .iter()
            .map(|version| (*version, parse_catalog(*version, embedded_catalog(*version))))
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalogs_cover_all_known_versions() {
        assert_eq!(canonical_object_catalog_versions(), OpenRtbVersion::all());
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
    fn catalog_3_0_contains_request_item_field() {
        let field = canonical_field(OpenRtbVersion::V3_0, "Request", "item")
            .expect("3.0 canonical catalog should include Request.item");
        let root = canonical_object(OpenRtbVersion::V3_0, "Openrtb")
            .expect("3.0 canonical catalog should include Openrtb");

        assert_eq!(field.citation.canonical_source_file, "source.md");
        assert_eq!(field.citation.helper_source_file, "source.md");
        assert_eq!(root.citation.section, "Object: Openrtb");
    }
}