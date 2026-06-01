use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use rtblint_core::{
    version_profile, CanonicalField, CanonicalObject, CanonicalObjectCatalog, CatalogCitation,
    OpenRtbFamily, OpenRtbVersion,
};

fn main() -> Result<(), Box<dyn Error>> {
    let canonical_root = env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".openrtb-specs/canonical"));
    let output_dir = env::args()
        .nth(2)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("crates/rtblint-core/specs"));

    fs::create_dir_all(&output_dir)?;

    for version in OpenRtbVersion::all() {
        let catalog = parse_catalog(&canonical_root, *version)?;
        let file_name = format!("openrtb-{}-object-catalog.json", version.id());
        let json = serde_json::to_string_pretty(&catalog)?;
        fs::write(output_dir.join(file_name), format!("{}\n", json))?;
    }

    Ok(())
}

fn parse_catalog(
    canonical_root: &Path,
    version: OpenRtbVersion,
) -> Result<CanonicalObjectCatalog, Box<dyn Error>> {
    let profile = version_profile(version)
        .ok_or_else(|| format!("missing version profile for {}", version.id()))?;

    if profile.archive_path.ends_with(".md") {
        return parse_markdown_catalog(canonical_root, version);
    }

    if version <= OpenRtbVersion::V2_2 {
        return parse_legacy_pdf_catalog(canonical_root, version);
    }

    parse_pdf_layout_catalog(canonical_root, version)
}

fn parse_markdown_catalog(
    canonical_root: &Path,
    version: OpenRtbVersion,
) -> Result<CanonicalObjectCatalog, Box<dyn Error>> {
    let profile = version_profile(version)
        .ok_or_else(|| format!("missing version profile for {}", version.id()))?;
    let source_path = canonical_root.join(version.id()).join("source.md");
    let raw = fs::read_to_string(&source_path)?;
    let lines = raw.lines().collect::<Vec<_>>();
    let mut objects = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        if let Some((section, name)) = parse_markdown_object_heading(lines[index]) {
            let end_index = find_next_markdown_object_heading(&lines, index + 1).unwrap_or(lines.len());
            let table_start =
                ((index + 1)..end_index).find(|candidate| lines[*candidate].contains("<table>"));
            let table_end = table_start.and_then(|start| {
                (start..end_index).find(|candidate| lines[*candidate].contains("</table>"))
            });
            let description_end = table_start.unwrap_or(end_index);
            let description = join_markdown_text(&lines[(index + 1)..description_end]);
            let fields = match (table_start, table_end) {
                (Some(start), Some(end)) => parse_markdown_table(
                    &lines,
                    start,
                    end,
                    &section,
                    "source.md",
                    "source.md",
                ),
                _ => Vec::new(),
            };

            objects.push(CanonicalObject {
                name,
                section: section.clone(),
                description,
                citation: CatalogCitation {
                    section,
                    canonical_source_file: String::from("source.md"),
                    helper_source_file: String::from("source.md"),
                    start_line: index + 1,
                    end_line: end_index,
                },
                fields,
            });
            index = end_index;
            continue;
        }

        index += 1;
    }

    Ok(build_catalog(profile, "source.md", "source.md", objects))
}

fn parse_pdf_layout_catalog(
    canonical_root: &Path,
    version: OpenRtbVersion,
) -> Result<CanonicalObjectCatalog, Box<dyn Error>> {
    let profile = version_profile(version)
        .ok_or_else(|| format!("missing version profile for {}", version.id()))?;
    let source_path = canonical_root.join(version.id()).join("source-layout.txt");
    let raw = fs::read_to_string(&source_path)?;
    let lines = raw.lines().collect::<Vec<_>>();
    let mut objects = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        if let Some((section, name)) = parse_pdf_object_heading(lines[index]) {
            let end_index = find_next_pdf_object_heading(&lines, index + 1).unwrap_or(lines.len());
            let header_index =
                ((index + 1)..end_index).find(|candidate| is_pdf_attribute_header(lines[*candidate]));
            let description_end = header_index.unwrap_or(end_index);
            let description = join_pdf_text(&lines[(index + 1)..description_end]);
            let fields = match header_index {
                Some(header_line) => parse_pdf_table(
                    &lines,
                    header_line,
                    end_index,
                    &section,
                    "source.pdf",
                    "source-layout.txt",
                ),
                None => Vec::new(),
            };

            objects.push(CanonicalObject {
                name,
                section: section.clone(),
                description,
                citation: CatalogCitation {
                    section,
                    canonical_source_file: String::from("source.pdf"),
                    helper_source_file: String::from("source-layout.txt"),
                    start_line: index + 1,
                    end_line: end_index,
                },
                fields,
            });
            index = end_index;
            continue;
        }

        index += 1;
    }

    Ok(build_catalog(profile, "source.pdf", "source-layout.txt", objects))
}

fn parse_legacy_pdf_catalog(
    canonical_root: &Path,
    version: OpenRtbVersion,
) -> Result<CanonicalObjectCatalog, Box<dyn Error>> {
    let profile = version_profile(version)
        .ok_or_else(|| format!("missing version profile for {}", version.id()))?;
    let source_path = canonical_root.join(version.id()).join("source-layout.txt");
    let raw = fs::read_to_string(&source_path)?;
    let lines = raw.lines().collect::<Vec<_>>();
    let mut objects = Vec::new();
    let mut index = 0usize;

    while index < lines.len() {
        if let Some((section, name)) = parse_legacy_pdf_object_heading(lines[index]) {
            let end_index =
                find_next_legacy_pdf_object_heading(&lines, index + 1).unwrap_or(lines.len());
            let header_index = ((index + 1)..end_index)
                .find(|candidate| is_legacy_pdf_field_header(lines[*candidate]));
            let description_end = header_index.unwrap_or(end_index);
            let description = join_pdf_text(&lines[(index + 1)..description_end]);
            let fields = match header_index {
                Some(header_line) => parse_legacy_pdf_table(
                    &lines,
                    header_line,
                    end_index,
                    &section,
                    "source.pdf",
                    "source-layout.txt",
                ),
                None => Vec::new(),
            };

            objects.push(CanonicalObject {
                name,
                section: section.clone(),
                description,
                citation: CatalogCitation {
                    section,
                    canonical_source_file: String::from("source.pdf"),
                    helper_source_file: String::from("source-layout.txt"),
                    start_line: index + 1,
                    end_line: end_index,
                },
                fields,
            });
            index = end_index;
            continue;
        }

        index += 1;
    }

    Ok(build_catalog(profile, "source.pdf", "source-layout.txt", objects))
}

fn build_catalog(
    profile: &rtblint_core::VersionProfile,
    canonical_source_file: &str,
    helper_source_file: &str,
    objects: Vec<CanonicalObject>,
) -> CanonicalObjectCatalog {
    let family = match profile.version.family() {
        OpenRtbFamily::TwoX => "2.x",
        OpenRtbFamily::ThreeZero => "3.0",
    };

    CanonicalObjectCatalog {
        kind: String::from("openrtb_canonical_object_catalog"),
        version: String::from(profile.version.id()),
        family: String::from(family),
        release_date: String::from(profile.release_date),
        archive_path: String::from(profile.archive_path),
        canonical_source_file: String::from(canonical_source_file),
        helper_source_file: String::from(helper_source_file),
        source_of_truth: String::from(
            "The canonical IAB source file is authoritative. Line citations refer to the helper source file used for structured extraction.",
        ),
        objects,
    }
}

fn parse_markdown_object_heading(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    let heading = trimmed.strip_prefix("### ")?;
    if let Some((section, remainder)) = heading.split_once(" - Object: ") {
        let name = remainder.split('<').next()?.trim();
        return Some((String::from(section.trim()), String::from(name)));
    }

    let remainder = heading.strip_prefix("Object:")?;
    let name = remainder.split('<').next()?.trim();
    Some((format!("Object: {}", name), String::from(name)))
}

fn find_next_markdown_object_heading(lines: &[&str], start: usize) -> Option<usize> {
    (start..lines.len()).find(|candidate| parse_markdown_object_heading(lines[*candidate]).is_some())
}

fn parse_markdown_table(
    lines: &[&str],
    table_start: usize,
    table_end: usize,
    section: &str,
    canonical_source_file: &str,
    helper_source_file: &str,
) -> Vec<CanonicalField> {
    let mut fields = Vec::new();
    let mut row_start = None;
    let mut row_buffer = String::new();

    for (line_index, line) in lines
        .iter()
        .enumerate()
        .take(table_end + 1)
        .skip(table_start)
    {
        let line = *line;
        if row_start.is_none() && line.contains("<tr>") {
            row_start = Some(line_index);
            row_buffer.clear();
        }

        if row_start.is_some() {
            row_buffer.push_str(line);
            row_buffer.push('\n');
        }

        if let Some(start_index) = row_start {
            if line.contains("</tr>") {
                if let Some(field) = build_markdown_field(
                    &row_buffer,
                    start_index + 1,
                    line_index + 1,
                    section,
                    canonical_source_file,
                    helper_source_file,
                ) {
                    fields.push(field);
                }
                row_start = None;
            }
        }
    }

    fields
}

fn build_markdown_field(
    row: &str,
    start_line: usize,
    end_line: usize,
    section: &str,
    canonical_source_file: &str,
    helper_source_file: &str,
) -> Option<CanonicalField> {
    let cells = extract_td_cells(row);
    if cells.len() < 3 {
        return None;
    }

    let name = cells[0].trim();
    if name.is_empty() || name == "Attribute" {
        return None;
    }

    Some(CanonicalField {
        name: String::from(name),
        type_spec: String::from(cells[1].trim()),
        description: String::from(cells[2].trim()),
        citation: CatalogCitation {
            section: String::from(section),
            canonical_source_file: String::from(canonical_source_file),
            helper_source_file: String::from(helper_source_file),
            start_line,
            end_line,
        },
    })
}

fn extract_td_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut remainder = row;

    while let Some(td_index) = remainder.find("<td") {
        remainder = &remainder[td_index..];
        let Some(tag_end) = remainder.find('>') else {
            break;
        };
        remainder = &remainder[(tag_end + 1)..];
        let Some(cell_end) = remainder.find("</td>") else {
            break;
        };
        cells.push(clean_html_text(&remainder[..cell_end]));
        remainder = &remainder[(cell_end + 5)..];
    }

    cells
}

fn clean_html_text(value: &str) -> String {
    let replaced = value
        .replace("<br>", " ")
        .replace("<br/>", " ")
        .replace("<br />", " ")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">");
    let mut out = String::new();
    let mut in_tag = false;

    for character in replaced.chars() {
        match character {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(character),
            _ => {}
        }
    }

    squash_whitespace(&out)
}

fn join_markdown_text(lines: &[&str]) -> String {
    let parts = lines
        .iter()
        .map(|line| clean_html_text(line))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>();

    parts.join(" ")
}

fn parse_pdf_object_heading(line: &str) -> Option<(String, String)> {
    let normalized = normalize_pdf_line(line);
    if normalized.contains("...") {
        return None;
    }

    let (section, name) = normalized.split_once(" Object: ")?;
    if !section.chars().all(|character| character.is_ascii_digit() || character == '.') {
        return None;
    }

    Some((String::from(section.trim()), String::from(name.trim())))
}

fn parse_legacy_pdf_object_heading(line: &str) -> Option<(String, String)> {
    let normalized = normalize_pdf_line(line);
    if normalized.contains("...") {
        return None;
    }

    let trimmed = normalized.trim();
    let mut parts = trimmed.split_whitespace();
    let section = parts.next()?;
    if !section.chars().all(|character| character.is_ascii_digit() || character == '.') {
        return None;
    }

    let remainder = trimmed[section.len()..].trim();
    let object_name = remainder.strip_suffix("Object")?.trim();
    Some((
        String::from(section),
        normalize_legacy_object_name(object_name),
    ))
}

fn find_next_pdf_object_heading(lines: &[&str], start: usize) -> Option<usize> {
    (start..lines.len()).find(|candidate| parse_pdf_object_heading(lines[*candidate]).is_some())
}

fn find_next_legacy_pdf_object_heading(lines: &[&str], start: usize) -> Option<usize> {
    (start..lines.len()).find(|candidate| parse_legacy_pdf_object_heading(lines[*candidate]).is_some())
}

fn is_pdf_attribute_header(line: &str) -> bool {
    let normalized = normalize_pdf_line(line);
    normalized.contains("Attribute")
        && normalized.contains("Type")
        && normalized.contains("Description")
}

fn is_legacy_pdf_field_header(line: &str) -> bool {
    let normalized = normalize_pdf_line(line);
    normalized.contains("Field")
        && normalized.contains("Scope")
        && normalized.contains("Type")
        && normalized.contains("Default")
        && normalized.contains("Description")
}

fn parse_pdf_table(
    lines: &[&str],
    header_index: usize,
    section_end: usize,
    section: &str,
    canonical_source_file: &str,
    helper_source_file: &str,
) -> Vec<CanonicalField> {
    let header = normalize_pdf_line(lines[header_index]);
    let type_start = header.find("Type").unwrap_or(29);
    let desc_start = header.find("Description").unwrap_or(type_start + 16);
    let mut fields = Vec::new();
    let mut current: Option<PendingPdfField> = None;

    for (line_index, line) in lines
        .iter()
        .enumerate()
        .take(section_end)
        .skip(header_index + 1)
    {
        let line = *line;
        let normalized = normalize_pdf_line(line);
        if normalized.is_empty() || is_pdf_noise_line(&normalized) {
            continue;
        }

        let indentation = count_leading_spaces(line);
        if indentation < type_start.saturating_sub(1) {
            let columns = split_multispace_parts(normalized.trim_start(), 3);
            if columns.is_empty() {
                continue;
            }

            let attribute = columns.first().cloned().unwrap_or_default();
            let type_spec = columns.get(1).cloned().unwrap_or_default();
            let description = columns.get(2).cloned().unwrap_or_default();
            if let Some(previous) = current.take() {
                fields.push(previous.finish(section, canonical_source_file, helper_source_file));
            }
            current = Some(PendingPdfField {
                name: attribute,
                type_spec,
                description,
                start_line: line_index + 1,
                end_line: line_index + 1,
            });
            continue;
        }

        if let Some(field) = current.as_mut() {
            if indentation >= desc_start.saturating_sub(1) {
                push_with_space(&mut field.description, normalized.trim());
                field.end_line = line_index + 1;
                continue;
            }

            let columns = split_multispace_parts(normalized.trim_start(), 2);
            if let Some(type_spec) = columns.first() {
                push_with_space(&mut field.type_spec, type_spec);
            }
            if let Some(description) = columns.get(1) {
                push_with_space(&mut field.description, description);
            }
            field.end_line = line_index + 1;
        }
    }

    if let Some(previous) = current {
        fields.push(previous.finish(section, canonical_source_file, helper_source_file));
    }

    fields
}

fn parse_legacy_pdf_table(
    lines: &[&str],
    header_index: usize,
    section_end: usize,
    section: &str,
    canonical_source_file: &str,
    helper_source_file: &str,
) -> Vec<CanonicalField> {
    let header = normalize_pdf_line(lines[header_index]);
    let scope_start = header.find("Scope").unwrap_or(20);
    let type_start = header.find("Type").unwrap_or(scope_start + 16);
    let default_start = header.find("Default").unwrap_or(type_start + 10);
    let desc_start = header.find("Description").unwrap_or(default_start + 10);
    let mut fields = Vec::new();
    let mut current: Option<PendingLegacyPdfField> = None;

    for (line_index, line) in lines
        .iter()
        .enumerate()
        .take(section_end)
        .skip(header_index + 1)
    {
        let line = *line;
        let normalized = normalize_pdf_line(line);
        if normalized.is_empty() || is_pdf_noise_line(&normalized) {
            continue;
        }

        let indentation = count_leading_spaces(line);
        if indentation < scope_start.saturating_sub(1) {
            let columns = split_multispace_parts(normalized.trim_start(), 5);
            if columns.is_empty() {
                continue;
            }

            let name = columns.first().cloned().unwrap_or_default();
            let scope = columns.get(1).cloned().unwrap_or_default();
            let value_type = columns.get(2).cloned().unwrap_or_default();
            let default_value = columns.get(3).cloned().unwrap_or_default();
            let description = columns.get(4).cloned().unwrap_or_default();
            if let Some(previous) = current.take() {
                fields.push(previous.finish(section, canonical_source_file, helper_source_file));
            }
            current = Some(PendingLegacyPdfField {
                name: normalize_legacy_field_name(&name),
                scope,
                value_type,
                default_value,
                description,
                start_line: line_index + 1,
                end_line: line_index + 1,
            });
            continue;
        }

        if let Some(field) = current.as_mut() {
            let characters = normalized.chars().collect::<Vec<_>>();
            let scope = squash_whitespace(&collect_char_range(&characters, scope_start, type_start));
            let value_type = squash_whitespace(&collect_char_range(&characters, type_start, default_start));
            let default_value = squash_whitespace(&collect_char_range(&characters, default_start, desc_start));
            let description = squash_whitespace(&collect_char_range(
                &characters,
                desc_start,
                characters.len(),
            ));

            push_with_space(&mut field.scope, &scope);
            push_with_space(&mut field.value_type, &value_type);
            push_with_space(&mut field.default_value, &default_value);
            push_with_space(&mut field.description, &description);
            field.end_line = line_index + 1;
        }
    }

    if let Some(previous) = current {
        fields.push(previous.finish(section, canonical_source_file, helper_source_file));
    }

    fields
}

struct PendingPdfField {
    name: String,
    type_spec: String,
    description: String,
    start_line: usize,
    end_line: usize,
}

impl PendingPdfField {
    fn finish(
        self,
        section: &str,
        canonical_source_file: &str,
        helper_source_file: &str,
    ) -> CanonicalField {
        CanonicalField {
            name: self.name,
            type_spec: self.type_spec,
            description: self.description,
            citation: CatalogCitation {
                section: String::from(section),
                canonical_source_file: String::from(canonical_source_file),
                helper_source_file: String::from(helper_source_file),
                start_line: self.start_line,
                end_line: self.end_line,
            },
        }
    }
}

struct PendingLegacyPdfField {
    name: String,
    scope: String,
    value_type: String,
    default_value: String,
    description: String,
    start_line: usize,
    end_line: usize,
}

impl PendingLegacyPdfField {
    fn finish(
        self,
        section: &str,
        canonical_source_file: &str,
        helper_source_file: &str,
    ) -> CanonicalField {
        let mut type_parts = Vec::new();
        if !self.scope.is_empty() {
            type_parts.push(format!("scope: {}", self.scope));
        }
        if !self.value_type.is_empty() {
            type_parts.push(format!("type: {}", self.value_type));
        }
        if !self.default_value.is_empty() && self.default_value != "-" {
            type_parts.push(format!("default: {}", self.default_value));
        }

        CanonicalField {
            name: self.name,
            type_spec: type_parts.join("; "),
            description: self.description,
            citation: CatalogCitation {
                section: String::from(section),
                canonical_source_file: String::from(canonical_source_file),
                helper_source_file: String::from(helper_source_file),
                start_line: self.start_line,
                end_line: self.end_line,
            },
        }
    }
}

fn split_multispace_parts(value: &str, max_parts: usize) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut spaces = 0usize;

    for character in value.chars() {
        if character == ' ' {
            spaces += 1;
            continue;
        }

        if spaces >= 2 && parts.len() + 1 < max_parts {
            if !current.trim().is_empty() {
                parts.push(squash_whitespace(&current));
                current.clear();
            }
        } else if spaces > 0 {
            current.push(' ');
        }

        spaces = 0;
        current.push(character);
    }

    if !current.trim().is_empty() {
        parts.push(squash_whitespace(&current));
    }

    parts
}

fn count_leading_spaces(line: &str) -> usize {
    line.trim_start_matches('\u{c}')
        .chars()
        .take_while(|character| *character == ' ')
        .count()
}

fn normalize_pdf_line(line: &str) -> String {
    line.trim_start_matches('\u{c}').trim_end().to_string()
}

fn normalize_legacy_object_name(value: &str) -> String {
    match squash_whitespace(value).as_str() {
        "Bid Request" => String::from("BidRequest"),
        "Bid Response" => String::from("BidResponse"),
        "Impression" => String::from("Imp"),
        "Seat Bid" => String::from("SeatBid"),
        other => other
            .split_whitespace()
            .map(capitalize_token)
            .collect::<Vec<_>>()
            .join(""),
    }
}

fn normalize_legacy_field_name(value: &str) -> String {
    squash_whitespace(value)
        .trim_matches(|character: char| character == '“' || character == '”' || character == '"')
        .to_string()
}

fn capitalize_token(token: &str) -> String {
    let cleaned = token.trim_matches(|character: char| !character.is_alphanumeric());
    let mut characters = cleaned.chars();
    let Some(first) = characters.next() else {
        return String::new();
    };

    let mut out = first.to_uppercase().collect::<String>();
    out.push_str(&characters.as_str().to_ascii_lowercase());
    out
}

fn collect_char_range(characters: &[char], start: usize, end: usize) -> String {
    let actual_start = start.min(characters.len());
    let actual_end = end.min(characters.len());
    characters[actual_start..actual_end].iter().collect::<String>()
}

fn is_pdf_noise_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("www.iab.com/openrtb")
        || trimmed.starts_with("Page ")
        || trimmed.starts_with("OpenRTB API Specification Version")
        || trimmed.starts_with("OPENRTB API Specification Version")
        || trimmed.ends_with("IAB Technology Lab")
        || trimmed.ends_with("RTB Project")
        || trimmed.starts_with("RTB Project")
}

fn join_pdf_text(lines: &[&str]) -> String {
    let parts = lines
        .iter()
        .map(|line| normalize_pdf_line(line))
        .filter(|line| !line.is_empty())
        .filter(|line| !is_pdf_noise_line(line))
        .map(|line| squash_whitespace(&line))
        .collect::<Vec<_>>();

    parts.join(" ")
}

fn push_with_space(target: &mut String, value: &str) {
    if value.is_empty() {
        return;
    }

    if !target.is_empty() {
        target.push(' ');
    }
    target.push_str(value);
}

fn squash_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}