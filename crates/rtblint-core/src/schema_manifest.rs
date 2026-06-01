use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::OpenRtbVersion;

const SUPPORTED_SCHEMA_MANIFESTS: [OpenRtbVersion; 2] =
    [OpenRtbVersion::V2_5, OpenRtbVersion::V2_6_202505];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaCoverage {
    BootstrapVersionedDeltaPaths,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaManifest {
    pub kind: String,
    pub version: String,
    pub family: String,
    pub release_date: String,
    pub archive_path: String,
    pub source_of_truth: String,
    pub coverage: SchemaCoverage,
    pub paths: Vec<SchemaPathEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SchemaPathEntry {
    pub path: String,
    pub state: SchemaPathState,
    pub since: String,
    #[serde(default)]
    pub replacement_paths: Vec<String>,
    #[serde(default)]
    pub matched_rules: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SchemaPathState {
    Available,
    Deprecated,
    Removed,
    Moved,
}

pub fn schema_manifest_versions() -> &'static [OpenRtbVersion] {
    &SUPPORTED_SCHEMA_MANIFESTS
}

pub fn schema_manifest(version: OpenRtbVersion) -> Option<&'static SchemaManifest> {
    match version {
        OpenRtbVersion::V2_5 => Some(load_2_5_manifest()),
        OpenRtbVersion::V2_6_202505 => Some(load_2_6_latest_manifest()),
        _ => None,
    }
}

pub fn schema_path_entry(version: OpenRtbVersion, path: &str) -> Option<&'static SchemaPathEntry> {
    schema_manifest(version).and_then(|manifest| manifest.paths.iter().find(|entry| entry.path == path))
}

fn load_2_5_manifest() -> &'static SchemaManifest {
    static MANIFEST: OnceLock<SchemaManifest> = OnceLock::new();
    MANIFEST.get_or_init(|| load_manifest("openrtb-2.5-versioned-paths.json"))
}

fn load_2_6_latest_manifest() -> &'static SchemaManifest {
    static MANIFEST: OnceLock<SchemaManifest> = OnceLock::new();
    MANIFEST.get_or_init(|| load_manifest("openrtb-2.6-202505-versioned-paths.json"))
}

fn load_manifest(file_name: &str) -> SchemaManifest {
    let path = manifest_dir().join(file_name);
    let raw = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read schema manifest {}: {error}", path.display()));

    serde_json::from_str(&raw)
        .unwrap_or_else(|error| panic!("failed to parse schema manifest {}: {error}", path.display()))
}

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("specs")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_manifests_start_with_2_5_and_latest_2_6() {
        assert_eq!(
            schema_manifest_versions(),
            &[OpenRtbVersion::V2_5, OpenRtbVersion::V2_6_202505]
        );
        assert!(schema_manifest(OpenRtbVersion::V2_4).is_none());
    }

    #[test]
    fn manifest_2_5_marks_bidrequest_source_available() {
        let entry = schema_path_entry(OpenRtbVersion::V2_5, "bidrequest.source")
            .expect("2.5 schema manifest should include bidrequest.source");

        assert_eq!(entry.state, SchemaPathState::Available);
        assert_eq!(entry.since, "2.5");
    }

    #[test]
    fn latest_2_6_manifest_marks_gdpr_move_and_video_deprecation() {
        let moved = schema_path_entry(OpenRtbVersion::V2_6_202505, "regs.ext.gdpr")
            .expect("latest 2.6 schema manifest should include regs.ext.gdpr");
        let deprecated = schema_path_entry(OpenRtbVersion::V2_6_202505, "imp.video.placement")
            .expect("latest 2.6 schema manifest should include imp.video.placement");
        let replacement = schema_path_entry(OpenRtbVersion::V2_6_202505, "imp.video.plcmt")
            .expect("latest 2.6 schema manifest should include imp.video.plcmt");

        assert_eq!(moved.state, SchemaPathState::Moved);
        assert_eq!(moved.replacement_paths, vec!["regs.gdpr"]);
        assert_eq!(deprecated.state, SchemaPathState::Deprecated);
        assert_eq!(replacement.state, SchemaPathState::Available);
    }
}