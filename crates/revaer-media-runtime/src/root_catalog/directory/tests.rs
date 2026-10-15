use std::os::unix::fs::{PermissionsExt, symlink};

use super::*;
use crate::root_catalog::parse_root_catalog_v1;

#[cfg(target_os = "linux")]
#[test]
fn linux_mount_identity_is_descriptor_derived_and_distinguishes_proc() -> anyhow::Result<()> {
    let root = File::open("/")?;
    let proc = File::open("/proc")?;
    let first = mount_id(&root, "", AtFlags::EMPTY_PATH)?;
    assert_eq!(first, mount_id(&root, "", AtFlags::EMPTY_PATH)?);
    let proc_id = mount_id(&proc, "", AtFlags::EMPTY_PATH)?;
    assert_ne!(first, proc_id);
    assert_eq!(proc_id, mount_id(&root, "proc", AtFlags::SYMLINK_NOFOLLOW)?);
    Ok(())
}

#[test]
fn missing_out_of_range_and_changed_mount_identity_fail_closed() -> anyhow::Result<()> {
    assert!(matches!(
        validate_mount_id(false, 42),
        Err(RootDirectoryError::MountIdentityUnavailable)
    ));
    assert!(matches!(
        validate_mount_id(true, u64::MAX),
        Err(RootDirectoryError::MountIdentityUnavailable)
    ));
    assert_eq!(validate_mount_id(true, 0)?, 0);
    assert_eq!(
        validate_mount_id(true, 9_223_372_036_854_775_807)?,
        9_223_372_036_854_775_807
    );
    assert!(matches!(
        same_mount(42, 43),
        Err(RootDirectoryError::IdentityChanged)
    ));
    same_mount(42, 42)?;
    Ok(())
}

fn fixture() -> anyhow::Result<tempfile::TempDir> {
    Ok(tempfile::Builder::new()
        .prefix(".root-descriptor-test-")
        .tempdir_in(std::env::current_dir()?)?)
}

#[test]
fn missing_candidate_parent_is_distinct_from_lost_declared_root() -> anyhow::Result<()> {
    let directory = fixture()?;
    let root = directory.path().join("root");
    let nested = root.join("Movies");
    std::fs::create_dir_all(&nested)?;
    let opened = OpenedRootDirectory::open_descriptors(
        &declaration(&root, false)?,
        rustix::process::geteuid().as_raw(),
    )?;
    std::fs::remove_dir(&nested)?;
    assert!(matches!(
        opened.open_read_parent(std::path::Path::new("Movies/original.mkv")),
        Err(RootDirectoryError::CandidateMissing)
    ));
    opened.revalidate()?;
    std::fs::rename(&root, directory.path().join("moved"))?;
    let lost = opened.open_read_parent(std::path::Path::new("Movies/original.mkv"));
    assert!(lost.is_err());
    assert!(!matches!(lost, Err(RootDirectoryError::CandidateMissing)));
    directory.close()?;
    Ok(())
}

#[test]
fn retained_candidate_parent_is_read_only_bounded_and_symlink_safe() -> anyhow::Result<()> {
    let directory = fixture()?;
    let root = directory.path().join("root");
    let nested = root.join("Movies");
    std::fs::create_dir_all(&nested)?;
    std::fs::write(nested.join("original.mkv"), b"preserve")?;
    let opened = OpenedRootDirectory::open_descriptors(
        &declaration(&root, false)?,
        rustix::process::geteuid().as_raw(),
    )?;
    let parent = opened.open_read_parent(std::path::Path::new("Movies/original.mkv"))?;
    let file = fs::openat(
        &parent,
        "original.mkv",
        OFlags::RDONLY | OFlags::NOFOLLOW,
        Mode::empty(),
    )?;
    assert_eq!(fs::fstat(&file)?.st_size, 8);
    let overlong = "a".repeat(4097);
    for path in [
        "",
        "/outside",
        "../outside",
        "Movies//a",
        "Movies/./a",
        "Movies/../a",
        "Movies\\a",
        overlong.as_str(),
    ] {
        assert!(matches!(
            opened.open_read_parent(std::path::Path::new(path)),
            Err(RootDirectoryError::InvalidPath)
        ));
    }
    symlink(&nested, root.join("alias"))?;
    assert!(
        opened
            .open_read_parent(std::path::Path::new("alias/original.mkv"))
            .is_err()
    );
    assert_eq!(std::fs::read(nested.join("original.mkv"))?, b"preserve");
    std::fs::rename(&root, directory.path().join("moved"))?;
    std::fs::create_dir(&root)?;
    assert!(matches!(
        opened.open_read_parent(std::path::Path::new("Movies/original.mkv")),
        Err(RootDirectoryError::IdentityChanged)
    ));
    drop(file);
    drop(parent);
    drop(opened);
    directory.close()?;
    Ok(())
}

#[test]
fn retained_directory_inventory_rejects_escape_and_replaced_roots() -> anyhow::Result<()> {
    let directory = fixture()?;
    let root = directory.path().join("root");
    let nested = root.join("Movies");
    std::fs::create_dir_all(&nested)?;
    std::fs::write(nested.join("original.mkv"), b"preserve")?;
    let opened = OpenedRootDirectory::open_descriptors(
        &declaration(&root, false)?,
        rustix::process::geteuid().as_raw(),
    )?;
    let root_reader = opened.open_read_directory(std::path::Path::new(""))?;
    let reader = opened.open_read_directory(std::path::Path::new("Movies"))?;
    assert_eq!(fs::fstat(&reader)?.st_ino, fs::stat(&nested)?.st_ino);
    assert_eq!(fs::fstat(&root_reader)?.st_ino, fs::stat(&root)?.st_ino);
    for invalid in [
        "/outside",
        "../outside",
        ".",
        "Movies/..",
        "Movies/",
        "Movies//child",
        "Movies\\child",
    ] {
        assert!(matches!(
            opened.open_read_directory(std::path::Path::new(invalid)),
            Err(RootDirectoryError::InvalidPath)
        ));
    }
    assert!(matches!(
        opened.open_read_directory(std::path::Path::new("missing")),
        Err(RootDirectoryError::CandidateMissing)
    ));
    symlink(&nested, root.join("alias"))?;
    assert!(
        opened
            .open_read_directory(std::path::Path::new("alias"))
            .is_err()
    );
    std::fs::rename(&root, directory.path().join("moved"))?;
    std::fs::create_dir(&root)?;
    assert!(matches!(
        opened.open_read_directory(std::path::Path::new("Movies")),
        Err(RootDirectoryError::IdentityChanged)
    ));
    assert_eq!(
        std::fs::read(directory.path().join("moved/Movies/original.mkv"))?,
        b"preserve"
    );
    drop(reader);
    drop(root_reader);
    drop(opened);
    directory.close()?;
    Ok(())
}

#[test]
fn read_probe_preserves_source_and_rejects_replacement() -> anyhow::Result<()> {
    let directory = fixture()?;
    let child = directory.path().join("root");
    std::fs::create_dir(&child)?;
    std::fs::write(child.join("original"), b"preserve")?;
    let opened = OpenedRootDirectory::open_descriptors(
        &declaration(&child, false)?,
        rustix::process::geteuid().as_raw(),
    )?;
    opened.probe_reads()?;
    opened.probe_reads()?;
    assert_eq!(std::fs::read(child.join("original"))?, b"preserve");
    assert_eq!(std::fs::read_dir(&child)?.count(), 1);
    std::fs::rename(&child, directory.path().join("moved"))?;
    std::fs::create_dir(&child)?;
    assert!(matches!(
        opened.probe_reads(),
        Err(RootDirectoryError::IdentityChanged)
    ));
    assert_eq!(std::fs::read_dir(&child)?.count(), 0);
    drop(opened);
    directory.close()?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn read_probe_rejects_root_without_search_permission() -> anyhow::Result<()> {
    let directory = fixture()?;
    let child = directory.path().join("root");
    std::fs::create_dir(&child)?;
    std::fs::set_permissions(&child, std::fs::Permissions::from_mode(0o400))?;
    let opened = OpenedRootDirectory::open_descriptors(
        &declaration(&child, false)?,
        rustix::process::geteuid().as_raw(),
    )?;
    let result = opened.probe_reads();
    std::fs::set_permissions(&child, std::fs::Permissions::from_mode(0o700))?;
    assert!(matches!(
        result,
        Err(RootDirectoryError::Filesystem {
            operation: "probe root traversal",
            source,
        }) if source.kind() == io::ErrorKind::PermissionDenied
    ));
    drop(opened);
    directory.close()?;
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn observed_identity_matches_descriptor_without_readiness_or_writes() -> anyhow::Result<()> {
    use crate::root_catalog::{ProcRootMountSource, RootMountSource};
    let directory = fixture()?;
    let child = directory.path().join("root");
    std::fs::create_dir(&child)?;
    std::fs::write(child.join("original"), b"preserve")?;
    let slot = declaration(&child, false)?;
    let opened = OpenedRootDirectory::open(&slot, rustix::process::geteuid().as_raw())?;
    let snapshot = ProcRootMountSource.snapshot()?;
    let observed = opened.observe(&snapshot)?;
    let stat = fs::fstat(&File::open(&child)?)?;
    assert_eq!(observed.canonical_path(), slot.path());
    assert_eq!(observed.filesystem_device(), stat.st_dev);
    assert_eq!(observed.filesystem_inode(), stat.st_ino);
    assert_eq!(
        observed.mount_id(),
        mount_id(&File::open(&child)?, "", AtFlags::EMPTY_PATH)?
    );
    assert_eq!(observed.owner_uid(), stat.st_uid);
    assert_eq!(observed.owner_gid(), stat.st_gid);
    assert_eq!(observed.mode_bits(), stat.st_mode & 0o7777);
    assert!(!observed.filesystem_type().is_empty());
    assert_eq!(format!("{observed:?}"), "RootDirectoryObservation");
    assert_eq!(std::fs::read(child.join("original"))?, b"preserve");
    assert_eq!(std::fs::read_dir(&child)?.count(), 1);
    std::fs::rename(&child, directory.path().join("moved"))?;
    std::fs::create_dir(&child)?;
    assert!(matches!(
        opened.observe(&ProcRootMountSource.snapshot()?),
        Err(RootDirectoryError::IdentityChanged)
    ));
    drop(opened);
    directory.close()?;
    Ok(())
}

#[test]
fn physical_overlap_rejects_equal_and_nested_roots_but_accepts_siblings() -> anyhow::Result<()> {
    let directory = fixture()?;
    let child = directory.path().join("child");
    let sibling = directory.path().join("child-prefix");
    std::fs::create_dir(&child)?;
    std::fs::create_dir(&sibling)?;
    let uid = rustix::process::geteuid().as_raw();
    let parent =
        OpenedRootDirectory::open_descriptors(&declaration(directory.path(), false)?, uid)?;
    let first = OpenedRootDirectory::open_descriptors(&declaration(&child, false)?, uid)?;
    let equal = OpenedRootDirectory::open_descriptors(&declaration(&child, false)?, uid)?;
    let separate = OpenedRootDirectory::open_descriptors(&declaration(&sibling, false)?, uid)?;
    for (left, right) in [(&parent, &first), (&first, &parent), (&first, &equal)] {
        assert!(matches!(
            left.reject_observed_overlap(right),
            Err(RootDirectoryError::Overlap)
        ));
    }
    first.reject_observed_overlap(&separate)?;
    separate.reject_observed_overlap(&first)?;
    std::fs::rename(&child, directory.path().join("moved"))?;
    std::fs::create_dir(&child)?;
    assert!(matches!(
        first.reject_observed_overlap(&separate),
        Err(RootDirectoryError::IdentityChanged)
    ));
    drop((parent, first, equal, separate));
    directory.close()?;
    Ok(())
}

#[test]
fn root_lock_is_exclusive_across_processes() -> anyhow::Result<()> {
    if std::env::var_os("REVAER_ROOT_LOCK_ISOLATED").is_none() {
        return run_isolated_lock_test("root_lock_is_exclusive_across_processes");
    }
    let directory = fixture()?;
    let slot = declaration(directory.path(), true)?;
    let owner = OpenedRootDirectory::open_descriptors(&slot, rustix::process::geteuid().as_raw())?;
    run_lock_child(directory.path(), "busy")?;
    drop(owner);
    run_lock_child(directory.path(), "free")?;
    directory.close()?;
    Ok(())
}

fn run_lock_child(path: &std::path::Path, expected: &str) -> anyhow::Result<()> {
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "root_catalog::directory::tests::root_lock_child_entry",
            "--test-threads=1",
        ])
        .env_clear()
        .env("REVAER_ROOT_LOCK_TEST_PATH", path)
        .env("REVAER_ROOT_LOCK_TEST_EXPECTED", expected)
        .stdin(std::process::Stdio::null());
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let output = command.output()?;
    anyhow::ensure!(output.status.success(), "root-lock child failed");
    anyhow::ensure!(
        String::from_utf8(output.stdout)?.contains("1 passed"),
        "root-lock child did not execute its case"
    );
    Ok(())
}

#[test]
fn root_lock_child_entry() -> anyhow::Result<()> {
    let Some(path) = std::env::var_os("REVAER_ROOT_LOCK_TEST_PATH") else {
        return Ok(());
    };
    let expected = std::env::var("REVAER_ROOT_LOCK_TEST_EXPECTED")?;
    let slot = declaration(std::path::Path::new(&path), true)?;
    let result = OpenedRootDirectory::open_descriptors(&slot, rustix::process::geteuid().as_raw());
    match expected.as_str() {
        "busy" => anyhow::ensure!(
            matches!(
                result,
                Err(RootDirectoryError::Filesystem {
                    operation: "acquire exclusive root lock",
                    ..
                })
            ),
            "child unexpectedly acquired locked root"
        ),
        "free" => result?.revalidate()?,
        _ => anyhow::bail!("invalid child lock expectation"),
    }
    Ok(())
}

fn declaration(path: &std::path::Path, writing: bool) -> anyhow::Result<RootCatalogSlot> {
    declaration_with_writer(path, writing, writing)
}

fn declaration_with_writer(
    path: &std::path::Path,
    writing: bool,
    exclusive: bool,
) -> anyhow::Result<RootCatalogSlot> {
    let document = serde_json::json!({
        "format_version": 1,
        "slots": [{
            "key": "probe-root", "path": path,
            "allowed_kinds": [if writing { "workspace" } else { "source" }],
            "durability_class": "disposable", "durability_evidence": "none",
            "sole_writer_class": if exclusive { "revaer_exclusive" } else { "uncontrolled" },
            "sole_writer_evidence": if exclusive { "linux_dedicated_service" } else { "none" }
        }]
    });
    let catalog = parse_root_catalog_v1(&serde_json::to_vec(&document)?)?;
    catalog
        .slots()
        .first()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("missing test slot"))
}

#[test]
fn exclusive_source_declaration_holds_lock_without_enabling_writes() -> anyhow::Result<()> {
    let directory = fixture()?;
    let slot = declaration_with_writer(directory.path(), false, true)?;
    let uid = rustix::process::geteuid().as_raw();
    let owner = OpenedRootDirectory::open_descriptors(&slot, uid)?;
    assert!(matches!(
        owner.probe_writes(),
        Err(RootDirectoryError::WriteNotDeclared)
    ));
    assert!(matches!(
        OpenedRootDirectory::open_descriptors(&slot, uid),
        Err(RootDirectoryError::Filesystem {
            operation: "acquire exclusive root lock",
            ..
        })
    ));
    drop(owner);
    directory.close()?;
    Ok(())
}

#[test]
fn root_lock_blocks_competitor_and_releases_with_owner() -> anyhow::Result<()> {
    if std::env::var_os("REVAER_ROOT_LOCK_ISOLATED").is_none() {
        return run_isolated_lock_test("root_lock_blocks_competitor_and_releases_with_owner");
    }
    let directory = fixture()?;
    let slot = declaration(directory.path(), true)?;
    let uid = rustix::process::geteuid().as_raw();
    let owner = OpenedRootDirectory::open_descriptors(&slot, uid)?;
    owner.probe_writes()?;
    assert!(matches!(
        OpenedRootDirectory::open_descriptors(&slot, uid),
        Err(RootDirectoryError::Filesystem {
            operation: "acquire exclusive root lock",
            ..
        })
    ));
    assert_eq!(std::fs::read_dir(directory.path())?.count(), 0);
    drop(owner);
    let successor = OpenedRootDirectory::open_descriptors(&slot, uid)?;
    successor.probe_writes()?;
    drop(successor);
    directory.close()?;
    Ok(())
}

#[test]
fn source_only_never_probes_or_locks_for_writing() -> anyhow::Result<()> {
    let directory = fixture()?;
    std::fs::write(directory.path().join("source"), b"original")?;
    let slot = declaration(directory.path(), false)?;
    let uid = rustix::process::geteuid().as_raw();
    let first = OpenedRootDirectory::open_descriptors(&slot, uid)?;
    let second = OpenedRootDirectory::open_descriptors(&slot, uid)?;
    assert!(matches!(
        first.probe_writes(),
        Err(RootDirectoryError::WriteNotDeclared)
    ));
    assert_eq!(std::fs::read(directory.path().join("source"))?, b"original");
    assert_eq!(std::fs::read_dir(directory.path())?.count(), 1);
    drop((first, second));
    directory.close()?;
    Ok(())
}

#[test]
fn replaced_root_fails_before_writing_to_either_directory() -> anyhow::Result<()> {
    let directory = fixture()?;
    let root = directory.path().join("root");
    let moved = directory.path().join("moved");
    std::fs::create_dir(&root)?;
    let slot = declaration(&root, true)?;
    let opened = OpenedRootDirectory::open_descriptors(&slot, rustix::process::geteuid().as_raw())?;
    std::fs::rename(&root, &moved)?;
    std::fs::create_dir(&root)?;
    assert!(matches!(
        opened.probe_writes(),
        Err(RootDirectoryError::IdentityChanged)
    ));
    assert_eq!(std::fs::read_dir(&root)?.count(), 0);
    assert_eq!(std::fs::read_dir(&moved)?.count(), 0);
    drop(opened);
    directory.close()?;
    Ok(())
}

#[test]
fn symlink_and_writable_ancestry_fail_without_path_diagnostics() -> anyhow::Result<()> {
    let directory = fixture()?;
    let child = directory.path().join("child");
    std::fs::create_dir(&child)?;
    let alias = directory.path().join("alias");
    symlink(&child, &alias)?;
    let uid = rustix::process::geteuid().as_raw();
    let error = OpenedRootDirectory::open_descriptors(&declaration(&alias, true)?, uid)
        .err()
        .ok_or_else(|| anyhow::anyhow!("symlink accepted"))?;
    assert!(
        !format!("{error:?}").contains(
            directory
                .path()
                .to_str()
                .ok_or_else(|| anyhow::anyhow!("fixture path encoding"))?
        )
    );
    let slot = declaration(&child, true)?;
    let opened = OpenedRootDirectory::open_descriptors(&slot, uid)?;
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o777))?;
    assert!(matches!(
        opened.revalidate(),
        Err(RootDirectoryError::UnsafeAncestry)
    ));
    assert!(matches!(
        OpenedRootDirectory::open_descriptors(&slot, uid),
        Err(RootDirectoryError::UnsafeAncestry)
    ));
    std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700))?;
    drop(opened);
    directory.close()?;
    Ok(())
}

#[test]
fn public_platform_and_service_identity_are_not_inferred() -> anyhow::Result<()> {
    let directory = fixture()?;
    let slot = declaration(directory.path(), false)?;
    let uid = rustix::process::geteuid().as_raw();
    let other_uid = uid
        .checked_add(1)
        .ok_or_else(|| anyhow::anyhow!("test uid bound"))?;
    assert!(matches!(
        OpenedRootDirectory::open_descriptors(&slot, other_uid),
        Err(RootDirectoryError::ServiceIdentityMismatch)
    ));
    let result = OpenedRootDirectory::open(&slot, uid);
    if cfg!(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )) {
        result?.revalidate()?;
    } else {
        assert!(matches!(
            result,
            Err(RootDirectoryError::PlatformUnsupported)
        ));
    }
    directory.close()?;
    Ok(())
}

#[test]
fn noncanonical_components_fail_without_normalization() {
    for path in [
        "",
        "relative",
        "/",
        "/root/",
        "/root//child",
        "/root/./child",
        "/root/../child",
        "/root\0child",
    ] {
        assert!(components(path).is_err());
    }
    assert!(components(&format!("/{}", "x".repeat(4096))).is_err());
    assert_eq!(components("/root/child").ok(), Some(vec!["root", "child"]));
}

#[test]
fn root_lock_remains_held_until_inherited_child_stops() -> anyhow::Result<()> {
    if std::env::var_os("REVAER_ROOT_LOCK_ISOLATED").is_none() {
        return run_isolated_lock_test("root_lock_remains_held_until_inherited_child_stops");
    }
    let directory = fixture()?;
    let slot = declaration(directory.path(), true)?;
    let uid = rustix::process::geteuid().as_raw();
    let owner = OpenedRootDirectory::open_descriptors(&slot, uid)?;
    let mut child = std::process::Command::new("/bin/sleep").arg("30").spawn()?;
    drop(owner);
    let admitted_while_child_active = OpenedRootDirectory::open_descriptors(&slot, uid).is_ok();
    let kill = child.kill();
    let wait = child.wait();
    kill?;
    wait?;
    anyhow::ensure!(
        !admitted_while_child_active,
        "root admitted replay while old child remained active"
    );
    let restarted = OpenedRootDirectory::open_descriptors(&slot, uid)?;
    drop(restarted);
    directory.close()?;
    Ok(())
}

fn run_isolated_lock_test(name: &str) -> anyhow::Result<()> {
    // Descriptor inheritance is process-wide. A dedicated test process keeps
    // unrelated parallel fixtures from extending this root's lock lifetime.
    let mut command = std::process::Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            &format!("root_catalog::directory::tests::{name}"),
            "--test-threads=1",
        ])
        .env_clear()
        .env("REVAER_ROOT_LOCK_ISOLATED", "1")
        .stdin(std::process::Stdio::null());
    if let Some(path) = std::env::var_os("PATH") {
        command.env("PATH", path);
    }
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    let output = command.output()?;
    anyhow::ensure!(
        output.status.success(),
        "isolated root-lock test failed: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    anyhow::ensure!(
        String::from_utf8(output.stdout)?.contains("1 passed"),
        "isolated root-lock test did not execute"
    );
    Ok(())
}

#[test]
fn root_lock_blocks_replay_until_actual_ffmpeg_stops() -> anyhow::Result<()> {
    use std::io::BufRead;

    if std::env::var_os("REVAER_ROOT_LOCK_ISOLATED").is_none() {
        return run_isolated_lock_test("root_lock_blocks_replay_until_actual_ffmpeg_stops");
    }
    let directory = fixture()?;
    let slot = declaration(directory.path(), true)?;
    let uid = rustix::process::geteuid().as_raw();
    let owner = OpenedRootDirectory::open_descriptors(&slot, uid)?;
    let mut child = std::process::Command::new("ffmpeg")
        .args([
            "-nostdin",
            "-hide_banner",
            "-loglevel",
            "error",
            "-re",
            "-f",
            "lavfi",
            "-i",
            "color=s=64x64:r=1",
            "-t",
            "30",
            "-progress",
            "pipe:1",
            "-f",
            "null",
            "-",
        ])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .spawn()?;
    let observed = (|| -> anyhow::Result<bool> {
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("FFmpeg progress pipe missing"))?;
        let mut progress = String::new();
        std::io::BufReader::new(stdout).read_line(&mut progress)?;
        anyhow::ensure!(
            progress.starts_with("frame="),
            "FFmpeg did not begin media work"
        );
        anyhow::ensure!(
            child.try_wait()?.is_none(),
            "FFmpeg stopped before admission proof"
        );
        drop(owner);
        Ok(OpenedRootDirectory::open_descriptors(&slot, uid).is_ok())
    })();
    let kill = child.kill();
    let wait = child.wait();
    kill?;
    wait?;
    anyhow::ensure!(
        !observed?,
        "root admitted replay while FFmpeg remained active"
    );
    let restarted = OpenedRootDirectory::open_descriptors(&slot, uid)?;
    drop(restarted);
    directory.close()?;
    Ok(())
}
