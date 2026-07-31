#[cfg(target_arch = "wasm32")]
use std::collections::HashMap;

#[cfg(target_arch = "wasm32")]
use crate::models::{
    MediaCapabilityReadinessResponse, MediaCapabilitySnapshotResponse,
    MediaCompatibilityTargetResponse, MediaComplianceResponse, MediaDiscoveryPreviewItemResponse,
    MediaJobResponse, MediaPolicyResponse, MediaProfileResponse,
};
use crate::models::{
    MediaJobArtifactResponse, MediaJobCompactAuditResponse, MediaJobOperationResponse,
    MediaJobPlanReasonResponse, MediaJobVerificationCheckResponse, MediaJobViolationResponse,
};
#[cfg(target_arch = "wasm32")]
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq, Default)]
pub(crate) struct MediaJobDiagnostics {
    pub operations: Vec<MediaJobOperationResponse>,
    pub violations: Vec<MediaJobViolationResponse>,
    pub plan_reasons: Vec<MediaJobPlanReasonResponse>,
    pub verification_checks: Vec<MediaJobVerificationCheckResponse>,
    pub artifacts: Vec<MediaJobArtifactResponse>,
    pub compact_audits: Vec<MediaJobCompactAuditResponse>,
}

#[cfg(target_arch = "wasm32")]
#[derive(Clone, PartialEq, Eq, Default)]
pub(crate) struct MediaViewState {
    pub profiles: Vec<MediaProfileResponse>,
    pub jobs: Vec<MediaJobResponse>,
    pub job_diagnostics: HashMap<Uuid, MediaJobDiagnostics>,
    pub readiness: Option<MediaCapabilityReadinessResponse>,
    pub latest_capability: Option<MediaCapabilitySnapshotResponse>,
    pub compliance: Option<MediaComplianceResponse>,
    pub compatibility_targets: Vec<MediaCompatibilityTargetResponse>,
    pub policies: Vec<MediaPolicyResponse>,
    pub yaml_export: Option<String>,
    pub discovery_preview_profile_id: Option<Uuid>,
    pub discovery_preview: Vec<MediaDiscoveryPreviewItemResponse>,
}
