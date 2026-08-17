//! Generation-time helpers that turn spec description prose into the
//! structured catalog fields the validator consumes (`child_object`,
//! `adcom_list`, `value_set`).
//!
//! These run only in the catalog export pipeline. The shipped catalogs and
//! the validator itself never carry or parse spec prose; provenance stays in
//! `CatalogCitation` as section and line references.

use std::collections::BTreeSet;

use crate::adcom_lists::adcom_list_value_set;

/// Structured value-set data extracted from a field's spec description.
pub struct ExtractedValueSet {
    /// Name of the referenced AdCOM list, when the description points at one.
    /// The validator resolves the actual values from its own list registry.
    pub adcom_list: Option<&'static str>,
    /// Inline documented values (`N = label` rows), when no AdCOM list applies.
    pub values: Vec<i64>,
    /// Open-ended vendor range floor ("values 500 and greater").
    pub minimum_inclusive: Option<i64>,
}

/// Extracts the documented integer value set from a field description, if any.
/// AdCOM list references win over inline value enumerations, matching the
/// validator's historical precedence.
pub fn extract_value_set(description: &str) -> Option<ExtractedValueSet> {
    if let Some(list) = adcom_list_value_set(description) {
        return Some(ExtractedValueSet {
            adcom_list: Some(list.name),
            values: Vec::new(),
            minimum_inclusive: None,
        });
    }

    let (values, minimum_inclusive) = parse_inline_integer_value_set(description)?;
    Some(ExtractedValueSet {
        adcom_list: None,
        values,
        minimum_inclusive,
    })
}

/// Resolves the catalog object a field nests into, using the description hint
/// first ("Array of Imp objects", "Details via a Site object"), then the
/// field name, then its singular form.
pub fn resolve_child_object(
    description: &str,
    field_name: &str,
    object_names: &[String],
) -> Option<String> {
    if let Some(candidate) = child_object_hint(description) {
        if let Some(object_name) = match_object_name(&candidate, object_names) {
            return Some(object_name);
        }
    }

    match_object_name(field_name, object_names).or_else(|| {
        field_name
            .strip_suffix('s')
            .and_then(|singular| match_object_name(singular, object_names))
    })
}

fn match_object_name(hint: &str, object_names: &[String]) -> Option<String> {
    object_names
        .iter()
        .find(|name| name.eq_ignore_ascii_case(hint))
        .cloned()
}

/// Names AdCOM uses in headings and "Refer to Object:" prose that are not
/// catalog identifiers. OpenRTB 2.x already calls these EID and UID.
pub fn canonical_adcom_object_name(name: &str) -> String {
    match name.trim() {
        "Extended Identifiers" => String::from("EID"),
        "Extended Identifier UIDs" => String::from("UID"),
        other => other.replace([' ', '-'], ""),
    }
}

fn child_object_hint(description: &str) -> Option<String> {
    let normalized = description.replace('`', "");

    if let Some(candidate) = object_mention_hint(&normalized) {
        return Some(candidate);
    }

    for marker in ["Array of ", "An array of ", "Details via ", "A ", "An "] {
        if let Some(start) = normalized.find(marker) {
            let rest = &normalized[start + marker.len()..];
            if let Some(end) = rest.find(" object") {
                let candidate = strip_leading_article(rest[..end].trim());
                if !candidate.is_empty() {
                    return Some(candidate.to_string());
                }
            }
            if let Some(end) = rest.find(" objects") {
                let candidate = strip_leading_article(rest[..end].trim());
                if !candidate.is_empty() {
                    return Some(candidate.to_string());
                }
            }
        }
    }

    None
}

/// Drops an article the marker did not consume, so "Details via the
/// SupplyChain object" resolves the same as "Details via a Site object".
fn strip_leading_article(candidate: &str) -> &str {
    for article in ["the ", "a ", "an "] {
        if candidate.len() > article.len()
            && candidate[..article.len()].eq_ignore_ascii_case(article)
        {
            return candidate[article.len()..].trim_start();
        }
    }

    candidate
}

/// AdCOM tables point at children with "Refer to Object: DisplayPlacement"
/// (and sometimes just "Object: UserAgent") rather than OpenRTB's
/// "Details via a Site object".
fn object_mention_hint(description: &str) -> Option<String> {
    let lowercased = description.to_ascii_lowercase();
    let marker = "object:";
    let relative = lowercased.find(marker)?;
    let rest = strip_leading_article(description[relative + marker.len()..].trim_start());
    if rest.is_empty() {
        return None;
    }

    for mapped in ["Extended Identifier UIDs", "Extended Identifiers"] {
        if rest.len() >= mapped.len() && rest[..mapped.len()].eq_ignore_ascii_case(mapped) {
            return Some(canonical_adcom_object_name(mapped));
        }
    }

    let end = rest
        .find(|character: char| !character.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    let candidate = rest[..end].trim();
    if candidate.is_empty() {
        return None;
    }

    Some(canonical_adcom_object_name(candidate))
}

fn parse_inline_integer_value_set(description: &str) -> Option<(Vec<i64>, Option<i64>)> {
    let mut allowed_values = BTreeSet::new();
    let bytes = description.as_bytes();
    let mut index = 0usize;

    while index < bytes.len() {
        if !(bytes[index].is_ascii_digit() || bytes[index] == b'-') {
            index += 1;
            continue;
        }

        let start = index;
        let mut end = index;
        if bytes[end] == b'-' {
            end += 1;
            if end >= bytes.len() || !bytes[end].is_ascii_digit() {
                index += 1;
                continue;
            }
        }

        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }

        let mut after = end;
        while after < bytes.len() && bytes[after].is_ascii_whitespace() {
            after += 1;
        }

        if after < bytes.len() && bytes[after] == b'=' {
            if let Ok(value) = description[start..end].parse::<i64>() {
                allowed_values.insert(value);
            }
            index = after + 1;
            continue;
        }

        index = end;
    }

    let minimum_inclusive = parse_minimum_inclusive_value(description);
    if allowed_values.len() < 2 && minimum_inclusive.is_none() {
        return None;
    }

    Some((allowed_values.into_iter().collect(), minimum_inclusive))
}

fn parse_minimum_inclusive_value(description: &str) -> Option<i64> {
    let normalized = description.to_ascii_lowercase();
    for marker in ["using values ", "values "] {
        let mut search_start = 0usize;
        while let Some(relative_index) = normalized[search_start..].find(marker) {
            let marker_start = search_start + relative_index + marker.len();
            let remainder = normalized[marker_start..].trim_start();
            let consumed_whitespace = normalized[marker_start..].len() - remainder.len();
            let bytes = remainder.as_bytes();
            if bytes.is_empty() {
                break;
            }

            let mut end = 0usize;
            if bytes[end] == b'-' {
                end += 1;
            }
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }

            if end > 0 && !(end == 1 && bytes[0] == b'-') {
                let suffix = remainder[end..].trim_start();
                if suffix.starts_with("and greater") || suffix.starts_with("or greater") {
                    if let Ok(value) = remainder[..end].parse::<i64>() {
                        return Some(value);
                    }
                }
            }

            search_start = marker_start + consumed_whitespace + end.max(1);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_adcom_list_reference() {
        let extracted = extract_value_set("Refer to List: API Frameworks in AdCOM 1.0.")
            .expect("adcom reference should extract");
        assert_eq!(extracted.adcom_list, Some("List: API Frameworks"));
        assert!(extracted.values.is_empty());
    }

    #[test]
    fn extracts_inline_value_set_with_vendor_floor() {
        let extracted = extract_value_set(
            "Auction type, where 1 = First Price, 2 = Second Price Plus. \
             Exchange-specific auction types can be defined using values 500 and greater.",
        )
        .expect("inline values should extract");
        assert_eq!(extracted.adcom_list, None);
        assert_eq!(extracted.values, vec![1, 2]);
        assert_eq!(extracted.minimum_inclusive, Some(500));
    }

    #[test]
    fn ignores_descriptions_without_value_sets() {
        assert!(extract_value_set("ID of the bid request.").is_none());
    }

    #[test]
    fn resolves_child_object_from_hint_and_field_name() {
        let objects = vec![String::from("Imp"), String::from("Site")];
        assert_eq!(
            resolve_child_object(
                "Array of Imp objects representing impressions.",
                "imp",
                &objects
            ),
            Some(String::from("Imp"))
        );
        assert_eq!(
            resolve_child_object("no hint here", "site", &objects),
            Some(String::from("Site"))
        );
        assert_eq!(resolve_child_object("no hint", "unknown", &objects), None);
    }

    #[test]
    fn resolves_child_object_from_adcom_refer_to_object() {
        let objects = vec![
            String::from("DisplayPlacement"),
            String::from("EID"),
            String::from("UID"),
            String::from("BrandVersion"),
        ];
        assert_eq!(
            resolve_child_object(
                "Placement Subtype Object. Refer to Object: DisplayPlacement.",
                "display",
                &objects
            ),
            Some(String::from("DisplayPlacement"))
        );
        assert_eq!(
            resolve_child_object(
                "Extended (third-party) identifiers. Refer to Object: Extended Identifiers.",
                "eids",
                &objects
            ),
            Some(String::from("EID"))
        );
        assert_eq!(
            resolve_child_object(
                "Refer to Object: BrandVersion that identifies the platform.",
                "platform",
                &objects
            ),
            Some(String::from("BrandVersion"))
        );
    }

    // Source.schain is described as "Details via the `SupplyChain` object" and
    // its field name does not match the object name, so the article-stripping
    // path is the only thing that wires it up. Losing it silently drops
    // SupplyChain validation on every 2.6 catalog.
    #[test]
    fn resolves_child_object_behind_a_definite_article() {
        let objects = vec![String::from("SupplyChain"), String::from("Site")];
        assert_eq!(
            resolve_child_object(
                "This object represents both the links in the supply chain as well as an \
                 indicator whether or not the supply chain is complete. Details via the \
                 SupplyChain object (section 3.2.25).",
                "schain",
                &objects
            ),
            Some(String::from("SupplyChain"))
        );
        assert_eq!(
            resolve_child_object("Details via an Site object.", "unknown", &objects),
            Some(String::from("Site"))
        );
    }
}
