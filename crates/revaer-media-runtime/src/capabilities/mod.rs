//! Tool capability models and detector adapters.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::io::{self, Read};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use thiserror::Error;

const CAPABILITY_PROBE_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_CAPABILITY_PROBE_STDOUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_CAPABILITY_PROBE_STDERR_BYTES: usize = 64 * 1024;
const CAPABILITY_PROBE_TRUNCATION_MARKER: &str = "...[truncated]";

/// Runtime snapshot of media tool capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    /// ffmpeg semantic version.
    pub ffmpeg_version: String,
    /// ffprobe semantic version.
    pub ffprobe_version: String,
    /// Available codec names.
    pub codecs: Vec<String>,
    /// Per-codec encode/decode support parsed from ffmpeg codec flags.
    pub codec_support: Vec<CodecCapability>,
    /// Available ffmpeg encoder names.
    pub encoders: Vec<String>,
    /// Available ffmpeg decoder names.
    pub decoders: Vec<String>,
    /// Available ffmpeg muxer names.
    pub muxers: Vec<String>,
    /// Available ffmpeg demuxer names.
    pub demuxers: Vec<String>,
    /// Hardware acceleration backends reported by ffmpeg.
    pub hardware_accelerators: Vec<String>,
    /// Subtitle codecs supported by this toolchain.
    pub subtitle_support: Vec<String>,
    /// Filesystem primitives the runtime requires for managed replacement.
    pub filesystem_utilities: Vec<String>,
    /// Runtime utilities available to media jobs.
    pub utility_capabilities: Vec<String>,
    /// License mode inferred from the runtime ffmpeg build.
    pub license_mode: String,
    /// License mode inferred specifically from the ffmpeg build flags.
    pub ffmpeg_license_mode: String,
    /// Whether ffmpeg was built with `--enable-gpl`.
    pub ffmpeg_enable_gpl: bool,
    /// Whether ffmpeg was built with `--enable-version3`.
    pub ffmpeg_enable_version3: bool,
    /// Whether ffmpeg was built with `--enable-nonfree`.
    pub ffmpeg_enable_nonfree: bool,
    /// Compliance artifact links bundled with the runtime image.
    pub compliance_links: Vec<String>,
    /// Capabilities intentionally absent from this runtime.
    pub absent_capabilities: Vec<String>,
}

/// Encode/decode support for one ffmpeg codec row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodecCapability {
    /// Codec name.
    pub name: String,
    /// Whether ffmpeg reports encoding support for this codec.
    pub encode_supported: bool,
    /// Whether ffmpeg reports decoding support for this codec.
    pub decode_supported: bool,
}

impl CapabilitySnapshot {
    /// Returns true when required binaries and supported codecs are present.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.ffmpeg_version.trim().is_empty()
            && !self.ffprobe_version.trim().is_empty()
            && !self.codecs.is_empty()
            && codec_support_covers_codecs(&self.codecs, &self.codec_support)
            && !self.encoders.is_empty()
            && !self.decoders.is_empty()
            && !self.muxers.is_empty()
            && !self.demuxers.is_empty()
            && !self.filesystem_utilities.is_empty()
            && ["ffmpeg", "ffprobe", "ffplay"].iter().all(|required| {
                self.utility_capabilities
                    .iter()
                    .any(|actual| actual == required)
            })
            && !self.license_mode.trim().is_empty()
    }

    /// Return support flags for a codec name, defaulting to unsupported flags.
    #[must_use]
    pub fn codec_capability(&self, name: &str) -> CodecCapability {
        self.codec_support
            .iter()
            .find(|item| item.name.eq_ignore_ascii_case(name))
            .cloned()
            .unwrap_or_else(|| CodecCapability {
                name: name.to_string(),
                encode_supported: false,
                decode_supported: false,
            })
    }
}

fn codec_support_covers_codecs(codecs: &[String], support: &[CodecCapability]) -> bool {
    !support.is_empty()
        && codecs.iter().all(|codec| {
            let normalized = codec.trim();
            !normalized.is_empty()
                && support.iter().any(|item| {
                    item.name.eq_ignore_ascii_case(normalized)
                        && (item.encode_supported || item.decode_supported)
                })
        })
}

impl Default for CapabilitySnapshot {
    fn default() -> Self {
        Self {
            ffmpeg_version: String::new(),
            ffprobe_version: String::new(),
            codecs: Vec::new(),
            codec_support: Vec::new(),
            encoders: Vec::new(),
            decoders: vec!["h264".to_string()],
            muxers: vec!["matroska".to_string()],
            demuxers: vec!["matroska".to_string()],
            hardware_accelerators: Vec::new(),
            subtitle_support: Vec::new(),
            filesystem_utilities: filesystem_utilities(),
            utility_capabilities: utility_capabilities(),
            license_mode: "gpl".to_string(),
            ffmpeg_license_mode: "gpl".to_string(),
            ffmpeg_enable_gpl: true,
            ffmpeg_enable_version3: false,
            ffmpeg_enable_nonfree: false,
            compliance_links: compliance_links(),
            absent_capabilities: vec!["--enable-nonfree".to_string()],
        }
    }
}

/// Capability detection error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityDetectError {
    /// Runtime detector is unavailable in this build/runtime context.
    #[error("capability detector unavailable")]
    Unavailable,
    /// Command invocation failed.
    #[error("capability probe command failed: {0}")]
    CommandFailed(String),
    /// Probe output could not be parsed.
    #[error("capability probe output malformed: {0}")]
    OutputMalformed(String),
}

/// Capability detector interface.
pub trait CapabilityDetector: Send + Sync {
    /// Detect runtime media capabilities.
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityDetectError`] when detection cannot complete.
    fn detect(&self) -> Result<CapabilitySnapshot, CapabilityDetectError>;
}

/// Default detector used when no concrete runtime probing adapter is configured.
#[derive(Debug, Default)]
pub struct UnavailableCapabilityDetector;

impl CapabilityDetector for UnavailableCapabilityDetector {
    fn detect(&self) -> Result<CapabilitySnapshot, CapabilityDetectError> {
        Err(CapabilityDetectError::Unavailable)
    }
}

/// Command probe abstraction for capability discovery.
pub trait CapabilityProbeExecutor: Send + Sync {
    /// Run one probe command and return stdout as UTF-8 text.
    ///
    /// # Errors
    ///
    /// Returns [`CapabilityDetectError::CommandFailed`] when command invocation fails.
    /// Returns [`CapabilityDetectError::OutputMalformed`] when stdout is not valid UTF-8.
    fn run(&self, program: &str, args: &[&str]) -> Result<String, CapabilityDetectError>;
}

/// System-process probe executor.
#[derive(Debug, Default)]
pub struct SystemCapabilityProbeExecutor;

impl CapabilityProbeExecutor for SystemCapabilityProbeExecutor {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, CapabilityDetectError> {
        let output = run_system_probe(program, args, CAPABILITY_PROBE_TIMEOUT)?;
        if !output.status_success {
            return Err(CapabilityDetectError::CommandFailed(format!(
                "{program} {:?} exited with status {}{}",
                args,
                output.status,
                stderr_suffix(&output.stderr)
            )));
        }
        if output.stdout.truncated {
            return Err(CapabilityDetectError::OutputMalformed(format!(
                "{program} {args:?} stdout exceeded {MAX_CAPABILITY_PROBE_STDOUT_BYTES} bytes",
            )));
        }
        String::from_utf8(output.stdout.bytes)
            .map_err(|err| CapabilityDetectError::OutputMalformed(err.to_string()))
    }
}

#[derive(Debug)]
struct SystemProbeOutput {
    status_success: bool,
    status: std::process::ExitStatus,
    stdout: BoundedProbeOutput,
    stderr: BoundedProbeOutput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BoundedProbeOutput {
    bytes: Vec<u8>,
    truncated: bool,
}

fn run_system_probe(
    program: &str,
    args: &[&str],
    timeout: Duration,
) -> Result<SystemProbeOutput, CapabilityDetectError> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| CapabilityDetectError::CommandFailed(err.to_string()))?;
    let Some(stdout) = child.stdout.take() else {
        terminate_probe(&mut child, program)?;
        return Err(CapabilityDetectError::CommandFailed(format!(
            "{program} probe stdout pipe unavailable"
        )));
    };
    let Some(stderr) = child.stderr.take() else {
        terminate_probe(&mut child, program)?;
        return Err(CapabilityDetectError::CommandFailed(format!(
            "{program} probe stderr pipe unavailable"
        )));
    };
    let stdout_reader =
        thread::spawn(move || read_bounded_stdout(stdout, MAX_CAPABILITY_PROBE_STDOUT_BYTES));
    let stderr_reader =
        thread::spawn(move || read_bounded_tail(stderr, MAX_CAPABILITY_PROBE_STDERR_BYTES));
    let started = Instant::now();

    loop {
        if started.elapsed() >= timeout {
            terminate_probe(&mut child, program)?;
            let output = join_probe_readers(program, stdout_reader, stderr_reader)?;
            return Err(CapabilityDetectError::CommandFailed(format!(
                "{program} {:?} timed out after {}ms{}",
                args,
                timeout.as_millis(),
                stderr_suffix(&output.stderr)
            )));
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let output = join_probe_readers(program, stdout_reader, stderr_reader)?;
                return Ok(SystemProbeOutput {
                    status_success: status.success(),
                    status,
                    stdout: output.stdout,
                    stderr: output.stderr,
                });
            }
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(err) => {
                terminate_probe(&mut child, program)?;
                let _output = join_probe_readers(program, stdout_reader, stderr_reader)?;
                return Err(CapabilityDetectError::CommandFailed(format!(
                    "{program} wait failed: {err}"
                )));
            }
        }
    }
}

fn terminate_probe(child: &mut Child, program: &str) -> Result<(), CapabilityDetectError> {
    match child.kill() {
        Ok(()) => {}
        Err(err) if err.kind() == io::ErrorKind::InvalidInput => {}
        Err(err) => {
            return Err(CapabilityDetectError::CommandFailed(format!(
                "{program} kill failed: {err}"
            )));
        }
    }
    child.wait().map_err(|err| {
        CapabilityDetectError::CommandFailed(format!("{program} reap failed: {err}"))
    })?;
    Ok(())
}

#[derive(Debug)]
struct JoinedProbeOutput {
    stdout: BoundedProbeOutput,
    stderr: BoundedProbeOutput,
}

fn join_probe_readers(
    program: &str,
    stdout_reader: thread::JoinHandle<io::Result<BoundedProbeOutput>>,
    stderr_reader: thread::JoinHandle<io::Result<BoundedProbeOutput>>,
) -> Result<JoinedProbeOutput, CapabilityDetectError> {
    let stdout = stdout_reader
        .join()
        .map_err(|_| {
            CapabilityDetectError::CommandFailed(format!("{program} stdout reader panicked"))
        })?
        .map_err(|err| {
            CapabilityDetectError::CommandFailed(format!("{program} stdout read failed: {err}"))
        })?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| {
            CapabilityDetectError::CommandFailed(format!("{program} stderr reader panicked"))
        })?
        .map_err(|err| {
            CapabilityDetectError::CommandFailed(format!("{program} stderr read failed: {err}"))
        })?;
    Ok(JoinedProbeOutput { stdout, stderr })
}

fn read_bounded_stdout(mut reader: impl Read, max_bytes: usize) -> io::Result<BoundedProbeOutput> {
    let mut bytes = Vec::with_capacity(max_bytes);
    let mut scratch = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut scratch)?;
        if read == 0 {
            break;
        }
        let remaining = max_bytes.saturating_sub(bytes.len());
        if remaining == 0 {
            truncated = true;
            continue;
        }
        let retained = read.min(remaining);
        bytes.extend_from_slice(&scratch[..retained]);
        if retained < read {
            truncated = true;
        }
    }
    Ok(BoundedProbeOutput { bytes, truncated })
}

fn read_bounded_tail(mut reader: impl Read, max_bytes: usize) -> io::Result<BoundedProbeOutput> {
    let mut bytes = Vec::with_capacity(max_bytes);
    let mut scratch = [0_u8; 8192];
    let mut truncated = false;
    loop {
        let read = reader.read(&mut scratch)?;
        if read == 0 {
            break;
        }
        retain_probe_tail(&mut bytes, &scratch[..read], max_bytes, &mut truncated);
    }
    Ok(BoundedProbeOutput { bytes, truncated })
}

fn retain_probe_tail(retained: &mut Vec<u8>, chunk: &[u8], max_bytes: usize, truncated: &mut bool) {
    if chunk.len() >= max_bytes {
        retained.clear();
        retained.extend_from_slice(&chunk[chunk.len() - max_bytes..]);
        *truncated = true;
        return;
    }
    let overflow = retained
        .len()
        .saturating_add(chunk.len())
        .saturating_sub(max_bytes);
    if overflow > 0 {
        retained.drain(..overflow);
        *truncated = true;
    }
    retained.extend_from_slice(chunk);
}

fn stderr_suffix(stderr: &BoundedProbeOutput) -> String {
    let detail = String::from_utf8_lossy(&stderr.bytes).trim().to_string();
    if detail.is_empty() {
        String::new()
    } else if stderr.truncated {
        format!(": {CAPABILITY_PROBE_TRUNCATION_MARKER}{detail}")
    } else {
        format!(": {detail}")
    }
}

/// ffmpeg/ffprobe-backed capability detector.
pub struct FfmpegCapabilityDetector {
    executor: Arc<dyn CapabilityProbeExecutor>,
    ffmpeg_bin: String,
    ffprobe_bin: String,
    ffplay_bin: String,
}

impl FfmpegCapabilityDetector {
    /// Construct detector with injected probe executor and binary names.
    #[must_use]
    pub fn new(
        executor: Arc<dyn CapabilityProbeExecutor>,
        ffmpeg_bin: impl Into<String>,
        ffprobe_bin: impl Into<String>,
        ffplay_bin: impl Into<String>,
    ) -> Self {
        Self {
            executor,
            ffmpeg_bin: ffmpeg_bin.into(),
            ffprobe_bin: ffprobe_bin.into(),
            ffplay_bin: ffplay_bin.into(),
        }
    }
}

impl CapabilityDetector for FfmpegCapabilityDetector {
    fn detect(&self) -> Result<CapabilitySnapshot, CapabilityDetectError> {
        let ffmpeg_version_output = self.executor.run(&self.ffmpeg_bin, &["-version"])?;
        let ffprobe_version_output = self.executor.run(&self.ffprobe_bin, &["-version"])?;
        let ffplay_version_output = self.executor.run(&self.ffplay_bin, &["-version"])?;
        let codecs_output = self.executor.run(&self.ffmpeg_bin, &["-codecs"])?;
        let encoders_output = self.executor.run(&self.ffmpeg_bin, &["-encoders"])?;
        let decoders_output = self.executor.run(&self.ffmpeg_bin, &["-decoders"])?;
        let muxers_output = self.executor.run(&self.ffmpeg_bin, &["-muxers"])?;
        let demuxers_output = self.executor.run(&self.ffmpeg_bin, &["-demuxers"])?;
        let hwaccels_output = self.executor.run(&self.ffmpeg_bin, &["-hwaccels"])?;

        let ffmpeg_version = parse_version_line(&ffmpeg_version_output).ok_or_else(|| {
            CapabilityDetectError::OutputMalformed("missing ffmpeg version line".to_string())
        })?;
        let ffprobe_version = parse_version_line(&ffprobe_version_output).ok_or_else(|| {
            CapabilityDetectError::OutputMalformed("missing ffprobe version line".to_string())
        })?;
        let _ffplay_version = parse_version_line(&ffplay_version_output).ok_or_else(|| {
            CapabilityDetectError::OutputMalformed("missing ffplay version line".to_string())
        })?;
        let codec_support = parse_codecs(&codecs_output);
        if codec_support.is_empty() {
            return Err(CapabilityDetectError::OutputMalformed(
                "no codecs parsed from ffmpeg -codecs".to_string(),
            ));
        }
        let codecs = codec_support
            .iter()
            .filter(|item| item.encode_supported || item.decode_supported)
            .map(|item| item.name.clone())
            .collect::<Vec<_>>();
        let encoders = parse_tool_names(&encoders_output);
        let decoders = parse_tool_names(&decoders_output);
        let muxers = parse_tool_names(&muxers_output);
        let demuxers = parse_tool_names(&demuxers_output);
        let hardware_accelerators = parse_hwaccels(&hwaccels_output);
        let subtitle_support = subtitle_codecs_from(&codec_support, &encoders, &decoders);
        let license_mode = license_mode_from_version_output(&ffmpeg_version_output);
        let ffmpeg_enable_gpl = ffmpeg_version_output.contains("--enable-gpl");
        let ffmpeg_enable_version3 = ffmpeg_version_output.contains("--enable-version3");
        let ffmpeg_enable_nonfree = ffmpeg_version_output.contains("--enable-nonfree");
        let mut absent_capabilities = absent_capabilities_from(&hardware_accelerators, &encoders);
        absent_capabilities.extend(license_excluded_capabilities(&ffmpeg_version_output));
        absent_capabilities.sort();
        absent_capabilities.dedup();

        Ok(CapabilitySnapshot {
            ffmpeg_version,
            ffprobe_version,
            codecs,
            codec_support,
            encoders,
            decoders,
            muxers,
            demuxers,
            hardware_accelerators,
            subtitle_support,
            filesystem_utilities: filesystem_utilities(),
            utility_capabilities: utility_capabilities(),
            license_mode: license_mode.clone(),
            ffmpeg_license_mode: license_mode,
            ffmpeg_enable_gpl,
            ffmpeg_enable_version3,
            ffmpeg_enable_nonfree,
            compliance_links: compliance_links(),
            absent_capabilities,
        })
    }
}

fn parse_version_line(output: &str) -> Option<String> {
    output.lines().next().and_then(|line| {
        let mut parts = line.split_whitespace();
        let _program = parts.next()?;
        let _keyword = parts.next()?;
        let version = parts.next()?;
        Some(version.to_string())
    })
}

fn parse_codecs(output: &str) -> Vec<CodecCapability> {
    let mut codecs = BTreeSet::new();
    for line in output.lines() {
        if !line.starts_with(' ') {
            continue;
        }
        let trimmed = line.trim();
        let mut tokens = trimmed.split_whitespace();
        let Some(flags) = tokens.next() else {
            continue;
        };
        if flags.len() < 6 {
            continue;
        }
        let Some(codec_name) = tokens.next() else {
            continue;
        };
        let mut chars = flags.chars();
        let decode_supported = chars.next().is_some_and(|item| item == 'D');
        let encode_supported = chars.next().is_some_and(|item| item == 'E');
        codecs.insert((codec_name.to_string(), encode_supported, decode_supported));
    }
    codecs
        .into_iter()
        .map(
            |(name, encode_supported, decode_supported)| CodecCapability {
                name,
                encode_supported,
                decode_supported,
            },
        )
        .collect()
}

fn parse_tool_names(output: &str) -> Vec<String> {
    let mut names = BTreeSet::new();
    for line in output.lines() {
        if !line.starts_with(' ') {
            continue;
        }
        let trimmed = line.trim();
        let mut tokens = trimmed.split_whitespace();
        let Some(flags) = tokens.next() else {
            continue;
        };
        if flags.is_empty() {
            continue;
        }
        let Some(name) = tokens.next() else {
            continue;
        };
        names.insert(name.to_string());
    }
    names.into_iter().collect()
}

fn parse_hwaccels(output: &str) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.ends_with(':'))
        .map(str::to_string)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn subtitle_codecs_from(
    codecs: &[CodecCapability],
    encoders: &[String],
    decoders: &[String],
) -> Vec<String> {
    let mut subtitle_codecs = BTreeSet::new();
    for name in codecs
        .iter()
        .map(|codec| codec.name.as_str())
        .chain(encoders.iter().map(String::as_str))
        .chain(decoders.iter().map(String::as_str))
    {
        let normalized = name.trim().to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "ass"
                | "ssa"
                | "srt"
                | "subrip"
                | "webvtt"
                | "mov_text"
                | "hdmv_pgs_subtitle"
                | "dvd_subtitle"
        ) {
            subtitle_codecs.insert(normalized);
        }
    }
    subtitle_codecs.into_iter().collect()
}

fn license_mode_from_version_output(output: &str) -> String {
    if output.contains("--enable-nonfree") {
        "nonfree".to_string()
    } else if output.contains("--enable-gpl") {
        "gpl".to_string()
    } else {
        "lgpl".to_string()
    }
}

fn absent_capabilities_from(hardware_accelerators: &[String], encoders: &[String]) -> Vec<String> {
    let mut absent = BTreeSet::new();
    if hardware_accelerators.is_empty() {
        absent.insert("hardware_acceleration".to_string());
    }
    if !encoders
        .iter()
        .any(|encoder| encoder.eq_ignore_ascii_case("libfdk_aac"))
    {
        absent.insert("libfdk_aac".to_string());
    }
    absent.into_iter().collect()
}

fn license_excluded_capabilities(output: &str) -> Vec<String> {
    let mut excluded = BTreeSet::new();
    if !output.contains("--enable-nonfree") {
        excluded.insert("--enable-nonfree".to_string());
        excluded.insert("proprietary-codecs".to_string());
        excluded.insert("license-incompatible-codecs".to_string());
    }
    excluded.into_iter().collect()
}

fn filesystem_utilities() -> Vec<String> {
    [
        "atomic_rename",
        "copy",
        "create_dir_all",
        "remove_file",
        "quarantine",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn utility_capabilities() -> Vec<String> {
    [
        "ffmpeg",
        "ffprobe",
        "ffplay",
        "managed_workspace",
        "final_graph_verify",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn compliance_links() -> Vec<String> {
    [
        "/app/compliance/SOURCE-OFFER.txt",
        "/app/compliance/THIRD-PARTY-NOTICES.md",
        "/app/compliance/media-runtime-inventory.spdx.json",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilityDetectError, CapabilityDetector, CapabilityProbeExecutor, CapabilitySnapshot,
        CodecCapability, FfmpegCapabilityDetector, SystemCapabilityProbeExecutor,
        UnavailableCapabilityDetector,
    };
    use std::collections::HashMap;
    use std::io::Cursor;
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    #[derive(Debug)]
    struct StaticDetector;

    impl CapabilityDetector for StaticDetector {
        fn detect(&self) -> Result<CapabilitySnapshot, CapabilityDetectError> {
            Ok(CapabilitySnapshot {
                ffmpeg_version: "7.0".to_string(),
                ffprobe_version: "7.0".to_string(),
                codecs: vec!["h264".to_string()],
                codec_support: vec![CodecCapability {
                    name: "h264".to_string(),
                    encode_supported: true,
                    decode_supported: true,
                }],
                encoders: vec!["libx264".to_string()],
                decoders: vec!["h264".to_string()],
                muxers: vec!["matroska".to_string()],
                demuxers: vec!["matroska".to_string()],
                hardware_accelerators: Vec::new(),
                subtitle_support: vec!["subrip".to_string()],
                filesystem_utilities: vec!["atomic_rename".to_string()],
                utility_capabilities: vec![
                    "ffmpeg".to_string(),
                    "ffprobe".to_string(),
                    "ffplay".to_string(),
                ],
                license_mode: "gpl".to_string(),
                ffmpeg_license_mode: "gpl".to_string(),
                ffmpeg_enable_gpl: true,
                ffmpeg_enable_version3: false,
                ffmpeg_enable_nonfree: false,
                compliance_links: vec!["/app/compliance/SOURCE-OFFER.txt".to_string()],
                absent_capabilities: vec!["--enable-nonfree".to_string()],
            })
        }
    }

    #[test]
    fn invalid_when_codecs_empty() {
        let snapshot = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: Vec::new(),
            codec_support: Vec::new(),
            encoders: Vec::new(),
            decoders: Vec::new(),
            muxers: Vec::new(),
            demuxers: Vec::new(),
            hardware_accelerators: Vec::new(),
            subtitle_support: Vec::new(),
            filesystem_utilities: Vec::new(),
            utility_capabilities: Vec::new(),
            license_mode: String::new(),
            ffmpeg_license_mode: String::new(),
            ffmpeg_enable_gpl: false,
            ffmpeg_enable_version3: false,
            ffmpeg_enable_nonfree: false,
            compliance_links: Vec::new(),
            absent_capabilities: Vec::new(),
        };
        assert!(!snapshot.is_valid());
    }

    #[test]
    fn invalid_when_codec_support_missing_for_advertised_codec() {
        let snapshot = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
            codec_support: Vec::new(),
            encoders: vec!["libx264".to_string()],
            ..CapabilitySnapshot::default()
        };

        assert!(!snapshot.is_valid());
    }

    #[test]
    fn invalid_when_codec_support_has_no_supported_direction() {
        let snapshot = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
            codec_support: vec![CodecCapability {
                name: "h264".to_string(),
                encode_supported: false,
                decode_supported: false,
            }],
            encoders: vec!["libx264".to_string()],
            ..CapabilitySnapshot::default()
        };

        assert!(!snapshot.is_valid());
    }

    #[test]
    fn decode_only_codec_support_can_be_valid() {
        let snapshot = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: vec!["h264".to_string()],
            codec_support: vec![CodecCapability {
                name: "H264".to_string(),
                encode_supported: false,
                decode_supported: true,
            }],
            encoders: vec!["libx264".to_string()],
            ..CapabilitySnapshot::default()
        };

        assert!(snapshot.is_valid());
    }

    #[test]
    fn unavailable_detector_returns_error() {
        let detector = UnavailableCapabilityDetector;
        assert_eq!(detector.detect(), Err(CapabilityDetectError::Unavailable));
    }

    #[test]
    fn static_detector_returns_valid_snapshot() {
        let detector = StaticDetector;
        let snapshot_result = detector.detect();
        assert!(snapshot_result.is_ok());
        let Ok(snapshot) = snapshot_result else {
            return;
        };
        assert!(snapshot.is_valid());
    }

    #[test]
    fn system_probe_stdout_reader_is_bounded() -> Result<(), Box<dyn std::error::Error>> {
        let output = super::read_bounded_stdout(Cursor::new(vec![b'x'; 128]), 32)?;

        assert!(output.truncated);
        assert_eq!(output.bytes.len(), 32);
        Ok(())
    }

    #[test]
    fn system_probe_stderr_reader_retains_tail() -> Result<(), Box<dyn std::error::Error>> {
        let mut bytes = vec![b'x'; 128];
        bytes.extend_from_slice(b"probe-summary");

        let output = super::read_bounded_tail(Cursor::new(bytes), 32)?;

        assert!(output.truncated);
        assert!(String::from_utf8(output.bytes)?.ends_with("probe-summary"));
        Ok(())
    }

    #[test]
    fn system_probe_stderr_reader_keeps_exact_limit_without_marker()
    -> Result<(), Box<dyn std::error::Error>> {
        let output = super::read_bounded_tail(Cursor::new(b"probe-summary".to_vec()), 32)?;

        assert!(!output.truncated);
        assert_eq!(super::stderr_suffix(&output), ": probe-summary");
        Ok(())
    }

    #[test]
    fn system_probe_reports_nonzero_status_with_bounded_stderr_tail() {
        let result = SystemCapabilityProbeExecutor.run(
            "/bin/sh",
            &[
                "-c",
                "i=0; while [ \"$i\" -lt 70000 ]; do printf x >&2; i=$((i + 1)); done; printf probe-summary >&2; exit 7",
            ],
        );

        let Err(CapabilityDetectError::CommandFailed(detail)) = result else {
            panic!("expected command failure with stderr detail");
        };
        assert!(detail.contains(super::CAPABILITY_PROBE_TRUNCATION_MARKER));
        assert!(detail.contains("probe-summary"));
    }

    #[test]
    fn system_probe_times_out_active_child() {
        let started = Instant::now();
        let result = super::run_system_probe("/bin/sleep", &["30"], Duration::from_millis(150));

        let Err(CapabilityDetectError::CommandFailed(detail)) = result else {
            panic!("expected timeout failure");
        };
        assert!(detail.contains("timed out"));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[derive(Default)]
    struct StubExecutor {
        outputs: HashMap<String, String>,
    }

    impl CapabilityProbeExecutor for StubExecutor {
        fn run(&self, program: &str, args: &[&str]) -> Result<String, CapabilityDetectError> {
            let key = format!("{program} {}", args.join(" "));
            self.outputs
                .get(&key)
                .cloned()
                .ok_or(CapabilityDetectError::CommandFailed(key))
        }
    }

    fn capability_probe_outputs() -> HashMap<String, String> {
        let mut outputs = HashMap::new();
        outputs.insert(
            "ffmpeg -version".to_string(),
            "ffmpeg version 7.0.2 Copyright --enable-gpl --enable-version3".to_string(),
        );
        outputs.insert(
            "ffprobe -version".to_string(),
            "ffprobe version 7.0.2 Copyright".to_string(),
        );
        outputs.insert(
            "ffplay -version".to_string(),
            "ffplay version 7.0.2 Copyright".to_string(),
        );
        outputs.insert(
            "ffmpeg -codecs".to_string(),
            "Codecs:\n DEVILS h264 H.264\n ..V.L. evc MPEG-5 EVC\n DEVILS hevc H.265\n".to_string(),
        );
        outputs.insert(
            "ffmpeg -encoders".to_string(),
            "Encoders:\n V..... libx265 H.265\n S..... subrip SubRip subtitle\n V..... hevc_nvenc NVIDIA HEVC\n".to_string(),
        );
        outputs.insert(
            "ffmpeg -decoders".to_string(),
            "Decoders:\n V..... h264 H.264\n S..... subrip SubRip subtitle\n".to_string(),
        );
        outputs.insert(
            "ffmpeg -muxers".to_string(),
            "Muxers:\n E matroska Matroska\n E mp4 MP4\n".to_string(),
        );
        outputs.insert(
            "ffmpeg -demuxers".to_string(),
            "Demuxers:\n D matroska Matroska\n D mov,mp4,m4a,3gp,3g2,mj2 QuickTime\n".to_string(),
        );
        outputs.insert(
            "ffmpeg -hwaccels".to_string(),
            "Hardware acceleration methods:\nvideotoolbox\n".to_string(),
        );
        outputs
    }

    #[test]
    fn ffmpeg_detector_parses_versions_and_codecs() {
        let detector = FfmpegCapabilityDetector::new(
            Arc::new(StubExecutor {
                outputs: capability_probe_outputs(),
            }),
            "ffmpeg",
            "ffprobe",
            "ffplay",
        );
        let snapshot_result = detector.detect();
        assert!(snapshot_result.is_ok());
        let Ok(snapshot) = snapshot_result else {
            return;
        };
        assert_eq!(snapshot.ffmpeg_version, "7.0.2");
        assert_eq!(snapshot.ffprobe_version, "7.0.2");
        assert_eq!(
            snapshot.codecs,
            vec!["h264".to_string(), "hevc".to_string()]
        );
        assert_eq!(
            snapshot.codec_support,
            vec![
                CodecCapability {
                    name: "evc".to_string(),
                    encode_supported: false,
                    decode_supported: false,
                },
                CodecCapability {
                    name: "h264".to_string(),
                    encode_supported: true,
                    decode_supported: true,
                },
                CodecCapability {
                    name: "hevc".to_string(),
                    encode_supported: true,
                    decode_supported: true,
                },
            ]
        );
        assert_eq!(
            snapshot.encoders,
            vec![
                "hevc_nvenc".to_string(),
                "libx265".to_string(),
                "subrip".to_string()
            ]
        );
        assert_eq!(
            snapshot.decoders,
            vec!["h264".to_string(), "subrip".to_string()]
        );
        assert_eq!(
            snapshot.muxers,
            vec!["matroska".to_string(), "mp4".to_string()]
        );
        assert_eq!(
            snapshot.demuxers,
            vec![
                "matroska".to_string(),
                "mov,mp4,m4a,3gp,3g2,mj2".to_string()
            ]
        );
        assert_eq!(
            snapshot.hardware_accelerators,
            vec!["videotoolbox".to_string()]
        );
        assert_eq!(snapshot.subtitle_support, vec!["subrip".to_string()]);
        assert_eq!(snapshot.license_mode, "gpl");
        assert_eq!(snapshot.ffmpeg_license_mode, "gpl");
        assert!(snapshot.ffmpeg_enable_gpl);
        assert!(snapshot.ffmpeg_enable_version3);
        assert!(!snapshot.ffmpeg_enable_nonfree);
        assert!(
            snapshot
                .compliance_links
                .iter()
                .any(|link| link.ends_with("SOURCE-OFFER.txt"))
        );
    }
}
