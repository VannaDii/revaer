//! Tool capability models and detector adapters.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::sync::Arc;
use thiserror::Error;

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
    /// Returns true when required binaries and at least one codec are present.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.ffmpeg_version.trim().is_empty()
            && !self.ffprobe_version.trim().is_empty()
            && !self.codecs.is_empty()
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
        let output = std::process::Command::new(program)
            .args(args)
            .output()
            .map_err(|err| CapabilityDetectError::CommandFailed(err.to_string()))?;
        if !output.status.success() {
            return Err(CapabilityDetectError::CommandFailed(format!(
                "{program} {:?} exited with status {}",
                args, output.status
            )));
        }
        String::from_utf8(output.stdout)
            .map_err(|err| CapabilityDetectError::OutputMalformed(err.to_string()))
    }
}

/// ffmpeg/ffprobe-backed capability detector.
pub struct FfmpegCapabilityDetector {
    executor: Arc<dyn CapabilityProbeExecutor>,
    ffmpeg_bin: String,
    ffprobe_bin: String,
}

impl FfmpegCapabilityDetector {
    /// Construct detector with injected probe executor and binary names.
    #[must_use]
    pub fn new(
        executor: Arc<dyn CapabilityProbeExecutor>,
        ffmpeg_bin: impl Into<String>,
        ffprobe_bin: impl Into<String>,
    ) -> Self {
        Self {
            executor,
            ffmpeg_bin: ffmpeg_bin.into(),
            ffprobe_bin: ffprobe_bin.into(),
        }
    }
}

impl CapabilityDetector for FfmpegCapabilityDetector {
    fn detect(&self) -> Result<CapabilitySnapshot, CapabilityDetectError> {
        let ffmpeg_version_output = self.executor.run(&self.ffmpeg_bin, &["-version"])?;
        let ffprobe_version_output = self.executor.run(&self.ffprobe_bin, &["-version"])?;
        let codecs_output = self.executor.run(&self.ffmpeg_bin, &["-codecs"])?;
        let encoders_output = self.executor.run(&self.ffmpeg_bin, &["-encoders"])?;

        let ffmpeg_version = parse_version_line(&ffmpeg_version_output).ok_or_else(|| {
            CapabilityDetectError::OutputMalformed("missing ffmpeg version line".to_string())
        })?;
        let ffprobe_version = parse_version_line(&ffprobe_version_output).ok_or_else(|| {
            CapabilityDetectError::OutputMalformed("missing ffprobe version line".to_string())
        })?;
        let codec_support = parse_codecs(&codecs_output);
        if codec_support.is_empty() {
            return Err(CapabilityDetectError::OutputMalformed(
                "no codecs parsed from ffmpeg -codecs".to_string(),
            ));
        }
        let codecs = codec_support
            .iter()
            .map(|item| item.name.clone())
            .collect::<Vec<_>>();
        let encoders = parse_tool_names(&encoders_output);

        Ok(CapabilitySnapshot {
            ffmpeg_version,
            ffprobe_version,
            codecs,
            codec_support,
            encoders,
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
        if flags.len() < 2 {
            continue;
        }
        let Some(name) = tokens.next() else {
            continue;
        };
        names.insert(name.to_string());
    }
    names.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::{
        CapabilityDetectError, CapabilityDetector, CapabilityProbeExecutor, CapabilitySnapshot,
        CodecCapability, FfmpegCapabilityDetector, UnavailableCapabilityDetector,
    };
    use std::collections::HashMap;
    use std::sync::Arc;

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
        };
        assert!(!snapshot.is_valid());
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

    #[test]
    fn ffmpeg_detector_parses_versions_and_codecs() {
        let mut outputs = HashMap::new();
        outputs.insert(
            "ffmpeg -version".to_string(),
            "ffmpeg version 7.0.2 Copyright".to_string(),
        );
        outputs.insert(
            "ffprobe -version".to_string(),
            "ffprobe version 7.0.2 Copyright".to_string(),
        );
        outputs.insert(
            "ffmpeg -codecs".to_string(),
            "Codecs:\n DEVILS h264 H.264\n DEVILS hevc H.265\n".to_string(),
        );
        outputs.insert(
            "ffmpeg -encoders".to_string(),
            "Encoders:\n V..... libx265 H.265\n V..... hevc_nvenc NVIDIA HEVC\n".to_string(),
        );

        let detector =
            FfmpegCapabilityDetector::new(Arc::new(StubExecutor { outputs }), "ffmpeg", "ffprobe");
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
            vec!["hevc_nvenc".to_string(), "libx265".to_string()]
        );
    }
}
