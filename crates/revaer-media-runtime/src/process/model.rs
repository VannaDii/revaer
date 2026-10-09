use std::ffi::{OsStr, OsString};
use std::fmt;
use std::time::Duration;

use thiserror::Error;

/// Maximum captured bytes for each native process output stream.
pub const NATIVE_PROCESS_STREAM_LIMIT_BYTES: usize = 16 * 1024 * 1024;
/// Grace period between process-group termination and forced cleanup.
pub const NATIVE_PROCESS_TERMINATION_GRACE: Duration = Duration::from_secs(5);
/// Maximum wall-clock time available to one media inspection process.
pub const INSPECTION_PROCESS_TIMEOUT: Duration = Duration::from_secs(30);

const DIAGNOSTIC_TRUNCATION_MARKER: &str = "...[truncated]";
const MAX_SECONDARY_EVIDENCE_ITEMS: usize =
    NATIVE_PROCESS_STREAM_LIMIT_BYTES / std::mem::size_of::<String>();

#[derive(Clone, Copy)]
struct SecondaryEvidenceBounds {
    maximum_bytes: usize,
    maximum_items: usize,
}

impl SecondaryEvidenceBounds {
    const REVIEWED: Self = Self {
        maximum_bytes: NATIVE_PROCESS_STREAM_LIMIT_BYTES,
        maximum_items: MAX_SECONDARY_EVIDENCE_ITEMS,
    };
}

/// One immutable invocation submitted to a native process supervisor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProcessRequest {
    pub(super) program: OsString,
    pub(super) args: Vec<OsString>,
    pub(super) timeout: Duration,
    pub(super) max_stdout_bytes: usize,
    pub(super) max_stderr_bytes: usize,
}

impl NativeProcessRequest {
    /// Build a request within the reviewed 30-second inspection envelope.
    #[must_use]
    pub fn inspection<I, S>(
        program: impl Into<OsString>,
        args: I,
        remaining: Duration,
        max_stdout_bytes: usize,
        max_stderr_bytes: usize,
    ) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        Self {
            program: program.into(),
            args: args.into_iter().map(Into::into).collect(),
            timeout: remaining.min(INSPECTION_PROCESS_TIMEOUT),
            max_stdout_bytes: max_stdout_bytes.min(NATIVE_PROCESS_STREAM_LIMIT_BYTES),
            max_stderr_bytes: max_stderr_bytes.min(NATIVE_PROCESS_STREAM_LIMIT_BYTES),
        }
    }

    /// Return the reviewed wall-clock timeout for this invocation.
    #[must_use]
    pub const fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Return the maximum captured stdout bytes.
    #[must_use]
    pub const fn max_stdout_bytes(&self) -> usize {
        self.max_stdout_bytes
    }

    /// Return the maximum captured stderr bytes.
    #[must_use]
    pub const fn max_stderr_bytes(&self) -> usize {
        self.max_stderr_bytes
    }

    /// Return the executable selected by runtime wiring.
    #[must_use]
    pub fn program(&self) -> &OsStr {
        &self.program
    }

    /// Return the exact argument vector without shell interpretation.
    #[must_use]
    pub fn arguments(&self) -> &[OsString] {
        &self.args
    }
}

/// Complete bounded output from a successful native process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeProcessOutput {
    pub(super) stdout: Vec<u8>,
    pub(super) stderr: Vec<u8>,
}

impl NativeProcessOutput {
    /// Construct bounded output in a custom injected supervisor.
    #[must_use]
    pub const fn from_streams(stdout: Vec<u8>, stderr: Vec<u8>) -> Self {
        Self { stdout, stderr }
    }

    /// Return complete captured stdout within the reviewed stream limit.
    #[must_use]
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// Return complete captured stderr within the reviewed stream limit.
    #[must_use]
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    /// Consume the result into its bounded stdout and stderr buffers.
    #[must_use]
    pub fn into_streams(self) -> (Vec<u8>, Vec<u8>) {
        (self.stdout, self.stderr)
    }
}

/// Cooperative reason for stopping an active native process tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeProcessStopReason {
    /// Operator or service cancellation was requested.
    Cancelled,
}

impl NativeProcessStopReason {
    /// Return a stable bounded-cardinality reason code for metrics and audit.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
        }
    }
}

/// Live stop signal observed by the shared supervisor.
pub trait NativeProcessControl: Send + Sync {
    /// Return the current stop reason, if process-tree cleanup must begin.
    fn stop_reason(&self) -> Option<NativeProcessStopReason>;
}

/// Live control for bounded bootstrap probes that have no cancellation source.
#[derive(Debug, Default, Clone, Copy)]
pub struct NeverStopNativeProcess;

impl NativeProcessControl for NeverStopNativeProcess {
    fn stop_reason(&self) -> Option<NativeProcessStopReason> {
        None
    }
}

/// Bounded secondary evidence collected while a primary process failure is handled.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeProcessSecondaryEvidence {
    messages: Vec<String>,
    message_bytes: usize,
    truncated: bool,
}

impl NativeProcessSecondaryEvidence {
    pub(super) const fn empty() -> Self {
        Self {
            messages: Vec::new(),
            message_bytes: 0,
            truncated: false,
        }
    }

    /// Build bounded secondary evidence from one diagnostic message.
    #[must_use]
    pub fn from_message(message: String) -> Self {
        let mut evidence = Self::empty();
        evidence.push(message);
        evidence
    }

    /// Return normalized secondary diagnostic messages.
    #[must_use]
    pub fn messages(&self) -> &[String] {
        &self.messages
    }

    /// Return whether no secondary failure was observed.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Return whether evidence was omitted to preserve the reviewed diagnostic bound.
    #[must_use]
    pub const fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub(super) fn push(&mut self, message: String) {
        self.push_with_bounds(message, SecondaryEvidenceBounds::REVIEWED);
    }

    fn push_with_bounds(&mut self, message: String, bounds: SecondaryEvidenceBounds) {
        if message.is_empty() {
            return;
        }
        if self.messages.len() >= bounds.maximum_items {
            self.truncated = true;
            return;
        }
        let remaining = bounds.maximum_bytes.saturating_sub(self.message_bytes);
        if remaining == 0 {
            self.truncated = true;
            return;
        }
        let (message, truncated) = bounded_diagnostic(message, remaining);
        self.truncated |= truncated;
        if message.is_empty() || self.messages.contains(&message) {
            return;
        }
        self.message_bytes = self.message_bytes.saturating_add(message.len());
        self.messages.push(message);
        self.messages.sort();
    }

    pub(super) fn extend(&mut self, evidence: Self) {
        for message in evidence.messages {
            self.push(message);
        }
        self.truncated |= evidence.truncated;
    }
}

impl fmt::Display for NativeProcessSecondaryEvidence {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.messages.is_empty() && !self.truncated {
            return Ok(());
        }
        formatter.write_str("; secondary process evidence: ")?;
        for (index, message) in self.messages.iter().enumerate() {
            if index != 0 {
                formatter.write_str("; ")?;
            }
            formatter.write_str(message)?;
        }
        if self.truncated {
            if !self.messages.is_empty() {
                formatter.write_str("; ")?;
            }
            formatter.write_str(DIAGNOSTIC_TRUNCATION_MARKER)?;
        }
        Ok(())
    }
}

/// Typed primary failure emitted by native process supervision.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum NativeProcessPrimaryError {
    /// The platform cannot provide process-group supervision.
    #[error("bounded process-tree execution is unavailable on this platform")]
    Unsupported,
    /// Process creation failed.
    #[error("failed to spawn process: {0}")]
    Spawn(String),
    /// A live control requested process-tree termination.
    #[error("native process stopped: {}", .0.code())]
    Stopped(NativeProcessStopReason),
    /// The reviewed invocation deadline elapsed.
    #[error("native process deadline exceeded after {0:?}")]
    DeadlineExceeded(Duration),
    /// One captured stream exceeded its reviewed limit.
    #[error("native process {stream} exceeded {maximum_bytes} bytes")]
    OutputLimitExceeded {
        /// Stream whose capture crossed its limit.
        stream: &'static str,
        /// Configured maximum bytes.
        maximum_bytes: usize,
    },
    /// The child exited unsuccessfully.
    #[error("native process exited with status {status}: {stderr}")]
    Exit {
        /// Platform exit status.
        status: String,
        /// Bounded stderr detail.
        stderr: String,
    },
    /// Setup, waiting, reading, or cleanup failed.
    #[error("native process supervision failed: {0}")]
    Supervision(String),
}

impl NativeProcessPrimaryError {
    fn code(&self) -> &'static str {
        match self {
            Self::Unsupported => "unsupported",
            Self::Spawn(_) => "spawn_failed",
            Self::Stopped(reason) => reason.code(),
            Self::DeadlineExceeded(_) => "deadline_exceeded",
            Self::OutputLimitExceeded {
                stream: "stdout", ..
            } => "stdout_limit_exceeded",
            Self::OutputLimitExceeded { .. } => "stderr_limit_exceeded",
            Self::Exit { .. } => "exit_failed",
            Self::Supervision(_) => "supervision_failed",
        }
    }
}

/// One typed primary process failure plus normalized secondary cleanup evidence.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
#[error("{primary}{secondary_evidence}")]
pub struct NativeProcessError {
    #[source]
    primary: NativeProcessPrimaryError,
    secondary_evidence: NativeProcessSecondaryEvidence,
}

impl NativeProcessError {
    /// Construct a process failure without secondary evidence.
    #[must_use]
    pub fn from_primary(primary: NativeProcessPrimaryError) -> Self {
        Self {
            primary,
            secondary_evidence: NativeProcessSecondaryEvidence::default(),
        }
    }

    /// Return the typed primary failure.
    #[must_use]
    pub const fn primary(&self) -> &NativeProcessPrimaryError {
        &self.primary
    }

    /// Return normalized evidence observed while handling the primary failure.
    #[must_use]
    pub const fn secondary_evidence(&self) -> &NativeProcessSecondaryEvidence {
        &self.secondary_evidence
    }

    /// Consume the error into its typed primary failure and secondary evidence.
    #[must_use]
    pub fn into_parts(self) -> (NativeProcessPrimaryError, NativeProcessSecondaryEvidence) {
        (self.primary, self.secondary_evidence)
    }

    /// Return a stable bounded-cardinality reason code for metrics and audit.
    #[must_use]
    pub fn code(&self) -> &'static str {
        self.primary.code()
    }

    #[cfg(not(unix))]
    pub(super) fn unsupported() -> Self {
        Self::from_primary(NativeProcessPrimaryError::Unsupported)
    }

    #[cfg(unix)]
    pub(super) fn spawn(message: String) -> Self {
        Self::from_primary(NativeProcessPrimaryError::Spawn(message))
    }

    #[cfg(unix)]
    pub(super) fn stopped(reason: NativeProcessStopReason) -> Self {
        Self::from_primary(NativeProcessPrimaryError::Stopped(reason))
    }

    #[cfg(unix)]
    pub(super) fn deadline_exceeded(timeout: Duration) -> Self {
        Self::from_primary(NativeProcessPrimaryError::DeadlineExceeded(timeout))
    }

    #[cfg(unix)]
    pub(super) fn output_limit_exceeded(stream: &'static str, maximum_bytes: usize) -> Self {
        Self::from_primary(NativeProcessPrimaryError::OutputLimitExceeded {
            stream,
            maximum_bytes,
        })
    }

    #[cfg(any(unix, test))]
    pub(super) fn supervision(message: String) -> Self {
        Self::from_primary(NativeProcessPrimaryError::Supervision(message))
    }

    /// Attach normalized secondary diagnostics without changing the typed primary failure.
    #[must_use]
    pub fn with_secondary_evidence(mut self, evidence: NativeProcessSecondaryEvidence) -> Self {
        self.secondary_evidence.extend(evidence);
        self
    }

    #[cfg(unix)]
    pub(super) fn exit(status: String, stderr: &[u8]) -> Self {
        Self::from_primary(NativeProcessPrimaryError::Exit {
            status,
            stderr: bounded_failure_detail(stderr),
        })
    }
}

/// Injected supervisor for every shipped native media process.
pub trait NativeProcessSupervisor: Send + Sync {
    /// Run one immutable request with bounded streams and process-tree cleanup.
    /// Implementations must enforce every timeout, stream, cancellation, and cleanup
    /// boundary carried by the request; callers may not compensate for weaker behavior.
    ///
    /// # Errors
    ///
    /// Returns a stable spawn, stop, deadline, output, exit, or supervision failure.
    fn run(
        &self,
        request: &NativeProcessRequest,
        control: &dyn NativeProcessControl,
    ) -> Result<NativeProcessOutput, NativeProcessError>;
}

#[cfg(unix)]
fn bounded_failure_detail(bytes: &[u8]) -> String {
    bounded_diagnostic(
        String::from_utf8_lossy(bytes).trim().to_string(),
        NATIVE_PROCESS_STREAM_LIMIT_BYTES,
    )
    .0
}

fn bounded_diagnostic(mut message: String, maximum_bytes: usize) -> (String, bool) {
    if message.len() <= maximum_bytes {
        return (message, false);
    }
    let marker_bytes = DIAGNOSTIC_TRUNCATION_MARKER.len();
    let prefix_bytes = if marker_bytes <= maximum_bytes {
        maximum_bytes - marker_bytes
    } else {
        maximum_bytes
    };
    let mut boundary = prefix_bytes.min(message.len());
    while boundary > 0 && !message.is_char_boundary(boundary) {
        boundary -= 1;
    }
    message.truncate(boundary);
    if marker_bytes <= maximum_bytes {
        message.push_str(DIAGNOSTIC_TRUNCATION_MARKER);
    }
    (message, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reviewed_secondary_evidence_bounds_derive_from_stream_budget() {
        assert_eq!(
            SecondaryEvidenceBounds::REVIEWED.maximum_bytes,
            NATIVE_PROCESS_STREAM_LIMIT_BYTES
        );
        assert_eq!(
            SecondaryEvidenceBounds::REVIEWED.maximum_items,
            NATIVE_PROCESS_STREAM_LIMIT_BYTES / std::mem::size_of::<String>()
        );
    }

    #[test]
    fn secondary_evidence_bounds_item_count_and_data() {
        let bounds = SecondaryEvidenceBounds {
            maximum_bytes: 8,
            maximum_items: 2,
        };
        let mut evidence = NativeProcessSecondaryEvidence::empty();
        evidence.push_with_bounds("123456789".to_string(), bounds);

        assert_eq!(evidence.messages(), ["12345678"]);
        assert!(evidence.is_truncated());

        let mut evidence = NativeProcessSecondaryEvidence::empty();
        for message in ["first", "second", "third"] {
            evidence.push_with_bounds(message.to_string(), bounds);
        }
        assert_eq!(evidence.messages().len(), 2);
        assert!(evidence.messages().iter().all(|message| message.len() <= 8));
        assert!(evidence.is_truncated());
    }
}
