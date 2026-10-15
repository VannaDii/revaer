//! Authenticated cadence configuration, separate from automatic activation.

use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

use super::{indexers::SYSTEM_ACTOR_PUBLIC_ID, media_preconditions::CreateProfilePrecondition};
use crate::models::media_schedule::{
    MediaScheduleConfigurationRequest, MediaScheduleConfigurationResponse,
};
use crate::{
    app::{
        media::{MediaServiceError, MediaServiceErrorKind},
        state::ApiState,
    },
    http::errors::ApiError,
};

pub(crate) async fn get(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    state
        .media
        .media_schedule_configuration(id)
        .await
        .map_err(|error| map_error(&error))?
        .ok_or_else(|| ApiError::not_found("schedule configuration not found"))
        .and_then(|row| representation(row, StatusCode::OK))
}

pub(crate) async fn create(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<Uuid>,
    _precondition: CreateProfilePrecondition,
    request: Result<
        Json<MediaScheduleConfigurationRequest>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Result<Response, ApiError> {
    let Json(request) = request.map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::media_profile_body_too_large()
        } else {
            ApiError::bad_request("invalid explicit schedule configuration")
        }
    })?;
    let result = state
        .media
        .media_schedule_configuration_create(SYSTEM_ACTOR_PUBLIC_ID, id, &request)
        .await
        .map_err(|error| map_error(&error))?;
    representation(result, StatusCode::CREATED)
}

pub(crate) async fn replace(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<Uuid>,
    precondition: super::media_schedule_preconditions::ReplaceSchedulePrecondition,
    request: Result<
        Json<MediaScheduleConfigurationRequest>,
        axum::extract::rejection::JsonRejection,
    >,
) -> Result<Response, ApiError> {
    let Json(request) = request.map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::media_profile_body_too_large()
        } else {
            ApiError::bad_request("invalid explicit schedule configuration")
        }
    })?;
    if id != precondition.id || request.association_version != precondition.version {
        return Err(ApiError::media_schedule_revision_conflict());
    }
    let result = state
        .media
        .media_schedule_configuration_replace(
            SYSTEM_ACTOR_PUBLIC_ID,
            id,
            &request,
            precondition.revision,
        )
        .await
        .map_err(|error| map_error(&error))?;
    representation(result, StatusCode::OK)
}

fn representation(
    row: MediaScheduleConfigurationResponse,
    status: StatusCode,
) -> Result<Response, ApiError> {
    let etag = HeaderValue::from_str(&row.etag())
        .map_err(|_| ApiError::internal("invalid schedule representation"))?;
    let mut response = (status, Json(row)).into_response();
    response.headers_mut().insert(header::ETAG, etag);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

fn map_error(error: &MediaServiceError) -> ApiError {
    let problem = match error.code() {
        Some("media_schedule_configuration_stale") => ApiError::media_schedule_revision_conflict(),
        Some("media_schedule_association_not_found") => {
            ApiError::not_found("schedule association not found")
        }
        Some("media_schedule_association_stale" | "media_schedule_configuration_conflict") => {
            ApiError::conflict("schedule configuration conflict")
        }
        Some("media_root_binding_incomplete") => {
            ApiError::conflict("schedule association unavailable")
        }
        _ if error.kind() == MediaServiceErrorKind::Invalid => {
            ApiError::bad_request("invalid schedule configuration")
        }
        _ => ApiError::internal("schedule configuration storage failure"),
    };
    match error.code() {
        Some(code) => problem.with_context_field("error_code", code),
        None => problem,
    }
}
