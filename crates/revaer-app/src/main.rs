#![forbid(unsafe_code)]
#![deny(
    warnings,
    dead_code,
    unused,
    unused_imports,
    unused_must_use,
    unreachable_pub,
    clippy::all,
    clippy::pedantic,
    rustdoc::broken_intra_doc_links,
    rustdoc::bare_urls,
    missing_docs
)]

//! Binary entrypoint that wires the Revaer services together and launches the
//! async orchestrators.

use revaer_app::{AppError, AppResult, run_app_with_database_url};
use std::process::{ExitCode, Termination};

/// Bootstraps the Revaer application and blocks until shutdown.
#[tokio::main]
async fn main() -> ExitCode {
    startup_exit_code(run_entrypoint().await)
}

fn startup_exit_code(result: AppResult<()>) -> ExitCode {
    match result {
        // The synchronous preflight already attempted its one bounded diagnostic.
        Err(AppError::Compliance { .. } | AppError::ComplianceDiagnostic { .. }) => {
            ExitCode::FAILURE
        }
        other => other.report(),
    }
}

async fn run_entrypoint() -> AppResult<()> {
    run_entrypoint_with(std::env::var("DATABASE_URL").ok()).await
}

async fn run_entrypoint_with(database_url: Option<String>) -> AppResult<()> {
    let database_url = database_url.ok_or(AppError::MissingEnv {
        name: "DATABASE_URL",
    })?;
    run_app_with_database_url(database_url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compliance_errors_exit_unsuccessfully_without_error_reporting() {
        use revaer_api::app::compliance::ComplianceMetadataError;
        for source in [
            ComplianceMetadataError::Read {
                source: std::io::ErrorKind::NotFound.into(),
            },
            ComplianceMetadataError::Read {
                source: std::io::ErrorKind::PermissionDenied.into(),
            },
            ComplianceMetadataError::MissingDigest,
            ComplianceMetadataError::WrongDigestType,
            ComplianceMetadataError::InvalidDigest,
        ] {
            assert_eq!(
                startup_exit_code(Err(AppError::Compliance { source })),
                ExitCode::FAILURE
            );
        }
        let json_error = serde_json::from_str::<serde_json::Value>("{");
        assert!(json_error.is_err());
        if let Err(source) = json_error {
            assert_eq!(
                startup_exit_code(Err(AppError::Compliance {
                    source: ComplianceMetadataError::MalformedJson { source },
                })),
                ExitCode::FAILURE
            );
        }
        assert_eq!(
            startup_exit_code(Err(AppError::ComplianceDiagnostic {
                compliance: ComplianceMetadataError::MissingDigest,
                source: std::io::ErrorKind::BrokenPipe.into(),
            })),
            ExitCode::FAILURE
        );
        assert_eq!(startup_exit_code(Ok(())), ExitCode::SUCCESS);
    }

    #[tokio::test]
    async fn run_entrypoint_requires_database_url() {
        let result = run_entrypoint_with(None).await;
        assert!(matches!(
            result,
            Err(AppError::MissingEnv {
                name: "DATABASE_URL"
            })
        ));
    }

    #[tokio::test]
    async fn run_entrypoint_with_database_url_surfaces_bootstrap_error() {
        let result = run_entrypoint_with(Some("not-a-url".to_string())).await;
        assert!(
            result.is_err(),
            "invalid database url should fail bootstrap"
        );
        assert!(
            !matches!(result, Err(AppError::MissingEnv { .. })),
            "bootstrap path should be exercised once a URL is present"
        );
    }

    #[tokio::test]
    async fn run_entrypoint_requires_process_database_url_when_unset() {
        if std::env::var("DATABASE_URL").is_ok() {
            return;
        }

        let result = run_entrypoint().await;
        assert!(matches!(
            result,
            Err(AppError::MissingEnv {
                name: "DATABASE_URL"
            })
        ));
    }
}
