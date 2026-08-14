use std::sync::Arc;
use std::time::Duration;

use thiserror::Error;

use crate::process::{ProcessLimits, run_bounded};

use super::model::CapabilitySnapshot;
use super::parse::{
    absent_capabilities_from, compliance_links, filesystem_utilities,
    license_excluded_capabilities, license_mode_from_version_output, parse_codecs, parse_hwaccels,
    parse_tool_names, parse_version_line, subtitle_codecs_from, utility_capabilities,
};

const CAPABILITY_PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const CAPABILITY_STDOUT_LIMIT: usize = 16 * 1024 * 1024;
const CAPABILITY_STDERR_LIMIT: usize = 1024 * 1024;

/// Capability detection error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityDetectError {
    /// Runtime detector is unavailable in this build/runtime context.
    #[error("capability detector unavailable")]
    Unavailable,
    /// Command invocation failed or exceeded its execution bounds.
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
    /// Returns [`CapabilityDetectError::CommandFailed`] when execution fails or exceeds a bound.
    /// Returns [`CapabilityDetectError::OutputMalformed`] when stdout is not valid UTF-8.
    fn run(&self, program: &str, args: &[&str]) -> Result<String, CapabilityDetectError>;
}

/// System-process probe executor with deadlines and bounded output capture.
#[derive(Debug, Default)]
pub struct SystemCapabilityProbeExecutor;

impl CapabilityProbeExecutor for SystemCapabilityProbeExecutor {
    fn run(&self, program: &str, args: &[&str]) -> Result<String, CapabilityDetectError> {
        let output = run_bounded(
            program,
            args,
            ProcessLimits {
                timeout: CAPABILITY_PROBE_TIMEOUT,
                max_stdout_bytes: CAPABILITY_STDOUT_LIMIT,
                max_stderr_bytes: CAPABILITY_STDERR_LIMIT,
            },
        )
        .map_err(|error| CapabilityDetectError::CommandFailed(error.to_string()))?;
        String::from_utf8(output)
            .map_err(|error| CapabilityDetectError::OutputMalformed(error.to_string()))
    }
}

/// FFmpeg/FFprobe-backed capability detector.
pub struct FfmpegCapabilityDetector {
    executor: Arc<dyn CapabilityProbeExecutor>,
    ffmpeg_bin: String,
    ffprobe_bin: String,
    ffplay_bin: String,
}

impl FfmpegCapabilityDetector {
    /// Construct a detector with injected probe executor and binary names.
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

    fn probe(&self) -> Result<ProbeOutputs, CapabilityDetectError> {
        Ok(ProbeOutputs {
            ffmpeg_version: self.executor.run(&self.ffmpeg_bin, &["-version"])?,
            ffprobe_version: self.executor.run(&self.ffprobe_bin, &["-version"])?,
            ffplay_version: self.executor.run(&self.ffplay_bin, &["-version"])?,
            codecs: self.executor.run(&self.ffmpeg_bin, &["-codecs"])?,
            encoders: self.executor.run(&self.ffmpeg_bin, &["-encoders"])?,
            decoders: self.executor.run(&self.ffmpeg_bin, &["-decoders"])?,
            muxers: self.executor.run(&self.ffmpeg_bin, &["-muxers"])?,
            demuxers: self.executor.run(&self.ffmpeg_bin, &["-demuxers"])?,
            hardware_accelerators: self.executor.run(&self.ffmpeg_bin, &["-hwaccels"])?,
        })
    }
}

struct ProbeOutputs {
    ffmpeg_version: String,
    ffprobe_version: String,
    ffplay_version: String,
    codecs: String,
    encoders: String,
    decoders: String,
    muxers: String,
    demuxers: String,
    hardware_accelerators: String,
}

impl CapabilityDetector for FfmpegCapabilityDetector {
    fn detect(&self) -> Result<CapabilitySnapshot, CapabilityDetectError> {
        let outputs = self.probe()?;
        snapshot_from_outputs(&outputs)
    }
}

fn snapshot_from_outputs(
    outputs: &ProbeOutputs,
) -> Result<CapabilitySnapshot, CapabilityDetectError> {
    let ffmpeg_version = required_version("ffmpeg", &outputs.ffmpeg_version)?;
    let ffprobe_version = required_version("ffprobe", &outputs.ffprobe_version)?;
    let _ffplay_version = required_version("ffplay", &outputs.ffplay_version)?;
    let codec_support = parse_codecs(&outputs.codecs);
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
    let encoders = parse_tool_names(&outputs.encoders);
    let decoders = parse_tool_names(&outputs.decoders);
    let muxers = parse_tool_names(&outputs.muxers);
    let demuxers = parse_tool_names(&outputs.demuxers);
    let hardware_accelerators = parse_hwaccels(&outputs.hardware_accelerators);
    let subtitle_support = subtitle_codecs_from(&codec_support, &encoders, &decoders);
    let license_mode = license_mode_from_version_output(&outputs.ffmpeg_version);
    let mut absent_capabilities = absent_capabilities_from(&hardware_accelerators, &encoders);
    absent_capabilities.extend(license_excluded_capabilities(&outputs.ffmpeg_version));
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
        ffmpeg_enable_gpl: outputs.ffmpeg_version.contains("--enable-gpl"),
        ffmpeg_enable_version3: outputs.ffmpeg_version.contains("--enable-version3"),
        ffmpeg_enable_nonfree: outputs.ffmpeg_version.contains("--enable-nonfree"),
        compliance_links: compliance_links(),
        absent_capabilities,
    })
}

fn required_version(program: &str, output: &str) -> Result<String, CapabilityDetectError> {
    parse_version_line(output).ok_or_else(|| {
        CapabilityDetectError::OutputMalformed(format!("missing {program} version line"))
    })
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    #[derive(Default)]
    struct StubExecutor {
        outputs: HashMap<String, Result<String, CapabilityDetectError>>,
    }

    impl CapabilityProbeExecutor for StubExecutor {
        fn run(&self, program: &str, args: &[&str]) -> Result<String, CapabilityDetectError> {
            let key = format!("{program} {}", args.join(" "));
            self.outputs.get(&key).cloned().ok_or_else(|| {
                CapabilityDetectError::CommandFailed(format!("unexpected probe: {key}"))
            })?
        }
    }

    fn detector(
        outputs: HashMap<String, Result<String, CapabilityDetectError>>,
    ) -> FfmpegCapabilityDetector {
        FfmpegCapabilityDetector::new(
            Arc::new(StubExecutor { outputs }),
            "ffmpeg",
            "ffprobe",
            "ffplay",
        )
    }

    fn valid_outputs() -> HashMap<String, Result<String, CapabilityDetectError>> {
        HashMap::from([
            (
                "ffmpeg -version".to_string(),
                Ok("ffmpeg version 7.0.2 --enable-gpl".to_string()),
            ),
            (
                "ffprobe -version".to_string(),
                Ok("ffprobe version 7.0.2".to_string()),
            ),
            (
                "ffplay -version".to_string(),
                Ok("ffplay version 7.0.2".to_string()),
            ),
            (
                "ffmpeg -codecs".to_string(),
                Ok("Codecs:\n DEVILS h264 H.264\n".to_string()),
            ),
            (
                "ffmpeg -encoders".to_string(),
                Ok("Encoders:\n V..... libx264 H.264\n".to_string()),
            ),
            (
                "ffmpeg -decoders".to_string(),
                Ok("Decoders:\n V..... h264 H.264\n".to_string()),
            ),
            (
                "ffmpeg -muxers".to_string(),
                Ok("Muxers:\n E matroska Matroska\n".to_string()),
            ),
            (
                "ffmpeg -demuxers".to_string(),
                Ok("Demuxers:\n D matroska Matroska\n".to_string()),
            ),
            (
                "ffmpeg -hwaccels".to_string(),
                Ok("Hardware acceleration methods:\nvideotoolbox\n".to_string()),
            ),
        ])
    }

    #[test]
    fn detector_propagates_probe_command_failure() {
        let mut outputs = valid_outputs();
        let expected = CapabilityDetectError::CommandFailed("encoder probe failed".to_string());
        outputs.insert("ffmpeg -encoders".to_string(), Err(expected.clone()));

        assert_eq!(detector(outputs).detect(), Err(expected));
    }

    #[test]
    fn detector_rejects_each_missing_tool_version() {
        for (probe, program) in [
            ("ffmpeg -version", "ffmpeg"),
            ("ffprobe -version", "ffprobe"),
            ("ffplay -version", "ffplay"),
        ] {
            let mut outputs = valid_outputs();
            outputs.insert(probe.to_string(), Ok("version unavailable".to_string()));

            assert_eq!(
                detector(outputs).detect(),
                Err(CapabilityDetectError::OutputMalformed(format!(
                    "missing {program} version line"
                )))
            );
        }
    }

    #[test]
    fn detector_rejects_output_without_codecs() {
        let mut outputs = valid_outputs();
        outputs.insert("ffmpeg -codecs".to_string(), Ok("Codecs:\n".to_string()));

        assert_eq!(
            detector(outputs).detect(),
            Err(CapabilityDetectError::OutputMalformed(
                "no codecs parsed from ffmpeg -codecs".to_string()
            ))
        );
    }

    #[cfg(unix)]
    #[test]
    fn system_executor_returns_utf8_stdout() -> Result<(), CapabilityDetectError> {
        let output = SystemCapabilityProbeExecutor.run("/bin/sh", &["-c", "printf detector"])?;

        assert_eq!(output, "detector");
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn system_executor_maps_process_failure() {
        let result = SystemCapabilityProbeExecutor.run("/revaer/missing-probe", &[]);

        assert!(matches!(
            result,
            Err(CapabilityDetectError::CommandFailed(message))
                if message.contains("failed to spawn process")
        ));
    }

    #[cfg(unix)]
    #[test]
    fn system_executor_rejects_non_utf8_stdout() {
        let result = SystemCapabilityProbeExecutor.run("/bin/sh", &["-c", "printf '\\377'"]);

        assert!(matches!(
            result,
            Err(CapabilityDetectError::OutputMalformed(message))
                if message.contains("invalid utf-8 sequence")
        ));
    }
}
