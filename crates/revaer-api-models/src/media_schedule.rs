//! Explicit path-free schedule configuration; there is no default cadence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Closed operator-selected cadence unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaScheduleIntervalUnit {
    /// One through 43,200 minutes.
    Minutes,
    /// One through 720 hours.
    Hours,
}

impl MediaScheduleIntervalUnit {
    /// Canonical persisted unit token.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Minutes => "minutes",
            Self::Hours => "hours",
        }
    }

    /// Maximum selected quantity under approved DISC-1 limits.
    #[must_use]
    pub const fn maximum(self) -> i32 {
        match self {
            Self::Minutes => 43_200,
            Self::Hours => 720,
        }
    }
}

/// Initial cadence configuration for one exact immutable association version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaScheduleConfigurationRequest {
    /// Exact latest association version observed by the operator.
    pub association_version: i32,
    /// Explicit positive cadence quantity; never supplied by a default.
    pub interval_quantity: i32,
    /// Explicit minute or hour unit.
    pub interval_unit: MediaScheduleIntervalUnit,
}

impl MediaScheduleConfigurationRequest {
    /// Validate the version and selected quantity without normalization.
    ///
    /// # Errors
    /// Rejects nonpositive versions or quantities outside the approved envelope.
    pub fn validate(&self) -> Result<(), crate::media_root_contract::RootInputError> {
        if self.association_version <= 0
            || !(1..=self.interval_unit.maximum()).contains(&self.interval_quantity)
        {
            return Err(crate::media_root_contract::RootInputError);
        }
        Ok(())
    }
}

/// Persisted cadence for an exact association; saving does not enable a trigger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MediaScheduleConfigurationResponse {
    /// Stable logical association identity, never a host path.
    pub media_discovery_association_public_id: Uuid,
    /// Exact version owning this configuration.
    pub association_version: i32,
    /// Persisted selected quantity.
    pub interval_quantity: i32,
    /// Persisted selected unit.
    pub interval_unit: MediaScheduleIntervalUnit,
    /// Durable progression anchor, not process-start time.
    pub anchor_due_at: DateTime<Utc>,
    /// Persisted state revision used for conditional edits.
    pub updated_at: DateTime<Utc>,
}

impl MediaScheduleConfigurationResponse {
    /// Strong fence for this complete persisted representation.
    #[must_use]
    pub fn etag(&self) -> String {
        format!(
            "\"media-schedule:{}:v{}:{}\"",
            self.media_discovery_association_public_id,
            self.association_version,
            self.updated_at.timestamp_micros()
        )
    }
}

/// Parse one canonical strong schedule fence.
///
/// # Errors
/// Rejects weak/list/wildcard tags and noncanonical identity/version tokens.
pub fn parse_schedule_etag(
    value: &str,
) -> Result<(Uuid, i32, i64), crate::media_root_contract::RootInputError> {
    let invalid = || crate::media_root_contract::RootInputError;
    let value = value.trim_matches([' ', '\t']);
    let body = value
        .strip_prefix("\"media-schedule:")
        .and_then(|text| text.strip_suffix('"'))
        .ok_or_else(invalid)?;
    let (id, remainder) = body.split_once(":v").ok_or_else(invalid)?;
    let (version, revision) = remainder.split_once(':').ok_or_else(invalid)?;
    let id: Uuid = id.parse().map_err(|_| invalid())?;
    let version: i32 = version.parse().map_err(|_| invalid())?;
    let revision: i64 = revision.parse().map_err(|_| invalid())?;
    if version <= 0
        || value != format!("\"media-schedule:{id}:v{version}:{revision}\"")
        || DateTime::from_timestamp_micros(revision).is_none()
    {
        return Err(invalid());
    }
    Ok((id, version, revision))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacement_fence_rejects_noncanonical_or_weak_tokens()
    -> Result<(), crate::media_root_contract::RootInputError> {
        let valid = "\"media-schedule:00000000-0000-0000-0000-000000000001:v1:42\"";
        assert_eq!(parse_schedule_etag(valid)?, (Uuid::from_u128(1), 1, 42));
        for value in [
            "*".to_owned(),
            format!("W/{valid}"),
            format!("{valid}, {valid}"),
            valid.replace(":v1", ":v01"),
            valid.replace(":v1", ":v0"),
            valid.replace(":42", ":042"),
            valid.replace(":42", ":9223372036854775808"),
        ] {
            assert!(parse_schedule_etag(&value).is_err());
        }
        Ok(())
    }

    #[test]
    fn cadence_requires_explicit_bounded_quantity_and_unit() {
        for unit in [
            MediaScheduleIntervalUnit::Minutes,
            MediaScheduleIntervalUnit::Hours,
        ] {
            for quantity in [0, 1, unit.maximum(), unit.maximum() + 1] {
                let request = MediaScheduleConfigurationRequest {
                    association_version: 1,
                    interval_quantity: quantity,
                    interval_unit: unit,
                };
                assert_eq!(
                    request.validate().is_ok(),
                    (1..=unit.maximum()).contains(&quantity)
                );
            }
        }
    }

    #[test]
    fn cadence_rejects_missing_unknown_duplicate_and_retired_fields() {
        for body in [
            "{}",
            r#"{"association_version":1,"interval_quantity":1}"#,
            r#"{"association_version":1,"interval_unit":"minutes"}"#,
            r#"{"association_version":1,"interval_quantity":1,"interval_unit":"days"}"#,
            r#"{"association_version":1,"interval_quantity":1,"interval_unit":"minutes","source_root":"/media"}"#,
            r#"{"association_version":1,"interval_quantity":1,"interval_quantity":2,"interval_unit":"minutes"}"#,
        ] {
            assert!(serde_json::from_str::<MediaScheduleConfigurationRequest>(body).is_err());
        }
    }
}
