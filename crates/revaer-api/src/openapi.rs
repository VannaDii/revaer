//! `OpenAPI` document helpers and dependency wiring.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use revaer_media_core::target::MAX_DESIRED_TARGET_STREAMS;
use serde_json::{Map, Value};
use tracing::error;

use crate::openapi_assets::OPENAPI_EMBEDDED_JSON;

const MEDIA_DISCOVERY_SOURCE_PATHS_MAX_LEN: usize = 1024;
const MEDIA_DISCOVERY_SOURCE_PATH_MAX_BYTES: usize = 4096;
const STALE_MEDIA_SCHEMAS: [&str; 2] = [
    "MediaCapabilityRecordRequest",
    "MediaCapabilityRecordResponse",
];
const STALE_MEDIA_PATHS: [&str; 1] = ["/v1/media/desired-targets"];

type OpenApiPersistFn =
    Arc<dyn Fn(&Path, &Value) -> Result<(), revaer_telemetry::TelemetryError> + Send + Sync>;

pub(crate) struct OpenApiDependencies {
    pub(crate) document: Arc<Value>,
    pub(crate) path: PathBuf,
    pub(crate) persist: OpenApiPersistFn,
}

impl OpenApiDependencies {
    pub(crate) fn new(document: Arc<Value>, path: PathBuf, persist: OpenApiPersistFn) -> Self {
        Self {
            document,
            path,
            persist,
        }
    }

    pub(crate) fn embedded_at(path: &Path) -> Self {
        Self::new(
            Arc::new(build_openapi_document()),
            path.to_path_buf(),
            Arc::new(|destination, document| {
                revaer_telemetry::persist_openapi(destination, document)?;
                Ok(())
            }),
        )
    }
}

pub(crate) fn build_openapi_document() -> Value {
    match serde_json::from_str(OPENAPI_EMBEDDED_JSON) {
        Ok(mut value) => {
            add_media_openapi(&mut value);
            value
        }
        Err(err) => {
            error!(error = %err, "failed to parse embedded OpenAPI document");
            Value::Object(serde_json::Map::new())
        }
    }
}

fn add_media_openapi(document: &mut Value) {
    let Some(root) = document.as_object_mut() else {
        return;
    };

    let paths = root
        .entry("paths")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(paths) = paths.as_object_mut() else {
        return;
    };
    for path in STALE_MEDIA_PATHS {
        paths.remove(path);
    }
    let mut media_paths = media_paths();
    media_paths.sort_by_key(|(path, _)| *path);
    for (path, path_item) in media_paths {
        let mut path_item = path_item;
        sort_openapi_value(&mut path_item);
        paths.insert(path.to_string(), path_item);
    }
    sort_openapi_map(paths);

    let components = root
        .entry("components")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(components) = components.as_object_mut() else {
        return;
    };
    let schemas = components
        .entry("schemas")
        .or_insert_with(|| Value::Object(Map::new()));
    let Some(schemas) = schemas.as_object_mut() else {
        return;
    };
    for name in STALE_MEDIA_SCHEMAS {
        schemas.remove(name);
    }
    let mut media_schemas = media_schemas();
    media_schemas.sort_by_key(|(name, _)| *name);
    for (name, schema) in media_schemas {
        let mut schema = schema;
        sort_openapi_value(&mut schema);
        schemas.insert(name.to_string(), schema);
    }
}

fn sort_openapi_value(value: &mut Value) {
    match value {
        Value::Object(map) => {
            sort_openapi_map(map);
        }
        Value::Array(values) => {
            for child in values {
                sort_openapi_value(child);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

fn sort_openapi_map(map: &mut Map<String, Value>) {
    for child in map.values_mut() {
        sort_openapi_value(child);
    }
    let mut entries = std::mem::take(map).into_iter().collect::<Vec<_>>();
    entries.sort_by(|(left, _), (right, _)| left.cmp(right));
    map.extend(entries);
}

fn media_paths() -> Vec<(&'static str, Value)> {
    let mut paths = Vec::new();
    paths.extend(media_profile_paths());
    paths.extend(media_discovery_paths());
    paths.extend(media_job_paths());
    paths.extend(media_capability_paths());
    paths.extend(media_yaml_paths());
    paths
}

fn media_profile_paths() -> Vec<(&'static str, Value)> {
    let mut paths = Vec::new();
    paths.extend(media_profile_core_paths());
    paths.extend(media_profile_support_paths());
    paths
}

#[derive(Clone, Copy)]
struct MediaOperationSpec {
    method: &'static str,
    summary: &'static str,
    success_status: &'static str,
    success_description: &'static str,
    response_schema: Option<&'static str>,
    request_schema: Option<&'static str>,
    parameters: MediaParameterSet,
}

#[derive(Clone, Copy)]
enum MediaParameterSet {
    None,
    PathUuid(&'static str),
    JobListQuery,
    RecentJobQuery,
    IncludeLocalPaths,
}

impl MediaParameterSet {
    fn values(self) -> Vec<Value> {
        match self {
            Self::None => Vec::new(),
            Self::PathUuid(name) => vec![path_uuid_parameter(name)],
            Self::JobListQuery => vec![
                query_uuid_parameter("media_profile_public_id"),
                query_string_parameter("status"),
            ],
            Self::RecentJobQuery => vec![
                query_uuid_parameter("media_profile_public_id"),
                query_string_parameter("cursor"),
                query_string_parameter("limit"),
            ],
            Self::IncludeLocalPaths => vec![query_bool_parameter("include_local_paths")],
        }
    }
}

const fn media_op(
    method: &'static str,
    summary: &'static str,
    success_status: &'static str,
    success_description: &'static str,
    response_schema: Option<&'static str>,
    request_schema: Option<&'static str>,
    parameters: MediaParameterSet,
) -> MediaOperationSpec {
    MediaOperationSpec {
        method,
        summary,
        success_status,
        success_description,
        response_schema,
        request_schema,
        parameters,
    }
}

fn media_path<const N: usize>(
    path: &'static str,
    operations: [MediaOperationSpec; N],
) -> (&'static str, Value) {
    let mut path_item = Map::new();
    for spec in operations {
        path_item.insert(
            spec.method.to_string(),
            media_operation(
                spec.summary,
                spec.success_status,
                spec.success_description,
                spec.response_schema,
                spec.request_schema,
                spec.parameters.values(),
            ),
        );
    }
    (path, Value::Object(path_item))
}

fn media_collection_path(
    path: &'static str,
    list_summary: &'static str,
    list_description: &'static str,
    list_schema: &'static str,
    write: MediaOperationSpec,
) -> (&'static str, Value) {
    media_path(
        path,
        [
            media_op(
                "get",
                list_summary,
                "200",
                list_description,
                Some(list_schema),
                None,
                MediaParameterSet::None,
            ),
            write,
        ],
    )
}

fn media_uuid_resource_path(
    path: &'static str,
    uuid_parameter: &'static str,
    read_summary: &'static str,
    read_description: &'static str,
    response_schema: &'static str,
    write: MediaOperationSpec,
) -> (&'static str, Value) {
    media_path(
        path,
        [
            media_op(
                "get",
                read_summary,
                "200",
                read_description,
                Some(response_schema),
                None,
                MediaParameterSet::PathUuid(uuid_parameter),
            ),
            MediaOperationSpec {
                parameters: MediaParameterSet::PathUuid(uuid_parameter),
                ..write
            },
        ],
    )
}

fn media_single_path(path: &'static str, operation: MediaOperationSpec) -> (&'static str, Value) {
    media_path(path, [operation])
}

fn media_profile_core_paths() -> Vec<(&'static str, Value)> {
    vec![
        media_collection_path(
            "/v1/media/profiles",
            "List media profiles",
            "Media profile collection",
            "MediaProfileListResponse",
            media_op(
                "post",
                "Create or update a media profile",
                "201",
                "Media profile saved",
                Some("MediaProfileResponse"),
                Some("MediaProfileUpsertRequest"),
                MediaParameterSet::None,
            ),
        ),
        media_uuid_resource_path(
            "/v1/media/profiles/{media_profile_public_id}",
            "media_profile_public_id",
            "Read a media profile",
            "Media profile",
            "MediaProfileResponse",
            media_op(
                "patch",
                "Patch a media profile",
                "200",
                "Media profile",
                Some("MediaProfileResponse"),
                Some("MediaProfilePatchRequest"),
                MediaParameterSet::None,
            ),
        ),
    ]
}

fn media_profile_support_paths() -> Vec<(&'static str, Value)> {
    let mut paths = vec![media_single_path(
        "/v1/media/profiles/validate",
        media_op(
            "post",
            "Validate a media profile",
            "200",
            "Media profile validation",
            Some("MediaProfileValidationResponse"),
            Some("MediaProfileUpsertRequest"),
            MediaParameterSet::None,
        ),
    )];
    paths.extend(media_configuration_paths());
    paths.push(media_single_path(
        "/v1/media/profiles/{media_profile_public_id}/desired-target",
        media_op(
            "patch",
            "Pin or clear a media profile desired target",
            "204",
            "Media profile desired target updated",
            None,
            Some("MediaProfileDesiredTargetRequest"),
            MediaParameterSet::PathUuid("media_profile_public_id"),
        ),
    ));
    paths.push(media_single_path(
        "/v1/media/planning/preview",
        media_op(
            "post",
            "Preview media planning",
            "200",
            "Media planning preview",
            Some("MediaPlanningPreviewResponse"),
            Some("MediaPlanningPreviewRequest"),
            MediaParameterSet::None,
        ),
    ));
    paths
}

fn media_configuration_paths() -> Vec<(&'static str, Value)> {
    media_collection_specs([
        MediaCollectionSpec::post(
            "/v1/media/compatibility-targets",
            "List media compatibility targets",
            "Media compatibility targets",
            "MediaCompatibilityTargetListResponse",
            "Create or replace a media compatibility target",
            "Media compatibility target",
            (
                "MediaCompatibilityTargetResponse",
                "MediaCompatibilityTargetUpsertRequest",
            ),
        ),
        MediaCollectionSpec::post(
            "/v1/media/targets",
            "List immutable media desired targets",
            "Media desired targets",
            "MediaDesiredTargetListResponse",
            "Create an immutable media desired target",
            "Media desired target",
            (
                "MediaDesiredTargetResponse",
                "MediaDesiredTargetCreateRequest",
            ),
        ),
        MediaCollectionSpec::post(
            "/v1/media/policies",
            "List media policies",
            "Media policies",
            "MediaPolicyListResponse",
            "Create or replace a media policy",
            "Media policy",
            ("MediaPolicyResponse", "MediaPolicyUpsertRequest"),
        ),
        MediaCollectionSpec::write(
            "/v1/media/job-retention",
            "Read media job retention policy",
            "Media job retention",
            "MediaJobRetentionResponse",
            media_op(
                "patch",
                "Update media job retention policy",
                "200",
                "Media job retention",
                Some("MediaJobRetentionResponse"),
                Some("MediaJobRetentionUpdateRequest"),
                MediaParameterSet::None,
            ),
        ),
    ])
}

#[derive(Clone, Copy)]
struct MediaCollectionSpec {
    path: &'static str,
    list_summary: &'static str,
    list_description: &'static str,
    list_schema: &'static str,
    write: MediaOperationSpec,
}

impl MediaCollectionSpec {
    const fn write(
        path: &'static str,
        list_summary: &'static str,
        list_description: &'static str,
        list_schema: &'static str,
        write: MediaOperationSpec,
    ) -> Self {
        Self {
            path,
            list_summary,
            list_description,
            list_schema,
            write,
        }
    }

    const fn post(
        path: &'static str,
        list_summary: &'static str,
        list_description: &'static str,
        list_schema: &'static str,
        write_summary: &'static str,
        write_description: &'static str,
        write_schemas: (&'static str, &'static str),
    ) -> Self {
        let (response_schema, request_schema) = write_schemas;
        Self::write(
            path,
            list_summary,
            list_description,
            list_schema,
            media_op(
                "post",
                write_summary,
                "201",
                write_description,
                Some(response_schema),
                Some(request_schema),
                MediaParameterSet::None,
            ),
        )
    }
}

fn media_collection_specs<const N: usize>(
    specs: [MediaCollectionSpec; N],
) -> Vec<(&'static str, Value)> {
    specs
        .into_iter()
        .map(|spec| {
            media_collection_path(
                spec.path,
                spec.list_summary,
                spec.list_description,
                spec.list_schema,
                spec.write,
            )
        })
        .collect()
}

fn media_discovery_paths() -> Vec<(&'static str, Value)> {
    vec![
        media_single_path(
            "/v1/media/discovery/preview",
            media_op(
                "post",
                "Preview media discovery",
                "200",
                "Media discovery preview",
                Some("MediaDiscoveryPreviewResponse"),
                Some("MediaDiscoveryPreviewRequest"),
                MediaParameterSet::None,
            ),
        ),
        media_discovery_run_path(
            "/v1/media/discovery/runs",
            "Run media discovery",
            "Media discovery run",
        ),
        media_discovery_run_collection_path(
            "/v1/media/discovery/schedules",
            "List media discovery schedules",
            "Media discovery schedules",
            "MediaDiscoveryScheduleListResponse",
            "Run scheduled media discovery",
            "Scheduled media discovery run",
        ),
        media_discovery_run_collection_path(
            "/v1/media/discovery/watchers",
            "List media discovery watchers",
            "Media discovery watchers",
            "MediaDiscoveryWatcherListResponse",
            "Run watcher media discovery",
            "Watcher media discovery run",
        ),
    ]
}

fn media_discovery_run_path(
    path: &'static str,
    summary: &'static str,
    description: &'static str,
) -> (&'static str, Value) {
    media_single_path(path, media_discovery_run_op(summary, description))
}

fn media_discovery_run_collection_path(
    path: &'static str,
    list_summary: &'static str,
    list_description: &'static str,
    list_schema: &'static str,
    run_summary: &'static str,
    run_description: &'static str,
) -> (&'static str, Value) {
    media_collection_path(
        path,
        list_summary,
        list_description,
        list_schema,
        media_discovery_run_op(run_summary, run_description),
    )
}

const fn media_discovery_run_op(
    summary: &'static str,
    description: &'static str,
) -> MediaOperationSpec {
    media_op(
        "post",
        summary,
        "201",
        description,
        Some("MediaDiscoveryRunResponse"),
        Some("MediaDiscoveryRunRequest"),
        MediaParameterSet::None,
    )
}

fn media_job_paths() -> Vec<(&'static str, Value)> {
    let mut paths = Vec::new();
    paths.extend(media_job_core_paths());
    paths.extend(media_job_lifecycle_paths());
    paths.extend(media_job_record_paths());
    paths
}

fn media_job_core_paths() -> Vec<(&'static str, Value)> {
    vec![
        media_path(
            "/v1/media/jobs",
            [
                media_op(
                    "get",
                    "List media jobs",
                    "200",
                    "Media job collection",
                    Some("MediaJobListResponse"),
                    None,
                    MediaParameterSet::JobListQuery,
                ),
                media_op(
                    "post",
                    "Create a media job",
                    "201",
                    "Media job created",
                    Some("MediaJobCreateResponse"),
                    Some("MediaJobCreateRequest"),
                    MediaParameterSet::None,
                ),
            ],
        ),
        media_single_path(
            "/v1/media/jobs/{media_job_public_id}",
            media_op(
                "get",
                "Read a media job",
                "200",
                "Media job",
                Some("MediaJobResponse"),
                None,
                MediaParameterSet::PathUuid("media_job_public_id"),
            ),
        ),
        media_single_path(
            "/v1/media/jobs/recent",
            media_op(
                "get",
                "List recent media jobs",
                "200",
                "Recent media job page",
                Some("MediaRecentJobPageResponse"),
                None,
                MediaParameterSet::RecentJobQuery,
            ),
        ),
        media_single_path(
            "/v1/media/jobs/{media_job_public_id}/diagnostics",
            media_op(
                "get",
                "Read media job diagnostics",
                "200",
                "Media job diagnostics",
                Some("MediaJobDiagnosticsResponse"),
                None,
                MediaParameterSet::PathUuid("media_job_public_id"),
            ),
        ),
    ]
}

fn media_job_lifecycle_paths() -> Vec<(&'static str, Value)> {
    vec![
        media_single_path(
            "/v1/media/jobs/{media_job_public_id}/cancel",
            media_op(
                "post",
                "Cancel a media job",
                "204",
                "Media job cancelled",
                None,
                None,
                MediaParameterSet::PathUuid("media_job_public_id"),
            ),
        ),
        media_single_path(
            "/v1/media/jobs/{media_job_public_id}/retry",
            media_op(
                "post",
                "Retry a media job",
                "204",
                "Media job retried",
                None,
                None,
                MediaParameterSet::PathUuid("media_job_public_id"),
            ),
        ),
        media_path(
            "/v1/media/jobs/{media_job_public_id}/phases",
            [
                media_op(
                    "get",
                    "List media job phases",
                    "200",
                    "Media job phases",
                    Some("MediaJobPhaseListResponse"),
                    None,
                    MediaParameterSet::PathUuid("media_job_public_id"),
                ),
                media_op(
                    "post",
                    "Append a media job phase",
                    "204",
                    "Media job phase appended",
                    None,
                    Some("MediaJobPhaseAppendRequest"),
                    MediaParameterSet::PathUuid("media_job_public_id"),
                ),
            ],
        ),
    ]
}

macro_rules! media_job_record_path_item {
    (operations) => {
        media_job_record_path(
            "/v1/media/jobs/{media_job_public_id}/operations",
            "List media job operations",
            "Media job operations",
            "MediaJobOperationListResponse",
        )
    };
    (violations) => {
        media_job_record_path(
            "/v1/media/jobs/{media_job_public_id}/violations",
            "List media job violations",
            "Media job violations",
            "MediaJobViolationListResponse",
        )
    };
    (plan_reasons) => {
        media_job_record_path(
            "/v1/media/jobs/{media_job_public_id}/plan-reasons",
            "List media job plan reasons",
            "Media job plan reasons",
            "MediaJobPlanReasonListResponse",
        )
    };
    (verification_checks) => {
        media_job_record_path(
            "/v1/media/jobs/{media_job_public_id}/verification-checks",
            "List media job verification checks",
            "Media job verification checks",
            "MediaJobVerificationCheckListResponse",
        )
    };
    (artifacts) => {
        media_job_record_path(
            "/v1/media/jobs/{media_job_public_id}/artifacts",
            "List media job artifacts",
            "Media job artifacts",
            "MediaJobArtifactListResponse",
        )
    };
    (compact_audits) => {
        media_job_record_path(
            "/v1/media/jobs/{media_job_public_id}/compact-audits",
            "List media job compact audits",
            "Media job compact audits",
            "MediaJobCompactAuditListResponse",
        )
    };
}

fn media_job_record_paths() -> Vec<(&'static str, Value)> {
    vec![
        media_job_record_path_item!(operations),
        media_job_record_path_item!(violations),
        media_job_record_path_item!(plan_reasons),
        media_job_record_path_item!(verification_checks),
        media_job_record_path_item!(artifacts),
        media_job_record_path_item!(compact_audits),
    ]
}

fn media_job_record_path(
    path: &'static str,
    list_summary: &'static str,
    list_description: &'static str,
    list_schema: &'static str,
) -> (&'static str, Value) {
    media_single_path(
        path,
        media_op(
            "get",
            list_summary,
            "200",
            list_description,
            Some(list_schema),
            None,
            MediaParameterSet::PathUuid("media_job_public_id"),
        ),
    )
}

macro_rules! media_capability_path_item {
    ($method:literal, $path:literal, $summary:literal, $status:literal, $description:literal, $schema:literal) => {
        media_single_path(
            $path,
            media_op(
                $method,
                $summary,
                $status,
                $description,
                Some($schema),
                None,
                MediaParameterSet::None,
            ),
        )
    };
}

fn media_capability_paths() -> Vec<(&'static str, Value)> {
    vec![
        media_capability_path_item!(
            "get",
            "/v1/media/capabilities",
            "Read latest media capability snapshot",
            "200",
            "Latest media capability snapshot",
            "MediaCapabilityLatestResponse"
        ),
        media_capability_path_item!(
            "get",
            "/v1/media/capabilities/readiness",
            "Read media capability readiness",
            "200",
            "Media capability readiness",
            "MediaCapabilityReadinessResponse"
        ),
        media_capability_path_item!(
            "post",
            "/v1/media/capabilities/refresh",
            "Refresh media capabilities",
            "201",
            "Media capability snapshot refreshed",
            "MediaCapabilityRefreshResponse"
        ),
        media_capability_path_item!(
            "get",
            "/v1/media/compliance",
            "Read media runtime compliance artifacts",
            "200",
            "Media runtime compliance artifacts",
            "MediaComplianceResponse"
        ),
    ]
}

fn media_yaml_paths() -> Vec<(&'static str, Value)> {
    vec![
        media_single_path(
            "/v1/media/export",
            media_op(
                "get",
                "Export media profile YAML",
                "200",
                "Media YAML export",
                Some("MediaYamlExportResponse"),
                None,
                MediaParameterSet::IncludeLocalPaths,
            ),
        ),
        media_single_path(
            "/v1/media/imports/validate",
            media_op(
                "post",
                "Validate media profile YAML",
                "200",
                "Media YAML validation result",
                Some("MediaYamlValidationResponse"),
                Some("MediaYamlImportRequest"),
                MediaParameterSet::None,
            ),
        ),
        media_single_path(
            "/v1/media/imports/apply",
            media_op(
                "post",
                "Apply media profile YAML",
                "201",
                "Media YAML apply result",
                Some("MediaYamlApplyResponse"),
                Some("MediaYamlImportRequest"),
                MediaParameterSet::None,
            ),
        ),
    ]
}

fn media_operation(
    summary: &'static str,
    success_status: &'static str,
    success_description: &'static str,
    response_schema: Option<&'static str>,
    request_schema: Option<&'static str>,
    parameters: Vec<Value>,
) -> Value {
    let mut operation = Map::new();
    operation.insert("summary".to_string(), Value::String(summary.to_string()));
    operation.insert(
        "security".to_string(),
        serde_json::json!([{ "ApiKeyAuth": [] }]),
    );
    if !parameters.is_empty() {
        operation.insert("parameters".to_string(), Value::Array(parameters));
    }
    if let Some(schema) = request_schema {
        operation.insert(
            "requestBody".to_string(),
            serde_json::json!({
                "required": true,
                "content": {
                    "application/json": {
                        "schema": schema_ref(schema)
                    }
                }
            }),
        );
    }
    operation.insert(
        "responses".to_string(),
        media_responses(success_status, success_description, response_schema),
    );
    Value::Object(operation)
}

fn media_responses(
    success_status: &'static str,
    success_description: &'static str,
    response_schema: Option<&'static str>,
) -> Value {
    let mut responses = Map::new();
    responses.insert(
        success_status.to_string(),
        response(success_description, response_schema),
    );
    for (status, description) in [
        ("400", "Invalid media request"),
        ("401", "Authentication failed"),
        ("404", "Media resource not found"),
        ("409", "Media workflow conflict"),
        ("429", "Rate limit exceeded"),
        ("500", "Server error"),
        ("503", "Media workflow unavailable"),
    ] {
        responses.insert(
            status.to_string(),
            response(description, Some("ProblemDetails")),
        );
    }
    Value::Object(responses)
}

fn response(description: &'static str, schema: Option<&'static str>) -> Value {
    schema.map_or_else(
        || serde_json::json!({ "description": description }),
        |schema| {
            serde_json::json!({
            "description": description,
            "content": {
                "application/json": {
                    "schema": schema_ref(schema)
                }
            }
            })
        },
    )
}

fn path_uuid_parameter(name: &'static str) -> Value {
    serde_json::json!({
        "in": "path",
        "name": name,
        "required": true,
        "schema": uuid_schema()
    })
}

fn query_uuid_parameter(name: &'static str) -> Value {
    serde_json::json!({
        "in": "query",
        "name": name,
        "schema": uuid_schema()
    })
}

fn query_string_parameter(name: &'static str) -> Value {
    serde_json::json!({
        "in": "query",
        "name": name,
        "schema": string_schema()
    })
}

fn query_bool_parameter(name: &'static str) -> Value {
    serde_json::json!({
        "in": "query",
        "name": name,
        "schema": bool_schema()
    })
}

fn media_schemas() -> Vec<(&'static str, Value)> {
    let mut schemas = Vec::new();
    schemas.extend(media_profile_schemas());
    schemas.extend(media_discovery_schemas());
    schemas.extend(media_job_schemas());
    schemas.extend(media_capability_schemas());
    schemas.extend(media_yaml_schemas());
    schemas
}

fn media_profile_schemas() -> Vec<(&'static str, Value)> {
    let mut schemas = Vec::new();
    schemas.extend(media_profile_core_schemas());
    schemas.extend(media_profile_support_schemas());
    schemas
}

fn media_profile_core_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaProfileUpsertRequest",
            object_schema(
                &[
                    "profile_key",
                    "source_root",
                    "output_root",
                    "dry_run_only",
                    "retention_days",
                ],
                [
                    ("profile_key", string_schema()),
                    ("source_root", string_schema()),
                    ("output_root", string_schema()),
                    ("dry_run_only", bool_schema()),
                    ("retention_days", integer_schema()),
                    ("compatibility_target_key", string_schema()),
                    ("policy_key", string_schema()),
                    ("watcher_enabled", bool_schema()),
                    ("schedule_enabled", bool_schema()),
                    ("schedule_interval_minutes", integer_schema()),
                ],
            ),
        ),
        (
            "MediaProfilePatchRequest",
            object_schema(
                &[],
                [
                    ("source_root", string_schema()),
                    ("output_root", string_schema()),
                    ("dry_run_only", bool_schema()),
                    ("retention_days", integer_schema()),
                    ("compatibility_target_key", string_schema()),
                    ("policy_key", string_schema()),
                    ("watcher_enabled", bool_schema()),
                    ("schedule_enabled", bool_schema()),
                    ("schedule_interval_minutes", integer_schema()),
                ],
            ),
        ),
        ("MediaProfileResponse", media_profile_response_schema()),
        (
            "MediaProfileListResponse",
            object_schema(
                &["profiles"],
                [("profiles", array_ref_schema("MediaProfileResponse"))],
            ),
        ),
    ]
}

fn media_profile_support_schemas() -> Vec<(&'static str, Value)> {
    let mut schemas = vec![(
        "MediaProfileValidationResponse",
        object_schema(
            &["valid", "issues"],
            [("valid", bool_schema()), ("issues", array_string_schema())],
        ),
    )];
    schemas.extend(media_configuration_schemas());
    schemas.extend([
        (
            "MediaPlanningPreviewRequest",
            object_schema(
                &["media_profile_public_id", "source_path"],
                [
                    ("media_profile_public_id", uuid_schema()),
                    ("source_path", string_schema()),
                ],
            ),
        ),
        (
            "MediaPlanningPreviewResponse",
            object_schema(
                &["accepted", "source_path", "dry_run"],
                [
                    ("accepted", bool_schema()),
                    ("source_path", string_schema()),
                    ("output_path", string_schema()),
                    ("reason", string_schema()),
                    ("dry_run", bool_schema()),
                ],
            ),
        ),
    ]);
    schemas
}

fn media_configuration_schemas() -> Vec<(&'static str, Value)> {
    let mut schemas = media_compatibility_target_schemas();
    schemas.extend(media_desired_target_schemas());
    schemas.extend(media_policy_profile_schemas());
    schemas.extend(media_job_retention_schemas());
    schemas
}

fn media_desired_target_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaDesiredTargetStream",
            object_schema(
                &["stream_key", "stream_kind", "sort_order", "codec"],
                [
                    ("stream_key", string_schema()),
                    ("stream_kind", string_schema()),
                    ("semantic_role", string_schema()),
                    ("language_code", string_schema()),
                    ("optional", bool_schema()),
                    ("sort_order", integer_schema()),
                    ("codec", string_schema()),
                    ("channel_count", integer_schema()),
                    ("channel_layout", string_schema()),
                    ("audio_bitrate_bps", integer_schema()),
                    ("audio_sample_rate_hz", integer_schema()),
                    ("video_profile", string_schema()),
                    ("video_level", string_schema()),
                    ("video_bitrate_bps", integer_schema()),
                    ("color_primaries", string_schema()),
                    ("color_transfer", string_schema()),
                    ("color_space", string_schema()),
                    ("hdr_format", string_schema()),
                    ("title", string_schema()),
                    ("default_disposition", bool_schema()),
                    ("forced_disposition", bool_schema()),
                    ("subtitle_placement", string_schema()),
                    ("image_subtitle_action", string_schema()),
                ],
            ),
        ),
        (
            "MediaDesiredTargetCreateRequest",
            object_schema(
                &[
                    "target_key",
                    "version",
                    "display_name",
                    "container_format",
                    "streams",
                ],
                [
                    ("target_key", string_schema()),
                    ("version", integer_schema()),
                    ("display_name", string_schema()),
                    ("container_format", string_schema()),
                    (
                        "streams",
                        array_ref_items_schema(
                            "MediaDesiredTargetStream",
                            1,
                            MAX_DESIRED_TARGET_STREAMS,
                        ),
                    ),
                ],
            ),
        ),
        (
            "MediaDesiredTargetResponse",
            object_schema(
                &[
                    "media_desired_target_profile_public_id",
                    "target_key",
                    "version",
                    "display_name",
                    "container_format",
                    "streams",
                ],
                [
                    ("media_desired_target_profile_public_id", uuid_schema()),
                    ("target_key", string_schema()),
                    ("version", integer_schema()),
                    ("display_name", string_schema()),
                    ("container_format", string_schema()),
                    ("streams", array_ref_schema("MediaDesiredTargetStream")),
                ],
            ),
        ),
        (
            "MediaDesiredTargetListResponse",
            object_schema(
                &["targets"],
                [("targets", array_ref_schema("MediaDesiredTargetResponse"))],
            ),
        ),
        (
            "MediaProfileDesiredTargetRequest",
            object_schema(
                &[],
                [
                    ("target_key", string_schema()),
                    ("version", integer_schema()),
                ],
            ),
        ),
    ]
}

fn media_compatibility_target_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaCompatibilityTargetResponse",
            media_compatibility_target_object_schema(),
        ),
        (
            "MediaCompatibilityTargetListResponse",
            object_schema(
                &["targets"],
                [(
                    "targets",
                    array_ref_schema("MediaCompatibilityTargetResponse"),
                )],
            ),
        ),
        (
            "MediaCompatibilityTargetUpsertRequest",
            media_compatibility_target_object_schema(),
        ),
    ]
}

fn media_compatibility_target_object_schema() -> Value {
    object_schema(
        &[
            "compatibility_target_key",
            "version",
            "display_name",
            "video_codec",
            "audio_codec",
            "subtitle_policy",
        ],
        [
            ("compatibility_target_key", string_schema()),
            ("version", integer_schema()),
            ("display_name", string_schema()),
            ("video_codec", string_schema()),
            ("audio_codec", string_schema()),
            ("audio_channels", integer_schema()),
            ("audio_channel_layout", string_schema()),
            ("subtitle_policy", string_schema()),
        ],
    )
}

fn media_policy_profile_schemas() -> Vec<(&'static str, Value)> {
    vec![
        ("MediaPolicyResponse", media_policy_object_schema()),
        (
            "MediaPolicyListResponse",
            object_schema(
                &["policies"],
                [("policies", array_ref_schema("MediaPolicyResponse"))],
            ),
        ),
        ("MediaPolicyUpsertRequest", media_policy_object_schema()),
    ]
}

fn media_policy_object_schema() -> Value {
    object_schema(
        &[
            "policy_key",
            "version",
            "display_name",
            "video_intent",
            "verification_strictness",
            "verification_duration_tolerance_millis",
            "verification_mux_validation",
            "verification_decode_all_streams",
            "verification_keyframe_seek",
            "verification_playback_probe",
        ],
        [
            ("policy_key", string_schema()),
            ("version", integer_schema()),
            ("display_name", string_schema()),
            ("video_intent", string_schema()),
            ("verification_strictness", verification_strictness_schema()),
            (
                "verification_duration_tolerance_millis",
                verification_duration_tolerance_schema(),
            ),
            ("verification_mux_validation", bool_schema()),
            ("verification_decode_all_streams", bool_schema()),
            ("verification_keyframe_seek", bool_schema()),
            ("verification_playback_probe", bool_schema()),
        ],
    )
}

fn verification_strictness_schema() -> Value {
    serde_json::json!({
        "type": "string",
        "enum": ["strict", "balanced", "fast"]
    })
}

fn verification_duration_tolerance_schema() -> Value {
    serde_json::json!({
        "type": "integer",
        "format": "int64",
        "minimum": 0,
        "maximum": 60000
    })
}

fn media_job_retention_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaJobRetentionResponse",
            media_job_retention_object_schema(),
        ),
        (
            "MediaJobRetentionUpdateRequest",
            media_job_retention_object_schema(),
        ),
    ]
}

fn media_job_retention_object_schema() -> Value {
    object_schema(
        &[
            "completed_enabled",
            "completed_mode",
            "completed_limit",
            "failed_diagnostic_enabled",
            "failed_diagnostic_mode",
            "failed_diagnostic_limit",
        ],
        [
            ("completed_enabled", bool_schema()),
            ("completed_mode", retention_mode_schema()),
            ("completed_limit", integer_schema()),
            ("failed_diagnostic_enabled", bool_schema()),
            ("failed_diagnostic_mode", retention_mode_schema()),
            ("failed_diagnostic_limit", integer_schema()),
        ],
    )
}

fn retention_mode_schema() -> Value {
    serde_json::json!({
        "type": "string",
        "enum": ["age", "count"]
    })
}

fn media_discovery_schemas() -> Vec<(&'static str, Value)> {
    let mut schemas = Vec::new();
    schemas.extend(media_discovery_preview_schemas());
    schemas.extend(media_discovery_run_schemas());
    schemas.extend(media_discovery_schedule_schemas());
    schemas.extend(media_discovery_watcher_schemas());
    schemas
}

fn media_discovery_preview_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaDiscoveryPreviewRequest",
            object_schema(
                &["media_profile_public_id", "source_paths"],
                [
                    ("media_profile_public_id", uuid_schema()),
                    ("source_paths", media_discovery_source_paths_schema()),
                ],
            ),
        ),
        (
            "MediaDiscoveryPreviewItemResponse",
            object_schema(
                &["source_path", "dry_run", "accepted"],
                [
                    ("source_path", string_schema()),
                    ("output_path", string_schema()),
                    ("dry_run", bool_schema()),
                    ("accepted", bool_schema()),
                    ("reason", string_schema()),
                ],
            ),
        ),
        (
            "MediaDiscoveryPreviewResponse",
            object_schema(
                &["previews"],
                [(
                    "previews",
                    array_ref_schema("MediaDiscoveryPreviewItemResponse"),
                )],
            ),
        ),
    ]
}

fn media_discovery_run_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaDiscoveryRunRequest",
            object_schema(
                &["media_profile_public_id", "source_paths"],
                [
                    ("media_profile_public_id", uuid_schema()),
                    ("source_paths", media_discovery_source_paths_schema()),
                ],
            ),
        ),
        (
            "MediaDiscoveryQueuedJobResponse",
            object_schema(
                &[
                    "media_job_public_id",
                    "source_path",
                    "output_path",
                    "dry_run",
                ],
                [
                    ("media_job_public_id", uuid_schema()),
                    ("source_path", string_schema()),
                    ("output_path", string_schema()),
                    ("dry_run", bool_schema()),
                ],
            ),
        ),
        (
            "MediaDiscoverySkippedItemResponse",
            object_schema(
                &["source_path"],
                [
                    ("source_path", string_schema()),
                    ("reason", string_schema()),
                ],
            ),
        ),
        (
            "MediaDiscoveryRunResponse",
            object_schema(
                &["queued_jobs", "skipped"],
                [
                    (
                        "queued_jobs",
                        array_ref_schema("MediaDiscoveryQueuedJobResponse"),
                    ),
                    (
                        "skipped",
                        array_ref_schema("MediaDiscoverySkippedItemResponse"),
                    ),
                ],
            ),
        ),
    ]
}

fn media_discovery_schedule_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaDiscoveryScheduleResponse",
            object_schema(
                &[
                    "media_profile_public_id",
                    "profile_key",
                    "source_root",
                    "enabled",
                    "dry_run",
                ],
                [
                    ("media_profile_public_id", uuid_schema()),
                    ("profile_key", string_schema()),
                    ("source_root", string_schema()),
                    ("enabled", bool_schema()),
                    ("interval_minutes", integer_schema()),
                    ("dry_run", bool_schema()),
                ],
            ),
        ),
        (
            "MediaDiscoveryScheduleListResponse",
            object_schema(
                &["schedules"],
                [(
                    "schedules",
                    array_ref_schema("MediaDiscoveryScheduleResponse"),
                )],
            ),
        ),
    ]
}

fn media_discovery_watcher_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaDiscoveryWatcherResponse",
            object_schema(
                &[
                    "media_profile_public_id",
                    "profile_key",
                    "source_root",
                    "enabled",
                    "dry_run",
                ],
                [
                    ("media_profile_public_id", uuid_schema()),
                    ("profile_key", string_schema()),
                    ("source_root", string_schema()),
                    ("enabled", bool_schema()),
                    ("dry_run", bool_schema()),
                ],
            ),
        ),
        (
            "MediaDiscoveryWatcherListResponse",
            object_schema(
                &["watchers"],
                [(
                    "watchers",
                    array_ref_schema("MediaDiscoveryWatcherResponse"),
                )],
            ),
        ),
    ]
}

fn media_job_schemas() -> Vec<(&'static str, Value)> {
    let mut schemas = Vec::new();
    schemas.extend(media_job_core_schemas());
    schemas.extend(media_job_recent_schemas());
    schemas.extend(media_job_phase_schemas());
    schemas.extend(media_job_operation_schemas());
    schemas.extend(media_job_violation_schemas());
    schemas.extend(media_job_plan_reason_schemas());
    schemas.extend(media_job_verification_check_schemas());
    schemas.extend(media_job_artifact_schemas());
    schemas.extend(media_job_compact_audit_schemas());
    schemas
}

fn media_job_core_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaJobCreateRequest",
            object_schema(
                &["media_profile_public_id", "source_path", "dry_run"],
                [
                    ("media_profile_public_id", uuid_schema()),
                    ("source_path", string_schema()),
                    ("output_path", string_schema()),
                    ("dry_run", bool_schema()),
                    ("replace_confirmation", string_schema()),
                ],
            ),
        ),
        (
            "MediaJobCreateResponse",
            object_schema(
                &["media_job_public_id"],
                [("media_job_public_id", uuid_schema())],
            ),
        ),
        ("MediaJobResponse", media_job_response_schema()),
        (
            "MediaJobListResponse",
            object_schema(&["jobs"], [("jobs", array_ref_schema("MediaJobResponse"))]),
        ),
    ]
}

fn media_job_recent_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaRecentJobPageResponse",
            object_schema(
                &["jobs"],
                [
                    ("jobs", array_ref_schema("MediaRecentJobSummaryResponse")),
                    ("next_cursor", string_schema()),
                ],
            ),
        ),
        (
            "MediaRecentJobSummaryResponse",
            object_schema(
                &[
                    "media_job_public_id",
                    "media_profile_public_id",
                    "diagnostic_counts",
                ],
                [
                    ("media_job_public_id", uuid_schema()),
                    ("media_profile_public_id", uuid_schema()),
                    ("diagnostic_counts", schema_ref("MediaJobDiagnosticCounts")),
                ],
            ),
        ),
        (
            "MediaJobDiagnosticCounts",
            object_schema(
                &[
                    "operations",
                    "violations",
                    "plan_reasons",
                    "verification_checks",
                    "artifacts",
                    "compact_audits",
                ],
                [
                    ("operations", integer_schema()),
                    ("violations", integer_schema()),
                    ("plan_reasons", integer_schema()),
                    ("verification_checks", integer_schema()),
                    ("artifacts", integer_schema()),
                    ("compact_audits", integer_schema()),
                ],
            ),
        ),
        (
            "MediaJobDiagnosticsResponse",
            object_schema(
                &[
                    "operations",
                    "violations",
                    "plan_reasons",
                    "verification_checks",
                    "artifacts",
                    "compact_audits",
                ],
                [
                    ("operations", array_ref_schema("MediaJobOperationResponse")),
                    ("violations", array_ref_schema("MediaJobViolationResponse")),
                    (
                        "plan_reasons",
                        array_ref_schema("MediaJobPlanReasonResponse"),
                    ),
                    (
                        "verification_checks",
                        array_ref_schema("MediaJobVerificationCheckResponse"),
                    ),
                    ("artifacts", array_ref_schema("MediaJobArtifactResponse")),
                    (
                        "compact_audits",
                        array_ref_schema("MediaJobCompactAuditResponse"),
                    ),
                ],
            ),
        ),
    ]
}

fn media_job_phase_schemas() -> Vec<(&'static str, Value)> {
    media_job_append_record_schemas(
        "MediaJobPhaseAppendRequest",
        "MediaJobPhaseResponse",
        "MediaJobPhaseListResponse",
        "phases",
        &["phase_index", "phase_name", "phase_status"],
        [
            schema_property("phase_index", SchemaKind::Integer),
            schema_property("phase_name", SchemaKind::String),
            schema_property("phase_status", SchemaKind::MediaStatus),
            schema_property("details_text", SchemaKind::String),
        ],
    )
}

fn media_job_operation_schemas() -> Vec<(&'static str, Value)> {
    media_job_append_record_schemas(
        "MediaJobOperationAppendRequest",
        "MediaJobOperationResponse",
        "MediaJobOperationListResponse",
        "operations",
        &["operation_index", "operation_kind", "command_bin"],
        [
            schema_property("operation_index", SchemaKind::Integer),
            schema_property("operation_kind", SchemaKind::OperationKind),
            schema_property("stream_id", SchemaKind::Integer),
            schema_property("command_bin", SchemaKind::String),
            schema_property("arg_1", SchemaKind::String),
            schema_property("arg_2", SchemaKind::String),
            schema_property("arg_3", SchemaKind::String),
            schema_property("arg_4", SchemaKind::String),
            schema_property("arg_5", SchemaKind::String),
        ],
    )
}

fn media_job_violation_schemas() -> Vec<(&'static str, Value)> {
    media_job_append_record_schemas(
        "MediaJobViolationAppendRequest",
        "MediaJobViolationResponse",
        "MediaJobViolationListResponse",
        "violations",
        &["violation_index", "violation_kind", "severity"],
        [
            schema_property("violation_index", SchemaKind::Integer),
            schema_property("violation_kind", SchemaKind::String),
            schema_property("severity", SchemaKind::ViolationSeverity),
            schema_property("stream_id", SchemaKind::Integer),
        ],
    )
}

fn media_job_plan_reason_schemas() -> Vec<(&'static str, Value)> {
    media_job_append_record_schemas(
        "MediaJobPlanReasonAppendRequest",
        "MediaJobPlanReasonResponse",
        "MediaJobPlanReasonListResponse",
        "reasons",
        &["reason_index", "selected", "reason_code", "reason_text"],
        [
            schema_property("reason_index", SchemaKind::Integer),
            schema_property("candidate_index", SchemaKind::Integer),
            schema_property("selected", SchemaKind::Boolean),
            schema_property("reason_code", SchemaKind::String),
            schema_property("reason_text", SchemaKind::String),
        ],
    )
}

fn media_job_verification_check_schemas() -> Vec<(&'static str, Value)> {
    media_job_append_record_schemas(
        "MediaJobVerificationCheckAppendRequest",
        "MediaJobVerificationCheckResponse",
        "MediaJobVerificationCheckListResponse",
        "checks",
        &["check_index", "check_kind", "check_status"],
        [
            schema_property("check_index", SchemaKind::Integer),
            schema_property("check_kind", SchemaKind::String),
            schema_property("check_status", SchemaKind::VerificationStatus),
            schema_property("expected_value", SchemaKind::String),
            schema_property("actual_value", SchemaKind::String),
            schema_property("details_text", SchemaKind::String),
        ],
    )
}

fn media_job_artifact_schemas() -> Vec<(&'static str, Value)> {
    media_job_append_record_schemas(
        "MediaJobArtifactAppendRequest",
        "MediaJobArtifactResponse",
        "MediaJobArtifactListResponse",
        "artifacts",
        &["artifact_index", "artifact_kind", "artifact_path"],
        [
            schema_property("artifact_index", SchemaKind::Integer),
            schema_property("artifact_kind", SchemaKind::String),
            schema_property("artifact_path", SchemaKind::String),
            schema_property("size_bytes", SchemaKind::Integer),
            schema_property("content_type", SchemaKind::String),
        ],
    )
}

fn media_job_compact_audit_schemas() -> Vec<(&'static str, Value)> {
    media_job_append_record_schemas(
        "MediaJobCompactAuditAppendRequest",
        "MediaJobCompactAuditResponse",
        "MediaJobCompactAuditListResponse",
        "audits",
        &["audit_index", "fact_kind", "fact_text"],
        [
            schema_property("audit_index", SchemaKind::Integer),
            schema_property("fact_kind", SchemaKind::String),
            schema_property("fact_text", SchemaKind::String),
        ],
    )
}

fn media_capability_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaCapabilityRefreshResponse",
            capability_id_response_schema(),
        ),
        (
            "MediaCapabilityCodecResponse",
            media_capability_codec_response_schema(),
        ),
        (
            "MediaCapabilityFeatureResponse",
            media_capability_feature_response_schema(),
        ),
        (
            "MediaCapabilitySnapshotResponse",
            media_capability_snapshot_response_schema(),
        ),
        (
            "MediaCapabilityLatestResponse",
            object_schema(
                &[],
                [("snapshot", schema_ref("MediaCapabilitySnapshotResponse"))],
            ),
        ),
        (
            "MediaCapabilityReadinessResponse",
            object_schema(
                &["ready"],
                [
                    ("ready", bool_schema()),
                    ("reason", string_schema()),
                    ("snapshot", schema_ref("MediaCapabilitySnapshotResponse")),
                ],
            ),
        ),
        (
            "MediaComplianceResponse",
            object_schema(
                &[
                    "license_mode",
                    "image_license_mode",
                    "ffmpeg_license_mode",
                    "ffmpeg_enable_gpl",
                    "ffmpeg_enable_version3",
                    "ffmpeg_enable_nonfree",
                    "source_offer_path",
                    "source_offer_url",
                    "third_party_notices_path",
                    "third_party_notices_url",
                    "sbom_path",
                    "sbom_url",
                    "inventory_path",
                    "exiftool_exception_path",
                    "source_compliance_bundle_digest",
                    "source_compliance_bundle_path",
                    "license_excluded_capabilities",
                    "absent_license_excluded_capabilities",
                ],
                [
                    ("license_mode", string_schema()),
                    ("image_license_mode", string_schema()),
                    ("ffmpeg_license_mode", string_schema()),
                    ("ffmpeg_enable_gpl", bool_schema()),
                    ("ffmpeg_enable_version3", bool_schema()),
                    ("ffmpeg_enable_nonfree", bool_schema()),
                    ("source_offer_path", string_schema()),
                    ("source_offer_url", string_schema()),
                    ("third_party_notices_path", string_schema()),
                    ("third_party_notices_url", string_schema()),
                    ("sbom_path", string_schema()),
                    ("sbom_url", string_schema()),
                    ("inventory_path", string_schema()),
                    ("exiftool_exception_path", string_schema()),
                    ("source_compliance_bundle_digest", string_schema()),
                    ("source_compliance_bundle_path", string_schema()),
                    ("license_excluded_capabilities", array_string_schema()),
                    (
                        "absent_license_excluded_capabilities",
                        array_string_schema(),
                    ),
                ],
            ),
        ),
    ]
}

fn media_capability_codec_response_schema() -> Value {
    object_schema(
        &["codec_name", "encode_supported", "decode_supported"],
        [
            ("codec_name", string_schema()),
            ("encode_supported", bool_schema()),
            ("decode_supported", bool_schema()),
        ],
    )
}

fn media_capability_feature_response_schema() -> Value {
    object_schema(
        &["feature_family", "feature_name", "supported"],
        [
            ("feature_family", string_schema()),
            ("feature_name", string_schema()),
            ("supported", bool_schema()),
            ("detail_text", string_schema()),
        ],
    )
}

fn media_capability_snapshot_response_schema() -> Value {
    object_schema(
        &[
            "media_capability_snapshot_id",
            "snapshot_run_public_id",
            "ffmpeg_version",
            "ffprobe_version",
            "codecs",
            "encoders",
            "decoders",
            "muxers",
            "demuxers",
            "subtitle_support",
            "hardware_accelerators",
            "filesystem_utilities",
            "utility_capabilities",
            "license_mode",
            "ffmpeg_license_mode",
            "ffmpeg_enable_gpl",
            "ffmpeg_enable_version3",
            "ffmpeg_enable_nonfree",
            "compliance_links",
            "absent_capabilities",
            "features",
            "observed_at",
        ],
        [
            ("media_capability_snapshot_id", integer_schema()),
            ("snapshot_run_public_id", uuid_schema()),
            ("ffmpeg_version", string_schema()),
            ("ffprobe_version", string_schema()),
            ("codecs", array_ref_schema("MediaCapabilityCodecResponse")),
            ("encoders", array_string_schema()),
            ("decoders", array_string_schema()),
            ("muxers", array_string_schema()),
            ("demuxers", array_string_schema()),
            ("subtitle_support", array_string_schema()),
            ("hardware_accelerators", array_string_schema()),
            ("filesystem_utilities", array_string_schema()),
            ("utility_capabilities", array_string_schema()),
            ("license_mode", string_schema()),
            ("ffmpeg_license_mode", string_schema()),
            ("ffmpeg_enable_gpl", bool_schema()),
            ("ffmpeg_enable_version3", bool_schema()),
            ("ffmpeg_enable_nonfree", bool_schema()),
            ("compliance_links", array_string_schema()),
            ("absent_capabilities", array_string_schema()),
            (
                "features",
                array_ref_schema("MediaCapabilityFeatureResponse"),
            ),
            ("observed_at", date_time_schema()),
        ],
    )
}

fn media_yaml_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaYamlExportResponse",
            object_schema(
                &["version", "yaml_payload"],
                [
                    ("version", string_schema()),
                    ("yaml_payload", string_schema()),
                ],
            ),
        ),
        (
            "MediaYamlImportRequest",
            object_schema(&["yaml_payload"], [("yaml_payload", string_schema())]),
        ),
        (
            "MediaYamlIssueResponse",
            object_schema(
                &["code", "pointer", "blocking"],
                [
                    ("code", string_schema()),
                    ("pointer", string_schema()),
                    ("blocking", bool_schema()),
                ],
            ),
        ),
        (
            "MediaYamlValidationResponse",
            object_schema(
                &["version", "valid", "issues", "profile_count"],
                [
                    ("version", string_schema()),
                    ("valid", bool_schema()),
                    ("issues", array_ref_schema("MediaYamlIssueResponse")),
                    ("profile_count", integer_schema()),
                ],
            ),
        ),
        (
            "MediaYamlApplyResponse",
            object_schema(
                &[
                    "forced_dry_run",
                    "media_profile_public_ids",
                    "media_profile_import_draft_public_ids",
                ],
                [
                    ("forced_dry_run", bool_schema()),
                    ("media_profile_public_ids", array_uuid_schema()),
                    ("media_profile_import_draft_public_ids", array_uuid_schema()),
                ],
            ),
        ),
    ]
}

fn media_profile_response_schema() -> Value {
    object_schema(
        &[
            "media_profile_public_id",
            "profile_key",
            "source_root",
            "output_root",
            "dry_run_only",
            "retention_days",
            "policy_key",
            "watcher_enabled",
            "schedule_enabled",
            "updated_at",
        ],
        [
            ("media_profile_public_id", uuid_schema()),
            ("profile_key", string_schema()),
            ("source_root", string_schema()),
            ("output_root", string_schema()),
            ("dry_run_only", bool_schema()),
            ("retention_days", integer_schema()),
            ("compatibility_target_key", string_schema()),
            ("desired_target_key", string_schema()),
            ("desired_target_version", integer_schema()),
            ("policy_key", string_schema()),
            ("watcher_enabled", bool_schema()),
            ("schedule_enabled", bool_schema()),
            ("schedule_interval_minutes", integer_schema()),
            ("updated_at", date_time_schema()),
        ],
    )
}

fn media_job_response_schema() -> Value {
    object_schema(
        &[
            "media_job_public_id",
            "source_path",
            "status",
            "dry_run",
            "queued_at",
        ],
        [
            ("media_job_public_id", uuid_schema()),
            ("source_path", string_schema()),
            ("output_path", string_schema()),
            ("status", media_status_schema()),
            ("dry_run", bool_schema()),
            ("queued_at", date_time_schema()),
            ("started_at", date_time_schema()),
            ("completed_at", date_time_schema()),
            ("last_error", string_schema()),
        ],
    )
}

fn capability_id_response_schema() -> Value {
    object_schema(
        &["media_capability_snapshot_id"],
        [("media_capability_snapshot_id", integer_schema())],
    )
}

#[derive(Clone, Copy)]
enum SchemaKind {
    String,
    Integer,
    Boolean,
    DateTime,
    MediaStatus,
    OperationKind,
    ViolationSeverity,
    VerificationStatus,
}

impl SchemaKind {
    fn value(self) -> Value {
        match self {
            Self::String => string_schema(),
            Self::Integer => integer_schema(),
            Self::Boolean => bool_schema(),
            Self::DateTime => date_time_schema(),
            Self::MediaStatus => media_status_schema(),
            Self::OperationKind => operation_kind_schema(),
            Self::ViolationSeverity => violation_severity_schema(),
            Self::VerificationStatus => verification_check_status_schema(),
        }
    }
}

#[derive(Clone, Copy)]
struct SchemaProperty {
    name: &'static str,
    kind: SchemaKind,
}

const fn schema_property(name: &'static str, kind: SchemaKind) -> SchemaProperty {
    SchemaProperty { name, kind }
}

fn media_job_append_record_schemas<const N: usize>(
    append_schema: &'static str,
    response_schema: &'static str,
    list_schema: &'static str,
    list_field: &'static str,
    append_required: &[&'static str],
    properties: [SchemaProperty; N],
) -> Vec<(&'static str, Value)> {
    let response_required = append_required
        .iter()
        .copied()
        .chain(["created_at"])
        .collect::<Vec<_>>();
    vec![
        (
            append_schema,
            typed_object_schema_from_fields(append_required, properties),
        ),
        (
            response_schema,
            typed_object_schema_from_fields(
                &response_required,
                properties
                    .into_iter()
                    .chain([schema_property("created_at", SchemaKind::DateTime)]),
            ),
        ),
        (
            list_schema,
            list_response_schema(list_field, response_schema),
        ),
    ]
}

fn typed_object_schema_from_fields<I>(required: &[&'static str], properties: I) -> Value
where
    I: IntoIterator<Item = SchemaProperty>,
{
    object_schema_from_iter(
        required,
        properties
            .into_iter()
            .map(|property| (property.name, property.kind.value())),
    )
}

fn list_response_schema(field: &'static str, item_schema: &'static str) -> Value {
    object_schema(&[field], [(field, array_ref_schema(item_schema))])
}

fn object_schema<const N: usize>(
    required: &[&'static str],
    properties: [(&'static str, Value); N],
) -> Value {
    object_schema_from_iter(required, properties)
}

fn object_schema_from_iter<I>(required: &[&'static str], properties: I) -> Value
where
    I: IntoIterator<Item = (&'static str, Value)>,
{
    let mut property_map = Map::new();
    for (name, schema) in properties {
        property_map.insert(name.to_string(), schema);
    }

    let mut schema = Map::new();
    schema.insert("type".to_string(), Value::String("object".to_string()));
    schema.insert("properties".to_string(), Value::Object(property_map));
    if !required.is_empty() {
        schema.insert("required".to_string(), serde_json::json!(required));
    }
    Value::Object(schema)
}

fn schema_ref(schema: &'static str) -> Value {
    serde_json::json!({ "$ref": format!("#/components/schemas/{schema}") })
}

fn string_schema() -> Value {
    serde_json::json!({ "type": "string" })
}

fn uuid_schema() -> Value {
    serde_json::json!({ "type": "string", "format": "uuid" })
}

fn date_time_schema() -> Value {
    serde_json::json!({ "type": "string", "format": "date-time" })
}

fn bool_schema() -> Value {
    serde_json::json!({ "type": "boolean" })
}

fn integer_schema() -> Value {
    serde_json::json!({ "type": "integer" })
}

fn array_ref_schema(schema: &'static str) -> Value {
    serde_json::json!({ "type": "array", "items": schema_ref(schema) })
}

fn array_ref_items_schema(schema: &'static str, min_items: usize, max_items: usize) -> Value {
    serde_json::json!({
        "type": "array",
        "minItems": min_items,
        "maxItems": max_items,
        "items": schema_ref(schema),
    })
}

fn array_string_schema() -> Value {
    serde_json::json!({ "type": "array", "items": string_schema() })
}

fn media_discovery_source_paths_schema() -> Value {
    serde_json::json!({
        "type": "array",
        "maxItems": MEDIA_DISCOVERY_SOURCE_PATHS_MAX_LEN,
        "items": {
            "type": "string",
            "maxLength": MEDIA_DISCOVERY_SOURCE_PATH_MAX_BYTES
        }
    })
}

fn array_uuid_schema() -> Value {
    serde_json::json!({ "type": "array", "items": uuid_schema() })
}

fn media_status_schema() -> Value {
    serde_json::json!({
        "type": "string",
        "enum": ["queued", "running", "verifying", "completed", "failed", "cancelled"]
    })
}

fn operation_kind_schema() -> Value {
    serde_json::json!({
        "type": "string",
        "enum": [
            "remux",
            "metadata_rewrite",
            "disposition_rewrite",
            "label_rewrite",
            "stream_reorder",
            "audio_transcode",
            "video_transcode"
        ]
    })
}

fn violation_severity_schema() -> Value {
    serde_json::json!({ "type": "string", "enum": ["low", "medium", "high"] })
}

fn verification_check_status_schema() -> Value {
    serde_json::json!({ "type": "string", "enum": ["passed", "failed", "skipped"] })
}

#[must_use]
/// Return a fresh copy of the embedded `OpenAPI` specification.
pub fn openapi_document() -> Value {
    build_openapi_document()
}

#[must_use]
/// Return the default `OpenAPI` output path.
pub fn openapi_output_path() -> PathBuf {
    crate::openapi_assets::openapi_output_path()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::openapi_assets::OPENAPI_FILENAME;
    use serde_json::json;
    use std::io;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    fn repo_root() -> PathBuf {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        for ancestor in manifest_dir.ancestors() {
            if ancestor.join("AGENT.md").is_file() {
                return ancestor.to_path_buf();
            }
        }
        manifest_dir
    }

    fn server_root() -> std::io::Result<PathBuf> {
        let root = repo_root().join(".server_root");
        fs::create_dir_all(&root)?;
        Ok(root)
    }

    #[test]
    fn build_openapi_document_parses_embedded_json() {
        let document = build_openapi_document();
        assert!(
            document.is_object(),
            "embedded OpenAPI document should decode to a JSON object"
        );
    }

    #[test]
    fn openapi_document_exports_media_routes() -> Result<(), Box<dyn std::error::Error>> {
        let document = openapi_document();
        let paths = document
            .get("paths")
            .and_then(Value::as_object)
            .ok_or_else(|| io::Error::other("expected paths object"))?;

        for route in [
            "/v1/media/profiles",
            "/v1/media/profiles/{media_profile_public_id}",
            "/v1/media/profiles/validate",
            "/v1/media/compatibility-targets",
            "/v1/media/targets",
            "/v1/media/profiles/{media_profile_public_id}/desired-target",
            "/v1/media/policies",
            "/v1/media/job-retention",
            "/v1/media/planning/preview",
            "/v1/media/discovery/preview",
            "/v1/media/discovery/runs",
            "/v1/media/discovery/schedules",
            "/v1/media/discovery/watchers",
            "/v1/media/jobs",
            "/v1/media/jobs/{media_job_public_id}",
            "/v1/media/jobs/{media_job_public_id}/cancel",
            "/v1/media/jobs/{media_job_public_id}/retry",
            "/v1/media/jobs/{media_job_public_id}/phases",
            "/v1/media/jobs/{media_job_public_id}/operations",
            "/v1/media/jobs/{media_job_public_id}/violations",
            "/v1/media/jobs/{media_job_public_id}/plan-reasons",
            "/v1/media/jobs/{media_job_public_id}/verification-checks",
            "/v1/media/jobs/{media_job_public_id}/artifacts",
            "/v1/media/jobs/{media_job_public_id}/compact-audits",
            "/v1/media/capabilities",
            "/v1/media/capabilities/readiness",
            "/v1/media/capabilities/refresh",
            "/v1/media/compliance",
            "/v1/media/export",
            "/v1/media/imports/validate",
            "/v1/media/imports/apply",
        ] {
            assert!(
                paths.contains_key(route),
                "missing media OpenAPI route {route}"
            );
        }
        assert!(!paths.contains_key("/v1/media/desired-targets"));
        assert!(
            paths
                .get("/v1/media/discovery/schedules")
                .and_then(Value::as_object)
                .is_some_and(|path_item| path_item.contains_key("post")),
            "missing media OpenAPI POST operation for scheduled discovery"
        );
        assert!(
            paths
                .get("/v1/media/discovery/watchers")
                .and_then(Value::as_object)
                .is_some_and(|path_item| path_item.contains_key("post")),
            "missing media OpenAPI POST operation for watcher discovery"
        );
        for (route, method) in [
            ("/v1/media/compatibility-targets", "post"),
            ("/v1/media/targets", "post"),
            (
                "/v1/media/profiles/{media_profile_public_id}/desired-target",
                "patch",
            ),
            ("/v1/media/policies", "post"),
            ("/v1/media/job-retention", "patch"),
        ] {
            assert!(
                paths
                    .get(route)
                    .and_then(Value::as_object)
                    .is_some_and(|path_item| path_item.contains_key(method)),
                "missing media OpenAPI {method} operation for {route}"
            );
        }
        assert!(
            paths
                .get("/v1/media/capabilities")
                .and_then(Value::as_object)
                .is_some_and(|path_item| !path_item.contains_key("post")),
            "capability snapshots should not expose a public record operation"
        );

        Ok(())
    }

    const MEDIA_SCHEMA_NAMES: &[&str] = &[
        "MediaProfileUpsertRequest",
        "MediaProfilePatchRequest",
        "MediaProfileListResponse",
        "MediaProfileResponse",
        "MediaProfileValidationResponse",
        "MediaCompatibilityTargetResponse",
        "MediaCompatibilityTargetListResponse",
        "MediaCompatibilityTargetUpsertRequest",
        "MediaDesiredTargetStream",
        "MediaDesiredTargetCreateRequest",
        "MediaDesiredTargetResponse",
        "MediaDesiredTargetListResponse",
        "MediaProfileDesiredTargetRequest",
        "MediaPolicyResponse",
        "MediaPolicyListResponse",
        "MediaPolicyUpsertRequest",
        "MediaJobRetentionResponse",
        "MediaJobRetentionUpdateRequest",
        "MediaPlanningPreviewRequest",
        "MediaPlanningPreviewResponse",
        "MediaDiscoveryPreviewRequest",
        "MediaDiscoveryPreviewItemResponse",
        "MediaDiscoveryPreviewResponse",
        "MediaDiscoveryRunRequest",
        "MediaDiscoveryQueuedJobResponse",
        "MediaDiscoverySkippedItemResponse",
        "MediaDiscoveryRunResponse",
        "MediaDiscoveryScheduleResponse",
        "MediaDiscoveryScheduleListResponse",
        "MediaDiscoveryWatcherResponse",
        "MediaDiscoveryWatcherListResponse",
        "MediaJobCreateRequest",
        "MediaJobCreateResponse",
        "MediaJobListResponse",
        "MediaJobResponse",
        "MediaRecentJobPageResponse",
        "MediaRecentJobSummaryResponse",
        "MediaJobDiagnosticCounts",
        "MediaJobDiagnosticsResponse",
        "MediaJobPhaseAppendRequest",
        "MediaJobPhaseListResponse",
        "MediaJobPhaseResponse",
        "MediaJobOperationAppendRequest",
        "MediaJobOperationListResponse",
        "MediaJobOperationResponse",
        "MediaJobViolationAppendRequest",
        "MediaJobViolationListResponse",
        "MediaJobViolationResponse",
        "MediaJobPlanReasonAppendRequest",
        "MediaJobPlanReasonListResponse",
        "MediaJobPlanReasonResponse",
        "MediaJobVerificationCheckAppendRequest",
        "MediaJobVerificationCheckListResponse",
        "MediaJobVerificationCheckResponse",
        "MediaJobArtifactAppendRequest",
        "MediaJobArtifactListResponse",
        "MediaJobArtifactResponse",
        "MediaJobCompactAuditAppendRequest",
        "MediaJobCompactAuditListResponse",
        "MediaJobCompactAuditResponse",
        "MediaCapabilityRefreshResponse",
        "MediaCapabilityCodecResponse",
        "MediaCapabilityFeatureResponse",
        "MediaCapabilityLatestResponse",
        "MediaCapabilityReadinessResponse",
        "MediaCapabilitySnapshotResponse",
        "MediaComplianceResponse",
        "MediaYamlExportResponse",
        "MediaYamlImportRequest",
        "MediaYamlIssueResponse",
        "MediaYamlValidationResponse",
        "MediaYamlApplyResponse",
    ];

    #[test]
    fn openapi_document_exports_media_schemas() -> Result<(), Box<dyn std::error::Error>> {
        let document = openapi_document();
        let schemas = document
            .get("components")
            .and_then(Value::as_object)
            .and_then(|components| components.get("schemas"))
            .and_then(Value::as_object)
            .ok_or_else(|| io::Error::other("expected component schemas object"))?;

        for schema in MEDIA_SCHEMA_NAMES {
            assert!(
                schemas.contains_key(*schema),
                "missing media OpenAPI schema {schema}"
            );
        }

        let desired_target_streams_min_items = schemas
            .get("MediaDesiredTargetCreateRequest")
            .and_then(|schema| schema.get("properties"))
            .and_then(Value::as_object)
            .and_then(|properties| properties.get("streams"))
            .and_then(|streams| streams.get("minItems"))
            .and_then(Value::as_u64);
        assert_eq!(desired_target_streams_min_items, Some(1));
        let desired_target_streams_max_items = schemas
            .get("MediaDesiredTargetCreateRequest")
            .and_then(|schema| schema.get("properties"))
            .and_then(Value::as_object)
            .and_then(|properties| properties.get("streams"))
            .and_then(|streams| streams.get("maxItems"))
            .and_then(Value::as_u64);
        assert_eq!(
            desired_target_streams_max_items,
            u64::try_from(MAX_DESIRED_TARGET_STREAMS).ok()
        );

        Ok(())
    }

    #[test]
    fn openapi_document_returns_fresh_instance() -> Result<(), Box<dyn std::error::Error>> {
        let a = openapi_document();
        let mut b = openapi_document();
        b.as_object_mut()
            .ok_or_else(|| io::Error::other("expected object"))?
            .insert("x".into(), json!(1));
        assert!(a.get("x").is_none(), "documents are independent");
        Ok(())
    }

    #[test]
    fn embedded_dependencies_invoke_persist_hook() -> Result<(), Box<dyn std::error::Error>> {
        let document = Arc::new(json!({"openapi": "3.0.0"}));
        let dir = server_root()?.join(format!("openapi-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir)?;
        let dest = dir.join(OPENAPI_FILENAME);
        let invoked = Arc::new(std::sync::Mutex::new(Vec::new()));
        let persist = {
            let record = Arc::clone(&invoked);
            Arc::new(move |path: &Path, value: &Value| {
                record
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(path.to_path_buf());
                assert_eq!(value["openapi"], "3.0.0");
                Ok(())
            }) as OpenApiPersistFn
        };

        let deps = OpenApiDependencies::new(document, dest.clone(), persist);
        (deps.persist)(&dest, &deps.document)?;

        let paths = invoked
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone();
        assert_eq!(paths.as_slice(), &[dest]);
        let _ = fs::remove_dir_all(&dir);
        Ok(())
    }

    #[test]
    fn embedded_at_uses_requested_path() {
        let path = std::env::temp_dir().join(OPENAPI_FILENAME);
        let deps = OpenApiDependencies::embedded_at(&path);
        assert_eq!(deps.path, path);
        assert!(deps.document.is_object());
    }

    #[test]
    fn openapi_output_path_uses_embedded_filename() {
        let path = openapi_output_path();
        assert!(path.ends_with(OPENAPI_FILENAME));
    }
}
