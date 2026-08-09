//! What produced a verdict.
//!
//! Same reasoning as the vastlint server: a version string names a release, a
//! digest identifies what that release actually carried, and the engine version
//! covers the evaluator. For rtblint the point is sharper, because the version
//! axis is part of the question. The same payload is legitimately valid under
//! OpenRTB 2.5 and invalid under 2.6, so a verdict without its version is not
//! merely hard to reproduce, it is ambiguous.

use std::sync::OnceLock;

use rtblint_core::{canonical_object_catalog, OpenRtbVersion};
use sha2::{Digest, Sha256};

use crate::proto;

static CATALOG_DIGEST: OnceLock<String> = OnceLock::new();

/// Content hash of the catalog this binary carries, as `sha256:<hex>`.
///
/// Computed over the linked catalog at first call rather than over the source
/// at build time, so it identifies what this binary will actually enforce.
///
/// Covers every tracked version and, for each, the objects and fields the
/// catalog defines. Those are what decide whether a payload passes. Prose
/// (descriptions, citations) is excluded: rewording a field description does
/// not change any verdict, and including it would churn the digest on every
/// documentation pass.
pub fn catalog_digest() -> &'static str {
    CATALOG_DIGEST.get_or_init(|| {
        let mut hasher = Sha256::new();

        for version in OpenRtbVersion::all() {
            hasher.update(version.id().as_bytes());
            hasher.update([0x1f]);

            if let Some(catalog) = canonical_object_catalog(*version) {
                for object in catalog.objects {
                    hasher.update(object.name.as_bytes());
                    hasher.update([0x1f]);
                    for field in object.fields {
                        hasher.update(field.name.as_bytes());
                        hasher.update([0x1d]);
                    }
                }
            }

            hasher.update([0x1e]);
        }

        format!("sha256:{:x}", hasher.finalize())
    })
}

/// The provenance stamped onto every response.
pub fn provenance() -> proto::Provenance {
    proto::Provenance {
        catalog_version: env!("CARGO_PKG_VERSION").to_string(),
        catalog_digest: catalog_digest().to_string(),
        engine_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn digest_is_stable_across_calls() {
        assert_eq!(catalog_digest(), catalog_digest());
    }

    #[test]
    fn digest_is_a_prefixed_sha256() {
        let hex = catalog_digest()
            .strip_prefix("sha256:")
            .expect("sha256: prefix");
        assert_eq!(hex.len(), 64);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn provenance_is_populated() {
        let provenance = provenance();
        assert!(!provenance.catalog_version.is_empty());
        assert!(!provenance.engine_version.is_empty());
        assert!(provenance.catalog_digest.starts_with("sha256:"));
    }
}
