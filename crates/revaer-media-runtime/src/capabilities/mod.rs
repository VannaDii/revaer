//! Tool capability models.

use serde::{Deserialize, Serialize};

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

#[cfg(test)]
mod tests {
    use super::CapabilitySnapshot;

    #[test]
    fn invalid_when_codecs_empty() {
        let snapshot = CapabilitySnapshot {
            ffmpeg_version: "7.0".to_string(),
            ffprobe_version: "7.0".to_string(),
            codecs: Vec::new(),
        };
        assert!(!snapshot.is_valid());
    }
}
