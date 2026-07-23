//! Deterministic discovery of subtitle files adjacent to source media.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use thiserror::Error;

const SUPPORTED_EXTENSIONS: [&str; 6] = ["ass", "idx", "srt", "sub", "sup", "vtt"];

/// Normalized semantic role encoded in a sidecar file name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SidecarRole {
    /// Forced dialogue or signs.
    Forced,
    /// Commentary subtitles.
    Commentary,
    /// Subtitles for deaf and hard-of-hearing audiences.
    Sdh,
    /// Signs and songs only.
    SignsSongs,
    /// Karaoke timing or lyrics.
    Karaoke,
}

/// Normalized sidecar subtitle format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SidecarFormat {
    /// `SubRip` text subtitles.
    Srt,
    /// Advanced `SubStation Alpha` text subtitles.
    Ass,
    /// `WebVTT` text subtitles.
    Vtt,
    /// Blu-ray PGS image subtitles.
    Sup,
    /// Standalone `.sub` subtitle data.
    Sub,
    /// Paired `VobSub` `.idx` and `.sub` files.
    VobSub,
}

impl SidecarFormat {
    /// Return whether the format contains image subtitles and cannot be text-converted without OCR.
    #[must_use]
    pub const fn image_based(self) -> bool {
        matches!(self, Self::Sup | Self::VobSub)
    }
}

/// One discovered sidecar subtitle and its normalized filename semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidecarSubtitle {
    /// Primary sidecar path. `VobSub` pairs use the `.idx` path.
    pub path: PathBuf,
    /// Required companion `.sub` path for `VobSub` pairs.
    pub companion_path: Option<PathBuf>,
    /// Normalized two- or three-letter language token when present.
    pub language: Option<String>,
    /// Normalized semantic role when present.
    pub role: Option<SidecarRole>,
    /// Discovered subtitle format.
    pub format: SidecarFormat,
}

/// Sidecar discovery failure.
#[derive(Debug, Error)]
pub enum SidecarDiscoveryError {
    /// Source path does not identify a usable file name and stem.
    #[error("sidecar source path has no valid UTF-8 file stem: {0}")]
    InvalidSourcePath(PathBuf),
    /// Source directory could not be enumerated.
    #[error("sidecar source directory read failed for {path}: {source}")]
    DirectoryRead {
        /// Directory containing the source media.
        path: PathBuf,
        /// Filesystem failure.
        source: io::Error,
    },
    /// One directory entry could not be inspected.
    #[error("sidecar directory entry read failed for {path}: {source}")]
    EntryRead {
        /// Directory containing the source media.
        path: PathBuf,
        /// Filesystem failure.
        source: io::Error,
    },
    /// Two files resolve to the same normalized language, role, and format identity.
    #[error("ambiguous sidecar subtitle identity: {0}")]
    AmbiguousIdentity(String),
    /// A `VobSub` index lacks its required `.sub` data companion.
    #[error("VobSub index is missing its .sub companion: {0}")]
    MissingVobSubCompanion(PathBuf),
}

/// Injected sidecar discovery boundary.
pub trait SidecarDiscoverer: Send + Sync {
    /// Discover supported subtitle files adjacent to `source_path`.
    ///
    /// # Errors
    ///
    /// Returns a deterministic error when the directory cannot be inspected, a `VobSub` pair is
    /// incomplete, or multiple files claim the same normalized identity.
    fn discover(&self, source_path: &Path) -> Result<Vec<SidecarSubtitle>, SidecarDiscoveryError>;
}

/// Filesystem-backed sidecar discoverer.
#[derive(Debug, Default, Clone, Copy)]
pub struct FilesystemSidecarDiscoverer;

impl SidecarDiscoverer for FilesystemSidecarDiscoverer {
    fn discover(&self, source_path: &Path) -> Result<Vec<SidecarSubtitle>, SidecarDiscoveryError> {
        discover_sidecar_subtitles(source_path)
    }
}

/// Discover sidecars using the supported Revaer filename patterns.
///
/// Accepted forms are `{stem}.{lang}.{role}.{ext}`, `{stem}.{lang}.{ext}`, and
/// `{stem}.{role}.{ext}`. Unknown or malformed neighboring files are ignored. A matching `.idx`
/// is accepted only with a regular `.sub` companion and is returned as one `VobSub` sidecar.
///
/// # Errors
///
/// Returns a deterministic error when directory access fails, a `VobSub` pair is incomplete, or
/// normalized identities are ambiguous.
pub fn discover_sidecar_subtitles(
    source_path: &Path,
) -> Result<Vec<SidecarSubtitle>, SidecarDiscoveryError> {
    let stem = source_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| SidecarDiscoveryError::InvalidSourcePath(source_path.to_path_buf()))?;
    let directory = source_path.parent().unwrap_or_else(|| Path::new("."));
    let entries =
        fs::read_dir(directory).map_err(|source| SidecarDiscoveryError::DirectoryRead {
            path: directory.to_path_buf(),
            source,
        })?;

    let mut files = BTreeMap::new();
    for entry in entries {
        let entry = entry.map_err(|source| SidecarDiscoveryError::EntryRead {
            path: directory.to_path_buf(),
            source,
        })?;
        let file_type = entry
            .file_type()
            .map_err(|source| SidecarDiscoveryError::EntryRead {
                path: directory.to_path_buf(),
                source,
            })?;
        if !file_type.is_file() {
            continue;
        }
        let Some(file_name) = entry.file_name().to_str().map(str::to_string) else {
            continue;
        };
        files.insert(file_name.to_ascii_lowercase(), entry.path());
    }

    let prefix = format!("{}.", stem.to_ascii_lowercase());
    let mut sidecars = Vec::new();
    let mut identities = BTreeSet::new();
    for (lower_name, path) in &files {
        let Some(suffix) = lower_name.strip_prefix(&prefix) else {
            continue;
        };
        let Some(parsed) = parse_suffix(suffix) else {
            continue;
        };
        if parsed.extension == "sub" && files.contains_key(&replace_extension(lower_name, "idx")) {
            continue;
        }

        let (format, companion_path) = if parsed.extension == "idx" {
            let companion_key = replace_extension(lower_name, "sub");
            let Some(companion) = files.get(&companion_key) else {
                return Err(SidecarDiscoveryError::MissingVobSubCompanion(path.clone()));
            };
            (SidecarFormat::VobSub, Some(companion.clone()))
        } else {
            let Some(format) = format_from_extension(parsed.extension) else {
                continue;
            };
            (format, None)
        };
        let identity = (parsed.language.clone(), parsed.role, format);
        if !identities.insert(identity.clone()) {
            return Err(SidecarDiscoveryError::AmbiguousIdentity(format!(
                "language={} role={} format={format:?}",
                identity.0.as_deref().unwrap_or("none"),
                role_name(identity.1)
            )));
        }
        sidecars.push(SidecarSubtitle {
            path: path.clone(),
            companion_path,
            language: parsed.language,
            role: parsed.role,
            format,
        });
    }
    sidecars.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(sidecars)
}

#[derive(Debug)]
struct ParsedSuffix<'a> {
    language: Option<String>,
    role: Option<SidecarRole>,
    extension: &'a str,
}

fn parse_suffix(suffix: &str) -> Option<ParsedSuffix<'_>> {
    let tokens = suffix.split('.').collect::<Vec<_>>();
    let extension = *tokens.last()?;
    if !SUPPORTED_EXTENSIONS.contains(&extension) {
        return None;
    }
    match tokens.as_slice() {
        [token, _] => role_from_token(token).map_or_else(
            || {
                language_from_token(token).map(|language| ParsedSuffix {
                    language: Some(language),
                    role: None,
                    extension,
                })
            },
            |role| {
                Some(ParsedSuffix {
                    language: None,
                    role: Some(role),
                    extension,
                })
            },
        ),
        [language, role, _] => Some(ParsedSuffix {
            language: Some(language_from_token(language)?),
            role: Some(role_from_token(role)?),
            extension,
        }),
        _ => None,
    }
}

fn language_from_token(token: &str) -> Option<String> {
    let normalized = token.trim().to_ascii_lowercase();
    if !(2..=3).contains(&normalized.len())
        || !normalized.bytes().all(|byte| byte.is_ascii_alphabetic())
    {
        return None;
    }
    Some(match normalized.as_str() {
        "en" => "eng".to_string(),
        "fr" => "fra".to_string(),
        "de" => "deu".to_string(),
        "es" => "spa".to_string(),
        "it" => "ita".to_string(),
        "ja" => "jpn".to_string(),
        "ko" => "kor".to_string(),
        "pt" => "por".to_string(),
        "zh" => "zho".to_string(),
        _ => normalized,
    })
}

fn role_from_token(token: &str) -> Option<SidecarRole> {
    match token.trim().to_ascii_lowercase().replace('-', "_").as_str() {
        "forced" | "foreign" => Some(SidecarRole::Forced),
        "commentary" | "comment" => Some(SidecarRole::Commentary),
        "sdh" | "hi" | "hearing_impaired" => Some(SidecarRole::Sdh),
        "signs" | "songs" | "signs_songs" => Some(SidecarRole::SignsSongs),
        "karaoke" => Some(SidecarRole::Karaoke),
        _ => None,
    }
}

const fn format_from_extension(extension: &str) -> Option<SidecarFormat> {
    match extension.as_bytes() {
        b"srt" => Some(SidecarFormat::Srt),
        b"ass" => Some(SidecarFormat::Ass),
        b"vtt" => Some(SidecarFormat::Vtt),
        b"sup" => Some(SidecarFormat::Sup),
        b"sub" => Some(SidecarFormat::Sub),
        _ => None,
    }
}

fn replace_extension(file_name: &str, extension: &str) -> String {
    file_name.rsplit_once('.').map_or_else(
        || file_name.to_string(),
        |(base, _)| format!("{base}.{extension}"),
    )
}

const fn role_name(role: Option<SidecarRole>) -> &'static str {
    match role {
        Some(SidecarRole::Forced) => "forced",
        Some(SidecarRole::Commentary) => "commentary",
        Some(SidecarRole::Sdh) => "sdh",
        Some(SidecarRole::SignsSongs) => "signs_songs",
        Some(SidecarRole::Karaoke) => "karaoke",
        None => "none",
    }
}

#[cfg(test)]
mod tests {
    use super::{SidecarDiscoveryError, SidecarFormat, SidecarRole, discover_sidecar_subtitles};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn discovers_supported_patterns_and_ignores_neighbors() {
        let directory = tempdir();
        assert!(directory.is_ok());
        let Ok(directory) = directory else {
            return;
        };
        let source = directory.path().join("Movie.Name.mkv");
        for name in [
            "Movie.Name.en.srt",
            "Movie.Name.eng.forced.ass",
            "Movie.Name.sdh.vtt",
            "Movie.Name.eng.sup",
            "Movie.Name.trailer.en.srt",
            "Other.en.srt",
            "Movie.Name.en.txt",
        ] {
            assert!(fs::write(directory.path().join(name), b"fixture").is_ok());
        }

        let result = discover_sidecar_subtitles(&source);
        assert!(result.is_ok());
        let Ok(sidecars) = result else {
            return;
        };
        assert_eq!(sidecars.len(), 4);
        assert!(sidecars.iter().any(|sidecar| {
            sidecar.language.as_deref() == Some("eng")
                && sidecar.role.is_none()
                && sidecar.format == SidecarFormat::Srt
        }));
        assert!(sidecars.iter().any(|sidecar| {
            sidecar.language.as_deref() == Some("eng")
                && sidecar.role == Some(SidecarRole::Forced)
                && sidecar.format == SidecarFormat::Ass
        }));
        assert!(sidecars.iter().any(|sidecar| {
            sidecar.language.is_none()
                && sidecar.role == Some(SidecarRole::Sdh)
                && sidecar.format == SidecarFormat::Vtt
        }));
        assert!(sidecars.iter().any(|sidecar| sidecar.format.image_based()));
    }

    #[test]
    fn pairs_vobsub_index_and_data_once() {
        let directory = tempdir();
        assert!(directory.is_ok());
        let Ok(directory) = directory else {
            return;
        };
        let source = directory.path().join("episode.mkv");
        assert!(fs::write(directory.path().join("episode.ja.forced.idx"), b"index").is_ok());
        assert!(fs::write(directory.path().join("episode.ja.forced.sub"), b"data").is_ok());

        let result = discover_sidecar_subtitles(&source);
        assert!(result.is_ok());
        let Ok(sidecars) = result else {
            return;
        };
        assert_eq!(sidecars.len(), 1);
        assert_eq!(sidecars[0].format, SidecarFormat::VobSub);
        assert_eq!(sidecars[0].language.as_deref(), Some("jpn"));
        assert_eq!(sidecars[0].role, Some(SidecarRole::Forced));
        assert!(sidecars[0].companion_path.is_some());
    }

    #[test]
    fn rejects_incomplete_vobsub_pair() {
        let directory = tempdir();
        assert!(directory.is_ok());
        let Ok(directory) = directory else {
            return;
        };
        let source = directory.path().join("movie.mkv");
        let index = directory.path().join("movie.eng.idx");
        assert!(fs::write(&index, b"index").is_ok());

        assert!(matches!(
            discover_sidecar_subtitles(&source),
            Err(SidecarDiscoveryError::MissingVobSubCompanion(path)) if path == index
        ));
    }

    #[test]
    fn rejects_normalized_identity_collisions() {
        let directory = tempdir();
        assert!(directory.is_ok());
        let Ok(directory) = directory else {
            return;
        };
        let source = directory.path().join("movie.mkv");
        assert!(fs::write(directory.path().join("movie.en.srt"), b"one").is_ok());
        assert!(fs::write(directory.path().join("movie.eng.srt"), b"two").is_ok());

        assert!(matches!(
            discover_sidecar_subtitles(&source),
            Err(SidecarDiscoveryError::AmbiguousIdentity(_))
        ));
    }
}
