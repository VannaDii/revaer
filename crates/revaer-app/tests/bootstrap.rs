use std::ffi::{OsStr, OsString};
use std::future::Future;
use std::pin::pin;
use std::process::Command;
use std::task::{Context, Poll, Waker};

use revaer_api::app::compliance::{SOURCE_COMPLIANCE_BUNDLE_PATH, SourceComplianceMetadata};
use revaer_app::{AppError, AppResult, run_app, run_app_with_database_url};

fn cleared_child_command(mut command: Command, llvm_profile_file: Option<OsString>) -> Command {
    command.env_clear();
    if let Some(profile_file) = llvm_profile_file {
        command.env("LLVM_PROFILE_FILE", profile_file);
    }
    command
}

#[test]
fn cleared_child_commands_only_restore_supplied_llvm_profile_file() {
    for profile_file in [
        Some(OsString::from("coverage/child profiles-%p-%m.profraw")),
        Some(OsString::new()),
        None,
    ] {
        let mut command = Command::new("unused-test-child");
        command
            .env("REVAER_TEST_SECRET", "must-not-be-inherited")
            .env("LLVM_PROFILE_FILE", "must-not-be-reused");
        let command = cleared_child_command(command, profile_file.clone());
        let expected: Vec<_> = profile_file
            .as_deref()
            .map(|value| (OsStr::new("LLVM_PROFILE_FILE"), Some(value)))
            .into_iter()
            .collect();
        assert_eq!(command.get_envs().collect::<Vec<_>>(), expected);
    }
}

#[test]
fn public_entrypoints_require_packaged_metadata_before_infrastructure() -> anyhow::Result<()> {
    if std::env::var_os("REVAER_C1_PUBLIC_ENTRYPOINT_CHILD").is_none() {
        let output = cleared_child_command(
            Command::new(std::env::current_exe()?),
            std::env::var_os("LLVM_PROFILE_FILE"),
        )
        .env("REVAER_C1_PUBLIC_ENTRYPOINT_CHILD", "1")
        .env("DATABASE_URL", "not-a-database-url")
        .env("REVAER_E2E_SERVING_ENTRY", "1")
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
        Ok(_) => assert!(matches!(result, Poll::Ready(Err(AppError::Config { .. })))),
    }
    Ok(())
}

#[test]
fn real_process_exits_nonzero_with_one_bounded_origin_diagnostic() -> anyhow::Result<()> {
    let output = cleared_child_command(
        Command::new(env!("CARGO_BIN_EXE_revaer-app")),
        std::env::var_os("LLVM_PROFILE_FILE"),
    )
    .env("DATABASE_URL", "not-a-database-url")
    .env("REVAER_E2E_SERVING_ENTRY", "1")
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
        // A packaged host can pass preflight. The deliberately invalid database
        // URL then fails configuration before infrastructure starts.
        Ok(_) => {
            assert!(!diagnostic.contains("compliance_metadata_startup_failed"));
            assert!(diagnostic.contains("configuration operation failed"));
        }
    }
    Ok(())
}
