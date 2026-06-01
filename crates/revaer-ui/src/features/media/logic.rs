//! Pure helpers for the media UI feature.

use uuid::Uuid;

pub(crate) fn media_jobs_path(media_profile_public_id: Option<Uuid>) -> String {
    media_profile_public_id.map_or_else(
        || "/v1/media/jobs".to_string(),
        |profile_id| format!("/v1/media/jobs?media_profile_public_id={profile_id}"),
    )
}

#[cfg(test)]
mod tests {
    use super::media_jobs_path;
    use uuid::Uuid;

    #[test]
    fn media_jobs_path_includes_profile_filter() {
        let profile_id = Uuid::from_u128(1);

        assert_eq!(
            media_jobs_path(Some(profile_id)),
            format!("/v1/media/jobs?media_profile_public_id={profile_id}")
        );
    }

    #[test]
    fn media_jobs_path_without_profile_uses_collection_route() {
        assert_eq!(media_jobs_path(None), "/v1/media/jobs");
    }
}
