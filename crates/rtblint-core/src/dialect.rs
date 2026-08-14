//! JSON dialects an OpenRTB 2.x payload can be written in.
//!
//! The OpenRTB spec types a large family of flag fields as integers with the
//! value set {0, 1}. The IAB protobuf schema for the same objects
//! (`com.iabtechlab.openrtb.v2`, the schema ARTF and every gRPC bidstream
//! integration compiles against) declares those same fields as `bool`. The two
//! JSON encodings are therefore incompatible in both directions:
//!
//! - protojson writes `"secure": true`, which a spec-JSON reader rejects.
//! - spec JSON writes `"secure": 1`, which protojson refuses to unmarshal into
//!   a bool field.
//!
//! Neither side is wrong for its own transport, so the payload alone cannot
//! settle it. The caller declares which dialect it meant and the validator
//! reports the mismatch against that, instead of guessing.
//!
//! The table below is derived mechanically: every field the IAB OpenRTB
//! protobuf schema declares `bool` while some tracked 2.x catalog types it as
//! an integer. It is version-independent on purpose. A field that changes type
//! across snapshots keeps the same proto declaration, so the divergence holds
//! wherever the field exists.

/// The JSON dialect a payload is written in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Dialect {
    /// The encoding the OpenRTB specification describes: flags are integers,
    /// field names are the spec's own names. The default.
    #[default]
    SpecJson,
    /// The canonical protobuf JSON mapping of the IAB OpenRTB protobuf schema:
    /// flags declared `bool` in the proto are `true`/`false`, and field names
    /// may arrive lowerCamelCased, which is what protojson emits unless the
    /// serializer sets `UseProtoNames`.
    ProtoJson,
}

impl Dialect {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SpecJson => "spec-json",
            Self::ProtoJson => "proto-json",
        }
    }

    /// Parses the dialect id used by the CLI, the MCP tools, and the npm
    /// bindings. Accepts the underscored spelling too, since JSON callers tend
    /// to write it that way.
    pub fn from_id(value: &str) -> Option<Self> {
        match value {
            "spec-json" | "spec_json" | "spec" => Some(Self::SpecJson),
            "proto-json" | "proto_json" | "protojson" | "proto" => Some(Self::ProtoJson),
            _ => None,
        }
    }
}

impl std::fmt::Display for Dialect {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Fields the IAB OpenRTB protobuf schema declares `bool` where the OpenRTB
/// specification types them as an integer flag. Ascending by (object, field);
/// the lookup binary-searches it.
const PROTO_BOOL_FIELDS: &[(&str, &str)] = &[
    ("App", "paid"),
    ("App", "privacypolicy"),
    ("Audio", "stitched"),
    ("Banner", "topframe"),
    ("Banner", "vcm"),
    ("Content", "embeddable"),
    ("Content", "livestream"),
    ("Content", "sourcerelationship"),
    ("Device", "dnt"),
    ("Device", "geofetch"),
    ("Device", "js"),
    ("Device", "lmt"),
    ("Imp", "clickbrowser"),
    ("Imp", "instl"),
    ("Imp", "rwdd"),
    ("Imp", "secure"),
    ("Pmp", "private_auction"),
    ("Regs", "coppa"),
    ("Regs", "gdpr"),
    ("SeatBid", "group"),
    ("Site", "mobile"),
    ("Site", "privacypolicy"),
    ("Source", "fd"),
    ("SupplyChain", "complete"),
    ("SupplyChainNode", "hp"),
    ("UserAgent", "mobile"),
    ("Video", "boxingallowed"),
    ("Video", "skip"),
];

/// Whether the IAB OpenRTB protobuf schema declares `object.field` as a bool.
pub fn proto_declares_bool(object_name: &str, field_name: &str) -> bool {
    PROTO_BOOL_FIELDS
        .binary_search(&(object_name, field_name))
        .is_ok()
}

/// Every field where the protobuf schema and the spec disagree on the type,
/// as `(object, field)` pairs. Exposed so callers can document or test the
/// divergence set without duplicating it.
pub fn proto_bool_fields() -> &'static [(&'static str, &'static str)] {
    PROTO_BOOL_FIELDS
}

/// The spec-name form of a lowerCamelCase field name, when the name actually
/// is camelCased. `privateAuction` becomes `private_auction`; `bidfloorcur`,
/// which has no case boundary, returns `None` so ordinary names never take
/// this path.
pub fn snake_case_of_camel(field_name: &str) -> Option<String> {
    if !field_name
        .chars()
        .any(|character| character.is_ascii_uppercase())
    {
        return None;
    }

    let mut snake = String::with_capacity(field_name.len() + 2);
    for character in field_name.chars() {
        if character.is_ascii_uppercase() {
            snake.push('_');
            snake.push(character.to_ascii_lowercase());
        } else {
            snake.push(character);
        }
    }
    Some(snake)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proto_bool_table_is_sorted_for_binary_search() {
        let mut sorted = PROTO_BOOL_FIELDS.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.as_slice(), PROTO_BOOL_FIELDS);
    }

    #[test]
    fn proto_bool_lookup_finds_both_ends_of_the_table() {
        assert!(proto_declares_bool("App", "paid"));
        assert!(proto_declares_bool("Video", "skip"));
        assert!(proto_declares_bool("Pmp", "private_auction"));
        assert!(!proto_declares_bool("Imp", "bidfloor"));
        assert!(!proto_declares_bool("Video", "mimes"));
    }

    #[test]
    fn camel_case_conversion_only_fires_on_case_boundaries() {
        assert_eq!(
            snake_case_of_camel("privateAuction").as_deref(),
            Some("private_auction")
        );
        assert_eq!(
            snake_case_of_camel("usPrivacy").as_deref(),
            Some("us_privacy")
        );
        assert_eq!(snake_case_of_camel("bidfloorcur"), None);
        assert_eq!(snake_case_of_camel("id"), None);
    }

    #[test]
    fn dialect_ids_round_trip() {
        assert_eq!(Dialect::from_id("proto-json"), Some(Dialect::ProtoJson));
        assert_eq!(Dialect::from_id("spec_json"), Some(Dialect::SpecJson));
        assert_eq!(Dialect::from_id("yaml"), None);
        assert_eq!(Dialect::default(), Dialect::SpecJson);
    }
}
