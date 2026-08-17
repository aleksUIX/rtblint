//! Guards on the shipped catalogs themselves.
//!
//! The catalogs are generated from the gitignored IAB archive by
//! `examples/export_canonical_object_catalogs.rs`, so nothing in CI re-derives
//! them. These tests assert the properties a regeneration must preserve: no
//! prose entries, and object wiring that a name-only match would miss.

use rtblint_core::{
    canonical_field, canonical_object, canonical_object_catalog, ExpectedShape, OpenRtbVersion,
};

fn is_identifier_shaped(name: &str) -> bool {
    let mut characters = name.chars();
    match characters.next() {
        Some(first) if first.is_ascii_alphabetic() => {}
        _ => return false,
    }

    characters.all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '.'))
}

/// The PDF and table parsers have picked up sentences, headings, and notes as
/// objects or fields before (5,722 of them across the 2.0-2.5 catalogs). Any
/// name that is not identifier-shaped is a mis-extraction, and build.rs derives
/// shape and required-ness from whatever ships.
#[test]
fn catalog_names_are_identifiers_not_prose() {
    for version in OpenRtbVersion::all() {
        let Some(catalog) = canonical_object_catalog(*version) else {
            continue;
        };

        for object in catalog.objects {
            assert!(
                is_identifier_shaped(object.name),
                "{}: object name {:?} is prose, not an identifier",
                version.id(),
                object.name
            );

            for field in object.fields {
                assert!(
                    is_identifier_shaped(field.name),
                    "{}: field {:?} on object {} is prose, not an identifier",
                    version.id(),
                    field.name,
                    object.name
                );
            }
        }
    }
}

/// `Source.schain` nests into `SupplyChain`, but the field name and the object
/// name do not match: the wiring comes from the description hint alone. It has
/// been dropped by a regeneration once already, and without it SupplyChain and
/// SupplyChainNode are never validated.
#[test]
fn source_schain_is_wired_to_the_supply_chain_object() {
    for version in OpenRtbVersion::all() {
        let Some(catalog) = canonical_object_catalog(*version) else {
            continue;
        };

        // Only versions whose catalog defines SupplyChain can wire it up; the
        // pre-2.6 specs carry schain as an ext with no object of its own.
        if !catalog
            .objects
            .iter()
            .any(|object| object.name == "SupplyChain")
        {
            continue;
        }

        let source = canonical_object(*version, "Source").unwrap_or_else(|| {
            panic!(
                "{}: catalog defines SupplyChain but no Source",
                version.id()
            )
        });
        let schain = source
            .fields
            .iter()
            .find(|field| field.name == "schain")
            .unwrap_or_else(|| panic!("{}: Source has no schain field", version.id()));

        assert_eq!(
            schain.child_object,
            Some("SupplyChain"),
            "{}: Source.schain lost its SupplyChain wiring",
            version.id()
        );
    }
}

/// OpenRTB 3.0's domain layer lives in the AdCOM catalog, merged into the 3.0
/// static catalog at build time. These edges are the Appendix C wrappers;
/// losing them silently stops all AdCOM validation.
#[test]
fn openrtb_3_0_domain_fields_are_wired_to_adcom() {
    let spec =
        canonical_field(OpenRtbVersion::V3_0, "Item", "spec").expect("3.0 Item.spec should exist");
    assert_eq!(spec.child_object, Some("Spec"));
    assert_eq!(
        canonical_field(OpenRtbVersion::V3_0, "Spec", "placement")
            .expect("Spec.placement")
            .child_object,
        Some("Placement")
    );
    assert_eq!(
        canonical_field(OpenRtbVersion::V3_0, "Request", "context")
            .expect("Request.context")
            .child_object,
        Some("Context")
    );
    assert_eq!(
        canonical_field(OpenRtbVersion::V3_0, "Bid", "media")
            .expect("Bid.media")
            .child_object,
        Some("Media")
    );
    assert!(canonical_object(OpenRtbVersion::V3_0, "Site")
        .expect("Site")
        .fields
        .iter()
        .any(|field| field.name == "id"));
    assert_eq!(
        canonical_field(OpenRtbVersion::V3_0, "App", "domain")
            .expect("App.domain")
            .type_spec,
        "string"
    );
    assert_eq!(
        canonical_field(OpenRtbVersion::V3_0, "UserAgent", "browsers")
            .expect("UserAgent.browsers")
            .shape,
        ExpectedShape::ObjectArray
    );
}

#[test]
fn banner_id_prose_does_not_become_an_object_array() {
    let field = canonical_field(OpenRtbVersion::V2_4, "Banner", "id").expect("Banner.id");
    assert_ne!(
        field.shape,
        ExpectedShape::ObjectArray,
        "Banner.id's type_spec mentions companion ads and 'each object'; that is not an object array"
    );
}

/// Type cells sometimes wrap the type in `<code>` and leave the attribute
/// bare. Those rows must not ship as a field named `string` / `integer`.
#[test]
fn catalog_field_names_are_not_type_words() {
    for version in OpenRtbVersion::all() {
        let Some(catalog) = canonical_object_catalog(*version) else {
            continue;
        };
        for object in catalog.objects {
            for field in object.fields {
                assert!(
                    !matches!(
                        field.name,
                        "string" | "integer" | "float" | "object" | "boolean" | "array" | "number"
                    ),
                    "{}: {}.{} is a type word, not a field",
                    version.id(),
                    object.name,
                    field.name
                );
            }
        }
    }
}

/// Every `child_object` must name an object the same catalog defines, or the
/// validator walks into a dead end and silently stops checking that subtree.
#[test]
fn child_object_references_resolve_within_their_catalog() {
    for version in OpenRtbVersion::all() {
        let Some(catalog) = canonical_object_catalog(*version) else {
            continue;
        };

        for object in catalog.objects {
            for field in object.fields {
                let Some(child) = field.child_object else {
                    continue;
                };

                assert!(
                    catalog
                        .objects
                        .iter()
                        .any(|candidate| candidate.name == child),
                    "{}: {}.{} points at unknown object {:?}",
                    version.id(),
                    object.name,
                    field.name,
                    child
                );
            }
        }
    }
}
