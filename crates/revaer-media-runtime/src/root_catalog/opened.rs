//! Catalog-scoped descriptor ownership; deployment attestation remains separate.

use super::{OpenedRootDirectory, RootCatalog, RootDirectoryError, RootMountTopology};

/// Retains all declared root descriptors and exclusive locks together.
///
/// Opening performs no capability writes. Observed root equality and ancestry
/// are rejected across distinct slots, regardless of logical kind. A single
/// source/output slot is opened once. The supplied mount snapshot also checks
/// filesystem-relative aliases, including mounts below each root. This remains
/// provisional: snapshot provenance/freshness, deployment ownership and
/// durability must be proven before writable probes or bound job admission.
#[derive(Debug)]
pub struct OpenedRootCatalog {
    roots: Vec<OpenedRootDirectory>,
}

impl OpenedRootCatalog {
    /// Open every declared slot, retaining locks until the catalog is dropped.
    ///
    /// # Errors
    /// Rejects unsupported platforms, unsafe or changed roots, competing locks
    /// and observed overlap. Failure drops every previously acquired handle.
    pub fn open(
        catalog: &RootCatalog,
        service_uid: u32,
        topology: &RootMountTopology,
    ) -> Result<Self, RootDirectoryError> {
        if !cfg!(all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )) {
            return Err(RootDirectoryError::PlatformUnsupported);
        }
        if service_uid != rustix::process::geteuid().as_raw() {
            return Err(RootDirectoryError::ServiceIdentityMismatch);
        }
        let mut opened = Self { roots: Vec::new() };
        for slot in catalog.slots() {
            opened
                .roots
                .push(OpenedRootDirectory::open(slot, service_uid)?);
        }
        opened.revalidate(topology)?;
        Ok(opened)
    }

    /// Recheck retained links and cross-slot directory identities without writes.
    ///
    /// # Errors
    /// Propagates identity, ancestry, mount-ID and observed overlap failures.
    pub fn revalidate(&self, topology: &RootMountTopology) -> Result<(), RootDirectoryError> {
        self.revalidate_directories()?;
        self.reject_mount_aliases(topology)?;
        self.revalidate_directories()
    }

    /// Probe root traversal/enumeration between fresh namespace observations.
    /// Retains all descriptors and locks and performs no capability writes.
    /// This establishes neither descendant access nor deployment/durability.
    ///
    /// # Errors
    /// Fails if either fresh snapshot, catalog revalidation or root read fails.
    pub fn probe_reads(
        &self,
        mounts: &dyn super::RootMountSource,
    ) -> Result<(), RootDirectoryError> {
        self.revalidate(&mounts.snapshot()?)?;
        for root in &self.roots {
            root.probe_reads()?;
        }
        self.revalidate(&mounts.snapshot()?)
    }

    /// Probe the declared root capabilities between fresh namespace snapshots.
    /// Source-only roots are never written. Owned write probes are cleaned up
    /// before returning; any failure prevents publication of all results.
    ///
    /// # Errors
    /// Rejects namespace, descriptor, capability and owned-cleanup failures.
    pub fn probe_capabilities(
        &self,
        mounts: &dyn super::RootMountSource,
    ) -> Result<Vec<[bool; 7]>, RootDirectoryError> {
        self.revalidate(&mounts.snapshot()?)?;
        let capabilities = self
            .roots
            .iter()
            .map(OpenedRootDirectory::probe_capabilities)
            .collect::<Result<Vec<_>, _>>()?;
        self.revalidate(&mounts.snapshot()?)?;
        Ok(capabilities)
    }

    /// Observe every retained root in the source catalog's logical-key order.
    /// Handles and locks stay owned by this catalog; observations confer no
    /// capabilities, deployment ownership or durability.
    ///
    /// # Errors
    /// Rejects changed descriptors or inconsistent/overlapping mount evidence.
    pub fn observations(
        &self,
        topology: &RootMountTopology,
    ) -> Result<Vec<super::RootDirectoryObservation>, RootDirectoryError> {
        self.revalidate(topology)?;
        let observed = self
            .roots
            .iter()
            .map(|root| root.observe(topology))
            .collect::<Result<Vec<_>, _>>()?;
        self.revalidate(topology)?;
        Ok(observed)
    }

    /// Open a read-only candidate parent from a retained catalog slot.
    /// `slot_index` is the trusted source catalog's logical-key order, not a
    /// path or an index accepted from an operator request.
    ///
    /// # Errors
    /// Rejects an absent slot, invalid relative path, changed root or unsafe
    /// descendant traversal. The caller must retain this catalog during use.
    pub fn open_read_parent(
        &self,
        slot_index: usize,
        relative_candidate: &std::path::Path,
    ) -> Result<std::os::fd::OwnedFd, RootDirectoryError> {
        self.roots
            .get(slot_index)
            .ok_or(RootDirectoryError::InvalidPath)?
            .open_read_parent(relative_candidate)
    }

    /// Open a read-only directory from a retained trusted catalog slot.
    /// Empty relative paths select the declared root. Slot indices come from
    /// the trusted catalog, never from an operator-supplied path.
    ///
    /// # Errors
    /// Rejects absent slots, invalid paths, changed roots and unsafe descendants.
    /// The caller must retain this catalog while using the descriptor.
    pub fn open_read_directory(
        &self,
        slot_index: usize,
        relative_directory: &std::path::Path,
    ) -> Result<std::os::fd::OwnedFd, RootDirectoryError> {
        self.roots
            .get(slot_index)
            .ok_or(RootDirectoryError::InvalidPath)?
            .open_read_directory(relative_directory)
    }

    fn revalidate_directories(&self) -> Result<(), RootDirectoryError> {
        for (index, root) in self.roots.iter().enumerate() {
            root.revalidate()?;
            for other in &self.roots[index + 1..] {
                root.reject_observed_overlap(other)?;
            }
        }
        Ok(())
    }

    /// Reject aliases of declared roots using a supplied current Linux snapshot.
    /// This compares filesystem-relative locations, not mount-point strings.
    /// Descendant mount records participate conservatively, including covered
    /// records. Deployment/durability proof remains separate.
    ///
    /// # Errors
    /// Rejects missing or mismatched descriptor observations and root aliases.
    #[cfg(target_os = "linux")]
    fn reject_mount_aliases(
        &self,
        topology: &super::RootMountTopology,
    ) -> Result<(), RootDirectoryError> {
        let locations = self
            .roots
            .iter()
            .map(|root| root.mount_regions(topology))
            .collect::<Result<Vec<_>, _>>()?;
        for (index, location) in locations.iter().enumerate() {
            for other in &locations[index + 1..] {
                for left in location {
                    for right in other {
                        left.reject_overlap(right)?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use crate::root_catalog::parse_root_catalog_v1;

    struct ObservedMounts {
        calls: std::sync::atomic::AtomicUsize,
        failure_call: Option<usize>,
    }

    impl super::super::RootMountSource for ObservedMounts {
        fn snapshot(&self) -> Result<RootMountTopology, super::super::RootMountReadError> {
            use super::super::{ProcRootMountSource, RootMountReadError};
            let call = self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
            if self.failure_call == Some(call) {
                return Err(RootMountReadError::Filesystem(std::io::Error::other(
                    "injected namespace observation failure",
                )));
            }
            ProcRootMountSource.snapshot()
        }
    }

    #[test]
    fn catalog_read_probe_requires_both_fresh_snapshots_without_writing() -> anyhow::Result<()> {
        use super::super::{ProcRootMountSource, RootMountSource};
        let directory = tempfile::Builder::new()
            .prefix(".catalog-read-test-")
            .tempdir_in(std::env::current_dir()?)?;
        std::fs::write(directory.path().join("original"), b"unchanged")?;
        let catalog = parse_root_catalog_v1(&serde_json::to_vec(&serde_json::json!({
            "format_version": 1,
            "slots": [{
                "key": "source", "path": directory.path(), "allowed_kinds": ["source"],
                "durability_class": "disposable", "durability_evidence": "none",
                "sole_writer_class": "uncontrolled", "sole_writer_evidence": "none"
            }]
        }))?)?;
        let opened = OpenedRootCatalog::open(
            &catalog,
            rustix::process::geteuid().as_raw(),
            &ProcRootMountSource.snapshot()?,
        )?;
        let parent = opened.open_read_parent(0, std::path::Path::new("original"))?;
        assert!(matches!(
            opened.open_read_parent(1, std::path::Path::new("original")),
            Err(RootDirectoryError::InvalidPath)
        ));
        drop(parent);
        let reader = opened.open_read_directory(0, std::path::Path::new(""))?;
        assert!(matches!(
            opened.open_read_directory(1, std::path::Path::new("")),
            Err(RootDirectoryError::InvalidPath)
        ));
        drop(reader);
        for failure_call in [None, Some(1), Some(2)] {
            let mounts = ObservedMounts {
                calls: std::sync::atomic::AtomicUsize::new(0),
                failure_call,
            };
            let result = opened.probe_reads(&mounts);
            if failure_call.is_some() {
                assert!(matches!(result, Err(RootDirectoryError::MountRead(_))));
            } else {
                result?;
            }
            assert_eq!(
                mounts.calls.load(std::sync::atomic::Ordering::SeqCst),
                failure_call.unwrap_or(2)
            );
            assert_eq!(
                std::fs::read(directory.path().join("original"))?,
                b"unchanged"
            );
            assert_eq!(std::fs::read_dir(directory.path())?.count(), 1);
        }
        drop(opened);
        directory.close()?;
        Ok(())
    }

    #[test]
    fn failed_catalog_open_releases_earlier_locks_without_writing() -> anyhow::Result<()> {
        let directory = tempfile::Builder::new()
            .prefix(".catalog-lock-test-")
            .tempdir_in(std::env::current_dir()?)?;
        let child = directory.path().join("child");
        std::fs::create_dir(&child)?;
        let slots =
            [("parent", directory.path()), ("child", child.as_path())].map(|(key, path)| {
                serde_json::json!({
                    "key": key, "path": path, "allowed_kinds": ["workspace"],
                    "durability_class": "disposable", "durability_evidence": "none",
                    "sole_writer_class": "revaer_exclusive",
                    "sole_writer_evidence": "linux_dedicated_service"
                })
            });
        let catalog = parse_root_catalog_v1(&serde_json::to_vec(&serde_json::json!({
            "format_version": 1, "slots": slots
        }))?)?;
        let uid = rustix::process::geteuid().as_raw();
        let topology = RootMountTopology::parse(&std::fs::read_to_string("/proc/self/mountinfo")?)?;
        assert!(matches!(
            OpenedRootCatalog::open(&catalog, uid, &topology),
            Err(RootDirectoryError::Overlap)
        ));
        for slot in catalog.slots() {
            let handle = OpenedRootDirectory::open(slot, uid)?;
            handle.revalidate()?;
        }
        assert_eq!(std::fs::read_dir(directory.path())?.count(), 1);
        assert_eq!(std::fs::read_dir(&child)?.count(), 0);
        directory.close()?;
        Ok(())
    }
}
