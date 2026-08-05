use super::{
    FilesystemSidecarDiscoverer, SidecarDiscoverer, SidecarDiscoveryError, SidecarDiscoveryLimits,
    SidecarFormat, SidecarRole, discover_sidecar_subtitles, discover_sidecar_subtitles_with_limits,
    source_directory,
};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn relative_source_uses_current_directory() {
    assert_eq!(source_directory(Path::new("movie.mkv")), Path::new("."));
}

#[test]
fn default_discoverer_uses_reviewed_limits() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        SidecarDiscoveryLimits::default(),
        SidecarDiscoveryLimits::reviewed()
    );
    let directory = temp_directory("default-discoverer")?;
    let source = directory.join("movie.mkv");
    fs::write(directory.join("movie.en.srt"), b"subtitle")?;

    let sidecars = FilesystemSidecarDiscoverer::default().discover(&source)?;
    assert_eq!(sidecars.len(), 1);
    assert_eq!(sidecars[0].format, SidecarFormat::Srt);

    remove_temp_directory(&directory)
}

#[test]
fn rejects_invalid_source_and_unreadable_parent() -> Result<(), Box<dyn std::error::Error>> {
    assert!(matches!(
        discover_sidecar_subtitles(Path::new("")),
        Err(SidecarDiscoveryError::InvalidSourcePath(_))
    ));
    let missing_parent = temp_directory("missing-parent")?;
    remove_temp_directory(&missing_parent)?;
    assert!(matches!(
        discover_sidecar_subtitles(&missing_parent.join("movie.mkv")),
        Err(SidecarDiscoveryError::DirectoryRead { .. })
    ));
    Ok(())
}

fn temp_directory(test_name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "revaer-sidecar-{test_name}-{}-{counter}",
        std::process::id()
    ));
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn remove_temp_directory(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    fs::remove_dir_all(path)?;
    Ok(())
}

#[test]
fn discovers_supported_patterns_in_deterministic_order() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("patterns")?;
    let source = directory.join("Movie.Name.mkv");
    for name in [
        "Movie.Name.sdh.vtt",
        "Movie.Name.eng.sup",
        "Movie.Name.en.srt",
        "Movie.Name.eng.forced.ass",
        "Movie.Name.trailer.en.srt",
        "Other.en.srt",
        "Movie.Name.en.txt",
    ] {
        fs::write(directory.join(name), b"fixture")?;
    }

    let sidecars = discover_sidecar_subtitles(&source)?;
    let names = sidecars
        .iter()
        .filter_map(|sidecar| sidecar.path.file_name())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "Movie.Name.en.srt",
            "Movie.Name.eng.forced.ass",
            "Movie.Name.eng.sup",
            "Movie.Name.sdh.vtt"
        ]
    );
    assert_eq!(sidecars[0].language.as_deref(), Some("eng"));
    assert_eq!(sidecars[1].role, Some(SidecarRole::Forced));
    assert!(sidecars[2].format.image_based());

    remove_temp_directory(&directory)
}

#[test]
fn sidecar_budget_accepts_maximum_and_rejects_maximum_plus_one()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("sidecar-budget")?;
    let source = directory.join("movie.mkv");
    fs::write(directory.join("movie.en.srt"), b"one")?;
    let limits = SidecarDiscoveryLimits {
        max_directory_entries: 2,
        max_sidecars: 1,
        max_total_bytes: 6,
    };
    assert_eq!(
        discover_sidecar_subtitles_with_limits(&source, limits)?.len(),
        1
    );

    fs::write(directory.join("movie.fr.srt"), b"two")?;
    assert!(matches!(
        discover_sidecar_subtitles_with_limits(&source, limits),
        Err(SidecarDiscoveryError::SidecarLimitExceeded(1))
    ));

    remove_temp_directory(&directory)
}

#[test]
fn total_bytes_accepts_maximum_and_rejects_maximum_plus_one()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("byte-budget")?;
    let source = directory.join("movie.mkv");
    fs::write(directory.join("movie.en.idx"), b"idx")?;
    fs::write(directory.join("movie.en.sub"), b"data")?;
    let exact = SidecarDiscoveryLimits {
        max_directory_entries: 2,
        max_sidecars: 1,
        max_total_bytes: 7,
    };
    let sidecars = discover_sidecar_subtitles_with_limits(&source, exact)?;
    assert_eq!(sidecars[0].size_bytes, 7);

    assert!(matches!(
        discover_sidecar_subtitles_with_limits(
            &source,
            SidecarDiscoveryLimits {
                max_total_bytes: 6,
                ..exact
            }
        ),
        Err(SidecarDiscoveryError::TotalByteLimitExceeded(6))
    ));

    remove_temp_directory(&directory)
}

#[test]
fn large_unrelated_directory_streams_under_exact_entry_budget()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("entry-budget")?;
    let source = directory.join("movie.mkv");
    for index in 0..512 {
        fs::write(directory.join(format!("unrelated-{index}.bin")), b"x")?;
    }
    fs::create_dir(directory.join("other.en.srt"))?;
    fs::create_dir(directory.join("movie.en.txt"))?;
    fs::write(directory.join("movie.en.srt"), b"subtitle")?;
    let exact = SidecarDiscoveryLimits {
        max_directory_entries: 515,
        max_sidecars: 1,
        max_total_bytes: 8,
    };
    assert_eq!(
        discover_sidecar_subtitles_with_limits(&source, exact)?.len(),
        1
    );
    assert!(matches!(
        discover_sidecar_subtitles_with_limits(
            &source,
            SidecarDiscoveryLimits {
                max_directory_entries: 514,
                ..exact
            }
        ),
        Err(SidecarDiscoveryError::DirectoryEntryLimitExceeded(514))
    ));

    remove_temp_directory(&directory)
}

#[test]
fn targeted_vobsub_lookup_coalesces_pair_once() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("vobsub")?;
    let source = directory.join("episode.mkv");
    let index = directory.join("episode.ja.forced.idx");
    let companion = directory.join("episode.ja.forced.sub");
    fs::write(&index, b"index")?;
    fs::write(&companion, b"data")?;

    let sidecars = discover_sidecar_subtitles(&source)?;
    assert_eq!(sidecars.len(), 1);
    assert_eq!(sidecars[0].path, index);
    assert_eq!(sidecars[0].companion_path.as_ref(), Some(&companion));
    assert_eq!(sidecars[0].format, SidecarFormat::VobSub);
    assert_eq!(sidecars[0].language.as_deref(), Some("jpn"));
    assert_eq!(sidecars[0].role, Some(SidecarRole::Forced));

    remove_temp_directory(&directory)
}

#[test]
fn rejects_incomplete_vobsub_pair() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("vobsub-missing")?;
    let source = directory.join("movie.mkv");
    let index = directory.join("movie.eng.idx");
    fs::write(&index, b"index")?;

    assert!(matches!(
        discover_sidecar_subtitles(&source),
        Err(SidecarDiscoveryError::MissingVobSubCompanion(path)) if path == index
    ));

    remove_temp_directory(&directory)
}

#[test]
fn rejects_normalized_identity_collisions() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("identity")?;
    let source = directory.join("movie.mkv");
    fs::write(directory.join("movie.en.srt"), b"one")?;
    fs::write(directory.join("movie.eng.srt"), b"two")?;

    assert!(matches!(
        discover_sidecar_subtitles(&source),
        Err(SidecarDiscoveryError::AmbiguousIdentity(_))
    ));

    remove_temp_directory(&directory)
}

#[test]
fn rejects_matching_non_regular_candidate() -> Result<(), Box<dyn std::error::Error>> {
    let directory = temp_directory("non-regular")?;
    let source = directory.join("movie.mkv");
    let candidate = directory.join("movie.en.srt");
    fs::create_dir(&candidate)?;

    assert!(matches!(
        discover_sidecar_subtitles(&source),
        Err(SidecarDiscoveryError::UnsafeCandidate(path)) if path == candidate
    ));

    remove_temp_directory(&directory)
}

#[cfg(unix)]
#[test]
fn rejects_matching_symlink_candidate() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::symlink;

    let directory = temp_directory("symlink")?;
    let source = directory.join("movie.mkv");
    let target = directory.join("target.srt");
    let candidate = directory.join("movie.en.srt");
    fs::write(&target, b"subtitle")?;
    symlink(&target, &candidate)?;

    assert!(matches!(
        discover_sidecar_subtitles(&source),
        Err(SidecarDiscoveryError::UnsafeCandidate(path)) if path == candidate
    ));

    remove_temp_directory(&directory)
}

#[cfg(unix)]
#[test]
fn rejects_symlinked_vobsub_companion() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::fs::symlink;

    let directory = temp_directory("vobsub-symlink")?;
    let source = directory.join("movie.mkv");
    let target = directory.join("target.sub");
    let companion = directory.join("movie.en.sub");
    fs::write(directory.join("movie.en.idx"), b"index")?;
    fs::write(&target, b"data")?;
    symlink(&target, &companion)?;

    assert!(matches!(
        discover_sidecar_subtitles(&source),
        Err(SidecarDiscoveryError::UnsafeCandidate(path)) if path == companion
    ));

    remove_temp_directory(&directory)
}

#[cfg(target_os = "linux")]
#[test]
fn rejects_non_utf8_directory_entry() -> Result<(), Box<dyn std::error::Error>> {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let directory = temp_directory("non-utf8")?;
    let source = directory.join("movie.mkv");
    let invalid_name = OsString::from_vec(b"movie.en.srt\xff".to_vec());
    fs::write(directory.join(invalid_name), b"subtitle")?;

    assert!(matches!(
        discover_sidecar_subtitles(&source),
        Err(SidecarDiscoveryError::NonUtf8Entry(_))
    ));

    remove_temp_directory(&directory)
}
