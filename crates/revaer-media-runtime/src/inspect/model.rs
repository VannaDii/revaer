use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use revaer_media_core::model::MediaGraph;
use thiserror::Error;

use crate::sidecar::{SidecarDiscoveryError, SidecarSubtitle};

const REVIEWED_MAX_INVOCATIONS: usize = 65;
const REVIEWED_MAX_SIDECARS: usize = 64;
const REVIEWED_DEADLINE: Duration = Duration::from_secs(30);
const REVIEWED_MAX_STDOUT_BYTES: usize = 16 * 1024 * 1024;
const REVIEWED_MAX_STDERR_BYTES: usize = 1024 * 1024;
const REVIEWED_MAX_TOTAL_OUTPUT_BYTES: usize = 32 * 1024 * 1024;

/// Per-media execution and output budgets for inspection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InspectionLimits {
    /// Maximum `FFprobe` process invocations, including the source probe.
    pub max_invocations: usize,
    /// Maximum logical sidecars accepted from discovery.
    pub max_sidecars: usize,
    /// End-to-end wall-clock deadline for discovery, probing, and parsing.
    pub deadline: Duration,
    /// Maximum stdout bytes captured from any one invocation.
    pub max_stdout_bytes: usize,
    /// Maximum stderr bytes captured from any one invocation.
    pub max_stderr_bytes: usize,
    /// Maximum aggregate stdout and stderr bytes across all invocations.
    pub max_total_output_bytes: usize,
}

impl InspectionLimits {
    /// Return the production-reviewed inspection limits.
    #[must_use]
    pub const fn reviewed() -> Self {
        Self {
            max_invocations: REVIEWED_MAX_INVOCATIONS,
            max_sidecars: REVIEWED_MAX_SIDECARS,
            deadline: REVIEWED_DEADLINE,
            max_stdout_bytes: REVIEWED_MAX_STDOUT_BYTES,
            max_stderr_bytes: REVIEWED_MAX_STDERR_BYTES,
            max_total_output_bytes: REVIEWED_MAX_TOTAL_OUTPUT_BYTES,
        }
    }
}

impl Default for InspectionLimits {
    fn default() -> Self {
        Self::reviewed()
    }
}

/// Cooperative cancellation boundary for one inspection request.
pub trait InspectCancellation: Send + Sync {
    /// Return whether cancellation has been requested.
    fn is_cancelled(&self) -> bool;
}

/// Cancellation signal that never requests cancellation.
#[derive(Debug, Default, Clone, Copy)]
pub struct NeverCancelled;

impl InspectCancellation for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Thread-safe cancellation token for an active inspection.
#[derive(Debug, Default)]
pub struct InspectCancellationToken {
    cancelled: AtomicBool,
}

impl InspectCancellationToken {
    /// Request cancellation. Repeated calls are idempotent.
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }
}

impl InspectCancellation for InspectCancellationToken {
    fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

/// One bounded native probe request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectProbeRequest {
    /// Executable selected by bootstrap.
    pub program: OsString,
    /// Exact argument vector without the executable.
    pub args: Vec<OsString>,
    /// Remaining request deadline available to this invocation.
    pub timeout: Duration,
    /// Maximum stdout bytes for this invocation.
    pub max_stdout_bytes: usize,
    /// Maximum stderr bytes for this invocation.
    pub max_stderr_bytes: usize,
}

/// Bounded process output returned by an inspection collaborator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectProbeOutput {
    /// Complete stdout, bounded by the request.
    pub stdout: Vec<u8>,
    /// Complete stderr, bounded by the request.
    pub stderr: Vec<u8>,
}

/// Injected process boundary used by the inspection service.
pub trait InspectProbeExecutor: Send + Sync {
    /// Execute one probe while enforcing the supplied process bounds and cancellation signal.
    ///
    /// # Errors
    ///
    /// Returns a deterministic execution, timeout, cancellation, or output-limit error.
    fn run(
        &self,
        request: &InspectProbeRequest,
        cancellation: &dyn InspectCancellation,
    ) -> Result<InspectProbeOutput, InspectError>;
}

/// Error emitted by bounded inspection.
#[derive(Debug, Error)]
pub enum InspectError {
    /// Adjacent sidecar discovery failed.
    #[error(transparent)]
    Sidecar(#[from] SidecarDiscoveryError),
    /// Cancellation was requested before inspection completed.
    #[error("media inspection cancelled")]
    Cancelled,
    /// The end-to-end inspection deadline elapsed.
    #[error("media inspection deadline exceeded after {0:?}")]
    DeadlineExceeded(Duration),
    /// More logical sidecars were returned than the reviewed budget permits.
    #[error("inspection sidecar budget exceeded: maximum {0}")]
    SidecarLimitExceeded(usize),
    /// The required process count exceeds the reviewed budget.
    #[error("inspection invocation budget exceeded: maximum {0}")]
    InvocationLimitExceeded(usize),
    /// One process stream exceeded its reviewed byte limit.
    #[error("inspection process {stream} exceeded {maximum_bytes} bytes")]
    ProcessOutputLimitExceeded {
        /// Process stream that exceeded its limit.
        stream: &'static str,
        /// Configured maximum bytes.
        maximum_bytes: usize,
    },
    /// Aggregate process output exceeded its reviewed byte limit.
    #[error("inspection aggregate output exceeded {0} bytes")]
    TotalOutputLimitExceeded(usize),
    /// Process creation, waiting, termination, or exit failed.
    #[error("inspection probe failed: {0}")]
    ProbeFailed(String),
    /// Probe output was not valid or could not be mapped unambiguously.
    #[error("inspection probe output malformed: {0}")]
    OutputMalformed(String),
    /// `FFprobe` returned an unsupported stream kind.
    #[error("invalid stream kind: {0}")]
    InvalidStreamKind(String),
    /// An input was not a regular non-symlink file at inspection start.
    #[error("inspection input is not a regular non-symlink file: {0}")]
    UnsafeInput(PathBuf),
    /// An input could not be inspected for stability.
    #[error("inspection input metadata failed for {path}: {message}")]
    InputMetadata {
        /// Input path.
        path: PathBuf,
        /// Filesystem failure.
        message: String,
    },
    /// An input identity or size changed while inspection was active.
    #[error("inspection input changed while probing: {0}")]
    InputChanged(PathBuf),
    /// A discovered sidecar has an invalid physical-file shape or byte count.
    #[error("inspection sidecar inventory is inconsistent: {0}")]
    InvalidSidecarInventory(String),
}

/// One normalized metadata key/value pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetadataEntry {
    /// Lowercase metadata key.
    pub key: String,
    /// Trimmed metadata value.
    pub value: String,
}

/// Normalized container-level inspection state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerInspection {
    /// Canonical format names reported by the demuxer.
    pub formats: Vec<String>,
    /// Container duration in milliseconds when known.
    pub duration_millis: Option<u64>,
    /// Container start time in milliseconds when known.
    pub start_time_millis: Option<i64>,
    /// Container size in bytes when reported by the probe.
    pub size_bytes: Option<u64>,
    /// Aggregate container bitrate in bits per second when known.
    pub bit_rate: Option<u64>,
    /// Normalized container metadata.
    pub metadata: Vec<MetadataEntry>,
}

/// Normalized chapter timeline entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChapterInspection {
    /// Probe chapter identifier.
    pub chapter_id: u32,
    /// Inclusive chapter start in milliseconds.
    pub start_millis: i64,
    /// Exclusive chapter end in milliseconds.
    pub end_millis: i64,
    /// Normalized chapter metadata.
    pub metadata: Vec<MetadataEntry>,
}

/// Technical properties retained for one normalized stream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamInspection {
    /// Stream identifier matching the planning graph.
    pub stream_id: u32,
    /// Codec profile when reported.
    pub profile: Option<String>,
    /// Stream duration in milliseconds when known.
    pub duration_millis: Option<u64>,
    /// Stream bitrate in bits per second when known.
    pub bit_rate: Option<u64>,
    /// Peak or maximum stream bitrate in bits per second when reported.
    pub max_bit_rate: Option<u64>,
    /// Audio sample rate in hertz when known.
    pub sample_rate: Option<u32>,
    /// Video width in pixels when applicable.
    pub width: Option<u32>,
    /// Video height in pixels when applicable.
    pub height: Option<u32>,
    /// Pixel format when applicable.
    pub pixel_format: Option<String>,
    /// Sample aspect ratio when applicable.
    pub sample_aspect_ratio: Option<String>,
    /// Display aspect ratio when applicable.
    pub display_aspect_ratio: Option<String>,
    /// Average frame rate as a probe fraction when applicable.
    pub average_frame_rate: Option<String>,
    /// Color range when reported.
    pub color_range: Option<String>,
    /// Color space when reported.
    pub color_space: Option<String>,
    /// Color transfer characteristic when reported.
    pub color_transfer: Option<String>,
    /// Color primaries when reported.
    pub color_primaries: Option<String>,
    /// Chroma location when reported.
    pub chroma_location: Option<String>,
    /// Field order when reported.
    pub field_order: Option<String>,
    /// Normalized stream metadata.
    pub metadata: Vec<MetadataEntry>,
    /// Normalized side-data type names.
    pub side_data_types: Vec<String>,
}

/// Complete normalized media inspection report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaInspection {
    /// Planning graph retained for deterministic planning.
    pub graph: MediaGraph,
    /// Container-level state.
    pub container: ContainerInspection,
    /// Technical stream details ordered by stream identifier.
    pub streams: Vec<StreamInspection>,
    /// Chapters ordered by timeline and identifier.
    pub chapters: Vec<ChapterInspection>,
    /// Adjacent sidecars ordered by path.
    pub sidecars: Vec<SidecarSubtitle>,
}

/// Probe-like stream shape accepted by normalizers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeStream {
    /// Stream id in the source container.
    pub stream_id: u32,
    /// Stream kind.
    pub kind: String,
    /// Codec identifier.
    pub codec: String,
    /// Audio channel count when known.
    pub channels: Option<u32>,
    /// Audio channel layout when known.
    pub channel_layout: Option<String>,
    /// Optional language code.
    pub language: Option<String>,
    /// Optional title.
    pub title: Option<String>,
    /// Raw dispositions.
    pub dispositions: Vec<String>,
}

/// Probe-like graph shape accepted by normalizers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeGraph {
    /// Source path from inspection context.
    pub source_path: String,
    /// Raw stream list.
    pub streams: Vec<ProbeStream>,
}
