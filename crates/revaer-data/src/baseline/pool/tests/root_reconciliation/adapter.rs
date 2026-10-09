use crate::media::root_catalog::{
    RootCatalogGenerationInput, RootCatalogReconciliation, RootCatalogSlotInput,
    read_root_catalog_page,
};

fn hex<const N: usize>(value: &str) -> anyhow::Result<[u8; N]> {
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
        .collect::<anyhow::Result<Vec<_>>>()?;
    bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("test digest length mismatch"))
}

fn identity() -> anyhow::Result<RootCatalogGenerationInput> {
    Ok(RootCatalogGenerationInput {
        source_format_version: 1,
        source_sha256: hex("121ea684c73474764ce8f45fe5bc9294977d17bc540cc485dcf433fe5e4a2945")?,
        attestation_sha256: hex(
            "d46abdc7f552b03a9e41bbcfdddd8e8cd17e7462c515fc801e144701456918d2",
        )?,
        generation_sha256: hex("c03388a6b0e9600eee95bf451ddd9f58c4f75f2c2700827dc06a5821c1e7b60f")?,
        slot_count: 2,
    })
}

fn slots() -> anyhow::Result<[RootCatalogSlotInput<'static>; 2]> {
    Ok([
        RootCatalogSlotInput {
            logical_key: "a",
            requested_path: "/m/\u{e9}",
            canonical_path: "/real/\u{e9}",
            filesystem_device: hex("0102030405060708")?,
            filesystem_inode: u64::MAX.to_be_bytes(),
            mount_id: 0,
            filesystem_type: "ext4",
            capabilities: [true, false, false, false, false, false, false],
            durability_class: "disposable",
            durability_evidence: "none",
            sole_writer_class: "uncontrolled",
            sole_writer_evidence: "none",
            owner_uid: 0,
            owner_gid: u32::MAX,
            mode_bits: 4095,
            root_identity_sha256: hex(
                "22595f1988e86623ad0b03c3ce791a0792a721808dd3d57e81585f1955e2bc4f",
            )?,
        },
        RootCatalogSlotInput {
            logical_key: "z9",
            requested_path: "/work",
            canonical_path: "/work",
            filesystem_device: (1_u64 << 63).to_be_bytes(),
            filesystem_inode: 0_u64.to_be_bytes(),
            mount_id: i64::MAX,
            filesystem_type: "xfs",
            capabilities: [false, true, true, true, true, true, true],
            durability_class: "restart_persistent",
            durability_evidence: "kubernetes_persistent_volume_claim",
            sole_writer_class: "revaer_exclusive",
            sole_writer_evidence: "kubernetes_read_write_once_pod",
            owner_uid: u32::MAX,
            owner_gid: 1_u32 << 31,
            mode_bits: 0,
            root_identity_sha256: hex(
                "cfdfe871f1d7a72b088aef25782ea79eef3ae26df61d9848c08a663b25ee4fde",
            )?,
        },
    ])
}

pub(super) async fn check(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    check_failed_activation(pool).await?;
    let input = identity()?;
    let inputs = slots()?;
    let before = read_root_catalog_page(pool, 1, None, None)
        .await?
        .remove(0)
        .state;
    let mut rejected = RootCatalogReconciliation::begin(pool, &input).await?;
    let slot = rejected.append_slot(&inputs[0]).await?;
    anyhow::ensure!(
        rejected.append_kind(slot, "unknown").await.is_err(),
        "unknown kind accepted"
    );
    rejected.rollback().await?;
    let after = read_root_catalog_page(pool, 1, None, None)
        .await?
        .remove(0)
        .state;
    anyhow::ensure!(
        before == after,
        "failed adapter transaction changed readiness"
    );

    let mut candidate = RootCatalogReconciliation::begin(pool, &input).await?;
    anyhow::ensure!(
        !candidate.already_current(),
        "populated adapter fixture already current"
    );
    let generation = candidate.generation();
    for (slot, kind) in inputs.iter().zip(["source", "workspace"]) {
        let id = candidate.append_slot(slot).await?;
        candidate.append_kind(id, kind).await?;
    }
    let active = candidate.activate().await?;
    anyhow::ensure!(
        active.attestation_generation == generation
            && active.slot_count == 2
            && active.source_sha256 == input.source_sha256
            && active.generation_sha256 == input.generation_sha256,
        "adapter returned mismatched committed metadata"
    );
    let current = RootCatalogReconciliation::begin(pool, &input).await?;
    anyhow::ensure!(
        current.already_current() && current.generation() == generation,
        "adapter did not reuse current generation"
    );
    let reused = current.activate().await?;
    anyhow::ensure!(
        reused.media_root_catalog_generation_public_id
            == active.media_root_catalog_generation_public_id
            && reused.activated_at == active.activated_at,
        "idempotent adapter changed occurrence or activation time"
    );
    Ok(())
}

async fn check_failed_activation(pool: &sqlx::PgPool) -> anyhow::Result<()> {
    let input = identity()?;
    let before = read_root_catalog_page(pool, 1, None, None)
        .await?
        .remove(0)
        .state;
    {
        let inputs = slots()?;
        let mut abandoned = RootCatalogReconciliation::begin(pool, &input).await?;
        let slot = abandoned.append_slot(&inputs[0]).await?;
        abandoned.append_kind(slot, "source").await?;
        // The fixture pool has one connection: readback requires the dropped
        // transaction's rollback to finish before the connection can be reused.
    }
    let after_drop = read_root_catalog_page(pool, 1, None, None)
        .await?
        .remove(0)
        .state;
    anyhow::ensure!(
        before == after_drop,
        "abandoned reconciliation changed readiness"
    );
    for mutation in [
        "missing-slot",
        "missing-kind",
        "missing-capability",
        "wrong-digest",
    ] {
        let mut inputs = slots()?;
        if mutation == "missing-capability" {
            inputs[1].capabilities[1] = false;
        }
        if mutation == "wrong-digest" {
            inputs[0].root_identity_sha256[0] ^= 1;
        }
        let mut candidate = RootCatalogReconciliation::begin(pool, &input).await?;
        for (index, (slot, kind)) in inputs.iter().zip(["source", "workspace"]).enumerate() {
            if mutation == "missing-slot" && index == 1 {
                break;
            }
            let id = candidate.append_slot(slot).await?;
            if mutation != "missing-kind" || index != 1 {
                candidate.append_kind(id, kind).await?;
            }
        }
        anyhow::ensure!(
            candidate.activate().await.is_err(),
            "invalid activation succeeded: {mutation}"
        );
        let after = read_root_catalog_page(pool, 1, None, None)
            .await?
            .remove(0)
            .state;
        anyhow::ensure!(
            before == after,
            "failed activation changed readiness: {mutation}"
        );
    }
    Ok(())
}
