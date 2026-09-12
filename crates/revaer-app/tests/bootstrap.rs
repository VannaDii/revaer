use std::future::Future;
use std::pin::pin;
use std::process::Command;
use std::task::{Context, Poll, Waker};

use revaer_api::app::compliance::{SOURCE_COMPLIANCE_BUNDLE_PATH, SourceComplianceMetadata};
use revaer_app::{AppError, AppResult, run_app, run_app_with_database_url};

#[test]
fn public_entrypoints_require_packaged_metadata_before_infrastructure() -> anyhow::Result<()> {
    if std::env::var_os("REVAER_C1_PUBLIC_ENTRYPOINT_CHILD").is_none() {
        let output = Command::new(std::env::current_exe()?)
            .env_clear()
            .env("REVAER_C1_PUBLIC_ENTRYPOINT_CHILD", "1")
            .env("DATABASE_URL", "not-a-database-url")
            .args([
                "--exact",
                "public_entrypoints_require_packaged_metadata_before_infrastructure",
            ])
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8(output.stdout)?
        );
        return Ok(());
    }
    let mut context = Context::from_waker(Waker::noop());
    let mut from_env = pin!(run_app());
    assert_preflight_result(from_env.as_mut().poll(&mut context))?;
    let mut from_url = pin!(run_app_with_database_url("not-a-database-url".to_string()));
    assert_preflight_result(from_url.as_mut().poll(&mut context))?;
    Ok(())
}

fn assert_preflight_result(result: Poll<AppResult<()>>) -> anyhow::Result<()> {
    match SourceComplianceMetadata::load(std::path::Path::new(SOURCE_COMPLIANCE_BUNDLE_PATH)) {
        Err(expected) => {
            let Poll::Ready(Err(AppError::Compliance { source })) = result else {
                anyhow::bail!("metadata failure did not stop the actual bootstrap synchronously");
            };
            assert_eq!(source.category(), expected.category());
        }
        Ok(_) => assert!(matches!(
            result,
            Poll::Ready(Err(AppError::InvalidConfig {
                field: "REVAER_MEDIA_WORKSPACE_ROOT",
                ..
            }))
        )),
    }
    Ok(())
}

#[test]
fn real_process_exits_nonzero_with_one_bounded_origin_diagnostic() -> anyhow::Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_revaer-app"))
        .env_clear()
        .env("DATABASE_URL", "not-a-database-url")
        .output()?;
    assert!(!output.status.success());
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let diagnostic = String::from_utf8(output.stderr)?;
    match SourceComplianceMetadata::load(std::path::Path::new(SOURCE_COMPLIANCE_BUNDLE_PATH)) {
        Err(expected) => assert_eq!(
            diagnostic,
            format!(
                "compliance_metadata_startup_failed cause={}\n",
                expected.category()
            )
        ),
        // A genuinely packaged host can pass preflight. The cleared environment
        // then rejects workspace configuration before infrastructure starts.
        Ok(_) => {
            assert!(!diagnostic.contains("compliance_metadata_startup_failed"));
            assert!(diagnostic.contains("REVAER_MEDIA_WORKSPACE_ROOT"));
        }
    }
    Ok(())
}
