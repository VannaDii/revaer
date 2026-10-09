//! Conditional, bounded and authenticated immutable association creation/read.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, Query, State, rejection::JsonRejection},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

#[derive(Default, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct AssociationPageQuery {
    limit: Option<u16>,
    cursor: Option<String>,
}

pub(crate) async fn list(
    State(state): State<Arc<ApiState>>,
    query: Result<Query<AssociationPageQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<Json<crate::models::media_root_contract::DiscoveryAssociationPageResponse>, Response> {
    page(&state, query).await.map(Json)
}

pub(crate) async fn page(
    state: &ApiState,
    query: Result<Query<AssociationPageQuery>, axum::extract::rejection::QueryRejection>,
) -> Result<crate::models::media_root_contract::DiscoveryAssociationPageResponse, Response> {
    use crate::models::media_root_contract::{
        AssociationCollectionCursor, validate_root_catalog_limit,
    };
    let invalid = || {
        super::media_roots::problem_response(
            ApiError::bad_request("invalid association collection query")
                .with_context_field("error_code", "media_configuration_invalid"),
        )
    };
    let Query(query) = query.map_err(|_| invalid())?;
    let limit = validate_root_catalog_limit(query.limit).map_err(|_| invalid())?;
    let cursor = query
        .cursor
        .as_deref()
        .map(AssociationCollectionCursor::decode)
        .transpose()
        .map_err(|_| invalid())?;
    state
        .media
        .media_association_page(limit, cursor)
        .await
        .map_err(|error| super::media_roots::problem_response(map_error(&error)))
}
use uuid::Uuid;

use super::{indexers::SYSTEM_ACTOR_PUBLIC_ID, media_preconditions::CreateProfilePrecondition};
use crate::{
    app::{
        media::{MediaServiceError, MediaServiceErrorKind},
        state::ApiState,
    },
    http::errors::ApiError,
    models::media_root_contract::{DiscoveryAssociationRequest, DiscoveryAssociationResponse},
};

pub(crate) async fn create(
    State(state): State<Arc<ApiState>>,
    _precondition: CreateProfilePrecondition,
    request: Result<Json<DiscoveryAssociationRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(request) = request.map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::media_profile_body_too_large()
        } else {
            ApiError::bad_request("invalid complete association body")
                .with_context_field("error_code", "media_configuration_invalid")
        }
    })?;
    let association = state
        .media
        .media_association_create(SYSTEM_ACTOR_PUBLIC_ID, &request)
        .await
        .map_err(|error| map_error(&error))?;
    representation(association, true)
}

pub(crate) async fn get(
    State(state): State<Arc<ApiState>>,
    Path(id): Path<Uuid>,
) -> Result<Response, ApiError> {
    let association = state
        .media
        .media_association(id)
        .await
        .map_err(|error| map_error(&error))?
        .ok_or_else(|| ApiError::not_found("discovery association not found"))?;
    representation(association, false)
}

fn map_error(error: &MediaServiceError) -> ApiError {
    let kind = error.kind();
    if kind == MediaServiceErrorKind::Storage {
        return ApiError::internal("discovery association storage failure");
    }
    let problem = match error.code() {
        Some(
            "media_configuration_root_unmapped"
            | "media_root_kind_forbidden"
            | "media_root_binding_incomplete",
        ) => ApiError::conflict("discovery association binding unavailable"),
        _ => match kind {
            MediaServiceErrorKind::Invalid => {
                ApiError::bad_request("invalid discovery association")
            }
            MediaServiceErrorKind::NotFound => {
                ApiError::not_found("discovery association reference not found")
            }
            MediaServiceErrorKind::Conflict => ApiError::conflict("discovery association conflict"),
            MediaServiceErrorKind::Unavailable => {
                ApiError::service_unavailable("discovery association service unavailable")
            }
            MediaServiceErrorKind::Storage => {
                ApiError::internal("discovery association storage failure")
            }
        },
    };
    match error.code() {
        Some(code) => problem.with_context_field("error_code", code),
        None => problem,
    }
}

fn representation(
    association: DiscoveryAssociationResponse,
    created: bool,
) -> Result<Response, ApiError> {
    let fields = association.fields();
    let etag = HeaderValue::from_str(&format!(
        "\"media-discovery-association:{}:v{}\"",
        fields.media_discovery_association_public_id, fields.latest_version
    ))
    .map_err(|_| ApiError::internal("invalid association representation"))?;
    let location = if created {
        Some(
            HeaderValue::from_str(&format!(
                "/v1/media/discovery-associations/{}",
                fields.media_discovery_association_public_id
            ))
            .map_err(|_| ApiError::internal("invalid association representation"))?,
        )
    } else {
        None
    };
    let mut response = Json(association).into_response();
    response.headers_mut().insert(header::ETAG, etag);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Some(location) = location {
        *response.status_mut() = StatusCode::CREATED;
        response.headers_mut().insert(header::LOCATION, location);
    }
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::media_root_contract::{
        DiscoveryAssociationResponseFields, DiscoveryModes, ProfileLifecycle, ProfileRootResolution,
    };

    #[test]
    fn association_errors_preserve_closed_status_mapping() {
        for (kind, code, expected) in [
            (
                MediaServiceErrorKind::Invalid,
                "media_root_binding_incomplete",
                StatusCode::CONFLICT,
            ),
            (
                MediaServiceErrorKind::Invalid,
                "media_configuration_invalid",
                StatusCode::BAD_REQUEST,
            ),
            (
                MediaServiceErrorKind::Conflict,
                "media_configuration_overlap",
                StatusCode::CONFLICT,
            ),
            (
                MediaServiceErrorKind::Unavailable,
                "media_runtime_unavailable",
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                MediaServiceErrorKind::NotFound,
                "media_association_missing",
                StatusCode::NOT_FOUND,
            ),
            (
                MediaServiceErrorKind::Storage,
                "/private/sql-detail",
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ] {
            assert_eq!(
                map_error(&MediaServiceError::new(kind).with_code(code))
                    .into_response()
                    .status(),
                expected
            );
        }
    }

    #[test]
    fn association_representation_fences_complete_body() -> Result<(), Box<dyn std::error::Error>> {
        let id = Uuid::from_u128(2);
        let association = DiscoveryAssociationResponse::new(DiscoveryAssociationResponseFields {
            request: DiscoveryAssociationRequest::new(
                "library",
                Uuid::from_u128(1),
                1,
                "source",
                "",
                DiscoveryModes {
                    manual_enabled: true,
                    watcher_enabled: false,
                    schedule_enabled: false,
                },
            )?,
            media_discovery_association_public_id: id,
            latest_version: 1,
            active_version: Some(1),
            lifecycle_state: ProfileLifecycle::Active,
            resolution_state: ProfileRootResolution::Resolved,
            binding_ready: true,
            binding_reason: None,
            destructive_ready: false,
            destructive_reason: Some("media_root_durability_unproven".into()),
            created_at: "2026-10-02T00:00:00Z".into(),
        })?;
        let created = representation(association.clone(), true)?;
        assert_eq!(created.status(), StatusCode::CREATED);
        assert_eq!(
            created.headers()[header::ETAG],
            format!("\"media-discovery-association:{id}:v1\"")
        );
        assert_eq!(
            created.headers()[header::LOCATION],
            format!("/v1/media/discovery-associations/{id}")
        );
        assert_eq!(created.headers()[header::CACHE_CONTROL], "no-store");
        let read = representation(association, false)?;
        assert_eq!(read.status(), StatusCode::OK);
        assert_eq!(
            read.headers()[header::ETAG],
            created.headers()[header::ETAG]
        );
        assert!(!read.headers().contains_key(header::LOCATION));
        Ok(())
    }
}
