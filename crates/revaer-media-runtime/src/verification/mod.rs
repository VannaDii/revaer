//! Candidate media safety verification before destructive replacement.

use crate::execute::ExecutionControl;
use crate::inspect::MediaInspection;
use revaer_media_core::model::StreamKind;
use std::io::{self, Read};
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;
use thiserror::Error;

const MAX_VERIFICATION_DETAIL_CHARS: usize = 2_048;
const TRUNCATION_MARKER: &str = "...[truncated]";
const BITRATE_TOLERANCE_BASIS_POINTS: u64 = 100;

/// Packet-derived bitrate evidence for one measured time window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitrateWindowSample {
    /// Output stream identifier.
    pub stream_id: u32,
    /// Window start timestamp in milliseconds.
    pub start_millis: u64,
    /// Positive measured window duration in milliseconds.
    pub duration_millis: u64,
    /// Aggregate encoded packet bytes within the window.
    pub packet_bytes: u64,
}

/// Verify packet-window peak bitrate against a maximum policy.
///
/// The one-percent tolerance applies only above the configured ceiling to absorb timestamp and
/// packet-boundary quantization. Outputs below the ceiling are always accepted.
#[must_use]
pub fn verify_max_bitrate(
    stream_id: u32,
    max_bitrate_bps: u64,
    samples: &[BitrateWindowSample],
) -> VerificationCheck {
    let allowed_bps = max_bitrate_bps.saturating_add(
        max_bitrate_bps
            .saturating_mul(BITRATE_TOLERANCE_BASIS_POINTS)
            .div_ceil(10_000),
    );
    let mut found_sample = false;
    let mut measured_peak = Some(0_u64);
    for sample in samples
        .iter()
        .filter(|sample| sample.stream_id == stream_id)
    {
        found_sample = true;
        measured_peak = measured_peak
            .zip(window_bitrate_bps(sample))
            .map(|(peak, measured)| peak.max(measured));
    }
    match (found_sample, measured_peak) {
        (true, Some(peak_bps)) => VerificationCheck {
            kind: "max_bitrate",
            passed: peak_bps <= allowed_bps,
            expected: format!("peak<={max_bitrate_bps}bps+tolerance"),
            actual: format!("peak={peak_bps}bps"),
            details: Some(format!("allowed={allowed_bps}bps tolerance=1%")),
        },
        (true, None) => VerificationCheck {
            kind: "max_bitrate",
            passed: false,
            expected: format!("peak<={max_bitrate_bps}bps+tolerance"),
            actual: "invalid zero-duration packet window".to_string(),
            details: None,
        },
        (false, _) => VerificationCheck {
            kind: "max_bitrate",
            passed: false,
            expected: format!("peak<={max_bitrate_bps}bps+tolerance"),
            actual: "packet-window evidence unavailable".to_string(),
            details: None,
        },
    }
}

const fn window_bitrate_bps(sample: &BitrateWindowSample) -> Option<u64> {
    if sample.duration_millis == 0 {
        return None;
    }
    Some(
        sample
            .packet_bytes
            .saturating_mul(8)
            .saturating_mul(1_000)
            .div_ceil(sample.duration_millis),
    )
}

/// Policy-selected candidate safety checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerificationPolicy {
    /// Maximum accepted source/candidate duration difference.
    pub duration_tolerance_millis: u64,
    /// Selected candidate checks.
    pub checks: VerificationChecks,
}

/// Compact selection of optional candidate checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerificationChecks(u8);

impl VerificationChecks {
    const MUX: u8 = 1;
    const DECODE: u8 = 1 << 1;
    const KEYFRAME: u8 = 1 << 2;
    const PLAYBACK: u8 = 1 << 3;

    /// Build an empty selection.
    #[must_use]
    pub const fn none() -> Self {
        Self(0)
    }

    /// Select or clear normalized mux validation.
    #[must_use]
    pub const fn with_mux_validation(self, enabled: bool) -> Self {
        self.with(Self::MUX, enabled)
    }

    /// Select or clear full-stream decode validation.
    #[must_use]
    pub const fn with_decode_all_streams(self, enabled: bool) -> Self {
        self.with(Self::DECODE, enabled)
    }

    /// Select or clear midpoint keyframe validation.
    #[must_use]
    pub const fn with_keyframe_seek(self, enabled: bool) -> Self {
        self.with(Self::KEYFRAME, enabled)
    }

    /// Select or clear `FFplay` smoke validation.
    #[must_use]
    pub const fn with_playback_probe(self, enabled: bool) -> Self {
        self.with(Self::PLAYBACK, enabled)
    }

    const fn with(self, check: u8, enabled: bool) -> Self {
        if enabled {
            Self(self.0 | check)
        } else {
            Self(self.0 & !check)
        }
    }

    const fn includes(self, check: u8) -> bool {
        self.0 & check != 0
    }

    const fn mux_validation(self) -> bool {
        self.includes(Self::MUX)
    }

    const fn decode_all_streams(self) -> bool {
        self.includes(Self::DECODE)
    }

    const fn keyframe_seek(self) -> bool {
        self.includes(Self::KEYFRAME)
    }

    const fn playback_probe(self) -> bool {
        self.includes(Self::PLAYBACK)
    }
}

impl Default for VerificationPolicy {
    fn default() -> Self {
        Self {
            duration_tolerance_millis: 100,
            checks: VerificationChecks::none()
                .with_mux_validation(true)
                .with_decode_all_streams(true)
                .with_keyframe_seek(true)
                .with_playback_probe(true),
        }
    }
}

/// One normalized candidate verification result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationCheck {
    /// Stable machine-readable check kind.
    pub kind: &'static str,
    /// Whether the check passed.
    pub passed: bool,
    /// Stable expected value.
    pub expected: String,
    /// Observed value or command outcome.
    pub actual: String,
    /// Bounded diagnostic detail.
    pub details: Option<String>,
}

/// Complete safety verification report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationReport {
    /// Individual normalized checks in execution order.
    pub checks: Vec<VerificationCheck>,
}

impl VerificationReport {
    /// Return true only when every selected check passed.
    #[must_use]
    pub fn passed(&self) -> bool {
        !self.checks.is_empty() && self.checks.iter().all(|check| check.passed)
    }
}

/// Injected process boundary used by decode and seek checks.
pub trait VerificationExecutor: Send + Sync {
    /// Run a verifier command with explicit argv.
    ///
    /// # Errors
    ///
    /// Returns bounded command failure detail when the process cannot run or exits non-zero.
    fn run(&self, bin: &str, argv: &[String]) -> Result<(), String>;

    /// Run a verifier command while observing cooperative cancellation.
    ///
    /// # Errors
    ///
    /// Returns a distinct cancellation error or bounded command failure detail.
    fn run_controlled(
        &self,
        bin: &str,
        argv: &[String],
        control: &dyn ExecutionControl,
    ) -> Result<(), VerificationExecutionError> {
        if control.cancellation_requested() {
            return Err(VerificationExecutionError::Cancelled);
        }
        self.run(bin, argv)
            .map_err(VerificationExecutionError::Failed)?;
        if control.cancellation_requested() {
            return Err(VerificationExecutionError::Cancelled);
        }
        Ok(())
    }
}

/// Controlled verifier process failure.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum VerificationExecutionError {
    /// Operator cancellation stopped active verification.
    #[error("verification cancelled")]
    Cancelled,
    /// Verifier startup or execution failed.
    #[error("verification failed: {0}")]
    Failed(String),
}

#[derive(Debug, Default, Clone, Copy)]
struct NeverCancel;

impl ExecutionControl for NeverCancel {
    fn cancellation_requested(&self) -> bool {
        false
    }
}

/// System process implementation of [`VerificationExecutor`].
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemVerificationExecutor;

impl VerificationExecutor for SystemVerificationExecutor {
    fn run(&self, bin: &str, argv: &[String]) -> Result<(), String> {
        match self.run_controlled(bin, argv, &NeverCancel) {
            Ok(()) => Ok(()),
            Err(VerificationExecutionError::Failed(detail)) => Err(detail),
            Err(VerificationExecutionError::Cancelled) => {
                Err("verification cancelled without a cancellation signal".to_string())
            }
        }
    }

    fn run_controlled(
        &self,
        bin: &str,
        argv: &[String],
        control: &dyn ExecutionControl,
    ) -> Result<(), VerificationExecutionError> {
        if control.cancellation_requested() {
            return Err(VerificationExecutionError::Cancelled);
        }
        let mut command = Command::new(bin);
        command
            .args(argv)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        if Path::new(bin).file_name().and_then(|name| name.to_str()) == Some("ffplay") {
            command
                .env("SDL_AUDIODRIVER", "dummy")
                .env("SDL_VIDEODRIVER", "dummy");
        }
        let mut child = command
            .spawn()
            .map_err(|error| verifier_failure(bin, "could not start", &error.to_string()))?;
        let Some(stderr) = child.stderr.take() else {
            terminate_verifier(&mut child, bin)?;
            return Err(VerificationExecutionError::Failed(bounded_detail(
                &format!("{bin} verifier stderr pipe unavailable"),
            )));
        };
        let stderr_reader = thread::spawn(move || read_stderr(stderr));

        let status = loop {
            if control.cancellation_requested() {
                terminate_verifier(&mut child, bin)?;
                let _stderr = join_stderr(stderr_reader, bin)?;
                return Err(VerificationExecutionError::Cancelled);
            }
            match child.try_wait() {
                Ok(Some(status)) => break status,
                Ok(None) => thread::sleep(Duration::from_millis(100)),
                Err(error) => {
                    terminate_verifier(&mut child, bin)?;
                    let _stderr = join_stderr(stderr_reader, bin)?;
                    return Err(verifier_failure(bin, "wait failed", &error.to_string()));
                }
            }
        };
        let stderr = join_stderr(stderr_reader, bin)?;
        if status.success() {
            return Ok(());
        }
        let detail = String::from_utf8_lossy(&stderr).trim().to_string();
        if detail.is_empty() {
            Err(VerificationExecutionError::Failed(bounded_detail(
                &format!("{bin} exited with status {status}"),
            )))
        } else {
            Err(VerificationExecutionError::Failed(bounded_detail(
                &format!("{bin} exited with status {status}: {detail}"),
            )))
        }
    }
}

fn verifier_failure(bin: &str, operation: &str, detail: &str) -> VerificationExecutionError {
    VerificationExecutionError::Failed(bounded_detail(&format!("{bin} {operation}: {detail}")))
}

fn terminate_verifier(
    child: &mut std::process::Child,
    bin: &str,
) -> Result<(), VerificationExecutionError> {
    match child.kill() {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::InvalidInput => {}
        Err(error) => return Err(verifier_failure(bin, "kill failed", &error.to_string())),
    }
    child
        .wait()
        .map_err(|error| verifier_failure(bin, "reap failed", &error.to_string()))?;
    Ok(())
}

fn read_stderr(mut stderr: std::process::ChildStderr) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    stderr.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn join_stderr(
    reader: thread::JoinHandle<io::Result<Vec<u8>>>,
    bin: &str,
) -> Result<Vec<u8>, VerificationExecutionError> {
    reader
        .join()
        .map_err(|_| verifier_failure(bin, "stderr reader panicked", "thread join failed"))?
        .map_err(|error| verifier_failure(bin, "stderr read failed", &error.to_string()))
}

fn bounded_detail(detail: &str) -> String {
    let marker_chars = TRUNCATION_MARKER.chars().count();
    let mut chars = detail.chars();
    let mut bounded = chars
        .by_ref()
        .take(MAX_VERIFICATION_DETAIL_CHARS - marker_chars)
        .collect::<String>();
    if chars.next().is_some() {
        bounded.push_str(TRUNCATION_MARKER);
    }
    bounded
}

/// Verify a fully inspected candidate against its source and selected safety policy.
#[must_use]
pub fn verify_candidate(
    source: &MediaInspection,
    candidate: &MediaInspection,
    candidate_path: &str,
    policy: VerificationPolicy,
    executor: &dyn VerificationExecutor,
) -> VerificationReport {
    match verify_candidate_controlled(
        source,
        candidate,
        candidate_path,
        policy,
        executor,
        &NeverCancel,
    ) {
        Ok(report) => report,
        Err(VerificationExecutionError::Failed(detail)) => VerificationReport {
            checks: vec![VerificationCheck {
                kind: "verification_executor",
                passed: false,
                expected: "verification process completes".to_string(),
                actual: "failed".to_string(),
                details: Some(detail),
            }],
        },
        Err(VerificationExecutionError::Cancelled) => VerificationReport {
            checks: vec![VerificationCheck {
                kind: "verification_executor",
                passed: false,
                expected: "verification process completes".to_string(),
                actual: "cancelled".to_string(),
                details: None,
            }],
        },
    }
}

/// Verify a candidate while allowing active verifier processes to be cancelled.
///
/// # Errors
///
/// Returns [`VerificationExecutionError::Cancelled`] when cancellation interrupts a selected
/// command check. Ordinary verifier failures remain normalized report checks.
pub fn verify_candidate_controlled(
    source: &MediaInspection,
    candidate: &MediaInspection,
    candidate_path: &str,
    policy: VerificationPolicy,
    executor: &dyn VerificationExecutor,
    control: &dyn ExecutionControl,
) -> Result<VerificationReport, VerificationExecutionError> {
    let mut checks = vec![duration_check(source, candidate, policy)];
    if policy.checks.mux_validation() {
        checks.push(mux_check(candidate));
    }
    if policy.checks.decode_all_streams() {
        checks.push(command_check_controlled(
            "decode_corruption",
            "all streams decode without corruption",
            "ffmpeg",
            &decode_argv(candidate_path),
            executor,
            control,
        )?);
    }
    if policy.checks.keyframe_seek() && has_video(candidate) {
        checks.push(command_check_controlled(
            "keyframe_seek",
            "video frame decodes after midpoint seek",
            "ffmpeg",
            &keyframe_argv(candidate_path, candidate.container.duration_millis),
            executor,
            control,
        )?);
    }
    if policy.checks.playback_probe() {
        checks.push(command_check_controlled(
            "playback_smoke",
            "bounded real-time playback path completes",
            "ffplay",
            &playback_argv(candidate_path),
            executor,
            control,
        )?);
    }
    Ok(VerificationReport { checks })
}

fn mux_check(candidate: &MediaInspection) -> VerificationCheck {
    let graph_ids = candidate
        .graph
        .streams
        .iter()
        .map(|stream| stream.stream_id)
        .collect::<Vec<_>>();
    let technical_ids = candidate
        .streams
        .iter()
        .map(|stream| stream.stream_id)
        .collect::<Vec<_>>();
    let passed = !candidate.container.formats.is_empty()
        && !candidate.graph.streams.is_empty()
        && graph_ids == technical_ids;
    VerificationCheck {
        kind: "mux_structure",
        passed,
        expected: "container and every stream fully inspected".to_string(),
        actual: format!(
            "formats={} graph_streams={} inspected_streams={}",
            candidate.container.formats.len(),
            graph_ids.len(),
            technical_ids.len()
        ),
        details: None,
    }
}

fn duration_check(
    source: &MediaInspection,
    candidate: &MediaInspection,
    policy: VerificationPolicy,
) -> VerificationCheck {
    let expected = format!("delta<={}ms", policy.duration_tolerance_millis);
    match (
        source.container.duration_millis,
        candidate.container.duration_millis,
    ) {
        (Some(source_duration), Some(candidate_duration)) => {
            let delta = source_duration.abs_diff(candidate_duration);
            VerificationCheck {
                kind: "duration_delta",
                passed: delta <= policy.duration_tolerance_millis,
                expected,
                actual: format!("delta={delta}ms"),
                details: Some(format!(
                    "source={source_duration}ms candidate={candidate_duration}ms"
                )),
            }
        }
        _ => VerificationCheck {
            kind: "duration_delta",
            passed: false,
            expected,
            actual: "duration unavailable".to_string(),
            details: None,
        },
    }
}

fn command_check_controlled(
    kind: &'static str,
    expected: &'static str,
    bin: &str,
    argv: &[String],
    executor: &dyn VerificationExecutor,
    control: &dyn ExecutionControl,
) -> Result<VerificationCheck, VerificationExecutionError> {
    match executor.run_controlled(bin, argv, control) {
        Ok(()) => Ok(VerificationCheck {
            kind,
            passed: true,
            expected: expected.to_string(),
            actual: "passed".to_string(),
            details: None,
        }),
        Err(VerificationExecutionError::Failed(detail)) => Ok(VerificationCheck {
            kind,
            passed: false,
            expected: expected.to_string(),
            actual: "failed".to_string(),
            details: Some(detail),
        }),
        Err(VerificationExecutionError::Cancelled) => Err(VerificationExecutionError::Cancelled),
    }
}

fn decode_argv(candidate_path: &str) -> Vec<String> {
    [
        "-nostdin",
        "-v",
        "error",
        "-xerror",
        "-i",
        candidate_path,
        "-map",
        "0",
        "-f",
        "null",
        "-",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn keyframe_argv(candidate_path: &str, duration_millis: Option<u64>) -> Vec<String> {
    let seek_millis = duration_millis.map_or(1000, |duration| duration / 2);
    let seek_seconds = format!("{}.{:03}", seek_millis / 1000, seek_millis % 1000);
    vec![
        "-nostdin".to_string(),
        "-v".to_string(),
        "error".to_string(),
        "-xerror".to_string(),
        "-ss".to_string(),
        seek_seconds,
        "-i".to_string(),
        candidate_path.to_string(),
        "-map".to_string(),
        "0:v:0".to_string(),
        "-frames:v".to_string(),
        "1".to_string(),
        "-f".to_string(),
        "null".to_string(),
        "-".to_string(),
    ]
}

fn playback_argv(candidate_path: &str) -> Vec<String> {
    ["-v", "error", "-autoexit", "-t", "1", candidate_path]
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn has_video(candidate: &MediaInspection) -> bool {
    candidate
        .graph
        .streams
        .iter()
        .any(|stream| stream.kind == StreamKind::Video)
}

#[cfg(test)]
mod tests {
    use super::{
        BitrateWindowSample, MAX_VERIFICATION_DETAIL_CHARS, SystemVerificationExecutor,
        TRUNCATION_MARKER, VerificationChecks, VerificationExecutionError, VerificationExecutor,
        VerificationPolicy, bounded_detail, verify_candidate, verify_max_bitrate,
    };
    use crate::execute::ExecutionControl;
    use crate::inspect::{ContainerInspection, MediaInspection, MetadataEntry, StreamInspection};
    use revaer_media_core::model::{MediaGraph, MediaStream, StreamKind};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::{Duration, Instant};

    struct AtomicVerificationControl {
        requested: Arc<AtomicBool>,
    }

    impl ExecutionControl for AtomicVerificationControl {
        fn cancellation_requested(&self) -> bool {
            self.requested.load(Ordering::Acquire)
        }
    }

    struct StubExecutor {
        fail_decode: bool,
        calls: Mutex<Vec<Vec<String>>>,
    }

    impl VerificationExecutor for StubExecutor {
        fn run(&self, bin: &str, argv: &[String]) -> Result<(), String> {
            let mut call = vec![bin.to_string()];
            call.extend(argv.iter().cloned());
            self.calls
                .lock()
                .map_err(|error| error.to_string())?
                .push(call);
            if self.fail_decode
                && argv
                    .iter()
                    .any(|value| value == "-map" && argv.contains(&"0".to_string()))
            {
                Err("truncated packet".to_string())
            } else {
                Ok(())
            }
        }
    }

    fn inspection(path: &str, duration_millis: u64) -> MediaInspection {
        MediaInspection {
            graph: MediaGraph {
                source_path: path.to_string(),
                container_formats: Vec::new(),
                streams: vec![MediaStream {
                    stream_id: 0,
                    kind: StreamKind::Video,
                    codec: "h264".to_string(),
                    channels: None,
                    channel_layout: None,
                    language: None,
                    title: None,
                    dispositions: Vec::new(),
                }],
            },
            container: ContainerInspection {
                formats: vec!["matroska".to_string()],
                duration_millis: Some(duration_millis),
                start_time_millis: Some(0),
                size_bytes: Some(1024),
                bit_rate: Some(8192),
                metadata: vec![MetadataEntry {
                    key: "title".to_string(),
                    value: "Fixture".to_string(),
                }],
            },
            streams: vec![StreamInspection {
                stream_id: 0,
                profile: Some("high".to_string()),
                duration_millis: Some(duration_millis),
                bit_rate: Some(8192),
                max_bit_rate: Some(8192),
                sample_rate: None,
                width: Some(1920),
                height: Some(1080),
                pixel_format: Some("yuv420p".to_string()),
                sample_aspect_ratio: Some("1:1".to_string()),
                display_aspect_ratio: Some("16:9".to_string()),
                average_frame_rate: Some("24/1".to_string()),
                color_range: Some("tv".to_string()),
                color_space: Some("bt709".to_string()),
                color_transfer: Some("bt709".to_string()),
                color_primaries: Some("bt709".to_string()),
                chroma_location: Some("left".to_string()),
                field_order: Some("progressive".to_string()),
                metadata: Vec::new(),
                side_data_types: Vec::new(),
            }],
            chapters: Vec::new(),
            sidecars: Vec::new(),
        }
    }

    #[test]
    fn strict_candidate_verification_passes_complete_media() {
        let executor = StubExecutor {
            fail_decode: false,
            calls: Mutex::new(Vec::new()),
        };
        let report = verify_candidate(
            &inspection("source.mkv", 10_000),
            &inspection("candidate.mkv", 9_950),
            "candidate.mkv",
            VerificationPolicy::default(),
            &executor,
        );
        assert!(report.passed());
        assert_eq!(report.checks.len(), 5);
    }

    #[test]
    fn truncated_probeable_candidate_fails_duration_and_decode() {
        let executor = StubExecutor {
            fail_decode: true,
            calls: Mutex::new(Vec::new()),
        };
        let report = verify_candidate(
            &inspection("source.mkv", 10_000),
            &inspection("candidate.mkv", 4_000),
            "candidate.mkv",
            VerificationPolicy::default(),
            &executor,
        );
        assert!(!report.passed());
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.kind == "duration_delta" && !check.passed)
        );
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.kind == "decode_corruption" && !check.passed)
        );
    }

    #[test]
    fn policy_selects_only_requested_command_checks() {
        let executor = StubExecutor {
            fail_decode: false,
            calls: Mutex::new(Vec::new()),
        };
        let report = verify_candidate(
            &inspection("source.mkv", 10_000),
            &inspection("candidate.mkv", 10_000),
            "candidate.mkv",
            VerificationPolicy {
                duration_tolerance_millis: 0,
                checks: VerificationChecks::none().with_playback_probe(true),
            },
            &executor,
        );
        assert!(report.passed());
        assert_eq!(report.checks.len(), 2);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.kind == "playback_smoke")
        );
        let calls = executor.calls.lock();
        assert!(calls.is_ok());
        if let Ok(calls) = calls {
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].first().map(String::as_str), Some("ffplay"));
        }
    }

    #[test]
    fn verifier_diagnostics_are_bounded_on_character_boundaries() {
        let detail = "x".repeat(MAX_VERIFICATION_DETAIL_CHARS * 2);
        let bounded = bounded_detail(&detail);
        assert_eq!(bounded.chars().count(), MAX_VERIFICATION_DETAIL_CHARS);
        assert!(bounded.ends_with(TRUNCATION_MARKER));
    }

    #[test]
    fn system_verifier_terminates_active_child_on_cancellation() {
        let requested = Arc::new(AtomicBool::new(false));
        let request_from_thread = Arc::clone(&requested);
        let request_thread = thread::spawn(move || {
            thread::sleep(Duration::from_millis(150));
            request_from_thread.store(true, Ordering::Release);
        });
        let control = AtomicVerificationControl { requested };
        let started = Instant::now();

        let result =
            SystemVerificationExecutor.run_controlled("/bin/sleep", &["30".to_string()], &control);
        let join_result = request_thread.join();

        assert!(join_result.is_ok());
        assert_eq!(result, Err(VerificationExecutionError::Cancelled));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    fn max_bitrate_accepts_below_and_equal_ceiling_without_lower_bound() {
        let below = [BitrateWindowSample {
            stream_id: 0,
            start_millis: 0,
            duration_millis: 1_000,
            packet_bytes: 900_000,
        }];
        let equal = [BitrateWindowSample {
            packet_bytes: 1_000_000,
            ..below[0]
        }];

        assert!(verify_max_bitrate(0, 8_000_000, &below).passed);
        assert!(verify_max_bitrate(0, 8_000_000, &equal).passed);
    }

    #[test]
    fn max_bitrate_rejects_above_ceiling_and_short_peak_burst() {
        let above = [BitrateWindowSample {
            stream_id: 0,
            start_millis: 0,
            duration_millis: 1_000,
            packet_bytes: 1_020_000,
        }];
        let short_burst = [BitrateWindowSample {
            stream_id: 0,
            start_millis: 1_500,
            duration_millis: 100,
            packet_bytes: 110_000,
        }];

        assert!(!verify_max_bitrate(0, 8_000_000, &above).passed);
        assert!(!verify_max_bitrate(0, 8_000_000, &short_burst).passed);
    }
}
