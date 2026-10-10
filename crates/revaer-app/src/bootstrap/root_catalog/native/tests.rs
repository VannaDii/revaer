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
fn retained_scan_batches_relative_paths_and_ignores_symlink_targets() -> anyhow::Result<()> {
    use crate::media_discovery_scan::ScanBudget;
    let (directory, source, roots) = fixture()?;
    let root = directory.path().join("source");
    for index in 0..16 {
        let branch = root.join(format!("branch-{index:02}"));
        std::fs::create_dir(&branch)?;
        std::fs::write(branch.join("video.mkv"), b"media")?;
    }
    let outside = directory.path().join("outside");
    std::fs::create_dir(&outside)?;
    std::fs::write(outside.join("escape.mkv"), b"outside")?;
    std::os::unix::fs::symlink(&outside, root.join("alias"))?;
    let retained = RetainedRootCatalog {
        source,
        roots,
        // Synthetic generation; actual retained descriptors and native inventory.
        generation: SourceGeneration {
            number: 7,
            sha256: [0x53; 32],
        },
    };
    verify_watch_directories(&retained)?;
    let budget = ScanBudget {
        depth: 3,
        entries: 3,
        files: 1,
        bytes: 64,
        elapsed: std::time::Duration::from_secs(1),
        run: crate::media_discovery_scan::SCAN_RUN_LIMITS,
    };
    let mut cursor = None;
    let mut found = std::collections::BTreeSet::new();
    let mut finished = false;
    for _ in 0..128 {
        let batch = retained.scan(
            "source",
            std::path::Path::new(""),
            &budget,
            cursor.take(),
            &|| false,
        )?;
        assert!(batch.paths.len() <= 1);
        for path in batch.paths {
            assert!(!std::path::Path::new(&path).is_absolute());
            assert!(found.insert(path));
        }
        cursor = batch.cursor;
        if cursor.is_none() {
            finished = true;
            break;
        }
    }
    assert!(finished);
    assert_eq!(found.len(), 16);
    assert!(found.iter().all(|path| path.starts_with("branch-")));
    let small_budget = ScanBudget {
        bytes: 4,
        ..ScanBudget::default()
    };
    assert!(matches!(
        retained.scan(
            "source",
            std::path::Path::new("branch-00"),
            &small_budget,
            None,
            &|| false
        ),
        Err(crate::media_discovery_scan::ScanError::Inventory(
            crate::media_discovery_fingerprint::FingerprintError::ResourceLimit(
                "scan primary bytes"
            )
        ))
    ));
    let cancelled = retained.scan("source", std::path::Path::new(""), &budget, None, &|| true)?;
    assert!(cancelled.paths.is_empty());
    assert_eq!(
        cancelled.limit,
        Some(crate::media_discovery_scan::ScanLimit::Cancelled)
    );
    assert!(
        retained
            .scan(
                "source",
                std::path::Path::new("alias"),
                &budget,
                None,
                &|| false
            )
            .is_err()
    );
    assert_eq!(std::fs::read(outside.join("escape.mkv"))?, b"outside");
    drop(retained);
    directory.close()?;
    Ok(())
}

fn verify_watch_directories(retained: &RetainedRootCatalog) -> anyhow::Result<()> {
    let watched = retained.watch_directory("source", std::path::Path::new("branch-00"))?;
    assert_eq!(
        rustix::fs::FileType::from_raw_mode(rustix::fs::fstat(&watched)?.st_mode),
        rustix::fs::FileType::Directory
    );
    assert!(
        retained
            .watch_directory("source", std::path::Path::new("alias"))
            .is_err()
    );
    assert!(
        retained
            .watch_directory("missing", std::path::Path::new(""))
            .is_err()
    );
    drop(watched);
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

#[test]
fn retained_scan_restarts_when_a_directory_changes_between_batches() -> anyhow::Result<()> {
    use crate::media_discovery_fingerprint::FingerprintError;
    use crate::media_discovery_scan::{ScanBudget, ScanError};
    for replace in [false, true] {
        let (directory, source, roots) = fixture()?;
        let branch = directory.path().join("source/branch");
        std::fs::create_dir(&branch)?;
        std::fs::write(branch.join("a.mkv"), b"first")?;
        std::fs::write(branch.join("z.mkv"), b"last")?;
        let retained = RetainedRootCatalog {
            source,
            roots,
            generation: SourceGeneration {
                number: 7,
                sha256: [0x53; 32],
            },
        };
        let budget = ScanBudget {
            files: 1,
            ..ScanBudget::default()
        };
        let first = retained.scan(
            "source",
            std::path::Path::new("branch"),
            &budget,
            None,
            &|| false,
        )?;
        assert_eq!(first.paths, ["branch/a.mkv"]);
        assert!(first.cursor.is_some());
        if replace {
            std::fs::rename(&branch, directory.path().join("previous-branch"))?;
            std::fs::create_dir(&branch)?;
        }
        std::fs::write(branch.join("0.mkv"), b"new before cursor")?;
        assert!(matches!(
            retained.scan(
                "source",
                std::path::Path::new("branch"),
                &budget,
                first.cursor,
                &|| false
            ),
            Err(ScanError::Inventory(FingerprintError::DirectoryChanged))
        ));
        let fresh = retained.scan(
            "source",
            std::path::Path::new("branch"),
            &ScanBudget::default(),
            None,
            &|| false,
        )?;
        assert!(fresh.cursor.is_none());
        assert!(fresh.paths.contains(&"branch/0.mkv".to_owned()));
        assert_eq!(
            std::fs::read(directory.path().join("source/original"))?,
            b"original source bytes"
        );
        drop(retained);
        directory.close()?;
    }
    Ok(())
}

fn fixture() -> anyhow::Result<(tempfile::TempDir, RootCatalogLoad, OpenedRootCatalog)> {
    let (parent, durability_class, durability_evidence) =
        if let Some(root) = std::env::var_os("REVAER_NATIVE_RECOVERY_ROOT") {
            (root, "restart_persistent", "linux_dedicated_mount")
        } else {
            (
                std::env::var_os("HOME")
                    .ok_or_else(|| anyhow::anyhow!("private fixture HOME required"))?,
                "disposable",
                "none",
            )
        };
    let directory = tempfile::Builder::new()
        .prefix(".revaer-root-startup-")
        .tempdir_in(parent)?;
    let output = directory.path().join("output");
    let source = directory.path().join("source");
    std::fs::create_dir(&output)?;
    std::fs::create_dir(&source)?;
    std::fs::write(source.join("original"), b"original source bytes")?;
    let document = serde_json::json!({ "format_version": 1, "slots": [
        { "key": "output", "path": output, "allowed_kinds": ["output", "workspace"],
          "durability_class": durability_class, "durability_evidence": durability_evidence,
          "sole_writer_class": "revaer_exclusive", "sole_writer_evidence": "linux_dedicated_service" },
        { "key": "source", "path": source, "allowed_kinds": ["source"],
          "durability_class": durability_class, "durability_evidence": durability_evidence,
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
    let snapshot = ProcRootMountSource.snapshot().map_err(|error| {
        let invalid_record = std::fs::read_to_string("/proc/self/mountinfo").and_then(|input| {
            input
                .lines()
                .find(|record| {
                    revaer_media_runtime::root_catalog::RootMountTopology::parse(record).is_err()
                })
                .map(str::to_owned)
                .ok_or_else(|| std::io::Error::other("no malformed individual mount record"))
        });
        anyhow::anyhow!("{error}; kernel record diagnostic: {invalid_record:?}")
    })?;
    let roots = OpenedRootCatalog::open(loaded.catalog(), uid, &snapshot)?;
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

#[test]
fn retained_source_fingerprints_only_declared_readable_source_paths() -> anyhow::Result<()> {
    let (directory, source, roots) = fixture()?;
    let source_path = directory.path().join("source");
    let nested = source_path.join("nested");
    std::fs::create_dir(&nested)?;
    std::fs::write(nested.join("video.mkv"), b"native media")?;
    std::fs::write(nested.join("video.en.srt"), b"native subtitles")?;
    // Generation identity is synthetic here; directory authority and reads are real.
    let retained = RetainedRootCatalog {
        source,
        roots,
        generation: SourceGeneration {
            number: 7,
            sha256: [0x53; 32],
        },
    };
    let expected = crate::media_discovery_fingerprint::fingerprint_media_aggregate(
        &nested.join("video.mkv"),
        &source_path,
    )?;
    assert!(expected.is_some());
    assert_eq!(
        retained.fingerprint(
            "source",
            std::path::Path::new("nested/video.mkv"),
            &|| { false },
            &mut std::time::Duration::default()
        )?,
        expected
    );
    assert_eq!(retained.generation().number, 7);
    assert_eq!(retained.generation().sha256, [0x53; 32]);
    for path in ["missing/video.mkv", "nested/missing.mkv"] {
        assert_eq!(
            retained.fingerprint(
                "source",
                std::path::Path::new(path),
                &|| false,
                &mut std::time::Duration::default()
            )?,
            None
        );
    }
    for key in ["unknown", "output"] {
        assert!(matches!(
            retained.fingerprint(
                key,
                std::path::Path::new("nested/video.mkv"),
                &|| false,
                &mut std::time::Duration::default()
            ),
            Err(crate::media_discovery_fingerprint::FingerprintError::Root(
                RootDirectoryError::InvalidPath
            ))
        ));
    }
    let outside = directory.path().join("outside");
    std::fs::create_dir(&outside)?;
    std::fs::write(outside.join("video.mkv"), b"outside source")?;
    std::os::unix::fs::symlink(&outside, source_path.join("linked"))?;
    assert!(matches!(
        retained.fingerprint(
            "source",
            std::path::Path::new("linked/video.mkv"),
            &|| { false },
            &mut std::time::Duration::default()
        ),
        Err(crate::media_discovery_fingerprint::FingerprintError::Root(
            RootDirectoryError::Filesystem { .. }
        ))
    ));
    assert_eq!(std::fs::read(outside.join("video.mkv"))?, b"outside source");
    assert_eq!(std::fs::read(nested.join("video.mkv"))?, b"native media");
    assert_eq!(
        std::fs::read(nested.join("video.en.srt"))?,
        b"native subtitles"
    );
    drop(retained);
    directory.close()?;
    Ok(())
}

#[test]
fn retained_source_rejects_a_replaced_root_instead_of_reporting_candidate_absence()
-> anyhow::Result<()> {
    let (directory, source, roots) = fixture()?;
    let retained = RetainedRootCatalog {
        source,
        roots,
        generation: SourceGeneration {
            number: 7,
            sha256: [0x53; 32],
        },
    };
    let moved = directory.path().join("moved-source");
    std::fs::rename(directory.path().join("source"), &moved)?;
    std::fs::create_dir(directory.path().join("source"))?;
    assert!(matches!(
        retained.fingerprint(
            "source",
            std::path::Path::new("original"),
            &|| false,
            &mut std::time::Duration::default()
        ),
        Err(crate::media_discovery_fingerprint::FingerprintError::Root(
            RootDirectoryError::IdentityChanged
        ))
    ));
    assert_eq!(
        std::fs::read(moved.join("original"))?,
        b"original source bytes"
    );
    drop(retained);
    directory.close()?;
    Ok(())
}
