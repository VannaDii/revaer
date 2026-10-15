//! Authenticated, path-free root catalog readiness.

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, Request, State, rejection::QueryRejection},
    http::{HeaderValue, header},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::models::media_root_contract::{RootCatalogCursor, validate_root_catalog_limit};
use crate::{app::state::ApiState, http::errors::ApiError};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CatalogQuery {
    limit: Option<u16>,
    cursor: Option<String>,
}

fn invalid_query() -> Response {
    problem_response(
        ApiError::bad_request("invalid root catalog query")
            .with_context_field("error_code", "media_configuration_invalid"),
    )
}

pub(crate) async fn catalog(
    State(state): State<Arc<ApiState>>,
    query: Result<Query<CatalogQuery>, QueryRejection>,
) -> Response {
    let Ok(Query(query)) = query else {
        return invalid_query();
    };
    let Ok(limit) = validate_root_catalog_limit(query.limit) else {
        return invalid_query();
    };
    let Ok(cursor) = query
        .cursor
        .as_deref()
        .map(RootCatalogCursor::decode)
        .transpose()
    else {
        return invalid_query();
    };
    match state.media.media_root_catalog_page(limit, cursor).await {
        Ok(page) => Json(page).into_response(),
        Err(error) if error.code() == Some("media_configuration_invalid") => invalid_query(),
        Err(_) => problem_response(ApiError::internal("failed to read root catalog")),
    }
}

pub(crate) async fn no_store(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(crate) async fn readiness(State(state): State<Arc<ApiState>>) -> Response {
    let mut response = state
        .media
        .media_root_catalog_readiness()
        .await
        .map_or_else(
            |_| problem_response(ApiError::internal("failed to read root catalog readiness")),
            |readiness| Json(readiness).into_response(),
        );
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(crate) async fn method_not_allowed() -> Response {
    let mut response = problem_response(ApiError::media_method_not_allowed());
    response
        .headers_mut()
        .insert(header::ALLOW, HeaderValue::from_static("GET, HEAD"));
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

pub(crate) fn problem_response(error: ApiError) -> Response {
    let mut response = error.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::http::handlers::indexers::test_support::{RecordingIndexers, indexer_test_state};
    use axum::{body::to_bytes, http::StatusCode};

    #[tokio::test]
    async fn unavailable_provider_does_not_fabricate_readiness() -> anyhow::Result<()> {
        let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
        let response = readiness(State(state)).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body = to_bytes(response.into_body(), 4096).await?;
        let value: serde_json::Value = serde_json::from_slice(&body)?;
        assert!(value.get("generation").is_none());
        assert!(value.get("kinds").is_none());
        Ok(())
    }

    #[tokio::test]
    async fn unsupported_method_returns_problem_allow_and_no_store() -> anyhow::Result<()> {
        let response = method_not_allowed().await;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        assert_eq!(response.headers()[header::ALLOW], "GET, HEAD");
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(
            response.headers()[header::CONTENT_TYPE],
            "application/problem+json"
        );
        let body = to_bytes(response.into_body(), 4096).await?;
        let value: serde_json::Value = serde_json::from_slice(&body)?;
        assert_eq!(value["status"], 405);
        assert_eq!(value["context"][0]["value"], "media_method_not_allowed");
        Ok(())
    }
}
