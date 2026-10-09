//! Explicit output settings for immutable policy creation and readback.

use serde::{Deserialize, Serialize};

/// Permission and ownership preservation intent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct MediaOutputPreservation {
    /// Preserve source permission bits during replacement.
    pub preserve_permissions: bool,
    /// Preserve source ownership during replacement.
    pub preserve_ownership: bool,
}

/// Complete output settings; dry-run is independent of the profile restriction.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MediaPolicyOutput {
    /// Forbid source mutation for every profile using this policy.
    pub dry_run: bool,
    /// `disabled` or the approved in-place `atomic_replace` mode.
    pub replacement_mode: String,
    /// Quarantine failed candidates for diagnosis.
    pub quarantine_enabled: bool,
    /// Explicit preservation settings.
    #[serde(flatten)]
    pub preservation: MediaOutputPreservation,
}

impl Default for MediaPolicyOutput {
    fn default() -> Self {
        Self {
            dry_run: true,
            replacement_mode: "disabled".into(),
            quarantine_enabled: true,
            preservation: MediaOutputPreservation {
                preserve_permissions: true,
                preserve_ownership: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::MediaPolicyOutput;

    #[test]
    fn complete_output_roundtrips_and_rejects_missing_or_unknown_fields()
    -> Result<(), serde_json::Error> {
        let output = MediaPolicyOutput::default();
        let body = serde_json::to_value(&output)?;
        assert_eq!(
            serde_json::from_value::<MediaPolicyOutput>(body.clone())?,
            output
        );
        for field in [
            "dry_run",
            "replacement_mode",
            "quarantine_enabled",
            "preserve_permissions",
            "preserve_ownership",
        ] {
            let mut incomplete = body.clone();
            incomplete
                .as_object_mut()
                .ok_or_else(|| {
                    serde_json::Error::io(std::io::Error::other("output must be an object"))
                })?
                .remove(field);
            assert!(serde_json::from_value::<MediaPolicyOutput>(incomplete).is_err());
        }
        let mut extra = body;
        extra["source_path"] = serde_json::json!("/private/unrequested");
        assert!(serde_json::from_value::<MediaPolicyOutput>(extra).is_err());
        Ok(())
    }
}
