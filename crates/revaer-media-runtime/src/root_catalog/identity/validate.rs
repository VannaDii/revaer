use std::collections::BTreeMap;

use super::super::{
    MAX_ROOT_CATALOG_PATH_BYTES, MAX_ROOT_CATALOG_SLOTS, RootCatalog, RootKind, SoleWriterClass,
};
use super::{RootIdentityEncodingError, RootSlotIdentityClaims};

pub(super) fn catalog_claims<'a>(
    catalog: &RootCatalog,
    claims: &'a [RootSlotIdentityClaims<'a>],
) -> Result<Vec<&'a RootSlotIdentityClaims<'a>>, RootIdentityEncodingError> {
    if claims.len() > MAX_ROOT_CATALOG_SLOTS {
        return Err(RootIdentityEncodingError::SlotBoundExceeded);
    }
    let mut keyed = BTreeMap::new();
    for claim in claims {
        if keyed.insert(claim.declaration.key(), claim).is_some() {
            return Err(RootIdentityEncodingError::DuplicateSlot);
        }
        let index = catalog
            .slots()
            .binary_search_by(|slot| {
                slot.key()
                    .as_bytes()
                    .cmp(claim.declaration.key().as_bytes())
            })
            .map_err(|_| RootIdentityEncodingError::UnexpectedSlot)?;
        if catalog.slots().get(index) != Some(claim.declaration) {
            return Err(RootIdentityEncodingError::DeclarationMismatch);
        }
        scalar_claims(claim)?;
    }
    if keyed.len() != catalog.slots().len() {
        return Err(RootIdentityEncodingError::MissingSlot);
    }
    let ordered: Vec<_> = keyed.into_values().collect();
    for (index, left) in ordered.iter().enumerate() {
        for right in ordered.iter().skip(index + 1) {
            if (left.filesystem_device, left.filesystem_inode)
                == (right.filesystem_device, right.filesystem_inode)
                || contains(left.canonical_path, right.canonical_path)
                || contains(right.canonical_path, left.canonical_path)
            {
                return Err(RootIdentityEncodingError::Overlap);
            }
        }
    }
    Ok(ordered)
}

fn scalar_claims(claim: &RootSlotIdentityClaims<'_>) -> Result<(), RootIdentityEncodingError> {
    let path = claim.canonical_path;
    if !path.starts_with('/')
        || path == "/"
        || path.len() > MAX_ROOT_CATALOG_PATH_BYTES
        || path.as_bytes().contains(&0)
    {
        return Err(RootIdentityEncodingError::InvalidCanonicalPath);
    }
    if claim.mount_id > i64::MAX.unsigned_abs() {
        return Err(RootIdentityEncodingError::InvalidMountId);
    }
    if claim.filesystem_type.is_empty()
        || claim.filesystem_type.len() > 64
        || claim.filesystem_type.as_bytes().contains(&0)
    {
        return Err(RootIdentityEncodingError::InvalidFilesystemType);
    }
    if claim.mode_bits > 4095 {
        return Err(RootIdentityEncodingError::InvalidModeBits);
    }
    let [read, write, create_new, fsync, rename, delete, capacity] = claim.capabilities;
    for kind in claim.declaration.allowed_kinds() {
        if *kind == RootKind::Source {
            if !read {
                return Err(RootIdentityEncodingError::CapabilityMismatch);
            }
        } else {
            if !(write && create_new && fsync && rename && delete && capacity) {
                return Err(RootIdentityEncodingError::CapabilityMismatch);
            }
            if claim.declaration.sole_writer_class() != SoleWriterClass::RevaerExclusive {
                return Err(RootIdentityEncodingError::WriterControlMismatch);
            }
        }
    }
    Ok(())
}

fn contains(parent: &str, child: &str) -> bool {
    parent == child
        || child
            .strip_prefix(parent)
            .is_some_and(|suffix| suffix.starts_with('/'))
}
