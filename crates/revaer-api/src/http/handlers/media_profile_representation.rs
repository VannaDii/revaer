//! Complete profile response headers; identity/version fence the immutable head.

use axum::{
    Json,
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};

use crate::{http::errors::ApiError, models::media_root_contract::ProfileVersionResponse};

pub(super) fn created(profile: ProfileVersionResponse) -> Result<Response, ApiError> {
    let location = HeaderValue::from_str(&format!(
        "/v1/media/profiles/{}",
        profile.fields().media_profile_public_id
    ))
    .map_err(|_| ApiError::internal("invalid media profile location"))?;
    let mut response = response(profile)?;
    *response.status_mut() = StatusCode::CREATED;
    response.headers_mut().insert(header::LOCATION, location);
    Ok(response)
}

pub(super) fn response(profile: ProfileVersionResponse) -> Result<Response, ApiError> {
    let fields = profile.fields();
    let etag = HeaderValue::from_str(&format!(
        "\"media-profile:{}:v{}\"",
        fields.media_profile_public_id, fields.latest_version
    ))
    .map_err(|_| ApiError::internal("invalid media profile representation"))?;
    let mut response = Json(profile).into_response();
    response.headers_mut().insert(header::ETAG, etag);
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::response;
    use crate::models::media_root_contract::ProfileVersionResponse;
    use axum::{body::to_bytes, http::header};

    #[tokio::test]
    async fn response_fences_latest_head_and_preserves_path_free_body() -> anyhow::Result<()> {
        let profile: ProfileVersionResponse = serde_json::from_value(serde_json::json!({
            "media_profile_public_id": "00000000-0000-0000-0000-000000000000",
            "profile_key": "movies", "display_name": " Exact name ", "description": "",
            "enabled": false, "dry_run_only": true, "desired_target_key": "target", "desired_target_version": 2,
            "policy_key": "safe-dry-run", "policy_version": 3, "output_root_key": "output", "workspace_root_key": "workspace",
            "latest_version": 4, "active_version": 3, "lifecycle_state": "draft",
            "created_at": "2026-10-01T00:00:00Z", "updated_at": "2026-10-01T00:00:00Z",
            "root_bindings": [
                {"kind": "output", "logical_key": "output", "resolution_state": "unmapped",
                    "binding_ready": false, "binding_reason": "media_root_binding_incomplete",
                    "destructive_ready": false, "destructive_reason": "media_root_binding_incomplete"},
                {"kind": "workspace", "logical_key": "workspace", "resolution_state": "unmapped",
                    "binding_ready": false, "binding_reason": "media_root_binding_incomplete",
                    "destructive_ready": false, "destructive_reason": "media_root_binding_incomplete"}
            ]
        }))?;
        let created = super::created(profile.clone())?;
        assert_eq!(created.status(), axum::http::StatusCode::CREATED);
        assert_eq!(
            created.headers()[header::LOCATION],
            "/v1/media/profiles/00000000-0000-0000-0000-000000000000"
        );
        assert_eq!(
            created.headers()[header::ETAG],
            "\"media-profile:00000000-0000-0000-0000-000000000000:v4\""
        );
        assert_eq!(created.headers()[header::CACHE_CONTROL], "no-store");
        let expected = serde_json::to_value(&profile)?;
        let response = response(profile)?;
        assert_eq!(
            response.headers()[header::ETAG],
            "\"media-profile:00000000-0000-0000-0000-000000000000:v4\""
        );
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let body = to_bytes(response.into_body(), 16384).await?;
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&body)?,
            expected
        );
        Ok(())
    }
}
