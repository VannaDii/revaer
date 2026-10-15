use std::sync::Arc;

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
};
use tower::ServiceExt;

use super::ApiServer;
use crate::http::handlers::indexers::test_support::{RecordingIndexers, indexer_test_state};

#[tokio::test]
async fn profile_create_authentication_precedes_precondition_and_body() -> anyhow::Result<()> {
    let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
    let router = ApiServer::v1_media_profile_job_routes(&state).with_state(state);
    for precondition in [None, Some("*"), Some("invalid")] {
        let mut request =
            Request::post("/v1/media/profiles").header("content-type", "application/json");
        if let Some(value) = precondition {
            request = request.header("if-none-match", value);
        }
        let response = router
            .clone()
            .oneshot(request.body(Body::from("not json"))?)
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    Ok(())
}

#[tokio::test]
async fn root_readiness_requires_authentication_for_reads_and_writes() -> anyhow::Result<()> {
    let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
    let router = ApiServer::v1_media_root_routes(&state).with_state(state);
    for (path, method) in ["/v1/media/root-catalog", "/v1/media/root-catalog/readiness"]
        .into_iter()
        .flat_map(|path| {
            [
                Method::GET,
                Method::HEAD,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
                Method::OPTIONS,
            ]
            .into_iter()
            .map(move |method| (path, method))
        })
    {
        let response = router
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .body(Body::empty())?,
            )
            .await?;
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    Ok(())
}

#[tokio::test]
async fn profile_create_authentication_precedes_oversized_body() -> anyhow::Result<()> {
    let state = indexer_test_state(Arc::new(RecordingIndexers::default()))?;
    let router = ApiServer::v1_media_profile_job_routes(&state).with_state(state);
    let response = router
        .oneshot(
            Request::post("/v1/media/profiles")
                .header("content-type", "application/json")
                .header("if-none-match", "*")
                .body(Body::from("x".repeat(
                    crate::http::handlers::media_preconditions::PROFILE_BODY_LIMIT + 1,
                )))?,
        )
        .await?;
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}
