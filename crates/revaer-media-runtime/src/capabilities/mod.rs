//! Tool capability models.

use serde::{Deserialize, Serialize};
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
}

impl CapabilitySnapshot {
    /// Returns true when required binaries and at least one codec are present.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.ffmpeg_version.trim().is_empty()
            && !self.ffprobe_version.trim().is_empty()
            && !self.codecs.is_empty()
    }
}

/// Capability detection error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CapabilityDetectError {
    /// Runtime detector is unavailable in this build/runtime context.
    #[error("capability detector unavailable")]
    Unavailable,
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

#[cfg(test)]
mod tests {
    use super::{
        CapabilityDetectError, CapabilityDetector, CapabilitySnapshot,
        UnavailableCapabilityDetector,
    };

    #[derive(Debug)]
    struct StaticDetector;

    impl CapabilityDetector for StaticDetector {
        fn detect(&self) -> Result<CapabilitySnapshot, CapabilityDetectError> {
            Ok(CapabilitySnapshot {
                ffmpeg_version: "7.0".to_string(),
                ffprobe_version: "7.0".to_string(),
                codecs: vec!["h264".to_string()],
            })
        }
    }

    #[test]
    fn invalid_when_codecs_empty() {
        let snapshot = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: Vec::new(),
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
}
