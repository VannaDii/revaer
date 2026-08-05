//! Deterministic, bounded discovery of subtitle files adjacent to source media.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

mod model;

pub use model::{
    SidecarDiscoveryError, SidecarDiscoveryLimits, SidecarFormat, SidecarRole, SidecarSubtitle,
};

/// Injected sidecar discovery boundary.
pub trait SidecarDiscoverer: Send + Sync {
    /// Discover supported subtitle files adjacent to `source_path`.
    ///
    /// # Errors
    ///
    /// Returns a deterministic access, safety, identity, or resource-budget error.
    fn discover(&self, source_path: &Path) -> Result<Vec<SidecarSubtitle>, SidecarDiscoveryError>;
}

/// Filesystem-backed sidecar discoverer with explicit per-media limits.
#[derive(Debug, Clone, Copy)]
pub struct FilesystemSidecarDiscoverer {
    limits: SidecarDiscoveryLimits,
}

impl FilesystemSidecarDiscoverer {
    /// Construct a filesystem discoverer with explicit resource limits.
    #[must_use]
    pub const fn new(limits: SidecarDiscoveryLimits) -> Self {
        Self { limits }
    }
}

impl Default for FilesystemSidecarDiscoverer {
    fn default() -> Self {
        Self::new(SidecarDiscoveryLimits::reviewed())
    }
}

impl SidecarDiscoverer for FilesystemSidecarDiscoverer {
    fn discover(&self, source_path: &Path) -> Result<Vec<SidecarSubtitle>, SidecarDiscoveryError> {
        discover_sidecar_subtitles_with_limits(source_path, self.limits)
    }
}

/// Discover adjacent sidecars under the production-reviewed limits.
///
/// Accepted forms are `{stem}.{lang}.{role}.{ext}`, `{stem}.{lang}.{ext}`, and
/// `{stem}.{role}.{ext}`. Unknown or malformed neighboring files are ignored.
///
/// # Errors
///
/// Returns a deterministic access, safety, identity, or resource-budget error.
pub fn discover_sidecar_subtitles(
    source_path: &Path,
) -> Result<Vec<SidecarSubtitle>, SidecarDiscoveryError> {
    discover_sidecar_subtitles_with_limits(source_path, SidecarDiscoveryLimits::reviewed())
}

/// Discover adjacent sidecars under explicit resource limits.
///
/// Directory entries are streamed. Only names with the normalized source stem and a supported
/// extension are inspected or retained. A matching `.idx` uses a targeted lookup for its required
/// regular `.sub` companion and is returned as one `VobSub` sidecar.
///
/// # Errors
///
/// Returns a deterministic access, safety, identity, or resource-budget error.
pub fn discover_sidecar_subtitles_with_limits(
    source_path: &Path,
    limits: SidecarDiscoveryLimits,
) -> Result<Vec<SidecarSubtitle>, SidecarDiscoveryError> {
    let stem = normalized_source_stem(source_path)?;
    let directory = source_directory(source_path);
    let entries =
        fs::read_dir(directory).map_err(|source| SidecarDiscoveryError::DirectoryRead {
            path: directory.to_path_buf(),
            source,
        })?;
    let prefix = format!("{stem}.");
    let mut state = DiscoveryState::default();
    let mut entry_count = 0_usize;

    for entry in entries {
        entry_count = entry_count.saturating_add(1);
        if entry_count > limits.max_directory_entries {
            return Err(SidecarDiscoveryError::DirectoryEntryLimitExceeded(
                limits.max_directory_entries,
            ));
        }
        let entry = entry.map_err(|source| SidecarDiscoveryError::EntryRead {
            path: directory.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        let file_name = entry
            .file_name()
            .into_string()
            .map_err(|_| SidecarDiscoveryError::NonUtf8Entry(path.clone()))?;
        let normalized_name = file_name.to_ascii_lowercase();
        let Some(parsed) = normalized_name.strip_prefix(&prefix).and_then(parse_suffix) else {
            continue;
        };

        let primary_bytes = regular_file_size(&path)?;
        let sidecar = build_sidecar(
            directory,
            &file_name,
            path,
            primary_bytes,
            parsed,
            limits.max_total_bytes,
        )?;
        if let Some(sidecar) = sidecar {
            state.retain(sidecar, &limits)?;
        }
    }

    state
        .sidecars
        .sort_by(|left, right| left.path.cmp(&right.path));
    Ok(state.sidecars)
}

#[derive(Default)]
struct DiscoveryState {
    total_bytes: u64,
    sidecars: Vec<SidecarSubtitle>,
    identities: BTreeSet<(Option<String>, Option<SidecarRole>, SidecarFormat)>,
}

impl DiscoveryState {
    fn retain(
        &mut self,
        sidecar: SidecarSubtitle,
        limits: &SidecarDiscoveryLimits,
    ) -> Result<(), SidecarDiscoveryError> {
        if self.sidecars.len() >= limits.max_sidecars {
            return Err(SidecarDiscoveryError::SidecarLimitExceeded(
                limits.max_sidecars,
            ));
        }
        let total_bytes = self
            .total_bytes
            .checked_add(sidecar.size_bytes)
            .filter(|total| *total <= limits.max_total_bytes)
            .ok_or(SidecarDiscoveryError::TotalByteLimitExceeded(
                limits.max_total_bytes,
            ))?;
        let identity = (sidecar.language.clone(), sidecar.role, sidecar.format);
        if !self.identities.insert(identity.clone()) {
            return Err(SidecarDiscoveryError::AmbiguousIdentity(format!(
                "language={} role={} format={:?}",
                identity.0.as_deref().unwrap_or("none"),
                role_name(identity.1),
                identity.2
            )));
        }
        self.total_bytes = total_bytes;
        self.sidecars.push(sidecar);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SidecarExtension {
    Ass,
    Idx,
    Srt,
    Sub,
    Sup,
    Vtt,
}

#[derive(Debug)]
struct ParsedSuffix {
    language: Option<String>,
    role: Option<SidecarRole>,
    extension: SidecarExtension,
}

fn normalized_source_stem(source_path: &Path) -> Result<String, SidecarDiscoveryError> {
    source_path
        .file_stem()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase)
        .ok_or_else(|| SidecarDiscoveryError::InvalidSourcePath(source_path.to_path_buf()))
}

fn source_directory(source_path: &Path) -> &Path {
    source_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn parse_suffix(suffix: &str) -> Option<ParsedSuffix> {
    let (semantics, extension) = suffix.rsplit_once('.')?;
    let extension = extension_from_token(extension)?;
    let mut tokens = semantics.split('.');
    let first = tokens.next()?;
    let second = tokens.next();
    if tokens.next().is_some() {
        return None;
    }
    match second {
        Some(role) => Some(ParsedSuffix {
            language: Some(language_from_token(first)?),
            role: Some(role_from_token(role)?),
            extension,
        }),
        None => role_from_token(first).map_or_else(
            || {
                language_from_token(first).map(|language| ParsedSuffix {
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
    }
}

const fn extension_from_token(token: &str) -> Option<SidecarExtension> {
    match token.as_bytes() {
        b"ass" => Some(SidecarExtension::Ass),
        b"idx" => Some(SidecarExtension::Idx),
        b"srt" => Some(SidecarExtension::Srt),
        b"sub" => Some(SidecarExtension::Sub),
        b"sup" => Some(SidecarExtension::Sup),
        b"vtt" => Some(SidecarExtension::Vtt),
        _ => None,
    }
}

fn build_sidecar(
    directory: &Path,
    file_name: &str,
    path: PathBuf,
    primary_bytes: u64,
    parsed: ParsedSuffix,
    max_total_bytes: u64,
) -> Result<Option<SidecarSubtitle>, SidecarDiscoveryError> {
    let (format, companion_path, size_bytes) = match parsed.extension {
        SidecarExtension::Idx => {
            let companion = directory.join(replace_extension(file_name, "sub"));
            let Some(companion_bytes) = optional_regular_file_size(&companion)? else {
                return Err(SidecarDiscoveryError::MissingVobSubCompanion(path));
            };
            let size_bytes = primary_bytes.checked_add(companion_bytes).ok_or(
                SidecarDiscoveryError::TotalByteLimitExceeded(max_total_bytes),
            )?;
            (SidecarFormat::VobSub, Some(companion), size_bytes)
        }
        SidecarExtension::Sub => {
            let index = directory.join(replace_extension(file_name, "idx"));
            if optional_regular_file_size(&index)?.is_some() {
                return Ok(None);
            }
            (SidecarFormat::Sub, None, primary_bytes)
        }
        extension => (format_from_extension(extension), None, primary_bytes),
    };
    Ok(Some(SidecarSubtitle {
        path,
        companion_path,
        language: parsed.language,
        role: parsed.role,
        format,
        size_bytes,
    }))
}

fn regular_file_size(path: &Path) -> Result<u64, SidecarDiscoveryError> {
    let metadata = fs::symlink_metadata(path).map_err(|source| {
        SidecarDiscoveryError::CandidateInspection {
            path: path.to_path_buf(),
            source,
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(SidecarDiscoveryError::UnsafeCandidate(path.to_path_buf()));
    }
    Ok(metadata.len())
}

fn optional_regular_file_size(path: &Path) -> Result<Option<u64>, SidecarDiscoveryError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(SidecarDiscoveryError::UnsafeCandidate(path.to_path_buf()))
        }
        Ok(metadata) => Ok(Some(metadata.len())),
        Err(source) if source.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(SidecarDiscoveryError::CandidateInspection {
            path: path.to_path_buf(),
            source,
        }),
    }
}

const fn format_from_extension(extension: SidecarExtension) -> SidecarFormat {
    match extension {
        SidecarExtension::Ass => SidecarFormat::Ass,
        SidecarExtension::Srt => SidecarFormat::Srt,
        SidecarExtension::Sub => SidecarFormat::Sub,
        SidecarExtension::Sup => SidecarFormat::Sup,
        SidecarExtension::Vtt => SidecarFormat::Vtt,
        SidecarExtension::Idx => SidecarFormat::VobSub,
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
mod tests;
