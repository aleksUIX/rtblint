use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

use rtblint_core::{version_profiles, OpenRtbFamily, VersionRuleKind};

fn main() -> Result<(), Box<dyn Error>> {
    let output_dir = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".openrtb-specs/distilled"));

    fs::create_dir_all(&output_dir)?;

    let mut index = String::from(
        "# OpenRTB Distilled Version Rules\n\nThis directory mirrors the curated version profiles in rtblint-core so they can be reviewed outside Rust source. Each file summarizes the release, points to the archived source spec, and lists the distilled rule deltas captured in the code registry. These summaries are derived aids, not canonical spec copies.\n\n## Files\n\n",
    );

    for profile in version_profiles() {
        let file_name = format!("openrtb-{}-rules.md", profile.version.id());
        let output_path = output_dir.join(&file_name);
        fs::write(&output_path, render_profile(profile))?;

        index.push_str(&format!("- {} -> {}\n", profile.version.id(), file_name));
    }

    index.push_str(
        "\n## Source Of Truth\n\n- Distilled from: crates/rtblint-core/src/version_rules.rs\n- Canonical source files live under .openrtb-specs/canonical/\n- Raw archived specs live in sibling folders under .openrtb-specs/\n",
    );
    fs::write(output_dir.join("README.md"), index)?;

    Ok(())
}

fn render_profile(profile: &rtblint_core::VersionProfile) -> String {
    let mut out = String::new();
    let family = match profile.version.family() {
        OpenRtbFamily::TwoX => "2.x",
        OpenRtbFamily::ThreeZero => "3.0",
    };

    out.push_str(&format!(
        "# OpenRTB {} Distilled Rules\n\n- Family: {}\n- Release date: {}\n- Archived source: {}\n- Summary: {}\n- Distilled from: crates/rtblint-core/src/version_rules.rs\n- Canonical source: .openrtb-specs/canonical/{}/\n- Status: Derived summary, not a canonical spec copy\n\n",
        profile.version.id(),
        family,
        profile.release_date,
        profile.archive_path,
        profile.summary,
        profile.version.id(),
    ));

    out.push_str("## Rules\n\n");
    if profile.rules.is_empty() {
        out.push_str(
            "No additional release-specific deltas are recorded for this profile. This version acts as a baseline tag in the distilled registry.\n",
        );
        return out;
    }

    for (index, rule) in profile.rules.iter().enumerate() {
        out.push_str(&format!("### {}. {}\n", index + 1, rule.code));
        out.push_str(&format!("- Kind: {}\n", rule_kind(rule.kind)));
        out.push_str(&format!("- Paths: {}\n", join_or_none(rule.paths)));
        out.push_str(&format!(
            "- Replacement paths: {}\n",
            join_or_none(rule.replacement_paths)
        ));
        out.push_str(&format!("- Section: {}\n", rule.section));
        out.push_str(&format!("- Source: {}\n", rule.source));
        out.push_str(&format!("- Summary: {}\n\n", rule.summary));
    }

    out
}

fn join_or_none(values: &[&str]) -> String {
    if values.is_empty() {
        return String::from("None");
    }

    values.join(", ")
}

fn rule_kind(kind: VersionRuleKind) -> &'static str {
    match kind {
        VersionRuleKind::AddedField => "added_field",
        VersionRuleKind::AddedObject => "added_object",
        VersionRuleKind::AddedMacro => "added_macro",
        VersionRuleKind::AddedHeader => "added_header",
        VersionRuleKind::AddedList => "added_list",
        VersionRuleKind::AddedGuidance => "added_guidance",
        VersionRuleKind::AddedBehavior => "added_behavior",
        VersionRuleKind::DeprecatedField => "deprecated_field",
        VersionRuleKind::RemovedField => "removed_field",
        VersionRuleKind::MovedField => "moved_field",
        VersionRuleKind::CorrectedField => "corrected_field",
        VersionRuleKind::StructuralShift => "structural_shift",
    }
}
