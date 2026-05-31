use crate::models::{
    MediaCapabilityReadinessResponse, MediaCapabilitySnapshotResponse, MediaJobResponse,
    MediaProfileResponse,
};

#[derive(Clone, PartialEq, Eq, Default)]
pub(crate) struct MediaViewState {
    pub profiles: Vec<MediaProfileResponse>,
    pub jobs: Vec<MediaJobResponse>,
    pub readiness: Option<MediaCapabilityReadinessResponse>,
    pub latest_capability: Option<MediaCapabilitySnapshotResponse>,
    pub yaml_export: Option<String>,
}
