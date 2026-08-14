#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

#[cfg(any(target_arch = "wasm32", test))]
use crate::models::MediaJobDiagnosticsResponse;
#[cfg(target_arch = "wasm32")]
use crate::models::{
    MediaCapabilityReadinessResponse, MediaCapabilitySnapshotResponse,
    MediaCompatibilityTargetResponse, MediaComplianceResponse, MediaDiscoveryPreviewItemResponse,
    MediaJobResponse, MediaPolicyResponse, MediaProfileResponse,
};
use uuid::Uuid;

#[cfg(any(target_arch = "wasm32", test))]
pub(crate) type MediaJobDiagnostics = MediaJobDiagnosticsResponse;

#[cfg(target_arch = "wasm32")]
#[derive(Clone, PartialEq, Eq)]
pub(crate) enum MediaJobDiagnosticsState {
    Loading { request_id: Uuid },
    Loaded(MediaJobDiagnostics),
    Failed(String),
}

#[cfg(target_arch = "wasm32")]
pub(crate) type MediaJobDiagnosticsMap = HashMap<Uuid, MediaJobDiagnosticsState>;

pub(crate) fn is_current_diagnostics_request(
    active_request_id: Option<Uuid>,
    request_id: Uuid,
) -> bool {
    active_request_id == Some(request_id)
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, PartialEq, Eq, Default)]
pub(crate) struct MediaViewState {
    pub profiles: Vec<MediaProfileResponse>,
    pub jobs: Vec<MediaJobResponse>,
    pub readiness: Option<MediaCapabilityReadinessResponse>,
    pub latest_capability: Option<MediaCapabilitySnapshotResponse>,
    pub compliance: Option<MediaComplianceResponse>,
    pub compatibility_targets: Vec<MediaCompatibilityTargetResponse>,
    pub policies: Vec<MediaPolicyResponse>,
    pub yaml_export: Option<String>,
    pub discovery_preview: Vec<MediaDiscoveryPreviewItemResponse>,
}

#[cfg(test)]
mod tests {
    use super::is_current_diagnostics_request;
    use uuid::Uuid;

    #[test]
    fn loading_state_matches_only_the_active_request() {
        let active_request = Uuid::from_u128(1);
        let stale_request = Uuid::from_u128(2);
        assert!(is_current_diagnostics_request(
            Some(active_request),
            active_request
        ));
        assert!(!is_current_diagnostics_request(
            Some(active_request),
            stale_request
        ));
    }

    #[test]
    fn terminal_states_do_not_accept_request_completions() {
        let request_id = Uuid::from_u128(1);

        assert!(!is_current_diagnostics_request(None, request_id));
    }
}
