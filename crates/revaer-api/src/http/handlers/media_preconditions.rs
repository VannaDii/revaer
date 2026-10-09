//! Request-part validation before profile creation can consume a body or write.

use axum::{
    extract::FromRequestParts,
    http::{
        HeaderValue,
        header::{CACHE_CONTROL, CONTENT_TYPE, IF_MATCH, IF_NONE_MATCH},
        request::Parts,
    },
    response::{IntoResponse, Response},
};

use crate::http::errors::ApiError;

pub(crate) const PROFILE_BODY_LIMIT: usize = 1024 * 1024;

pub(crate) struct CreateProfilePrecondition;

pub(crate) async fn profile_item_method_not_allowed() -> Response {
    let mut response = profile_problem(ApiError::media_method_not_allowed());
    response.headers_mut().insert(
        axum::http::header::ALLOW,
        HeaderValue::from_static("GET, HEAD, PUT"),
    );
    response
}

pub(crate) struct ReplaceProfilePrecondition {
    pub(crate) id: uuid::Uuid,
    pub(crate) version: i32,
}

impl<S: Send + Sync> FromRequestParts<S> for ReplaceProfilePrecondition {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let mut values = parts.headers.get_all(IF_MATCH).iter();
        let Some(value) = values.next() else {
            return Err(profile_problem(
                ApiError::media_replace_precondition_required(),
            ));
        };
        let invalid = || {
            profile_problem(
                ApiError::bad_request("invalid profile replacement precondition")
                    .with_context_field("error_code", "media_configuration_invalid"),
            )
        };
        if values.next().is_some() {
            return Err(invalid());
        }
        let text = value
            .to_str()
            .map_err(|_| invalid())?
            .trim_matches([' ', '\t']);
        let tag = text
            .strip_prefix("\"media-profile:")
            .and_then(|text| text.strip_suffix('"'))
            .ok_or_else(invalid)?;
        let (id, version) = tag.split_once(":v").ok_or_else(invalid)?;
        let id: uuid::Uuid = id.parse().map_err(|_| invalid())?;
        let version: i32 = version.parse().map_err(|_| invalid())?;
        if version <= 0 || text != format!("\"media-profile:{id}:v{version}\"") {
            return Err(invalid());
        }
        Ok(Self { id, version })
    }
}

impl<S: Send + Sync> FromRequestParts<S> for CreateProfilePrecondition {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let mut values = parts.headers.get_all(IF_NONE_MATCH).iter();
        let Some(value) = values.next() else {
            return Err(profile_problem(
                ApiError::media_create_precondition_required(),
            ));
        };
        let valid = values.next().is_none()
            && value
                .to_str()
                .is_ok_and(|text| text.trim_matches([' ', '\t']) == "*");
        if !valid {
            return Err(profile_problem(
                ApiError::bad_request("invalid profile creation precondition")
                    .with_context_field("error_code", "media_configuration_invalid"),
            ));
        }
        Ok(Self)
    }
}

fn profile_problem(error: ApiError) -> Response {
    let mut response = error.into_response();
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        Json, Router,
        body::Body,
        extract::DefaultBodyLimit,
        http::{Request, StatusCode},
        routing::post,
    };
    use tower::ServiceExt;

    async fn guarded_body(
        _precondition: CreateProfilePrecondition,
        Json(_body): Json<serde_json::Value>,
    ) -> StatusCode {
        StatusCode::NO_CONTENT
    }

    async fn guarded_replacement(
        precondition: ReplaceProfilePrecondition,
        Json(_body): Json<serde_json::Value>,
    ) -> StatusCode {
        assert_eq!(precondition.id, uuid::Uuid::nil());
        assert_eq!(precondition.version, 1);
        StatusCode::NO_CONTENT
    }

    #[tokio::test]
    async fn replacement_requires_one_canonical_strong_version_before_body() -> anyhow::Result<()> {
        let app = Router::new().route("/profiles", axum::routing::put(guarded_replacement));
        let valid = "\"media-profile:00000000-0000-0000-0000-000000000000:v1\"";
        for (headers, expected) in [
            (vec![], StatusCode::PRECONDITION_REQUIRED),
            (vec!["*"], StatusCode::BAD_REQUEST),
            (
                vec!["W/\"media-profile:00000000-0000-0000-0000-000000000000:v1\""],
                StatusCode::BAD_REQUEST,
            ),
            (
                vec!["\"media-profile:00000000-0000-0000-0000-000000000000:v01\""],
                StatusCode::BAD_REQUEST,
            ),
            (
                vec!["\"media-profile:00000000-0000-0000-0000-000000000000:v0\""],
                StatusCode::BAD_REQUEST,
            ),
            (
                vec!["\"media-profile:00000000-0000-0000-0000-000000000000:v2147483648\""],
                StatusCode::BAD_REQUEST,
            ),
            (vec![valid, valid], StatusCode::BAD_REQUEST),
            (vec![valid], StatusCode::NO_CONTENT),
        ] {
            let mut request = Request::put("/profiles").header("content-type", "application/json");
            for value in headers {
                request = request.header(IF_MATCH, value);
            }
            let body = if expected == StatusCode::NO_CONTENT {
                "{}"
            } else {
                "invalid json"
            };
            let response = app.clone().oneshot(request.body(Body::from(body))?).await?;
            assert_eq!(response.status(), expected);
        }
        Ok(())
    }

    #[tokio::test]
    async fn missing_or_malformed_header_precedes_body_decoding() -> anyhow::Result<()> {
        let app = Router::new().route("/profiles", post(guarded_body));
        for (headers, expected) in [
            (vec![], StatusCode::PRECONDITION_REQUIRED),
            (vec![""], StatusCode::BAD_REQUEST),
            (vec!["\"*\""], StatusCode::BAD_REQUEST),
            (vec!["W/\"tag\""], StatusCode::BAD_REQUEST),
            (vec!["*, \"tag\""], StatusCode::BAD_REQUEST),
            (vec!["*", "*"], StatusCode::BAD_REQUEST),
        ] {
            let mut request = Request::post("/profiles").header("content-type", "application/json");
            for value in headers {
                request = request.header(IF_NONE_MATCH, value);
            }
            let response = app
                .clone()
                .oneshot(request.body(Body::from("not json"))?)
                .await?;
            assert_eq!(response.status(), expected);
            assert_eq!(
                response.headers()["content-type"],
                "application/problem+json"
            );
            assert_eq!(response.headers()["cache-control"], "no-store");
        }
        Ok(())
    }

    #[tokio::test]
    async fn exact_wildcard_allows_body_validation_but_does_not_create_a_profile()
    -> anyhow::Result<()> {
        let app = Router::new().route("/profiles", post(guarded_body));
        for value in ["*", " \t*\t "] {
            let request = Request::post("/profiles")
                .header(IF_NONE_MATCH, value)
                .header("content-type", "application/json")
                .body(Body::from("{}"))?;
            assert_eq!(
                app.clone().oneshot(request).await?.status(),
                StatusCode::NO_CONTENT
            );
        }
        Ok(())
    }

    #[tokio::test]
    async fn profile_body_limit_accepts_boundary_and_rejects_excess() -> anyhow::Result<()> {
        let app = Router::new().route(
            "/profiles",
            post(guarded_body).layer(DefaultBodyLimit::max(PROFILE_BODY_LIMIT)),
        );
        for (size, expected) in [
            (PROFILE_BODY_LIMIT, StatusCode::NO_CONTENT),
            (PROFILE_BODY_LIMIT + 1, StatusCode::PAYLOAD_TOO_LARGE),
        ] {
            let body = format!("\"{}\"", "x".repeat(size - 2));
            let response = app
                .clone()
                .oneshot(
                    Request::post("/profiles")
                        .header(IF_NONE_MATCH, "*")
                        .header("content-type", "application/json")
                        .body(Body::from(body))?,
                )
                .await?;
            assert_eq!(response.status(), expected);
        }
        Ok(())
    }
}
