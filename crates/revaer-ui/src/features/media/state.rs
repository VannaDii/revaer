use crate::models::{
    MediaCapabilityReadinessResponse, MediaCapabilitySnapshotResponse, MediaComplianceResponse,
    MediaJobResponse, MediaProfileResponse,
};

#[derive(Clone, PartialEq, Eq, Default)]
pub(crate) struct MediaViewState {
    pub profiles: Vec<MediaProfileResponse>,
    pub jobs: Vec<MediaJobResponse>,
    pub readiness: Option<MediaCapabilityReadinessResponse>,
    pub latest_capability: Option<MediaCapabilitySnapshotResponse>,
    pub compliance: Option<MediaComplianceResponse>,
    pub yaml_export: Option<String>,
}
