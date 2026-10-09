//! Explicit cadence representation and native procedure calls.

use revaer_api::models::media_schedule::{
    MediaScheduleConfigurationResponse, MediaScheduleIntervalUnit,
};
use revaer_data::media::schedules::ScheduleConfigurationRow;

use super::{MediaServiceError, MediaServiceErrorKind};

pub(super) fn response(
    row: &ScheduleConfigurationRow,
) -> Result<MediaScheduleConfigurationResponse, MediaServiceError> {
    let unit = match row.interval_unit.as_str() {
        "minutes" => MediaScheduleIntervalUnit::Minutes,
        "hours" => MediaScheduleIntervalUnit::Hours,
        _ => return Err(invalid()),
    };
    if row.association_version <= 0 || !(1..=unit.maximum()).contains(&row.interval_quantity) {
        return Err(invalid());
    }
    Ok(MediaScheduleConfigurationResponse {
        media_discovery_association_public_id: row.media_discovery_association_public_id,
        association_version: row.association_version,
        interval_quantity: row.interval_quantity,
        interval_unit: unit,
        anchor_due_at: row.anchor_due_at,
        updated_at: row.updated_at,
    })
}

fn invalid() -> MediaServiceError {
    tracing::error!("invalid persisted native schedule configuration");
    MediaServiceError::new(MediaServiceErrorKind::Storage)
}
