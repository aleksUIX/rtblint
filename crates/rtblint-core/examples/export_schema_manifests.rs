use std::collections::BTreeSet;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use rtblint_core::{
    path_status, version_profile, version_profiles, OpenRtbFamily, OpenRtbVersion, PathStateKind,
    SchemaCoverage, SchemaManifest, SchemaPathEntry, SchemaPathState,
};

const TARGET_VERSIONS: [OpenRtbVersion; 3] = [
    OpenRtbVersion::V2_5,
    OpenRtbVersion::V2_6_202505,
    OpenRtbVersion::V2_6_202606,
];

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("crates/rtblint-core/specs"));

    fs::create_dir_all(&output_dir)?;

    for version in TARGET_VERSIONS {
        let manifest = build_manifest(version)?;
        let file_name = format!("openrtb-{}-versioned-paths.json", version.id());
        let output_path = output_dir.join(file_name);
        let json = serde_json::to_string_pretty(&manifest)?;
        fs::write(output_path, format!("{}\n", json))?;
    }

    Ok(())
}

fn build_manifest(version: OpenRtbVersion) -> Result<SchemaManifest, Box<dyn Error>> {
    let profile = version_profile(version)
        .ok_or_else(|| format!("missing version profile for {}", version.id()))?;
    let family = match version.family() {
        OpenRtbFamily::TwoX => "2.x",
        OpenRtbFamily::ThreeZero => "3.0",
    };

    let mut known_paths = BTreeSet::new();
    for profile in version_profiles()
        .iter()
        .filter(|profile| profile.version.family() == version.family())
        .filter(|profile| profile.version <= version)
    {
        for rule in profile.rules {
            for path in rule.paths.iter().chain(rule.replacement_paths.iter()) {
                known_paths.insert((*path).to_string());
            }
        }
    }

    let paths = known_paths
        .into_iter()
        .filter_map(|path| build_entry(version, path))
        .collect::<Vec<_>>();

    Ok(SchemaManifest {
        kind: String::from("openrtb_versioned_schema_manifest"),
        version: String::from(version.id()),
        family: String::from(family),
        release_date: String::from(profile.release_date),
        archive_path: String::from(profile.archive_path),
        source_of_truth: String::from(
            "Canonical IAB source is the archived spec referenced by archive_path; this manifest is a structured derivative for validator use.",
        ),
        coverage: SchemaCoverage::BootstrapVersionedDeltaPaths,
        paths,
    })
}

fn build_entry(version: OpenRtbVersion, path: String) -> Option<SchemaPathEntry> {
    let status = path_status(version, &path);
    let state = match status.kind {
        PathStateKind::Available => SchemaPathState::Available,
        PathStateKind::Deprecated => SchemaPathState::Deprecated,
        PathStateKind::Removed => SchemaPathState::Removed,
        PathStateKind::Moved => SchemaPathState::Moved,
        PathStateKind::Unknown | PathStateKind::NotYetAvailable => return None,
    };
    let matched_rules = status
        .matched_rules
        .iter()
        .map(|matched| String::from(matched.rule.code))
        .collect::<Vec<_>>();

    Some(SchemaPathEntry {
        path,
        state,
        since: String::from(status.since.unwrap_or(version).id()),
        replacement_paths: status
            .replacement_paths
            .into_iter()
            .map(String::from)
            .collect(),
        matched_rules,
    })
}