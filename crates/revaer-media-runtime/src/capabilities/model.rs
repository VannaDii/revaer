use serde::{Deserialize, Serialize};

use super::parse::{compliance_links, filesystem_utilities, utility_capabilities};

/// Runtime snapshot of media tool capabilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySnapshot {
    /// `FFmpeg` semantic version.
    pub ffmpeg_version: String,
    /// `FFprobe` semantic version.
    pub ffprobe_version: String,
    /// Available codec names.
    pub codecs: Vec<String>,
    /// Per-codec encode/decode support parsed from `FFmpeg` codec flags.
    pub codec_support: Vec<CodecCapability>,
    /// Available `FFmpeg` encoder names.
    pub encoders: Vec<String>,
    /// Available `FFmpeg` decoder names.
    pub decoders: Vec<String>,
    /// Available `FFmpeg` muxer names.
    pub muxers: Vec<String>,
    /// Available `FFmpeg` demuxer names.
    pub demuxers: Vec<String>,
    /// Hardware acceleration backends reported by `FFmpeg`.
    pub hardware_accelerators: Vec<String>,
    /// Subtitle codecs supported by this toolchain.
    pub subtitle_support: Vec<String>,
    /// Filesystem primitives the runtime requires for managed replacement.
    pub filesystem_utilities: Vec<String>,
    /// Runtime utilities available to media jobs.
    pub utility_capabilities: Vec<String>,
    /// License mode inferred from the runtime `FFmpeg` build.
    pub license_mode: String,
    /// License mode inferred specifically from the `FFmpeg` build flags.
    pub ffmpeg_license_mode: String,
    /// Whether `FFmpeg` was built with `--enable-gpl`.
    pub ffmpeg_enable_gpl: bool,
    /// Whether `FFmpeg` was built with `--enable-version3`.
    pub ffmpeg_enable_version3: bool,
    /// Whether `FFmpeg` was built with `--enable-nonfree`.
    pub ffmpeg_enable_nonfree: bool,
    /// Compliance artifact links bundled with the runtime image.
    pub compliance_links: Vec<String>,
    /// Capabilities intentionally absent from this runtime.
    pub absent_capabilities: Vec<String>,
}

/// Encode/decode support for one `FFmpeg` codec row.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CodecCapability {
    /// Codec name.
    pub name: String,
    /// Whether `FFmpeg` reports encoding support for this codec.
    pub encode_supported: bool,
    /// Whether `FFmpeg` reports decoding support for this codec.
    pub decode_supported: bool,
}

impl CapabilitySnapshot {
    /// Return whether required binaries and supported codecs are present.
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
