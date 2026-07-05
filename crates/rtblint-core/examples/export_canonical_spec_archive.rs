use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rtblint_core::{version_profiles, OpenRtbFamily, VersionProfile};

fn main() -> Result<(), Box<dyn Error>> {
    let archive_root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".openrtb-specs"));
    let output_dir = env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| archive_root.join("canonical"));

    fs::create_dir_all(&output_dir)?;

    let mut readme = String::from(
        "# OpenRTB Canonical Spec Archive\n\nThis directory is the canonical validation source layer for local OpenRTB work. Each version folder contains an exact source copy of the archived IAB spec, a manifest with provenance and checksums, and helper text extracted from PDFs for search and later rule encoding.\n\n- Canonical source of truth: `source.pdf` or `source.md`\n- Helper files: `source-layout.txt`, `manifest.json`\n- Derived summaries in `.openrtb-specs/distilled/` are convenience references only and are not authoritative.\n\n## Versions\n\n",
    );
    let mut index_entries = Vec::new();

    for profile in version_profiles() {
        let version_dir = output_dir.join(profile.version.id());
        fs::create_dir_all(&version_dir)?;

        let original_relative_path = relative_archive_path(profile.archive_path);
        let source_path = archive_root.join(&original_relative_path);
        if !source_path.exists() {
            return Err(format!("missing archived source: {}", source_path.display()).into());
        }

        let extension = source_path
            .extension()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("missing file extension: {}", source_path.display()))?
            .to_ascii_lowercase();

        let canonical_source_name = match extension.as_str() {
            "pdf" => "source.pdf",
            "md" => "source.md",
            other => {
                return Err(format!(
                    "unsupported archived source extension `{other}` for {}",
                    source_path.display()
                )
                .into())
            }
        };

        let canonical_source_path = version_dir.join(canonical_source_name);
        fs::copy(&source_path, &canonical_source_path)?;

        let source_sha256 = sha256(&canonical_source_path)?;
        let page_count = if extension == "pdf" {
            Some(pdf_page_count(&canonical_source_path)?)
        } else {
            None
        };

        let extracted_text = if extension == "pdf" {
            let extracted_path = version_dir.join("source-layout.txt");
            run_command(
                Command::new("pdftotext")
                    .arg("-layout")
                    .arg(&canonical_source_path)
                    .arg(&extracted_path),
                "pdftotext",
            )?;

            Some(FileDigest {
                name: String::from("source-layout.txt"),
                sha256: sha256(&extracted_path)?,
            })
        } else {
            None
        };

        let manifest = render_manifest(
            profile,
            &original_relative_path,
            canonical_source_name,
            &source_sha256,
            page_count,
            extracted_text.as_ref(),
        );
        fs::write(version_dir.join("manifest.json"), manifest)?;

        readme.push_str(&format!(
            "- `{}` -> `{}/manifest.json`\n",
            profile.version.id(),
            profile.version.id()
        ));
        index_entries.push(IndexEntry {
            version: String::from(profile.version.id()),
            manifest_path: format!("{}/manifest.json", profile.version.id()),
        });
    }

    fs::write(output_dir.join("README.md"), readme)?;
    fs::write(output_dir.join("index.json"), render_index(&index_entries))?;

    Ok(())
}

struct FileDigest {
    name: String,
    sha256: String,
}

struct IndexEntry {
    version: String,
    manifest_path: String,
}

fn relative_archive_path(path: &str) -> PathBuf {
    match path.strip_prefix(".openrtb-specs/") {
        Some(relative) => PathBuf::from(relative),
        None => PathBuf::from(path),
    }
}

fn render_manifest(
    profile: &VersionProfile,
    original_relative_path: &Path,
    canonical_source_name: &str,
    source_sha256: &str,
    page_count: Option<u32>,
    extracted_text: Option<&FileDigest>,
) -> String {
    let family = match profile.version.family() {
        OpenRtbFamily::TwoX => "2.x",
        OpenRtbFamily::ThreeZero => "3.0",
    };

    let page_count_value = page_count
        .map(|count| count.to_string())
        .unwrap_or_else(|| String::from("null"));
    let extracted_name = extracted_text
        .map(|file| json_string(&file.name))
        .unwrap_or_else(|| String::from("null"));
    let extracted_sha256 = extracted_text
        .map(|file| json_string(&file.sha256))
        .unwrap_or_else(|| String::from("null"));

    let mut out = String::from("{\n");
    out.push_str(&format!(
        "  \"version\": {},\n",
        json_string(profile.version.id())
    ));
    out.push_str(&format!("  \"family\": {},\n", json_string(family)));
    out.push_str(&format!(
        "  \"release_date\": {},\n",
        json_string(profile.release_date)
    ));
    out.push_str(&format!(
        "  \"summary\": {},\n",
        json_string(profile.summary)
    ));
    out.push_str("  \"canonical\": {\n");
    out.push_str("    \"authoritative\": true,\n");
    out.push_str(&format!(
        "    \"archive_path\": {},\n",
        json_string(&path_to_string(original_relative_path))
    ));
    out.push_str(&format!(
        "    \"source_file\": {},\n",
        json_string(canonical_source_name)
    ));
    out.push_str(&format!(
        "    \"source_sha256\": {},\n",
        json_string(source_sha256)
    ));
    out.push_str(&format!(
        "    \"source_format\": {},\n",
        json_string(source_format(canonical_source_name))
    ));
    out.push_str(&format!("    \"page_count\": {},\n", page_count_value));
    out.push_str(&format!(
        "    \"extracted_text_file\": {},\n",
        extracted_name
    ));
    out.push_str(&format!(
        "    \"extracted_text_sha256\": {}\n",
        extracted_sha256
    ));
    out.push_str("  },\n");
    out.push_str("  \"notes\": {\n");
    out.push_str(
        "    \"validation_source_of_truth\": \"The raw canonical source file is authoritative. Derived text and distilled summaries are helpers only.\",\n",
    );
    out.push_str(&format!(
        "    \"distilled_summary_path\": {}\n",
        json_string(&format!(
            ".openrtb-specs/distilled/openrtb-{}-rules.md",
            profile.version.id()
        ))
    ));
    out.push_str("  }\n");
    out.push_str("}\n");
    out
}

fn render_index(entries: &[IndexEntry]) -> String {
    let items = entries
        .iter()
        .map(|entry| {
            format!(
                "    {{\n      \"version\": {},\n      \"manifest_path\": {}\n    }}",
                json_string(&entry.version),
                json_string(&entry.manifest_path)
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");

    let mut out = String::from("{\n");
    out.push_str("  \"kind\": \"openrtb_canonical_archive\",\n");
    out.push_str("  \"entries\": [\n");
    out.push_str(&items);
    out.push_str("\n  ]\n");
    out.push_str("}\n");
    out
}

fn source_format(file_name: &str) -> &str {
    if file_name.ends_with(".pdf") {
        "pdf"
    } else {
        "markdown"
    }
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn json_string(value: &str) -> String {
    let mut escaped = String::from("\"");

    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(character),
        }
    }

    escaped.push('"');
    escaped
}

fn sha256(path: &Path) -> Result<String, Box<dyn Error>> {
    let output = Command::new("shasum")
        .arg("-a")
        .arg("256")
        .arg(path)
        .output()?;
    if !output.status.success() {
        return Err(format!(
            "shasum failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    stdout
        .split_whitespace()
        .next()
        .map(String::from)
        .ok_or_else(|| format!("missing shasum output for {}", path.display()).into())
}

fn pdf_page_count(path: &Path) -> Result<u32, Box<dyn Error>> {
    let output = Command::new("pdfinfo").arg(path).output()?;
    if !output.status.success() {
        return Err(format!(
            "pdfinfo failed for {}: {}",
            path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    let stdout = String::from_utf8(output.stdout)?;
    for line in stdout.lines() {
        if let Some(value) = line.strip_prefix("Pages:") {
            return Ok(value.trim().parse()?);
        }
    }

    Err(format!("page count missing for {}", path.display()).into())
}

fn run_command(command: &mut Command, command_name: &str) -> Result<(), Box<dyn Error>> {
    let output = command.output()?;
    if output.status.success() {
        return Ok(());
    }

    Err(format!(
        "{} failed: {}",
        command_name,
        String::from_utf8_lossy(&output.stderr).trim()
    )
    .into())
}
