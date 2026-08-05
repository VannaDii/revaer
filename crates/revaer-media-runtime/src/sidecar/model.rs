use std::io;
use std::path::PathBuf;
use thiserror::Error;

const REVIEWED_MAX_DIRECTORY_ENTRIES: usize = 4_096;
const REVIEWED_MAX_SIDECARS: usize = 64;
const REVIEWED_MAX_TOTAL_BYTES: u64 = 256 * 1024 * 1024;

/// Per-media resource budgets for adjacent subtitle discovery.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SidecarDiscoveryLimits {
    /// Maximum directory entries examined during one discovery pass.
    pub max_directory_entries: usize,
    /// Maximum logical sidecars retained for one source.
    pub max_sidecars: usize,
    /// Maximum aggregate bytes across retained physical sidecar files.
    pub max_total_bytes: u64,
}

impl SidecarDiscoveryLimits {
    /// Return the production-reviewed discovery limits.
    #[must_use]
    pub const fn reviewed() -> Self {
        Self {
            max_directory_entries: REVIEWED_MAX_DIRECTORY_ENTRIES,
            max_sidecars: REVIEWED_MAX_SIDECARS,
            max_total_bytes: REVIEWED_MAX_TOTAL_BYTES,
        }
    }
}

impl Default for SidecarDiscoveryLimits {
    fn default() -> Self {
        Self::reviewed()
    }
}

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
    /// Return whether the format is image-based and requires OCR for text conversion.
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
    /// Required `.sub` companion path for `VobSub` pairs.
    pub companion_path: Option<PathBuf>,
    /// Normalized two- or three-letter language token when present.
    pub language: Option<String>,
    /// Normalized semantic role when present.
    pub role: Option<SidecarRole>,
    /// Discovered subtitle format.
    pub format: SidecarFormat,
    /// Aggregate bytes across the primary file and optional companion.
    pub size_bytes: u64,
}

/// Sidecar discovery failure.
#[derive(Debug, Error)]
pub enum SidecarDiscoveryError {
    /// Source path does not identify a usable UTF-8 file stem.
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
    /// One directory entry could not be read.
    #[error("sidecar directory entry read failed for {path}: {source}")]
    EntryRead {
        /// Directory containing the source media.
        path: PathBuf,
        /// Filesystem failure.
        source: io::Error,
    },
    /// An entry name could not be safely normalized.
    #[error("sidecar directory contains a non-UTF-8 entry: {0}")]
    NonUtf8Entry(PathBuf),
    /// A matching candidate is a symlink or is not a regular file.
    #[error("sidecar candidate is not a regular non-symlink file: {0}")]
    UnsafeCandidate(PathBuf),
    /// A matching candidate could not be inspected.
    #[error("sidecar candidate inspection failed for {path}: {source}")]
    CandidateInspection {
        /// Candidate path.
        path: PathBuf,
        /// Filesystem failure.
        source: io::Error,
    },
    /// Two files resolve to the same normalized language, role, and format identity.
    #[error("ambiguous sidecar subtitle identity: {0}")]
    AmbiguousIdentity(String),
    /// A `VobSub` index lacks its required regular `.sub` data companion.
    #[error("VobSub index is missing its .sub companion: {0}")]
    MissingVobSubCompanion(PathBuf),
    /// The reviewed directory-entry budget was exceeded.
    #[error("sidecar directory entry budget exceeded: maximum {0}")]
    DirectoryEntryLimitExceeded(usize),
    /// The reviewed logical-sidecar budget was exceeded.
    #[error("sidecar count budget exceeded: maximum {0}")]
    SidecarLimitExceeded(usize),
    /// The reviewed aggregate-byte budget was exceeded.
    #[error("sidecar aggregate byte budget exceeded: maximum {0}")]
    TotalByteLimitExceeded(u64),
}
