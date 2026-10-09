//! ADR 557 identity framing. Validating claimed scalars is not attestation.

mod model;
mod validate;

use sha2::{Digest, Sha256};

use super::{RootCatalogLoad, RootCatalogSourceState};
pub use model::{
    RootCatalogIdentityEncoding, RootIdentityEncodingError, RootSlotIdentityClaims,
    RootSlotIdentityEncoding,
};

/// Encode the complete ADR 557 version-1 identity from catalog-matched claims.
///
/// The injected source must have loaded successfully. Source declarations are
/// compared exactly, except that their parser already canonicalized slot/kind
/// order. No field here can extend that source's authority. This function does
/// no filesystem I/O and does not turn claims into descriptor evidence; callers
/// must never use this result as root attestation or readiness.
///
/// # Errors
///
/// Rejects missing sources, bounds, duplicate/missing/extra slots, declaration
/// mismatches, invalid normalized scalars, capability/writer incoherence, and
/// overlaps observable in the supplied paths and device/inode pairs. Mount-table
/// aliases and the truth of every claim still require independent root proof.
pub fn encode_root_catalog_identity_v1(
    source: &RootCatalogLoad,
    claims: &[RootSlotIdentityClaims<'_>],
) -> Result<RootCatalogIdentityEncoding, RootIdentityEncodingError> {
    if source.state() != RootCatalogSourceState::Loaded {
        return Err(RootIdentityEncodingError::SourceNotLoaded);
    }
    let ordered = validate::catalog_claims(source.catalog(), claims)?;
    let slots = ordered
        .iter()
        .map(|claim| slot_encoding(claim))
        .collect::<Result<Box<[_]>, _>>()?;
    let source_sha256 = source.catalog().semantic_sha256();
    let attestation_frame = aggregate_frame(source_sha256, &slots)?;
    let attestation_sha256 = Sha256::digest(attestation_frame).into();
    let generation_sha256 =
        Sha256::digest(generation_frame(source_sha256, attestation_sha256)).into();
    Ok(RootCatalogIdentityEncoding {
        source_sha256,
        slots,
        attestation_sha256,
        generation_sha256,
    })
}

fn frame(domain: &[u8]) -> Vec<u8> {
    let mut bytes = domain.to_vec();
    bytes.push(0);
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    bytes
}

fn text(
    frame: &mut Vec<u8>,
    value: &str,
    error: RootIdentityEncodingError,
) -> Result<(), RootIdentityEncodingError> {
    let byte_count = u32::try_from(value.len()).map_err(|_| error)?;
    frame.extend_from_slice(&byte_count.to_be_bytes());
    frame.extend_from_slice(value.as_bytes());
    Ok(())
}

fn slot_encoding(
    claim: &RootSlotIdentityClaims<'_>,
) -> Result<RootSlotIdentityEncoding, RootIdentityEncodingError> {
    let slot = claim.declaration;
    let mut bytes = frame(b"revaer-media-root-slot-attestation");
    text(
        &mut bytes,
        slot.key(),
        RootIdentityEncodingError::DeclarationMismatch,
    )?;
    text(
        &mut bytes,
        slot.path(),
        RootIdentityEncodingError::DeclarationMismatch,
    )?;
    text(
        &mut bytes,
        claim.canonical_path,
        RootIdentityEncodingError::InvalidCanonicalPath,
    )?;
    bytes.push(
        slot.allowed_kinds()
            .iter()
            .fold(0, |mask, kind| mask | kind.mask()),
    );
    bytes.extend_from_slice(&claim.filesystem_device.to_be_bytes());
    bytes.extend_from_slice(&claim.filesystem_inode.to_be_bytes());
    bytes.extend_from_slice(&claim.mount_id.to_be_bytes());
    text(
        &mut bytes,
        claim.filesystem_type,
        RootIdentityEncodingError::InvalidFilesystemType,
    )?;
    bytes.push(capability_mask(claim.capabilities));
    bytes.extend_from_slice(&[
        slot.durability_class().canonical_byte(),
        slot.durability_evidence().canonical_byte(),
        slot.sole_writer_class().canonical_byte(),
        slot.sole_writer_evidence().canonical_byte(),
    ]);
    bytes.extend_from_slice(&claim.owner_uid.to_be_bytes());
    bytes.extend_from_slice(&claim.owner_gid.to_be_bytes());
    bytes.extend_from_slice(&claim.mode_bits.to_be_bytes());
    let sha256 = Sha256::digest(&bytes).into();
    Ok(RootSlotIdentityEncoding {
        logical_key: slot.key().into(),
        frame: bytes.into_boxed_slice(),
        sha256,
    })
}

fn capability_mask(capabilities: [bool; 7]) -> u8 {
    capabilities
        .into_iter()
        .enumerate()
        .fold(0, |mask, (bit, value)| mask | (u8::from(value) << bit))
}

fn aggregate_frame(
    source: [u8; 32],
    slots: &[RootSlotIdentityEncoding],
) -> Result<Vec<u8>, RootIdentityEncodingError> {
    let mut bytes = frame(b"revaer-media-root-attestation");
    bytes.extend_from_slice(&source);
    let count =
        u32::try_from(slots.len()).map_err(|_| RootIdentityEncodingError::SlotBoundExceeded)?;
    bytes.extend_from_slice(&count.to_be_bytes());
    for slot in slots {
        text(
            &mut bytes,
            slot.logical_key(),
            RootIdentityEncodingError::DeclarationMismatch,
        )?;
        bytes.extend_from_slice(&slot.root_identity_sha256());
    }
    Ok(bytes)
}

fn generation_frame(source: [u8; 32], attestation: [u8; 32]) -> Vec<u8> {
    let mut bytes = frame(b"revaer-media-root-generation");
    bytes.extend_from_slice(&source);
    bytes.extend_from_slice(&attestation);
    bytes
}

#[cfg(test)]
mod tests;
