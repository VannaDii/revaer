use super::*;

const SNAPSHOT: &str = "10 1 8:1 / /base rw - ext4 /dev/disk rw\n11 1 8:1 /library /alias rw shared:2 future:3 - ext4 /dev/disk rw\n12 1 8:2 / /other rw - ext4 /dev/other rw\n";

#[test]
fn namespace_mounts_preserve_kernel_identity_and_bind_aliases() -> anyhow::Result<()> {
    let topology = RootMountTopology::parse(concat!(
        "34 1 8:1 / / rw - ext4 /dev/root rw\n",
        "390 34 0:4 net:[4026532274] /run/docker/netns/one rw shared:333 - nsfs nsfs rw\n",
        "391 34 0:4 net:[4026532274] /run/docker/netns/two rw - nsfs nsfs rw\n",
        "392 34 0:4 net:[4026532275] /run/docker/netns/other rw - nsfs nsfs rw\n",
    ))?;
    let first = topology.resolve(390, (0, 4), Path::new("/run/docker/netns/one"))?;
    let alias = topology.resolve(391, (0, 4), Path::new("/run/docker/netns/two"))?;
    let other = topology.resolve(392, (0, 4), Path::new("/run/docker/netns/other"))?;
    assert_eq!(first.filesystem_type(), "nsfs");
    assert_eq!(first.path, Path::new("net:[4026532274]"));
    assert_eq!(first.reject_overlap(&alias), Err(RootMountError::Overlap));
    first.reject_overlap(&other)?;
    for kind in ["cgroup", "ipc", "mnt", "pid", "time", "user", "uts"] {
        decode_root(&format!("{kind}:[4026532274]"), "nsfs")?;
    }
    Ok(())
}

#[test]
fn namespace_identity_does_not_relax_directory_path_validation() {
    for input in [
        "390 34 0:4 net:[4026532274] /run/netns rw - ext4 none rw",
        "390 34 0:4 net:[4026532274] relative rw - nsfs nsfs rw",
        "390 34 0:4 net:[0] /run/netns rw - nsfs nsfs rw",
        "390 34 0:4 net:[01] /run/netns rw - nsfs nsfs rw",
        "390 34 0:4 net:[+1] /run/netns rw - nsfs nsfs rw",
        "390 34 0:4 net:[18446744073709551616] /run/netns rw - nsfs nsfs rw",
        "390 34 0:4 other:[1] /run/netns rw - nsfs nsfs rw",
        "390 34 0:4 net:[1]/../path /run/netns rw - nsfs nsfs rw",
    ] {
        assert!(RootMountTopology::parse(input).is_err());
    }
}

fn compare_regions(left: &[MountLocation], right: &[MountLocation]) -> Result<(), RootMountError> {
    for first in left {
        for second in right {
            first.reject_overlap(second)?;
        }
    }
    Ok(())
}

#[test]
fn descendant_mount_cannot_alias_another_root_or_its_descendant() -> anyhow::Result<()> {
    let topology = RootMountTopology::parse(concat!(
        "10 1 8:1 / /base rw - ext4 none rw\n",
        "11 10 8:2 /library /base/source/alias rw - xfs none rw\n",
        "12 1 8:2 / /other rw - xfs none rw\n",
        "13 10 8:2 /library/season /base/output/nested rw - xfs none rw\n",
    ))?;
    let source = topology.regions(10, (8, 1), Path::new("/base/source"))?;
    let direct = topology.regions(12, (8, 2), Path::new("/other/library"))?;
    let output = topology.regions(10, (8, 1), Path::new("/base/output"))?;
    assert_eq!(
        compare_regions(&source, &direct),
        Err(RootMountError::Overlap)
    );
    assert_eq!(
        compare_regions(&source, &output),
        Err(RootMountError::Overlap)
    );
    assert_eq!(
        compare_regions(&output, &source),
        Err(RootMountError::Overlap)
    );
    let separate = topology.regions(12, (8, 2), Path::new("/other/library-sibling"))?;
    compare_regions(&source, &separate)?;
    Ok(())
}

#[test]
fn covered_descendant_mount_does_not_hide_observed_alias() -> anyhow::Result<()> {
    let topology = RootMountTopology::parse(concat!(
        "10 1 8:1 / /base rw - ext4 none rw\n",
        "11 10 8:2 /library /base/source/nested rw - xfs none rw\n",
        "12 11 8:3 /unrelated /base/source/nested rw - ext4 none rw\n",
        "13 1 8:2 / /other rw - xfs none rw\n",
    ))?;
    let source = topology.regions(10, (8, 1), Path::new("/base/source"))?;
    let other = topology.regions(13, (8, 2), Path::new("/other/library"))?;
    assert_eq!(
        compare_regions(&source, &other),
        Err(RootMountError::Overlap)
    );
    let sibling = topology.regions(10, (8, 1), Path::new("/base/source-sibling"))?;
    compare_regions(&sibling, &other)?;
    Ok(())
}

#[test]
fn bind_aliases_use_filesystem_paths_not_visible_paths() -> anyhow::Result<()> {
    let topology = RootMountTopology::parse(SNAPSHOT)?;
    let original = topology.resolve(10, (8, 1), Path::new("/base/library"))?;
    let alias = topology.resolve(11, (8, 1), Path::new("/alias"))?;
    let nested = topology.resolve(11, (8, 1), Path::new("/alias/season"))?;
    for other in [&alias, &nested] {
        assert_eq!(original.reject_overlap(other), Err(RootMountError::Overlap));
        assert_eq!(
            other.reject_overlap(&original),
            Err(RootMountError::Overlap)
        );
    }
    let sibling = topology.resolve(10, (8, 1), Path::new("/base/library-other"))?;
    original.reject_overlap(&sibling)?;
    let different_device = topology.resolve(12, (8, 2), Path::new("/other/library"))?;
    original.reject_overlap(&different_device)?;
    Ok(())
}

#[test]
fn missing_mount_wrong_device_and_path_are_not_evidence() -> anyhow::Result<()> {
    let topology = RootMountTopology::parse(SNAPSHOT)?;
    for (id, device, path) in [
        (13, (8, 1), "/base"),
        (10, (8, 2), "/base"),
        (10, (8, 1), "/base-other"),
    ] {
        assert!(matches!(
            topology.resolve(id, device, Path::new(path)),
            Err(RootMountError::IdentityMismatch)
        ));
    }
    Ok(())
}

#[test]
fn escaped_kernel_paths_round_trip_without_double_decoding() -> anyhow::Result<()> {
    let topology =
        RootMountTopology::parse("10 1 8:1 /a\\040b /a\\011b\\012c\\134040 rw - ext4 none rw\n")?;
    let location = topology.resolve(10, (8, 1), Path::new("/a\tb\nc\\040/child"))?;
    assert_eq!(location.path, Path::new("/a b/child"));
    assert_eq!(format!("{topology:?}"), "RootMountTopology");
    Ok(())
}

#[test]
fn malformed_records_and_duplicate_ids_fail_closed() {
    for input in [
        "",
        "\n",
        "10 1 8:1 / / rw ext4 none rw",
        "10 1 8:1 / / rw - ext4 none",
        "10 1 8:1 / / rw - ext4 none rw trailing",
        "10 1 8:1 / / rw -  none rw",
        "+10 1 8:1 / / rw - ext4 none rw",
        "18446744073709551615 1 8:1 / / rw - ext4 none rw",
        "10 1 4294967296:1 / / rw - ext4 none rw",
        "10 1 8:1 /../a / rw - ext4 none rw",
        "10 1 8:1 / /a\\000 rw - ext4 none rw",
        "10 1 8:1 / /a\\04 rw - ext4 none rw",
        "10 1 8:1 / /a//b rw - ext4 none rw",
        "10 1 8:1 / relative rw - ext4 none rw",
    ] {
        assert!(
            RootMountTopology::parse(input).is_err(),
            "malformed record was accepted"
        );
    }
    assert!(matches!(
        RootMountTopology::parse(&format!("{SNAPSHOT}{SNAPSHOT}")),
        Err(RootMountError::Duplicate)
    ));
}

#[test]
fn contradictory_filesystem_observations_fail_closed() -> anyhow::Result<()> {
    let topology = RootMountTopology::parse(
        "10 1 8:1 / /first rw - ext4 none rw\n11 1 8:1 / /second rw - xfs none rw\n",
    )?;
    let first = topology.resolve(10, (8, 1), Path::new("/first"))?;
    let second = topology.resolve(11, (8, 1), Path::new("/second"))?;
    assert_eq!(
        first.reject_overlap(&second),
        Err(RootMountError::IdentityMismatch)
    );
    Ok(())
}

#[test]
fn linux_snapshot_matches_opened_root_descriptor() -> anyhow::Result<()> {
    use crate::root_catalog::{ProcRootMountSource, RootMountSource};
    let directory = tempfile::Builder::new()
        .prefix(".root-mount-test-")
        .tempdir_in(std::env::current_dir()?)?;
    let catalog =
        crate::root_catalog::parse_root_catalog_v1(&serde_json::to_vec(&serde_json::json!({
            "format_version": 1,
            "slots": [{
                "key": "source", "path": directory.path(), "allowed_kinds": ["source"],
                "durability_class": "disposable", "durability_evidence": "none",
                "sole_writer_class": "uncontrolled", "sole_writer_evidence": "none"
            }]
        }))?)?;
    let topology = ProcRootMountSource.snapshot()?;
    let opened = crate::root_catalog::OpenedRootCatalog::open(
        &catalog,
        rustix::process::geteuid().as_raw(),
        &topology,
    )?;
    opened.revalidate(&topology)?;
    let observed = opened.observations(&topology)?;
    assert_eq!(observed.len(), 1);
    assert_eq!(Path::new(observed[0].canonical_path()), directory.path());
    let missing = RootMountTopology::parse(SNAPSHOT)?;
    assert!(opened.revalidate(&missing).is_err());
    assert!(
        crate::root_catalog::OpenedRootCatalog::open(
            &catalog,
            rustix::process::geteuid().as_raw(),
            &missing
        )
        .is_err()
    );
    assert_eq!(std::fs::read_dir(directory.path())?.count(), 0);
    drop(opened);
    directory.close()?;
    Ok(())
}
