//! Shared attempt identity for workspace creation and retention protection.

pub(crate) fn workspace_key(job_id: uuid::Uuid, attempt: i32, generation: i64) -> String {
    format!("{job_id}-attempt-{attempt}-claim-{generation}")
}

#[cfg(test)]
mod tests {
    use super::workspace_key;

    #[test]
    fn attempts_and_claim_generations_cannot_alias() {
        let job = uuid::Uuid::from_u128(1);
        assert_eq!(
            workspace_key(job, 1, 1),
            "00000000-0000-0000-0000-000000000001-attempt-1-claim-1"
        );
        assert_ne!(workspace_key(job, 1, 1), workspace_key(job, 2, 1));
        assert_ne!(workspace_key(job, 1, 1), workspace_key(job, 1, 2));
        assert_ne!(
            workspace_key(job, 1, 1),
            workspace_key(uuid::Uuid::from_u128(2), 1, 1)
        );
    }

    #[test]
    fn retry_preserves_prior_workspace_diagnostics() -> anyhow::Result<()> {
        let temp = tempfile::tempdir()?;
        let root = temp.path().join("workspace");
        let job = uuid::Uuid::from_u128(1);
        let prior = revaer_media_runtime::workspace::create_managed_workspace(
            &root,
            &workspace_key(job, 1, 1),
        )?;
        let evidence = prior.diagnostics_path.join("failure.txt");
        std::fs::write(&evidence, b"prior failure")?;
        let retry = revaer_media_runtime::workspace::create_managed_workspace(
            &root,
            &workspace_key(job, 2, 2),
        )?;
        assert_ne!(prior.job_path, retry.job_path);
        assert_eq!(std::fs::read(evidence)?, b"prior failure");
        Ok(())
    }
}
