//! `OpenAPI` document helpers and dependency wiring.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use serde_json::{Map, Value};
use tracing::error;

use crate::openapi_assets::OPENAPI_EMBEDDED_JSON;

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
    for (path, path_item) in media_paths() {
        paths.insert(path.to_string(), path_item);
    }

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
    for (name, schema) in media_schemas() {
        schemas.insert(name.to_string(), schema);
    }
}

fn media_paths() -> Vec<(&'static str, Value)> {
    let mut paths = Vec::new();
    paths.extend(media_profile_paths());
    paths.extend(media_job_paths());
    paths.extend(media_capability_paths());
    paths.extend(media_yaml_paths());
    paths
}

fn media_profile_paths() -> Vec<(&'static str, Value)> {
    vec![
        (
            "/v1/media/profiles",
            path_item([
                (
                    "get",
                    media_operation(
                        "List media profiles",
                        "200",
                        "Media profile collection",
                        Some("MediaProfileListResponse"),
                        None,
                        Vec::new(),
                    ),
                ),
                (
                    "post",
                    media_operation(
                        "Create or update a media profile",
                        "201",
                        "Media profile saved",
                        Some("MediaProfileResponse"),
                        Some("MediaProfileUpsertRequest"),
                        Vec::new(),
                    ),
                ),
            ]),
        ),
        (
            "/v1/media/profiles/{media_profile_public_id}",
            path_item([
                (
                    "get",
                    media_operation(
                        "Read a media profile",
                        "200",
                        "Media profile",
                        Some("MediaProfileResponse"),
                        None,
                        vec![path_uuid_parameter("media_profile_public_id")],
                    ),
                ),
                (
                    "patch",
                    media_operation(
                        "Patch a media profile",
                        "200",
                        "Media profile",
                        Some("MediaProfileResponse"),
                        Some("MediaProfilePatchRequest"),
                        vec![path_uuid_parameter("media_profile_public_id")],
                    ),
                ),
            ]),
        ),
    ]
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
        (
            "/v1/media/jobs",
            path_item([
                (
                    "get",
                    media_operation(
                        "List media jobs",
                        "200",
                        "Media job collection",
                        Some("MediaJobListResponse"),
                        None,
                        vec![
                            query_uuid_parameter("media_profile_public_id"),
                            query_string_parameter("status"),
                        ],
                    ),
                ),
                (
                    "post",
                    media_operation(
                        "Create a media job",
                        "201",
                        "Media job created",
                        Some("MediaJobCreateResponse"),
                        Some("MediaJobCreateRequest"),
                        Vec::new(),
                    ),
                ),
            ]),
        ),
        (
            "/v1/media/jobs/{media_job_public_id}",
            path_item([(
                "get",
                media_operation(
                    "Read a media job",
                    "200",
                    "Media job",
                    Some("MediaJobResponse"),
                    None,
                    vec![path_uuid_parameter("media_job_public_id")],
                ),
            )]),
        ),
    ]
}

fn media_job_lifecycle_paths() -> Vec<(&'static str, Value)> {
    vec![
        (
            "/v1/media/jobs/{media_job_public_id}/cancel",
            path_item([(
                "post",
                media_operation(
                    "Cancel a media job",
                    "204",
                    "Media job cancelled",
                    None,
                    None,
                    vec![path_uuid_parameter("media_job_public_id")],
                ),
            )]),
        ),
        (
            "/v1/media/jobs/{media_job_public_id}/retry",
            path_item([(
                "post",
                media_operation(
                    "Retry a media job",
                    "204",
                    "Media job retried",
                    None,
                    None,
                    vec![path_uuid_parameter("media_job_public_id")],
                ),
            )]),
        ),
        (
            "/v1/media/jobs/{media_job_public_id}/phases",
            path_item([(
                "post",
                media_operation(
                    "Append a media job phase",
                    "204",
                    "Media job phase appended",
                    None,
                    Some("MediaJobPhaseAppendRequest"),
                    vec![path_uuid_parameter("media_job_public_id")],
                ),
            )]),
        ),
    ]
}

fn media_job_record_paths() -> Vec<(&'static str, Value)> {
    vec![
        (
            "/v1/media/jobs/{media_job_public_id}/operations",
            path_item([
                (
                    "get",
                    media_operation(
                        "List media job operations",
                        "200",
                        "Media job operations",
                        Some("MediaJobOperationListResponse"),
                        None,
                        vec![path_uuid_parameter("media_job_public_id")],
                    ),
                ),
                (
                    "post",
                    media_operation(
                        "Append a media job operation",
                        "204",
                        "Media job operation appended",
                        None,
                        Some("MediaJobOperationAppendRequest"),
                        vec![path_uuid_parameter("media_job_public_id")],
                    ),
                ),
            ]),
        ),
        (
            "/v1/media/jobs/{media_job_public_id}/violations",
            path_item([
                (
                    "get",
                    media_operation(
                        "List media job violations",
                        "200",
                        "Media job violations",
                        Some("MediaJobViolationListResponse"),
                        None,
                        vec![path_uuid_parameter("media_job_public_id")],
                    ),
                ),
                (
                    "post",
                    media_operation(
                        "Append a media job violation",
                        "204",
                        "Media job violation appended",
                        None,
                        Some("MediaJobViolationAppendRequest"),
                        vec![path_uuid_parameter("media_job_public_id")],
                    ),
                ),
            ]),
        ),
    ]
}

fn media_capability_paths() -> Vec<(&'static str, Value)> {
    vec![
        (
            "/v1/media/capabilities",
            path_item([
                (
                    "get",
                    media_operation(
                        "Read latest media capability snapshot",
                        "200",
                        "Latest media capability snapshot",
                        Some("MediaCapabilityLatestResponse"),
                        None,
                        Vec::new(),
                    ),
                ),
                (
                    "post",
                    media_operation(
                        "Record a media capability snapshot",
                        "201",
                        "Media capability snapshot recorded",
                        Some("MediaCapabilityRecordResponse"),
                        Some("MediaCapabilityRecordRequest"),
                        Vec::new(),
                    ),
                ),
            ]),
        ),
        (
            "/v1/media/capabilities/readiness",
            path_item([(
                "get",
                media_operation(
                    "Read media capability readiness",
                    "200",
                    "Media capability readiness",
                    Some("MediaCapabilityReadinessResponse"),
                    None,
                    Vec::new(),
                ),
            )]),
        ),
        (
            "/v1/media/capabilities/refresh",
            path_item([(
                "post",
                media_operation(
                    "Refresh media capabilities",
                    "201",
                    "Media capability snapshot refreshed",
                    Some("MediaCapabilityRefreshResponse"),
                    None,
                    Vec::new(),
                ),
            )]),
        ),
        (
            "/v1/media/compliance",
            path_item([(
                "get",
                media_operation(
                    "Read media runtime compliance artifacts",
                    "200",
                    "Media runtime compliance artifacts",
                    Some("MediaComplianceResponse"),
                    None,
                    Vec::new(),
                ),
            )]),
        ),
    ]
}

fn media_yaml_paths() -> Vec<(&'static str, Value)> {
    vec![
        (
            "/v1/media/export",
            path_item([(
                "get",
                media_operation(
                    "Export media profile YAML",
                    "200",
                    "Media YAML export",
                    Some("MediaYamlExportResponse"),
                    None,
                    Vec::new(),
                ),
            )]),
        ),
        (
            "/v1/media/imports/validate",
            path_item([(
                "post",
                media_operation(
                    "Validate media profile YAML",
                    "200",
                    "Media YAML validation result",
                    Some("MediaYamlValidationResponse"),
                    Some("MediaYamlImportRequest"),
                    Vec::new(),
                ),
            )]),
        ),
        (
            "/v1/media/imports/apply",
            path_item([(
                "post",
                media_operation(
                    "Apply media profile YAML",
                    "201",
                    "Media YAML apply result",
                    Some("MediaYamlApplyResponse"),
                    Some("MediaYamlImportRequest"),
                    Vec::new(),
                ),
            )]),
        ),
    ]
}

fn path_item<const N: usize>(operations: [(&'static str, Value); N]) -> Value {
    let mut path_item = Map::new();
    for (method, operation) in operations {
        path_item.insert(method.to_string(), operation);
    }
    Value::Object(path_item)
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

fn media_schemas() -> Vec<(&'static str, Value)> {
    let mut schemas = Vec::new();
    schemas.extend(media_profile_schemas());
    schemas.extend(media_job_schemas());
    schemas.extend(media_capability_schemas());
    schemas.extend(media_yaml_schemas());
    schemas
}

fn media_profile_schemas() -> Vec<(&'static str, Value)> {
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

fn media_job_schemas() -> Vec<(&'static str, Value)> {
    let mut schemas = Vec::new();
    schemas.extend(media_job_core_schemas());
    schemas.extend(media_job_operation_schemas());
    schemas.extend(media_job_violation_schemas());
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
        (
            "MediaJobPhaseAppendRequest",
            object_schema(
                &["phase_index", "phase_name", "phase_status"],
                [
                    ("phase_index", integer_schema()),
                    ("phase_name", string_schema()),
                    ("phase_status", media_status_schema()),
                    ("details_text", string_schema()),
                ],
            ),
        ),
    ]
}

fn media_job_operation_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaJobOperationAppendRequest",
            object_schema(
                &["operation_index", "operation_kind", "command_bin"],
                [
                    ("operation_index", integer_schema()),
                    ("operation_kind", operation_kind_schema()),
                    ("stream_id", integer_schema()),
                    ("command_bin", string_schema()),
                    ("arg_1", string_schema()),
                    ("arg_2", string_schema()),
                    ("arg_3", string_schema()),
                    ("arg_4", string_schema()),
                    ("arg_5", string_schema()),
                ],
            ),
        ),
        (
            "MediaJobOperationResponse",
            object_schema(
                &[
                    "operation_index",
                    "operation_kind",
                    "command_bin",
                    "created_at",
                ],
                [
                    ("operation_index", integer_schema()),
                    ("operation_kind", operation_kind_schema()),
                    ("stream_id", integer_schema()),
                    ("command_bin", string_schema()),
                    ("arg_1", string_schema()),
                    ("arg_2", string_schema()),
                    ("arg_3", string_schema()),
                    ("arg_4", string_schema()),
                    ("arg_5", string_schema()),
                    ("created_at", date_time_schema()),
                ],
            ),
        ),
        (
            "MediaJobOperationListResponse",
            object_schema(
                &["operations"],
                [("operations", array_ref_schema("MediaJobOperationResponse"))],
            ),
        ),
    ]
}

fn media_job_violation_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaJobViolationAppendRequest",
            object_schema(
                &["violation_index", "violation_kind", "severity"],
                [
                    ("violation_index", integer_schema()),
                    ("violation_kind", string_schema()),
                    ("severity", violation_severity_schema()),
                    ("stream_id", integer_schema()),
                ],
            ),
        ),
        (
            "MediaJobViolationResponse",
            object_schema(
                &[
                    "violation_index",
                    "violation_kind",
                    "severity",
                    "created_at",
                ],
                [
                    ("violation_index", integer_schema()),
                    ("violation_kind", string_schema()),
                    ("severity", violation_severity_schema()),
                    ("stream_id", integer_schema()),
                    ("created_at", date_time_schema()),
                ],
            ),
        ),
        (
            "MediaJobViolationListResponse",
            object_schema(
                &["violations"],
                [("violations", array_ref_schema("MediaJobViolationResponse"))],
            ),
        ),
    ]
}

fn media_capability_schemas() -> Vec<(&'static str, Value)> {
    vec![
        (
            "MediaCapabilityRecordRequest",
            object_schema(
                &[
                    "ffmpeg_version",
                    "ffprobe_version",
                    "codec_name",
                    "encode_supported",
                    "decode_supported",
                ],
                [
                    ("ffmpeg_version", string_schema()),
                    ("ffprobe_version", string_schema()),
                    ("codec_name", string_schema()),
                    ("encode_supported", bool_schema()),
                    ("decode_supported", bool_schema()),
                ],
            ),
        ),
        (
            "MediaCapabilityRecordResponse",
            capability_id_response_schema(),
        ),
        (
            "MediaCapabilityRefreshResponse",
            capability_id_response_schema(),
        ),
        (
            "MediaCapabilitySnapshotResponse",
            object_schema(
                &[
                    "media_capability_snapshot_id",
                    "ffmpeg_version",
                    "ffprobe_version",
                    "codec_name",
                    "encode_supported",
                    "decode_supported",
                    "observed_at",
                ],
                [
                    ("media_capability_snapshot_id", integer_schema()),
                    ("ffmpeg_version", string_schema()),
                    ("ffprobe_version", string_schema()),
                    ("codec_name", string_schema()),
                    ("encode_supported", bool_schema()),
                    ("decode_supported", bool_schema()),
                    ("observed_at", date_time_schema()),
                ],
            ),
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
                    "source_offer_path",
                    "third_party_notices_path",
                    "sbom_path",
                    "inventory_path",
                    "exiftool_exception_path",
                    "license_excluded_capabilities",
                ],
                [
                    ("license_mode", string_schema()),
                    ("source_offer_path", string_schema()),
                    ("third_party_notices_path", string_schema()),
                    ("sbom_path", string_schema()),
                    ("inventory_path", string_schema()),
                    ("exiftool_exception_path", string_schema()),
                    ("license_excluded_capabilities", array_string_schema()),
                ],
            ),
        ),
    ]
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
            "MediaYamlValidationResponse",
            object_schema(
                &["version", "valid", "issues", "profile_count"],
                [
                    ("version", string_schema()),
                    ("valid", bool_schema()),
                    ("issues", array_string_schema()),
                    ("profile_count", integer_schema()),
                ],
            ),
        ),
        (
            "MediaYamlApplyResponse",
            object_schema(
                &["forced_dry_run", "media_profile_public_ids"],
                [
                    ("forced_dry_run", bool_schema()),
                    ("media_profile_public_ids", array_uuid_schema()),
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

fn object_schema<const N: usize>(
    required: &[&'static str],
    properties: [(&'static str, Value); N],
) -> Value {
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

fn array_string_schema() -> Value {
    serde_json::json!({ "type": "array", "items": string_schema() })
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
            "/v1/media/jobs",
            "/v1/media/jobs/{media_job_public_id}",
            "/v1/media/jobs/{media_job_public_id}/cancel",
            "/v1/media/jobs/{media_job_public_id}/retry",
            "/v1/media/jobs/{media_job_public_id}/phases",
            "/v1/media/jobs/{media_job_public_id}/operations",
            "/v1/media/jobs/{media_job_public_id}/violations",
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

        let schemas = document
            .get("components")
            .and_then(Value::as_object)
            .and_then(|components| components.get("schemas"))
            .and_then(Value::as_object)
            .ok_or_else(|| io::Error::other("expected component schemas object"))?;

        for schema in [
            "MediaProfileUpsertRequest",
            "MediaProfilePatchRequest",
            "MediaProfileListResponse",
            "MediaProfileResponse",
            "MediaJobCreateRequest",
            "MediaJobCreateResponse",
            "MediaJobListResponse",
            "MediaJobResponse",
            "MediaJobPhaseAppendRequest",
            "MediaJobOperationAppendRequest",
            "MediaJobOperationListResponse",
            "MediaJobOperationResponse",
            "MediaJobViolationAppendRequest",
            "MediaJobViolationListResponse",
            "MediaJobViolationResponse",
            "MediaCapabilityRecordRequest",
            "MediaCapabilityRecordResponse",
            "MediaCapabilityRefreshResponse",
            "MediaCapabilityLatestResponse",
            "MediaCapabilityReadinessResponse",
            "MediaCapabilitySnapshotResponse",
            "MediaComplianceResponse",
            "MediaYamlExportResponse",
            "MediaYamlImportRequest",
            "MediaYamlValidationResponse",
            "MediaYamlApplyResponse",
        ] {
            assert!(
                schemas.contains_key(schema),
                "missing media OpenAPI schema {schema}"
            );
        }

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
