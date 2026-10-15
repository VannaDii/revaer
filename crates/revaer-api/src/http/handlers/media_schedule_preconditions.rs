//! Schedule revision validation before decoding a replacement body.

use crate::{http::errors::ApiError, models::media_schedule::parse_schedule_etag};
use axum::{
    extract::FromRequestParts,
    http::{header::IF_MATCH, request::Parts},
};

pub(crate) struct ReplaceSchedulePrecondition {
    pub(crate) id: uuid::Uuid,
    pub(crate) version: i32,
    pub(crate) revision: i64,
}

impl<S: Send + Sync> FromRequestParts<S> for ReplaceSchedulePrecondition {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let mut values = parts.headers.get_all(IF_MATCH).iter();
        let Some(value) = values.next() else {
            return Err(ApiError::media_replace_precondition_required());
        };
        let invalid = || ApiError::bad_request("invalid schedule replacement precondition");
        if values.next().is_some() {
            return Err(invalid());
        }
        let (id, version, revision) =
            parse_schedule_etag(value.to_str().map_err(|_| invalid())?).map_err(|_| invalid())?;
        Ok(Self {
            id,
            version,
            revision,
        })
    }
}
