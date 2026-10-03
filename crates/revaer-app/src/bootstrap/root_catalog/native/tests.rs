use super::*;
use revaer_media_runtime::root_catalog::{RootCatalogSource, TrustedLocalRootCatalogSource};

#[test]
fn unqualified_filesystems_do_not_receive_a_capability_grant() -> anyhow::Result<()> {
    disposable_filesystem("tmpfs")?;
    for filesystem in ["", "ext4", "virtiofs", "nfs4", "fuse", "unqualified"] {
        assert!(matches!(
            disposable_filesystem(filesystem),
            Err(ProofError::Durability)
        ));
    }
    Ok(())
}

#[test]
fn native_persistence_requires_external_nonvolatile_qualified_mount() -> anyhow::Result<()> {
    persistent_filesystem("ext4", true)?;
    assert!(persistent_filesystem("ext4", false).is_err());
    for filesystem in [
        "tmpfs", "overlay", "ramfs", "virtiofs", "nfs4", "xfs", "btrfs", "",
    ] {
        assert!(persistent_filesystem(filesystem, true).is_err());
    }
    let mounts = revaer_media_runtime::root_catalog::RootMountTopology::parse(
        "1 0 8:1 / / rw - ext4 /dev/vda rw\n2 1 8:2 / /media rw - ext4 /dev/vdb rw\n4 1 8:1 /alias /image-alias rw - ext4 /dev/vda rw",
    )?;
    assert!(!mounts.is_external_mount(1)?);
    assert!(mounts.is_external_mount(2)?);
    assert!(!mounts.is_external_mount(4)?);
    assert!(mounts.is_external_mount(3).is_err());
    Ok(())
}

fn fixture() -> anyhow::Result<(tempfile::TempDir, RootCatalogLoad, OpenedRootCatalog)> {
    let directory = tempfile::Builder::new()
        .prefix(".revaer-root-startup-")
        .tempdir_in(
            std::env::var_os("HOME")
                .ok_or_else(|| anyhow::anyhow!("private fixture HOME required"))?,
        )?;
    let output = directory.path().join("output");
    let source = directory.path().join("source");
    std::fs::create_dir(&output)?;
    std::fs::create_dir(&source)?;
    std::fs::write(source.join("original"), b"original source bytes")?;
    let document = serde_json::json!({ "format_version": 1, "slots": [
        { "key": "output", "path": output, "allowed_kinds": ["output", "workspace"],
          "durability_class": "disposable", "durability_evidence": "none",
          "sole_writer_class": "revaer_exclusive", "sole_writer_evidence": "linux_dedicated_service" },
        { "key": "source", "path": source, "allowed_kinds": ["source"],
          "durability_class": "disposable", "durability_evidence": "none",
          "sole_writer_class": "uncontrolled", "sole_writer_evidence": "none" }
    ] });
    let location = directory.path().join("catalog.json");
    std::fs::write(&location, serde_json::to_vec(&document)?)?;
    let uid = rustix::process::geteuid().as_raw();
    let loaded = TrustedLocalRootCatalogSource::native_override(
        location
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("fixture path encoding"))?,
        uid,
    )?
    .load()?;
    let roots = OpenedRootCatalog::open(loaded.catalog(), uid, &ProcRootMountSource.snapshot()?)?;
    Ok((directory, loaded, roots))
}

#[test]
fn native_proof_preserves_source_cleans_probes_and_retains_locks() -> anyhow::Result<()> {
    let (directory, source, roots) = fixture()?;
    let proof = prove(&source, &roots, &ProcRootMountSource)?;
    assert_eq!(
        proof.capabilities,
        vec![[true; 7], [true, false, false, false, false, false, false]]
    );
    assert_eq!(proof.identity.slots().len(), 2);
    revalidate(&source, &roots, &ProcRootMountSource, &proof.observations)?;
    assert_eq!(
        std::fs::read(directory.path().join("source/original"))?,
        b"original source bytes"
    );
    assert_eq!(
        std::fs::read_dir(directory.path().join("source"))?.count(),
        1
    );
    assert_eq!(
        std::fs::read_dir(directory.path().join("output"))?.count(),
        0
    );
    assert!(
        OpenedRootCatalog::open(
            source.catalog(),
            rustix::process::geteuid().as_raw(),
            &ProcRootMountSource.snapshot()?
        )
        .is_err()
    );
    drop(roots);
    let reopened = OpenedRootCatalog::open(
        source.catalog(),
        rustix::process::geteuid().as_raw(),
        &ProcRootMountSource.snapshot()?,
    )?;
    drop(reopened);
    drop(source);
    directory.close()?;
    Ok(())
}

#[test]
fn source_document_or_root_change_rejects_final_activation_proof() -> anyhow::Result<()> {
    for change_document in [true, false] {
        let (directory, source, roots) = fixture()?;
        let proof = prove(&source, &roots, &ProcRootMountSource)?;
        if change_document {
            std::fs::write(directory.path().join("catalog.json"), b"changed")?;
        } else {
            std::fs::rename(
                directory.path().join("output"),
                directory.path().join("moved-output"),
            )?;
            std::fs::create_dir(directory.path().join("output"))?;
        }
        assert!(revalidate(&source, &roots, &ProcRootMountSource, &proof.observations).is_err());
        assert_eq!(
            std::fs::read(directory.path().join("source/original"))?,
            b"original source bytes"
        );
        drop(roots);
        drop(source);
        directory.close()?;
    }
    Ok(())
}
