use std::env;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};

use rtblint_core::catalog_extract::{extract_value_set, resolve_child_object};
use rtblint_core::{
    version_profile, CanonicalField, CanonicalObject, CanonicalObjectCatalog, CatalogCitation,
    CatalogValueSet, OpenRtbFamily, OpenRtbVersion,
};

/// Parsed object carrying raw spec prose, before enrichment strips it.
struct RawObject {
    name: String,
    section: String,
    citation: CatalogCitation,
    fields: Vec<RawField>,
}

struct RawField {
    name: String,
    type_spec: String,
    description: String,
    citation: CatalogCitation,
}

/// Converts raw parsed objects into the shipped catalog schema: computes the
/// structured validation fields from the description prose, then drops the
/// prose so no verbatim spec text leaves the generation pipeline.
fn enrich_and_strip(raw_objects: Vec<RawObject>) -> Vec<CanonicalObject> {
    let object_names: Vec<String> = raw_objects
        .iter()
        .map(|object| object.name.clone())
        .collect();

    raw_objects
        .into_iter()
        .map(|raw_object| CanonicalObject {
            name: raw_object.name,
            section: raw_object.section,
            citation: raw_object.citation,
            fields: raw_object
                .fields
                .into_iter()
                .map(|raw_field| enrich_field(raw_field, &object_names))
                .collect(),
        })
        .collect()
}

fn enrich_field(raw_field: RawField, object_names: &[String]) -> CanonicalField {
    let child_object = if raw_field.type_spec.to_ascii_lowercase().contains("object") {
        resolve_child_object(&raw_field.description, &raw_field.name, object_names)
    } else {
        None
    };

    let mut adcom_list = None;
    let mut value_set = None;
    if let Some(extracted) = extract_value_set(&raw_field.description) {
        if let Some(list_name) = extracted.adcom_list {
            adcom_list = Some(String::from(list_name));
        } else {
            value_set = Some(CatalogValueSet {
                values: extracted.values,
                minimum_inclusive: extracted.minimum_inclusive,
            });
        }
    }

    CanonicalField {
        name: raw_field.name,
        type_spec: raw_field.type_spec,
        child_object,
        adcom_list,
        value_set,
        citation: raw_field.citation,
    }
}

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
            let end_index =
                find_next_markdown_object_heading(&lines, index + 1).unwrap_or(lines.len());
            let table_start =
                ((index + 1)..end_index).find(|candidate| lines[*candidate].contains("<table>"));
            let table_end = table_start.and_then(|start| {
                (start..end_index).find(|candidate| lines[*candidate].contains("</table>"))
            });
            let pipe_table_start = find_pipe_table_header(&lines, index + 1, end_index);
            let fields = match (table_start, table_end, pipe_table_start) {
                (Some(start), Some(end), pipe) if pipe.is_none() || pipe > Some(start) => {
                    parse_markdown_table(&lines, start, end, &section, "source.md", "source.md")
                }
                (_, _, Some(pipe)) => parse_markdown_pipe_table(
                    &lines,
                    pipe,
                    end_index,
                    &section,
                    "source.md",
                    "source.md",
                ),
                _ => Vec::new(),
            };

            objects.push(RawObject {
                name,
                section: section.clone(),
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
            let header_index = ((index + 1)..end_index)
                .find(|candidate| is_pdf_attribute_header(lines[*candidate]));
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

            objects.push(RawObject {
                name,
                section: section.clone(),
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

    Ok(build_catalog(
        profile,
        "source.pdf",
        "source-layout.txt",
        objects,
    ))
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

            objects.push(RawObject {
                name,
                section: section.clone(),
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

    Ok(build_catalog(
        profile,
        "source.pdf",
        "source-layout.txt",
        objects,
    ))
}

/// True when a line opens a new numbered section ("5.1 Content Categories"),
/// which ends whatever field table was being read. Headings sit at the left
/// margin; a wrapped description that happens to start with a cross-reference
/// ("6.9 Video Start Delay for generic placement values") is indented under the
/// description column and must not end the table.
fn is_numbered_section_heading(raw_line: &str, normalized: &str) -> bool {
    if count_leading_spaces(raw_line) > 1 {
        return false;
    }

    let trimmed = normalized.trim();
    let mut parts = trimmed.split_whitespace();
    let Some(number) = parts.next() else {
        return false;
    };

    if !number
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
        || !number
            .chars()
            .all(|character| character.is_ascii_digit() || character == '.')
    {
        return false;
    }

    parts
        .next()
        .is_some_and(|word| word.chars().next().is_some_and(char::is_uppercase))
}

/// True when a lone token in the name column is the tail of a field name the
/// PDF wrapped, rather than a word of prose.
fn is_wrapped_name_fragment(candidate: &str) -> bool {
    !candidate.is_empty()
        && candidate.len() <= 12
        && candidate
            .chars()
            .all(|character| character.is_ascii_lowercase() || character.is_ascii_digit())
}

/// True when an object name looks like a spec identifier rather than prose. The
/// PDF parsers occasionally pick up a sentence, heading, or note as if it were
/// an object; those entries are junk and must never reach a catalog, because
/// build.rs derives shape and required-ness from whatever ships.
fn is_object_identifier(name: &str) -> bool {
    name.chars()
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic())
        && name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

/// Field names in every OpenRTB version are lowercase identifiers. PDF rows
/// sometimes capitalize the first letter at a line break ("Id", "Osv"), which
/// is worth repairing; anything else that is not a lowercase identifier is a
/// mis-extraction and gets dropped.
fn normalize_field_identifier(name: &str) -> Option<String> {
    let mut characters = name.chars();
    let first = characters.next()?;
    if !first.is_ascii_alphabetic() {
        return None;
    }

    if !characters.clone().all(|character| {
        character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
    }) {
        return None;
    }

    Some(format!(
        "{}{}",
        first.to_ascii_lowercase(),
        characters.collect::<String>()
    ))
}

/// Drops mis-extracted objects and fields before enrichment, so a regeneration
/// reproduces the shipped catalogs instead of reintroducing prose entries.
fn discard_mis_extracted(objects: Vec<RawObject>) -> Vec<RawObject> {
    objects
        .into_iter()
        .filter(|object| is_object_identifier(&object.name))
        .map(|mut object| {
            object
                .fields
                .retain_mut(|field| match normalize_field_identifier(&field.name) {
                    Some(normalized) => {
                        field.name = normalized;
                        true
                    }
                    None => false,
                });
            // A capitalized duplicate ("Id" alongside "id") collapses onto the
            // same name once repaired; keep the first definition.
            let mut seen = Vec::new();
            object.fields.retain(|field| {
                if seen.iter().any(|name| name == &field.name) {
                    return false;
                }
                seen.push(field.name.clone());
                true
            });
            object
        })
        .collect()
}

fn build_catalog(
    profile: &rtblint_core::VersionProfile,
    canonical_source_file: &str,
    helper_source_file: &str,
    objects: Vec<RawObject>,
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
        objects: enrich_and_strip(discard_mis_extracted(objects)),
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
    (start..lines.len())
        .find(|candidate| parse_markdown_object_heading(lines[*candidate]).is_some())
}

/// Finds the header row of a GitHub-style pipe table: a `|`-prefixed line
/// immediately followed by a `|---|---|` separator row.
fn find_pipe_table_header(lines: &[&str], start: usize, end: usize) -> Option<usize> {
    (start..end.min(lines.len().saturating_sub(1))).find(|candidate| {
        lines[*candidate].trim_start().starts_with('|')
            && is_pipe_separator_row(lines[candidate + 1])
    })
}

fn is_pipe_separator_row(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('|')
        && trimmed.contains('-')
        && trimmed
            .chars()
            .all(|character| matches!(character, '|' | '-' | ':' | ' '))
}

fn parse_markdown_pipe_table(
    lines: &[&str],
    header_index: usize,
    section_end: usize,
    section: &str,
    canonical_source_file: &str,
    helper_source_file: &str,
) -> Vec<RawField> {
    let mut fields = Vec::new();

    for (line_index, line) in lines
        .iter()
        .enumerate()
        .take(section_end)
        .skip(header_index + 2)
    {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            break;
        }

        if is_pipe_separator_row(trimmed) {
            continue;
        }

        let Some((name, type_spec, description)) = split_pipe_row(trimmed) else {
            continue;
        };

        let name = clean_html_text(&name);
        let name = name.trim_matches('`').trim();
        if name.is_empty() || name.eq_ignore_ascii_case("attribute") {
            continue;
        }

        fields.push(RawField {
            name: String::from(name),
            type_spec: clean_html_text(&type_spec),
            description: clean_html_text(&description),
            citation: CatalogCitation {
                section: String::from(section),
                canonical_source_file: String::from(canonical_source_file),
                helper_source_file: String::from(helper_source_file),
                start_line: line_index + 1,
                end_line: line_index + 1,
            },
        });
    }

    fields
}

/// Splits a `|name|type|description|` row into its three cells, keeping any
/// extra `|` characters inside the description cell.
fn split_pipe_row(row: &str) -> Option<(String, String, String)> {
    let inner = row.strip_prefix('|')?;
    let inner = inner.strip_suffix('|').unwrap_or(inner);
    let mut cells = inner.splitn(3, '|');
    let name = cells.next()?.trim().to_string();
    let type_spec = cells.next()?.trim().to_string();
    let description = cells.next()?.trim().to_string();
    Some((name, type_spec, description))
}

fn parse_markdown_table(
    lines: &[&str],
    table_start: usize,
    table_end: usize,
    section: &str,
    canonical_source_file: &str,
    helper_source_file: &str,
) -> Vec<RawField> {
    // The IAB HTML tables are not well formed: several of them misnest or swap
    // <tr> and </tr>, so grouping cells by row silently loses every field after
    // the break (the 2.6-202409 Site table lost 6 of 18). Read the cells as a
    // stream instead and start a new field whenever a cell is nothing but a
    // <code>identifier</code>, which is exactly how the attribute column is
    // written.
    let mut fields = Vec::new();
    let mut pending: Option<PendingMarkdownField> = None;
    let mut buffer = String::new();

    for (line_index, line) in lines
        .iter()
        .enumerate()
        .take(table_end + 1)
        .skip(table_start)
    {
        // Cells can span lines, so complete ones are drained from a buffer
        // rather than read line by line.
        buffer.push_str(line);
        buffer.push('\n');
        let (cells, remainder) = drain_complete_td_cells(&buffer);
        buffer = remainder;

        for cell in cells {
            let text = clean_html_text(&cell);
            let text = text.trim();

            if let Some(name) = attribute_cell_name(&cell) {
                if let Some(previous) = pending.take() {
                    fields.push(previous.finish(
                        section,
                        canonical_source_file,
                        helper_source_file,
                    ));
                }
                pending = Some(PendingMarkdownField {
                    name,
                    type_spec: String::new(),
                    description: String::new(),
                    start_line: line_index + 1,
                    end_line: line_index + 1,
                });
                continue;
            }

            let Some(field) = pending.as_mut() else {
                continue;
            };

            if field.type_spec.is_empty() {
                field.type_spec = String::from(text);
            } else {
                push_with_space(&mut field.description, text);
            }
            field.end_line = line_index + 1;
        }
    }

    if let Some(previous) = pending {
        fields.push(previous.finish(section, canonical_source_file, helper_source_file));
    }

    fields
}

struct PendingMarkdownField {
    name: String,
    type_spec: String,
    description: String,
    start_line: usize,
    end_line: usize,
}

impl PendingMarkdownField {
    fn finish(
        self,
        section: &str,
        canonical_source_file: &str,
        helper_source_file: &str,
    ) -> RawField {
        RawField {
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

/// The attribute column is always a bare `<code>name</code>`; type and
/// description cells are prose, so anything else is not a field start.
fn attribute_cell_name(cell: &str) -> Option<String> {
    let trimmed = cell.trim();
    let inner = trimmed.strip_prefix("<code>")?.strip_suffix("</code>")?;
    let name = inner.trim();
    if name.is_empty() {
        return None;
    }

    name.chars()
        .all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
        .then(|| String::from(name))
}

/// Pulls every complete `<td>…</td>` out of the buffer, keeping inner markup so
/// the attribute column can be told apart from a description that merely
/// mentions a `<code>` field name. Returns the cells and whatever tail is still
/// waiting for its closing tag.
fn drain_complete_td_cells(buffer: &str) -> (Vec<String>, String) {
    let mut cells = Vec::new();
    let mut remainder = buffer;

    while let Some(td_index) = remainder.find("<td") {
        let candidate = &remainder[td_index..];
        let Some(tag_end) = candidate.find('>') else {
            break;
        };
        let body = &candidate[(tag_end + 1)..];
        // Some cells in the IAB sources are never closed (the 2.6-202505
        // Content.genres description). The next <td> implicitly closes them;
        // without this the following field gets swallowed as description text.
        let closing = body.find("</td>");
        let next_cell = body.find("<td");
        match (closing, next_cell) {
            (Some(cell_end), next) if next.map_or(true, |start| cell_end < start) => {
                cells.push(String::from(&body[..cell_end]));
                remainder = &body[(cell_end + 5)..];
            }
            (_, Some(start)) => {
                cells.push(String::from(&body[..start]));
                remainder = &body[start..];
            }
            _ => {
                // Still waiting for a close: hold it for the next line.
                return (cells, String::from(candidate));
            }
        }
    }

    (cells, String::from(remainder))
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

fn parse_pdf_object_heading(line: &str) -> Option<(String, String)> {
    let normalized = normalize_pdf_line(line);
    if normalized.contains("...") {
        return None;
    }

    let (section, name) = normalized.split_once(" Object: ")?;
    if !section
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
    {
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
    if !section
        .chars()
        .all(|character| character.is_ascii_digit() || character == '.')
    {
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
    (start..lines.len())
        .find(|candidate| parse_legacy_pdf_object_heading(lines[*candidate]).is_some())
}

fn is_pdf_attribute_header(line: &str) -> bool {
    let normalized = normalize_pdf_line(line);
    normalized.contains("Attribute")
        && normalized.contains("Type")
        && normalized.contains("Description")
}

/// Field tables in the 2.0-2.2 PDFs come in two shapes: the bid-request
/// objects carry a Default column, the bid-response objects (Bid Response,
/// Seat Bid, Bid) do not. Requiring Default left every response object with an
/// empty field list, which made response validation on those versions pass
/// anything.
fn is_legacy_pdf_field_header(line: &str) -> bool {
    let normalized = normalize_pdf_line(line);
    normalized.contains("Field")
        && normalized.contains("Scope")
        && normalized.contains("Type")
        && normalized.contains("Description")
}

fn parse_pdf_table(
    lines: &[&str],
    header_index: usize,
    section_end: usize,
    section: &str,
    canonical_source_file: &str,
    helper_source_file: &str,
) -> Vec<RawField> {
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

        // The last object in a section runs to the end of the document, so
        // without this the enumerated-list tables in later sections get read
        // as if they were fields of that object.
        if is_numbered_section_heading(line, &normalized) {
            break;
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
) -> Vec<RawField> {
    let header = normalize_pdf_line(lines[header_index]);
    let scope_start = header.find("Scope").unwrap_or(20);
    let type_start = header.find("Type").unwrap_or(scope_start + 16);
    // Response tables have no Default column: collapse it onto the description
    // boundary so continuation lines read an empty range instead of eating the
    // first characters of the description.
    let desc_start = header
        .find("Description")
        .unwrap_or_else(|| header.find("Default").map_or(type_start + 20, |at| at + 10));
    let default_start = header.find("Default").unwrap_or(desc_start);
    let has_default_column = header.contains("Default");
    let column_count = if has_default_column { 5 } else { 4 };
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

        if is_numbered_section_heading(line, &normalized) {
            break;
        }

        let indentation = count_leading_spaces(line);
        if indentation < scope_start.saturating_sub(1) {
            let columns = split_multispace_parts(normalized.trim_start(), column_count);
            if columns.is_empty() {
                continue;
            }

            // A long field name wraps in the PDF's name column, leaving a row
            // with a bare name fragment and description text but no scope or
            // type ("connectiontyp" then "e"). Stitch it back onto the open
            // field instead of inventing one. Prose lines that happen to sit in
            // the name column (best-practice notes) are not fragments and must
            // not be stitched, hence the identifier shape check.
            if columns.len() == 2 && is_wrapped_name_fragment(&columns[0]) {
                if let Some(field) = current.as_mut() {
                    field.name.push_str(&columns[0]);
                    push_with_space(&mut field.description, &columns[1]);
                    field.end_line = line_index + 1;
                    continue;
                }
            }

            let name = columns.first().cloned().unwrap_or_default();
            let scope = columns.get(1).cloned().unwrap_or_default();
            let value_type = columns.get(2).cloned().unwrap_or_default();
            let (default_value, description) = if has_default_column {
                (
                    columns.get(3).cloned().unwrap_or_default(),
                    columns.get(4).cloned().unwrap_or_default(),
                )
            } else {
                (String::new(), columns.get(3).cloned().unwrap_or_default())
            };
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
            let scope =
                squash_whitespace(&collect_char_range(&characters, scope_start, type_start));
            let value_type =
                squash_whitespace(&collect_char_range(&characters, type_start, default_start));
            let default_value =
                squash_whitespace(&collect_char_range(&characters, default_start, desc_start));
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
    ) -> RawField {
        RawField {
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
    ) -> RawField {
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

        RawField {
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
    characters[actual_start..actual_end]
        .iter()
        .collect::<String>()
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
