use super::*;

#[test]
fn replacement_path_does_not_redirect_descriptor_operations() -> anyhow::Result<()> {
    let parent = tempfile::tempdir()?;
    let path = parent.path().join("root");
    let moved = parent.path().join("moved");
    std::fs::create_dir(&path)?;
    let descriptor = File::open(&path)?;
    probe(&descriptor, "owned-probe", |stage| {
        if stage == "written" {
            std::fs::rename(&path, &moved)?;
            std::fs::create_dir(&path)?;
            std::fs::write(path.join("replacement"), b"untouched")?;
        }
        Ok(())
    })?;
    assert_eq!(std::fs::read(path.join("replacement"))?, b"untouched");
    assert_eq!(std::fs::read_dir(&path)?.count(), 1);
    assert_eq!(std::fs::read_dir(&moved)?.count(), 0);
    parent.close()?;
    Ok(())
}

#[test]
fn rename_collision_never_overwrites_or_deletes_unowned_content() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let collision = directory.path().join("owned-probe/after");
    let result = probe(&File::open(directory.path())?, "owned-probe", |stage| {
        if stage == "written" {
            std::fs::write(&collision, b"preserve")?;
        }
        Ok(())
    });
    match result {
        Err(RootWriteProbeError::Cleanup { original, failures }) => {
            assert!(matches!(
                original.as_deref(),
                Some(RootWriteProbeError::Operation {
                    operation: "rename probe file",
                    ..
                })
            ));
            assert_eq!(failures.len(), 1);
        }
        other => anyhow::bail!("unexpected rename outcome: {other:?}"),
    }
    assert_eq!(std::fs::read(collision)?, b"preserve");
    assert!(!directory.path().join("owned-probe/before").exists());
    directory.close()?;
    Ok(())
}

#[test]
fn writes_probe_and_cleans_only_owned_entries() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let source = directory.path().join("original.txt");
    std::fs::write(&source, b"untouched source")?;
    probe_root_writes(&File::open(directory.path())?)?;
    assert_eq!(std::fs::read(&source)?, b"untouched source");
    assert_eq!(std::fs::read_dir(directory.path())?.count(), 1);
    directory.close()?;
    Ok(())
}

#[test]
fn every_injected_failure_removes_probe_entries() -> anyhow::Result<()> {
    for stage in ["written", "renamed"] {
        let directory = tempfile::tempdir()?;
        let result = probe(&File::open(directory.path())?, "owned-probe", |observed| {
            if stage == observed {
                Err(io::Error::other("injected failure"))
            } else {
                Ok(())
            }
        });
        assert!(matches!(result, Err(RootWriteProbeError::Operation { .. })));
        assert_eq!(std::fs::read_dir(directory.path())?.count(), 0);
        directory.close()?;
    }
    Ok(())
}

#[test]
fn collision_preserves_existing_entry_and_contents() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let collision = directory.path().join("collision");
    std::fs::create_dir(&collision)?;
    std::fs::write(collision.join("original"), b"preserve")?;
    assert!(probe(&File::open(directory.path())?, "collision", |_| Ok(())).is_err());
    assert_eq!(std::fs::read(collision.join("original"))?, b"preserve");
    directory.close()?;
    Ok(())
}

#[test]
fn cleanup_failure_preserves_original_error_and_unowned_entry() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let result = probe(&File::open(directory.path())?, "owned-probe", |_| {
        std::fs::write(
            directory.path().join("owned-probe/unexpected"),
            b"not owned by probe",
        )?;
        Err(io::Error::other("injected failure"))
    });
    match result {
        Err(RootWriteProbeError::Cleanup { original, failures }) => {
            assert!(original.is_some());
            assert_eq!(failures.len(), 1);
        }
        other => anyhow::bail!("unexpected probe outcome: {other:?}"),
    }
    assert_eq!(
        std::fs::read(directory.path().join("owned-probe/unexpected"))?,
        b"not owned by probe"
    );
    assert!(!directory.path().join("owned-probe/before").exists());
    directory.close()?;
    Ok(())
}
