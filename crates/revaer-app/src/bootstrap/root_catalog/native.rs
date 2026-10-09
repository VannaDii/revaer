//! Native catalog proof, transaction sequencing and retained root ownership.

use revaer_config::ConfigService;
use revaer_data::media::root_catalog::{
    RootAttestationFailure, RootCatalogGenerationInput, RootCatalogReconciliation,
    RootCatalogSlotInput, mark_root_attestation_invalid,
};
use revaer_media_runtime::root_catalog::{
    DurabilityClass, DurabilityEvidence, OpenedRootCatalog, ProcRootMountSource,
    RootCatalogIdentityEncoding, RootCatalogLoad, RootDirectoryError, RootDirectoryObservation,
    RootIdentityEncodingError, RootKind, RootMountError, RootMountSource, RootSlotIdentityClaims,
    SoleWriterClass, SoleWriterEvidence, encode_root_catalog_identity_v1,
};
use thiserror::Error;

use crate::AppResult;
use crate::media::source::{AssociationSource, SourceGeneration};

pub(in crate::bootstrap) struct RetainedRootCatalog {
    source: RootCatalogLoad,
    roots: OpenedRootCatalog,
    generation: SourceGeneration,
}

impl AssociationSource for RetainedRootCatalog {
    fn generation(&self) -> SourceGeneration {
        self.generation
    }

    fn watch_directory(
        &self,
        logical_key: &str,
        relative_prefix: &std::path::Path,
    ) -> Result<std::os::fd::OwnedFd, crate::media_discovery_fingerprint::FingerprintError> {
        let index = self
            .source
            .catalog()
            .slots()
            .iter()
            .position(|slot| {
                slot.key() == logical_key && slot.allowed_kinds().contains(&RootKind::Source)
            })
            .ok_or(RootDirectoryError::InvalidPath)?;
        self.roots
            .open_read_directory(index, relative_prefix)
            .map_err(Into::into)
    }

    fn scan(
        &self,
        logical_key: &str,
        relative_prefix: &std::path::Path,
        budget: &crate::media_discovery_scan::ScanBudget,
        cursor: Option<crate::media_discovery_scan::ScanCursor>,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<crate::media_discovery_scan::ScanBatch, crate::media_discovery_scan::ScanError>
    {
        use crate::media_discovery_fingerprint::{FingerprintError, read_media_directory_at};
        let index = self
            .source
            .catalog()
            .slots()
            .iter()
            .position(|slot| {
                slot.key() == logical_key && slot.allowed_kinds().contains(&RootKind::Source)
            })
            .ok_or(FingerprintError::Root(RootDirectoryError::InvalidPath))?;
        crate::media_discovery_scan::scan_retained_media_source_paths(
            relative_prefix,
            budget,
            cursor,
            cancelled,
            |path, entry_limit| {
                let directory = self.roots.open_read_directory(index, path)?;
                let entries = read_media_directory_at(&directory, entry_limit)?;
                let current = self.roots.open_read_directory(index, path)?;
                let observe = |descriptor: &std::os::fd::OwnedFd| {
                    rustix::fs::fstat(descriptor).map_err(|source| FingerprintError::Io {
                        path: path.to_path_buf(),
                        source: std::io::Error::from(source),
                    })
                };
                let after = observe(&current)?;
                if !crate::media_discovery_fingerprint::directory_observations_match(
                    &entries.observation,
                    &after,
                ) {
                    return Err(FingerprintError::DirectoryChanged);
                }
                Ok(entries)
            },
        )
    }

    fn fingerprint(
        &self,
        logical_key: &str,
        relative_path: &std::path::Path,
        cancelled: &dyn Fn() -> bool,
        hash_elapsed: &mut std::time::Duration,
    ) -> Result<
        Option<crate::media_discovery_fingerprint::MediaAggregateFingerprint>,
        crate::media_discovery_fingerprint::FingerprintError,
    > {
        let (index, slot) = self
            .source
            .catalog()
            .slots()
            .iter()
            .enumerate()
            .find(|(_, slot)| {
                slot.key() == logical_key && slot.allowed_kinds().contains(&RootKind::Source)
            })
            .ok_or(RootDirectoryError::InvalidPath)?;
        let parent = match self.roots.open_read_parent(index, relative_path) {
            Ok(parent) => parent,
            Err(RootDirectoryError::CandidateMissing) => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let root = std::path::Path::new(slot.path());
        let fingerprint =
            crate::media_discovery_fingerprint::fingerprint_media_aggregate_at_measured(
                &root.join(relative_path),
                root,
                parent,
                cancelled,
                hash_elapsed,
            )?;
        match self.roots.open_read_parent(index, relative_path) {
            Ok(_) => {}
            Err(RootDirectoryError::CandidateMissing) => return Ok(None),
            Err(error) => return Err(error.into()),
        }
        Ok(fingerprint)
    }
}

#[derive(Debug, Error)]
enum ProofError {
    #[error(transparent)]
    Directory(#[from] RootDirectoryError),
    #[error(transparent)]
    Source(#[from] revaer_media_runtime::root_catalog::RootCatalogSourceError),
    #[error(transparent)]
    Identity(#[from] RootIdentityEncodingError),
    #[error("root persistence evidence is unproven")]
    Durability,
    #[error("root deployment writer evidence is unproven")]
    Writer,
    #[error("root observations changed before activation")]
    Changed,
    #[error(transparent)]
    Storage(#[from] revaer_data::DataError),
}

impl ProofError {
    const fn failure(&self) -> RootAttestationFailure {
        match self {
            Self::Directory(
                RootDirectoryError::Overlap | RootDirectoryError::Mount(RootMountError::Overlap),
            ) => RootAttestationFailure::Overlap,
            Self::Directory(RootDirectoryError::UnsafeAncestry) => {
                RootAttestationFailure::UnsafeAncestry
            }
            Self::Changed
            | Self::Source(_)
            | Self::Identity(_)
            | Self::Directory(
                RootDirectoryError::IdentityChanged
                | RootDirectoryError::Mount(RootMountError::IdentityMismatch),
            ) => RootAttestationFailure::IdentityMismatch,
            Self::Durability => RootAttestationFailure::DurabilityUnproven,
            Self::Writer | Self::Directory(RootDirectoryError::WriteNotDeclared) => {
                RootAttestationFailure::WriterControlUnproven
            }
            Self::Directory(_) | Self::Storage(_) => RootAttestationFailure::Invalid,
        }
    }
}

pub(super) async fn start(
    config: &ConfigService,
    source: RootCatalogLoad,
) -> AppResult<Option<RetainedRootCatalog>> {
    let mounts = ProcRootMountSource;
    let roots = match mounts
        .snapshot()
        .map_err(RootDirectoryError::MountRead)
        .and_then(|snapshot| {
            OpenedRootCatalog::open(
                source.catalog(),
                rustix::process::geteuid().as_raw(),
                &snapshot,
            )
        }) {
        Ok(roots) => roots,
        Err(error) => return invalidate(config, ProofError::Directory(error)).await,
    };
    match reconcile(config, &source, &roots, &mounts).await {
        Ok(generation) => Ok(Some(RetainedRootCatalog {
            source,
            roots,
            generation,
        })),
        Err(error) => invalidate(config, error).await,
    }
}

async fn invalidate(
    config: &ConfigService,
    error: ProofError,
) -> AppResult<Option<RetainedRootCatalog>> {
    if let ProofError::Storage(source) = error {
        return Err(super::storage(source));
    }
    let failure = error.failure();
    tracing::warn!(failure = ?failure, error = %error, "media root catalog attestation failed");
    mark_root_attestation_invalid(config.pool(), failure)
        .await
        .map_err(super::storage)?;
    Ok(None)
}

async fn reconcile(
    config: &ConfigService,
    source: &RootCatalogLoad,
    roots: &OpenedRootCatalog,
    mounts: &dyn RootMountSource,
) -> Result<SourceGeneration, ProofError> {
    let proof = prove(source, roots, mounts)?;
    let claims = claims(source, &proof.observations, &proof.capabilities);
    let input = RootCatalogGenerationInput {
        source_format_version: 1,
        source_sha256: proof.identity.source_sha256(),
        attestation_sha256: proof.identity.attestation_sha256(),
        generation_sha256: proof.identity.generation_sha256(),
        slot_count: i16::try_from(claims.len()).map_err(|_| ProofError::Changed)?,
    };
    let mut transaction = RootCatalogReconciliation::begin(config.pool(), &input).await?;
    if !transaction.already_current() {
        for (claim, encoded) in claims.iter().zip(proof.identity.slots()) {
            let slot = transaction
                .append_slot(&slot_input(claim, encoded.root_identity_sha256())?)
                .await?;
            for kind in claim.declaration.allowed_kinds() {
                transaction.append_kind(slot, kind_name(*kind)).await?;
            }
        }
    }
    if let Err(error) = revalidate(source, roots, mounts, &proof.observations) {
        transaction.rollback().await?;
        return Err(error);
    }
    let activated = transaction.activate().await?;
    Ok(SourceGeneration {
        number: activated.attestation_generation,
        sha256: input.generation_sha256,
    })
}

struct ProofSnapshot {
    observations: Vec<RootDirectoryObservation>,
    capabilities: Vec<[bool; 7]>,
    identity: RootCatalogIdentityEncoding,
}

fn prove(
    source: &RootCatalogLoad,
    roots: &OpenedRootCatalog,
    mounts: &dyn RootMountSource,
) -> Result<ProofSnapshot, ProofError> {
    // Do not convert declarations into deployment or persistence proof.
    for slot in source.catalog().slots() {
        if slot.sole_writer_evidence() == SoleWriterEvidence::KubernetesReadWriteOncePod {
            return Err(ProofError::Writer);
        }
    }
    source.revalidate()?;
    let topology = mounts.snapshot().map_err(RootDirectoryError::MountRead)?;
    let observations = roots.observations(&topology)?;
    for (slot, observation) in source.catalog().slots().iter().zip(&observations) {
        match (slot.durability_class(), slot.durability_evidence()) {
            (DurabilityClass::Disposable, DurabilityEvidence::None) => {
                disposable_filesystem(observation.filesystem_type())?;
            }
            (DurabilityClass::RestartPersistent, DurabilityEvidence::LinuxDedicatedMount) => {
                let external = topology
                    .is_external_mount(observation.mount_id())
                    .map_err(RootDirectoryError::Mount)?;
                persistent_filesystem(observation.filesystem_type(), external)?;
            }
            _ => return Err(ProofError::Durability),
        }
    }
    let capabilities = roots.probe_capabilities(mounts)?;
    let identity =
        encode_root_catalog_identity_v1(source, &claims(source, &observations, &capabilities))?;
    Ok(ProofSnapshot {
        observations,
        capabilities,
        identity,
    })
}

fn disposable_filesystem(filesystem: &str) -> Result<(), ProofError> {
    // Only private Linux tmpfs has the existing non-destructive proof here.
    // Other filesystems require their package evidence, not an inferred grant.
    if filesystem != "tmpfs" {
        return Err(ProofError::Durability);
    }
    Ok(())
}

fn persistent_filesystem(filesystem: &str, external_mount: bool) -> Result<(), ProofError> {
    // Native ext4 mounts are exercised by the owned-volume restart fixture.
    // Other native filesystems and PVCs remain pending their package evidence.
    if filesystem != "ext4" || !external_mount {
        return Err(ProofError::Durability);
    }
    Ok(())
}

fn claims<'a>(
    source: &'a RootCatalogLoad,
    observations: &'a [RootDirectoryObservation],
    capabilities: &[[bool; 7]],
) -> Vec<RootSlotIdentityClaims<'a>> {
    source
        .catalog()
        .slots()
        .iter()
        .zip(observations)
        .zip(capabilities)
        .map(
            |((declaration, observation), capabilities)| RootSlotIdentityClaims {
                declaration,
                canonical_path: observation.canonical_path(),
                filesystem_device: observation.filesystem_device(),
                filesystem_inode: observation.filesystem_inode(),
                mount_id: observation.mount_id(),
                filesystem_type: observation.filesystem_type(),
                capabilities: *capabilities,
                owner_uid: observation.owner_uid(),
                owner_gid: observation.owner_gid(),
                mode_bits: observation.mode_bits(),
            },
        )
        .collect()
}

fn revalidate(
    source: &RootCatalogLoad,
    roots: &OpenedRootCatalog,
    mounts: &dyn RootMountSource,
    prior: &[RootDirectoryObservation],
) -> Result<(), ProofError> {
    source.revalidate()?;
    let current = roots.observations(&mounts.snapshot().map_err(RootDirectoryError::MountRead)?)?;
    if current != prior {
        return Err(ProofError::Changed);
    }
    Ok(())
}

fn slot_input<'a>(
    claim: &RootSlotIdentityClaims<'a>,
    digest: [u8; 32],
) -> Result<RootCatalogSlotInput<'a>, ProofError> {
    let (writer, evidence) = match claim.declaration.sole_writer_class() {
        SoleWriterClass::Uncontrolled => ("uncontrolled", "none"),
        SoleWriterClass::RevaerExclusive => ("revaer_exclusive", "linux_dedicated_service"),
    };
    Ok(RootCatalogSlotInput {
        logical_key: claim.declaration.key(),
        requested_path: claim.declaration.path(),
        canonical_path: claim.canonical_path,
        filesystem_device: claim.filesystem_device.to_be_bytes(),
        filesystem_inode: claim.filesystem_inode.to_be_bytes(),
        mount_id: i64::try_from(claim.mount_id).map_err(|_| ProofError::Changed)?,
        filesystem_type: claim.filesystem_type,
        capabilities: claim.capabilities,
        durability_class: match claim.declaration.durability_class() {
            DurabilityClass::Disposable => "disposable",
            DurabilityClass::RestartPersistent => "restart_persistent",
        },
        durability_evidence: match claim.declaration.durability_evidence() {
            DurabilityEvidence::None => "none",
            DurabilityEvidence::LinuxDedicatedMount => "linux_dedicated_mount",
            DurabilityEvidence::KubernetesPersistentVolumeClaim => {
                "kubernetes_persistent_volume_claim"
            }
        },
        sole_writer_class: writer,
        sole_writer_evidence: evidence,
        owner_uid: claim.owner_uid,
        owner_gid: claim.owner_gid,
        mode_bits: i32::try_from(claim.mode_bits).map_err(|_| ProofError::Changed)?,
        root_identity_sha256: digest,
    })
}

const fn kind_name(kind: RootKind) -> &'static str {
    match kind {
        RootKind::Source => "source",
        RootKind::Output => "output",
        RootKind::Workspace => "workspace",
        RootKind::Backup => "backup",
        RootKind::Quarantine => "quarantine",
    }
}

#[cfg(test)]
mod tests;
