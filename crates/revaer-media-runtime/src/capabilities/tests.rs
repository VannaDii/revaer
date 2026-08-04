use std::collections::HashMap;
use std::sync::Arc;

use super::{
    CapabilityDetectError, CapabilityDetector, CapabilityProbeExecutor, CapabilitySnapshot,
    CodecCapability, FfmpegCapabilityDetector, UnavailableCapabilityDetector,
};

#[derive(Debug)]
struct StaticDetector;

impl CapabilityDetector for StaticDetector {
    fn detect(&self) -> Result<CapabilitySnapshot, CapabilityDetectError> {
        Ok(valid_snapshot())
    }
}

fn valid_snapshot() -> CapabilitySnapshot {
    CapabilitySnapshot {
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
fn codec_capability_returns_case_insensitive_match_or_unsupported_default() {
    let snapshot = valid_snapshot();
    assert_eq!(
        snapshot.codec_capability("H264"),
        CodecCapability {
            name: "h264".to_string(),
            encode_supported: true,
            decode_supported: true,
        }
    );
    assert_eq!(
        snapshot.codec_capability("vp9"),
        CodecCapability {
            name: "vp9".to_string(),
            encode_supported: false,
            decode_supported: false,
        }
    );
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
    HashMap::from([
        (
            "ffmpeg -version".to_string(),
            "ffmpeg version 7.0.2 Copyright --enable-gpl --enable-version3".to_string(),
        ),
        (
            "ffprobe -version".to_string(),
            "ffprobe version 7.0.2 Copyright".to_string(),
        ),
        (
            "ffplay -version".to_string(),
            "ffplay version 7.0.2 Copyright".to_string(),
        ),
        (
            "ffmpeg -codecs".to_string(),
            "Codecs:\n DEVILS h264 H.264\n ..V.L. evc MPEG-5 EVC\n DEVILS hevc H.265\n"
                .to_string(),
        ),
        (
            "ffmpeg -encoders".to_string(),
            "Encoders:\n V..... libx265 H.265\n S..... subrip SubRip subtitle\n V..... hevc_nvenc NVIDIA HEVC\n"
                .to_string(),
        ),
        (
            "ffmpeg -decoders".to_string(),
            "Decoders:\n V..... h264 H.264\n S..... subrip SubRip subtitle\n".to_string(),
        ),
        (
            "ffmpeg -muxers".to_string(),
            "Muxers:\n E matroska Matroska\n E mp4 MP4\n".to_string(),
        ),
        (
            "ffmpeg -demuxers".to_string(),
            "Demuxers:\n D matroska Matroska\n D mov,mp4,m4a,3gp,3g2,mj2 QuickTime\n"
                .to_string(),
        ),
        (
            "ffmpeg -hwaccels".to_string(),
            "Hardware acceleration methods:\nvideotoolbox\n".to_string(),
        ),
    ])
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
            "subrip".to_string(),
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
            "mov,mp4,m4a,3gp,3g2,mj2".to_string(),
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
