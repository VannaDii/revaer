//! Explicit cadence drafting and complete server confirmation.

use revaer_api_models::media_schedule::{
    MediaScheduleConfigurationRequest, MediaScheduleConfigurationResponse,
    MediaScheduleIntervalUnit,
};
use uuid::Uuid;

pub(crate) fn request(
    version: i32,
    quantity: &str,
    unit: &str,
) -> Result<MediaScheduleConfigurationRequest, &'static str> {
    let interval_unit = match unit {
        "minutes" => MediaScheduleIntervalUnit::Minutes,
        "hours" => MediaScheduleIntervalUnit::Hours,
        _ => return Err("Select minutes or hours."),
    };
    let request = MediaScheduleConfigurationRequest {
        association_version: version,
        interval_quantity: quantity
            .parse()
            .map_err(|_| "Enter a positive whole-number interval.")?,
        interval_unit,
    };
    request
        .validate()
        .map_err(|_| "Interval must be 1-43200 minutes or 1-720 hours.")?;
    Ok(request)
}

pub(crate) fn confirm(
    id: Uuid,
    request: &MediaScheduleConfigurationRequest,
    response: &MediaScheduleConfigurationResponse,
) -> Result<(), &'static str> {
    if response.media_discovery_association_public_id != id
        || response.association_version != request.association_version
        || response.interval_quantity != request.interval_quantity
        || response.interval_unit != request.interval_unit
    {
        return Err("Schedule confirmation is inconsistent. Reload before resubmitting.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_has_no_default_and_rejects_out_of_range_intervals() {
        for (quantity, unit) in [
            ("", ""),
            ("1", ""),
            ("0", "hours"),
            ("721", "hours"),
            ("43201", "minutes"),
            ("1.5", "minutes"),
        ] {
            assert!(request(1, quantity, unit).is_err());
        }
        assert!(request(1, "720", "hours").is_ok());
        assert!(request(1, "43200", "minutes").is_ok());
        assert!(request(0, "1", "minutes").is_err());
    }

    #[test]
    fn confirmation_requires_exact_identity_version_quantity_and_unit()
    -> Result<(), Box<dyn std::error::Error>> {
        let id = Uuid::from_u128(1);
        let requested = request(1, "2", "hours")?;
        let confirmed: MediaScheduleConfigurationResponse =
            serde_json::from_value(serde_json::json!({
                "media_discovery_association_public_id": id,
                "association_version": 1, "interval_quantity": 2, "interval_unit": "hours",
            "anchor_due_at": "2026-10-02T00:00:00Z", "updated_at": "2026-10-02T00:00:00Z"
            }))?;
        assert!(confirm(id, &requested, &confirmed).is_ok());
        let mut changed = confirmed.clone();
        changed.media_discovery_association_public_id = Uuid::from_u128(2);
        assert!(confirm(id, &requested, &changed).is_err());
        changed = confirmed.clone();
        changed.association_version = 2;
        assert!(confirm(id, &requested, &changed).is_err());
        changed = confirmed.clone();
        changed.interval_quantity = 3;
        assert!(confirm(id, &requested, &changed).is_err());
        changed = confirmed;
        changed.interval_unit = MediaScheduleIntervalUnit::Minutes;
        assert!(confirm(id, &requested, &changed).is_err());
        Ok(())
    }
}
