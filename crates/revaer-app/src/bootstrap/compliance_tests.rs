use super::*;
use std::future::Future;
use std::path::Path;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

pub(super) fn fixture_metadata() -> Result<SourceComplianceMetadata, ComplianceMetadataError> {
    let mut file = tempfile::NamedTempFile::new()
        .map_err(|source| ComplianceMetadataError::Read { source })?;
    write!(
        file,
        "{{\"source_compliance_sha256\":\"{}\"}}",
        "AB".repeat(32)
    )
    .map_err(|source| ComplianceMetadataError::Read { source })?;
    SourceComplianceMetadata::load(file.path())
}

fn assert_preflight_failure(path: &Path, category: &str) -> anyhow::Result<()> {
    for database_url in [None, Some("not-a-database-url".to_string())] {
        let mut diagnostic = Vec::new();
        let mut loads = 0;
        {
            let future = run_app_with_compliance_loader(
                database_url,
                || {
                    loads += 1;
                    SourceComplianceMetadata::load(path)
                },
                &mut diagnostic,
            );
            let mut future = pin!(future);
            // No Tokio runtime exists. The actual bootstrap must return on its
            // first poll, before any database, native session or worker can run.
            let Poll::Ready(Err(AppError::Compliance { source })) = future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop()))
            else {
                anyhow::bail!("preflight did not return a typed failure synchronously");
            };
            assert_eq!(source.category(), category);
            assert_eq!(
                source.to_string(),
                format!("compliance metadata failed: {category}")
            );
            assert_eq!(
                std::error::Error::source(&source).is_some(),
                matches!(
                    &source,
                    ComplianceMetadataError::Read { .. }
                        | ComplianceMetadataError::MalformedJson { .. }
                )
            );
        }
        assert_eq!(loads, 1);
        assert_eq!(
            String::from_utf8(diagnostic)?,
            format!("compliance_metadata_startup_failed cause={category}\n")
        );
    }
    Ok(())
}

#[test]
fn compliance_real_file_failures_precede_infrastructure_once() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    assert_preflight_failure(
        &directory.path().join("missing-private-name"),
        "missing_file",
    )?;
    assert_preflight_failure(directory.path(), "unreadable_file")?;
    let path = directory.path().join("private-manifest");
    for (contents, category) in [
        ("{private-source-content", "malformed_json"),
        ("{}", "missing_digest"),
        ("[]", "missing_digest"),
        ("null", "missing_digest"),
        (r#"{"source_compliance_sha256":null}"#, "wrong_digest_type"),
        (r#"{"source_compliance_sha256":17}"#, "wrong_digest_type"),
        (r#"{"source_compliance_sha256":true}"#, "wrong_digest_type"),
        (r#"{"source_compliance_sha256":[]}"#, "wrong_digest_type"),
        (r#"{"source_compliance_sha256":{}}"#, "wrong_digest_type"),
        (r#"{"source_compliance_sha256":""}"#, "invalid_digest"),
        (
            r#"{"source_compliance_sha256":"private-invalid-digest"}"#,
            "invalid_digest",
        ),
    ] {
        std::fs::write(&path, contents)?;
        assert_preflight_failure(&path, category)?;
    }
    for digest in [
        "a".repeat(63),
        "b".repeat(65),
        "g".repeat(64),
        "\u{e9}".repeat(32),
    ] {
        std::fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({
                "source_compliance_sha256": digest,
            }))?,
        )?;
        assert_preflight_failure(&path, "invalid_digest")?;
    }
    std::fs::write(&path, [0xff])?;
    assert_preflight_failure(&path, "unreadable_file")?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn compliance_permission_failure_is_a_real_read_error() -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let file = tempfile::NamedTempFile::new()?;
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o0))?;
    let result = assert_preflight_failure(file.path(), "unreadable_file");
    std::fs::set_permissions(file.path(), std::fs::Permissions::from_mode(0o600))?;
    result
}

#[test]
fn compliance_success_preserves_trim_case_and_json_behavior() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("bundle");
    for digest in [
        "ab".repeat(32),
        "AB".repeat(32),
        format!(" \t{}\n", "aB".repeat(32)),
    ] {
        std::fs::write(
            &path,
            serde_json::to_vec(&serde_json::json!({
                "source_compliance_sha256": digest,
                "unrelated": "not-certification",
            }))?,
        )?;
        let loaded = SourceComplianceMetadata::load(&path)?;
        assert_eq!(loaded.digest(), format!("sha256:{}", "ab".repeat(32)));
    }
    std::fs::write(
        &path,
        format!(
            "{{\"source_compliance_sha256\":false,\"source_compliance_sha256\":\"{}\"}}",
            "AB".repeat(32),
        ),
    )?;
    assert_eq!(
        SourceComplianceMetadata::load(&path)?.digest(),
        fixture_metadata()?.digest()
    );
    #[cfg(unix)]
    {
        let link = directory.path().join("bundle-link");
        std::os::unix::fs::symlink(&path, &link)?;
        assert_eq!(
            SourceComplianceMetadata::load(&link)?.digest(),
            fixture_metadata()?.digest()
        );
    }
    Ok(())
}

#[test]
fn compliance_diagnostic_failure_is_not_silenced_or_retried() -> anyhow::Result<()> {
    struct BrokenSink(usize);
    impl Write for BrokenSink {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            self.0 += 1;
            Err(std::io::ErrorKind::BrokenPipe.into())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let directory = tempfile::tempdir()?;
    let mut sink = BrokenSink(0);
    {
        let future = run_app_with_compliance_loader(
            None,
            || SourceComplianceMetadata::load(&directory.path().join("missing")),
            &mut sink,
        );
        let mut future = pin!(future);
        let Poll::Ready(Err(AppError::ComplianceDiagnostic { compliance, source })) = future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop()))
        else {
            anyhow::bail!("expected diagnostic failure");
        };
        assert_eq!(compliance.category(), "missing_file");
        assert_eq!(source.kind(), std::io::ErrorKind::BrokenPipe);
    }
    assert_eq!(sink.0, 1);
    Ok(())
}
