use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

use crate::OpenRtbVersion;

const SUPPORTED_SCHEMA_MANIFESTS: [OpenRtbVersion; 3] = [
    OpenRtbVersion::V2_5,
    OpenRtbVersion::V2_6_202505,
    OpenRtbVersion::V2_6_202606,
];

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
        OpenRtbVersion::V2_6_202505 => Some(load_2_6_202505_manifest()),
        OpenRtbVersion::V2_6_202606 => Some(load_2_6_latest_manifest()),
        _ => None,
    }
}

pub fn schema_path_entry(version: OpenRtbVersion, path: &str) -> Option<&'static SchemaPathEntry> {
    schema_manifest(version).and_then(|manifest| manifest.paths.iter().find(|entry| entry.path == path))
}

/// Manifests are embedded at compile time, like the canonical object
/// catalogs, so the crate carries no runtime filesystem dependency and runs
/// unchanged in WASM and other no-fs targets.
fn load_2_5_manifest() -> &'static SchemaManifest {
    static MANIFEST: OnceLock<SchemaManifest> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        parse_manifest(
            "openrtb-2.5-versioned-paths.json",
            include_str!("../specs/openrtb-2.5-versioned-paths.json"),
        )
    })
}

fn load_2_6_202505_manifest() -> &'static SchemaManifest {
    static MANIFEST: OnceLock<SchemaManifest> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        parse_manifest(
            "openrtb-2.6-202505-versioned-paths.json",
            include_str!("../specs/openrtb-2.6-202505-versioned-paths.json"),
        )
    })
}

fn load_2_6_latest_manifest() -> &'static SchemaManifest {
    static MANIFEST: OnceLock<SchemaManifest> = OnceLock::new();
    MANIFEST.get_or_init(|| {
        parse_manifest(
            "openrtb-2.6-202606-versioned-paths.json",
            include_str!("../specs/openrtb-2.6-202606-versioned-paths.json"),
        )
    })
}

fn parse_manifest(file_name: &str, raw: &str) -> SchemaManifest {
    serde_json::from_str(raw)
        .unwrap_or_else(|error| panic!("failed to parse schema manifest {file_name}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_manifests_cover_2_5_and_tracked_2_6_snapshots() {
        assert_eq!(
            schema_manifest_versions(),
            &[
                OpenRtbVersion::V2_5,
                OpenRtbVersion::V2_6_202505,
                OpenRtbVersion::V2_6_202606
            ]
        );
        assert!(schema_manifest(OpenRtbVersion::V2_4).is_none());
    }

    #[test]
    fn latest_2_6_manifest_includes_content_liveness_fields() {
        let realtime = schema_path_entry(OpenRtbVersion::V2_6_202606, "content.realtime")
            .expect("2.6-202606 schema manifest should include content.realtime");

        assert_eq!(realtime.state, SchemaPathState::Available);
        assert_eq!(realtime.since, "2.6-202606");
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