use std::cell::Cell;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::{Builder, TempDir};

use super::{digest_hex, valid_document};
use crate::root_catalog::local_file::LoadPhase;
use crate::root_catalog::{
    MAX_ROOT_CATALOG_DOCUMENT_BYTES, MAX_ROOT_CATALOG_PATH_BYTES, RootCatalogSource,
    RootCatalogSourceError, RootCatalogSourceState, RootCatalogSourceTrust,
    RootCatalogTrustViolation, TrustedLocalRootCatalogSource,
};

fn operator_uid() -> u32 {
    rustix::process::geteuid().as_raw()
}

fn trusted_tempdir() -> TempDir {
    // Shared /tmp ancestry is intentionally untrusted, even with a private child.
    let home = std::env::var_os("HOME").expect("operator home for trusted source fixtures");
    let trusted_base = fs::canonicalize(home).expect("canonical operator home");
    Builder::new()
        .prefix("revaer-root-catalog.")
        .tempdir_in(trusted_base)
        .expect("trusted test directory")
}

fn write_valid(path: &Path) {
    fs::write(path, valid_document()).expect("write valid catalog");
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).expect("secure catalog mode");
}

fn source(path: &Path) -> TrustedLocalRootCatalogSource {
    TrustedLocalRootCatalogSource::native_override(
        path.to_str().expect("UTF-8 test path"),
        operator_uid(),
    )
    .expect("valid source location")
}

trait HostFilesystemLoad {
    fn load_host(&self) -> Result<crate::root_catalog::RootCatalogLoad, RootCatalogSourceError>;
}

impl HostFilesystemLoad for TrustedLocalRootCatalogSource {
    fn load_host(&self) -> Result<crate::root_catalog::RootCatalogLoad, RootCatalogSourceError> {
        self.load_with_observer(|_phase| {})
    }
}

fn assert_untrusted(result: Result<crate::root_catalog::RootCatalogLoad, RootCatalogSourceError>) {
    let error = result.expect_err("source must fail closed");
    assert_eq!(error.reason_code(), "media_root_catalog_source_untrusted");
}

#[test]
fn trusted_native_file_loads_once_with_bounded_evidence() -> Result<(), Box<dyn std::error::Error>>
{
    let directory = trusted_tempdir();
    let path = directory.path().join("catalog.json");
    write_valid(&path);
    let load = source(&path).load_host()?;
    assert_eq!(load.state(), RootCatalogSourceState::Loaded);
    assert_eq!(load.state().remediation_reason(), None);
    assert_eq!(load.catalog().slots().len(), 1);
    let evidence = load.file_evidence().ok_or("loaded file evidence missing")?;
    assert_eq!(evidence.trust(), RootCatalogSourceTrust::NativeOverride);
    assert_eq!(evidence.owner_uid(), operator_uid());
    assert_eq!(evidence.mode() & 0o777, 0o600);
    assert_eq!(evidence.mode(), fs::metadata(&path)?.mode());
    assert_eq!(evidence.document_bytes(), valid_document().len());
    Ok(())
}

#[test]
fn public_local_source_support_is_exactly_linux_amd64_and_arm64() {
    let directory = trusted_tempdir();
    let path = directory.path().join("catalog.json");
    write_valid(&path);
    let source = source(&path);
    let result = RootCatalogSource::load(&source);
    if cfg!(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    )) {
        assert_eq!(
            result.expect("supported release target").state(),
            RootCatalogSourceState::Loaded
        );
    } else {
        assert!(matches!(
            result,
            Err(RootCatalogSourceError::PlatformUnsupported)
        ));
    }
}

#[test]
fn loaded_empty_source_is_distinct_from_missing_source() -> anyhow::Result<()> {
    let directory = trusted_tempdir();
    let path = directory.path().join("catalog.json");
    fs::write(&path, br#"{"format_version":1,"slots":[]}"#)?;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
    let loaded = source(&path).load_host()?;
    let missing = source(&directory.path().join("missing.json")).load_host()?;
    assert_eq!(loaded.state(), RootCatalogSourceState::Loaded);
    assert_eq!(loaded.state().remediation_reason(), None);
    assert!(loaded.file_evidence().is_some());
    assert_eq!(missing.state(), RootCatalogSourceState::Missing);
    assert!(missing.file_evidence().is_none());
    assert!(loaded.catalog().is_empty());
    assert_eq!(loaded.catalog(), missing.catalog());
    Ok(())
}

#[test]
fn missing_file_and_missing_parent_return_empty_remediation_catalogs() {
    let directory = trusted_tempdir();
    for path in [
        directory.path().join("missing.json"),
        directory.path().join("absent").join("catalog.json"),
    ] {
        let load = source(&path)
            .load_host()
            .expect("missing is remediation state");
        assert_eq!(load.state(), RootCatalogSourceState::Missing);
        assert_eq!(
            load.state().remediation_reason(),
            Some("media_root_catalog_source_missing")
        );
        assert!(load.catalog().is_empty());
        assert!(load.file_evidence().is_none());
        assert_eq!(
            digest_hex(load.catalog().semantic_sha256()),
            "8f0b256ad26c139e8c51bd426ba510eb43aefca9cb1d58fb0737b6518e6bc867"
        );
    }
}

#[test]
fn native_override_location_bound_is_exact_and_never_canonicalized() {
    for length in [MAX_ROOT_CATALOG_PATH_BYTES - 1, MAX_ROOT_CATALOG_PATH_BYTES] {
        let location = format!("/{}", "a".repeat(length - 1));
        assert!(TrustedLocalRootCatalogSource::native_override(&location, operator_uid()).is_ok());
    }
    for invalid in [
        "relative/catalog.json".to_owned(),
        "/catalog\0.json".to_owned(),
        format!("/{}", "a".repeat(MAX_ROOT_CATALOG_PATH_BYTES)),
    ] {
        let error = TrustedLocalRootCatalogSource::native_override(&invalid, operator_uid())
            .expect_err("invalid override");
        assert!(matches!(
            error,
            RootCatalogSourceError::SourceUntrusted {
                violation: RootCatalogTrustViolation::InvalidLocation
            }
        ));
    }

    let directory = trusted_tempdir();
    let valid_path = directory.path().join("catalog.json");
    write_valid(&valid_path);
    let directory_text = directory.path().to_str().expect("UTF-8 directory");
    for location in [
        format!("{directory_text}//catalog.json"),
        format!("{directory_text}/./catalog.json"),
        format!("{directory_text}/child/../catalog.json"),
        format!("{directory_text}/catalog.json/"),
    ] {
        let source = TrustedLocalRootCatalogSource::native_override(&location, operator_uid())
            .expect("bounded absolute location");
        assert!(matches!(
            source.load_host(),
            Err(RootCatalogSourceError::SourceUntrusted {
                violation: RootCatalogTrustViolation::NonCanonicalLocation
            })
        ));
    }
}

#[test]
fn policy_identity_and_root_service_mismatches_fail_before_open() {
    let directory = trusted_tempdir();
    let path = directory.path().join("catalog.json");
    write_valid(&path);
    let different_uid = operator_uid().checked_add(1).expect("test uid increment");
    let wrong_operator = TrustedLocalRootCatalogSource::native_override(
        path.to_str().expect("UTF-8 path"),
        different_uid,
    )
    .expect("location is valid");
    assert!(matches!(
        wrong_operator.load_host(),
        Err(RootCatalogSourceError::SourceUntrusted {
            violation: RootCatalogTrustViolation::PolicyIdentityMismatch
        })
    ));
    assert!(matches!(
        TrustedLocalRootCatalogSource::packaged(0).load_host(),
        Err(RootCatalogSourceError::SourceUntrusted {
            violation: RootCatalogTrustViolation::PackagedServiceIsRoot
        })
    ));
}

#[test]
fn invalid_document_and_one_extra_source_byte_keep_distinct_reason_codes() {
    let directory = trusted_tempdir();
    let path = directory.path().join("catalog.json");
    fs::write(&path, b"not-json").expect("write invalid catalog");
    let error = source(&path).load_host().expect_err("invalid document");
    assert_eq!(error.reason_code(), "media_root_catalog_format_invalid");

    let base = br#"{"format_version":1,"slots":[]}"#;
    let mut exact = base.to_vec();
    exact.resize(MAX_ROOT_CATALOG_DOCUMENT_BYTES, b' ');
    fs::write(&path, &exact).expect("write exact bound");
    assert!(source(&path).load_host().is_ok());

    exact.push(b' ');
    fs::write(&path, &exact).expect("write one above bound");
    let error = source(&path).load_host().expect_err("bound exceeded");
    assert!(matches!(
        error,
        RootCatalogSourceError::BoundExceeded {
            maximum_bytes: MAX_ROOT_CATALOG_DOCUMENT_BYTES
        }
    ));
    assert_eq!(error.reason_code(), "media_root_catalog_bound_exceeded");
}

#[test]
fn invalid_utf8_from_a_trusted_descriptor_is_format_invalid() {
    let directory = trusted_tempdir();
    let path = directory.path().join("catalog.json");
    fs::write(&path, [b'{', 0xff, b'}']).expect("write invalid UTF-8");
    let error = source(&path).load_host().expect_err("invalid UTF-8");
    assert_eq!(error.reason_code(), "media_root_catalog_format_invalid");
}

#[test]
fn symlinked_final_and_parent_entries_fail_closed() {
    let directory = trusted_tempdir();
    let real = directory.path().join("real.json");
    write_valid(&real);
    let link = directory.path().join("link.json");
    symlink(&real, &link).expect("final symlink");
    assert_untrusted(source(&link).load_host());

    let real_parent = directory.path().join("real-parent");
    fs::create_dir(&real_parent).expect("real parent");
    fs::set_permissions(&real_parent, fs::Permissions::from_mode(0o700))
        .expect("secure parent mode");
    write_valid(&real_parent.join("catalog.json"));
    let linked_parent = directory.path().join("linked-parent");
    symlink(&real_parent, &linked_parent).expect("parent symlink");
    assert_untrusted(source(&linked_parent.join("catalog.json")).load_host());
}

#[test]
fn hard_links_directories_sockets_and_fifos_are_rejected_without_blocking() {
    let directory = trusted_tempdir();
    let regular = directory.path().join("regular.json");
    write_valid(&regular);
    let hard_link = directory.path().join("hard-link.json");
    fs::hard_link(&regular, &hard_link).expect("hard link");
    assert!(matches!(
        source(&regular).load_host(),
        Err(RootCatalogSourceError::SourceUntrusted {
            violation: RootCatalogTrustViolation::MultipleHardLinks
        })
    ));

    let child_directory = directory.path().join("directory.json");
    fs::create_dir(&child_directory).expect("directory source");
    assert_untrusted(source(&child_directory).load_host());

    let socket = directory.path().join("socket.json");
    let _listener = UnixListener::bind(&socket).expect("Unix socket");
    assert_untrusted(source(&socket).load_host());

    let fifo = directory.path().join("fifo.json");
    let status = Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .expect("POSIX mkfifo available");
    assert!(status.success());
    assert_untrusted(source(&fifo).load_host());
}

#[test]
fn group_world_writable_files_and_unsafe_ancestry_are_rejected() {
    let directory = trusted_tempdir();
    for mode in [0o620, 0o602, 0o666] {
        let path = directory.path().join(format!("catalog-{mode:o}.json"));
        write_valid(&path);
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).expect("set unsafe mode");
        assert!(matches!(
            source(&path).load_host(),
            Err(RootCatalogSourceError::SourceUntrusted {
                violation: RootCatalogTrustViolation::UntrustedWritableMode
            })
        ));
    }

    let unsafe_parent = directory.path().join("unsafe-parent");
    fs::create_dir(&unsafe_parent).expect("unsafe parent");
    fs::set_permissions(&unsafe_parent, fs::Permissions::from_mode(0o770))
        .expect("unsafe parent mode");
    let path = unsafe_parent.join("catalog.json");
    write_valid(&path);
    assert!(matches!(
        source(&path).load_host(),
        Err(RootCatalogSourceError::SourceUntrusted {
            violation: RootCatalogTrustViolation::UnsafeDirectory
        })
    ));
}

#[test]
fn mismatched_file_owner_is_rejected() {
    let directory = trusted_tempdir();
    let path = directory.path().join("catalog.json");
    write_valid(&path);
    let source = TrustedLocalRootCatalogSource::native_with_file_owner_for_test(
        path.to_str().expect("UTF-8 path"),
        operator_uid(),
        operator_uid().checked_add(1).expect("test uid increment"),
    );
    assert!(matches!(
        source.load_host(),
        Err(RootCatalogSourceError::SourceUntrusted {
            violation: RootCatalogTrustViolation::OwnerMismatch
        })
    ));
}

fn mutate_same_length(path: &Path) {
    let replacement = valid_document().replace("media-library", "media-librarx");
    assert_eq!(replacement.len(), valid_document().len());
    fs::write(path, replacement).expect("same-length mutation");
}

#[test]
fn same_length_content_mutation_is_detected_at_every_load_phase() {
    for target_phase in [
        LoadPhase::Opened,
        LoadPhase::ReadProgress,
        LoadPhase::ReadComplete,
        LoadPhase::Parsed,
    ] {
        let directory = trusted_tempdir();
        let path = directory.path().join("catalog.json");
        write_valid(&path);
        let mutated = Cell::new(false);
        let result = source(&path).load_with_observer(|phase| {
            if phase == target_phase && !mutated.replace(true) {
                mutate_same_length(&path);
            }
        });
        assert_untrusted(result);
        assert!(mutated.get(), "observer reached {target_phase:?}");
    }
}

#[test]
fn truncation_extension_chmod_and_unlink_during_read_fail_closed() {
    for mutation in ["truncate", "extend", "chmod", "unlink"] {
        let directory = trusted_tempdir();
        let path = directory.path().join("catalog.json");
        write_valid(&path);
        let mutated = Cell::new(false);
        let result = source(&path).load_with_observer(|phase| {
            if phase != LoadPhase::ReadProgress || mutated.replace(true) {
                return;
            }
            match mutation {
                "truncate" => {
                    OpenOptions::new()
                        .write(true)
                        .truncate(true)
                        .open(&path)
                        .expect("truncate source");
                }
                "extend" => {
                    OpenOptions::new()
                        .append(true)
                        .open(&path)
                        .expect("open for append")
                        .write_all(b" ")
                        .expect("extend source");
                }
                "chmod" => fs::set_permissions(&path, fs::Permissions::from_mode(0o640))
                    .expect("chmod source"),
                "unlink" => fs::remove_file(&path).expect("unlink source"),
                _ => unreachable!("closed test mutation set"),
            }
        });
        assert_untrusted(result);
        assert!(mutated.get(), "mutation {mutation}");
    }
}

#[test]
fn final_entry_rename_and_replacement_are_detected() {
    let directory = trusted_tempdir();
    let path = directory.path().join("catalog.json");
    let moved = directory.path().join("moved.json");
    write_valid(&path);
    let mutated = Cell::new(false);
    let result = source(&path).load_with_observer(|phase| {
        if phase == LoadPhase::ReadComplete && !mutated.replace(true) {
            fs::rename(&path, &moved).expect("rename opened source");
            write_valid(&path);
        }
    });
    assert_untrusted(result);
    assert!(mutated.get(), "source entry replacement hook ran");
}

#[test]
fn parent_rename_and_replacement_after_parse_are_detected() {
    let directory = trusted_tempdir();
    let parent = directory.path().join("trusted-parent");
    let moved = directory.path().join("moved-parent");
    fs::create_dir(&parent).expect("trusted parent");
    fs::set_permissions(&parent, fs::Permissions::from_mode(0o700)).expect("secure parent");
    let path = parent.join("catalog.json");
    write_valid(&path);
    let mutated = Cell::new(false);
    let result = source(&path).load_with_observer(|phase| {
        if phase == LoadPhase::Parsed && !mutated.replace(true) {
            fs::rename(&parent, &moved).expect("rename source parent");
            fs::create_dir(&parent).expect("replacement parent");
            fs::set_permissions(&parent, fs::Permissions::from_mode(0o700))
                .expect("secure replacement parent");
            write_valid(&parent.join("catalog.json"));
        }
    });
    assert_untrusted(result);
    assert!(mutated.get(), "source parent replacement hook ran");
}

#[test]
fn hard_link_creation_after_open_is_detected() {
    let directory = trusted_tempdir();
    let path = directory.path().join("catalog.json");
    let hard_link = directory.path().join("second-name.json");
    write_valid(&path);
    let linked = Cell::new(false);
    let result = source(&path).load_with_observer(|phase| {
        if phase == LoadPhase::Opened && !linked.replace(true) {
            fs::hard_link(&path, &hard_link).expect("create second link");
        }
    });
    assert!(matches!(
        result,
        Err(RootCatalogSourceError::SourceUntrusted {
            violation: RootCatalogTrustViolation::MultipleHardLinks
        })
    ));
}

#[test]
fn observer_without_mutation_preserves_valid_load() {
    let directory = trusted_tempdir();
    let path = directory.path().join("catalog.json");
    write_valid(&path);
    let phases = Cell::new(0_u8);
    let load = source(&path)
        .load_with_observer(|_phase| phases.set(phases.get().saturating_add(1)))
        .expect("stable observed load");
    assert_eq!(load.state(), RootCatalogSourceState::Loaded);
    assert_eq!(phases.get(), 4);
}

#[test]
fn source_error_reason_codes_cover_all_stable_classes() {
    assert_eq!(
        RootCatalogSourceError::SourceUntrusted {
            violation: RootCatalogTrustViolation::SourceChanged
        }
        .reason_code(),
        "media_root_catalog_source_untrusted"
    );
    assert_eq!(
        RootCatalogSourceError::PlatformUnsupported.reason_code(),
        "media_root_platform_unsupported"
    );
}

#[test]
fn source_path_is_not_returned_in_filesystem_errors() {
    let directory = trusted_tempdir();
    let path: PathBuf = directory.path().join("catalog.json");
    write_valid(&path);
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o770))
        .expect("make ancestry untrusted");
    let result = source(&path).load_host();
    fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))
        .expect("restore directory access");
    let error = result.expect_err("untrusted ancestry");
    assert!(
        !error
            .to_string()
            .contains(path.to_str().expect("UTF-8 path"))
    );
}
