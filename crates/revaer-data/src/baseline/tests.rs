use std::error::Error;

use chrono::{DateTime, Utc};
use sqlx::Connection;

use super::row::{BaselineRow, validate_role, verify};
use super::{BaselineReadError, BaselineReadReason, READ_BASELINE, read_runtime_baseline};

mod logging;

const DIGEST: [u8; 32] = [0x5a; 32];

fn row() -> BaselineRow {
    BaselineRow {
        contract_version: 1,
        init_sha256: DIGEST.to_vec(),
        postgres_version_num: 180_006,
        schema_owner_role: "fixture_owner".to_owned(),
        runtime_role: "fixture_runtime".to_owned(),
        sealed_at: DateTime::<Utc>::UNIX_EPOCH,
    }
}

#[test]
fn baseline_read_verifies_exact_identity_without_exposing_principals() -> anyhow::Result<()> {
    let verified = verify(vec![row()], &DIGEST, "fixture_runtime")
        .map_err(|reason| anyhow::anyhow!(reason.code()))?;
    assert_eq!(verified.contract_version(), 1);
    assert_eq!(verified.postgres_version_num(), 180_006);
    assert_eq!(verified.init_sha256(), &DIGEST);
    assert_eq!(verified.sealed_at(), DateTime::<Utc>::UNIX_EPOCH);
    let debug = format!("{verified:?}");
    assert!(!debug.contains("fixture_owner"));
    assert!(!debug.contains("fixture_runtime"));
    Ok(())
}

#[test]
fn baseline_read_rejects_missing_and_duplicate_rows() {
    for rows in [Vec::new(), vec![row(), row()]] {
        assert_eq!(
            verify(rows, &DIGEST, "fixture_runtime"),
            Err(BaselineReadReason::BaselineShapeInvalid)
        );
    }
    assert!(READ_BASELINE.ends_with(" LIMIT 2"));
    assert!(READ_BASELINE.contains("FROM revaer_system.read_database_baseline_v1()"));
}

#[test]
fn baseline_read_rejects_other_contract_versions() {
    for contract_version in [i16::MIN, -1, 0, 2, i16::MAX] {
        let mut invalid = row();
        invalid.contract_version = contract_version;
        assert_eq!(
            verify(vec![invalid], &DIGEST, "fixture_runtime"),
            Err(BaselineReadReason::BaselineContractUnsupported)
        );
    }
}

#[test]
fn baseline_read_requires_exact_digest_length_and_bytes() {
    for length in [0, 1, 31, 33, 64] {
        let mut invalid = row();
        invalid.init_sha256 = vec![0x5a; length];
        assert_eq!(
            verify(vec![invalid], &DIGEST, "fixture_runtime"),
            Err(BaselineReadReason::BaselineShapeInvalid)
        );
    }
    for byte in 0..DIGEST.len() {
        let mut invalid = row();
        invalid.init_sha256[byte] ^= 1;
        assert_eq!(
            verify(vec![invalid], &DIGEST, "fixture_runtime"),
            Err(BaselineReadReason::BaselineDigestMismatch)
        );
    }
}

#[test]
fn baseline_read_rejects_other_postgres_versions() {
    for postgres_version_num in [0, 160_014, 170_000, 180_005, 180_007, 190_000] {
        let mut invalid = row();
        invalid.postgres_version_num = postgres_version_num;
        assert_eq!(
            verify(vec![invalid], &DIGEST, "fixture_runtime"),
            Err(BaselineReadReason::PostgresIdentityUnsupported)
        );
    }
}

#[test]
fn baseline_read_rejects_bad_role_shapes_and_shared_owner() {
    for role in [String::new(), "x".repeat(64), "role\0suffix".to_owned()] {
        let mut invalid = row();
        invalid.schema_owner_role.clone_from(&role);
        assert_eq!(
            verify(vec![invalid], &DIGEST, "fixture_runtime"),
            Err(BaselineReadReason::BaselineShapeInvalid)
        );
        let mut invalid = row();
        invalid.runtime_role = role;
        assert_eq!(
            verify(vec![invalid], &DIGEST, "fixture_runtime"),
            Err(BaselineReadReason::BaselineShapeInvalid)
        );
    }
    let mut invalid = row();
    invalid.schema_owner_role.clone_from(&invalid.runtime_role);
    assert_eq!(
        verify(vec![invalid], &DIGEST, "fixture_runtime"),
        Err(BaselineReadReason::BaselineShapeInvalid)
    );
}

#[test]
fn baseline_read_never_normalizes_or_substitutes_a_login() {
    for login in [
        "fixture_owner",
        "FIXTURE_RUNTIME",
        " fixture_runtime",
        "unrelated",
    ] {
        assert_eq!(
            verify(vec![row()], &DIGEST, login),
            Err(BaselineReadReason::BaselineRuntimeRoleMismatch)
        );
    }
    for login in [
        " quoted role ".to_owned(),
        format!("{}x", "\u{00e9}".repeat(31)),
    ] {
        let mut valid = row();
        valid.runtime_role.clone_from(&login);
        assert!(verify(vec![valid], &DIGEST, &login).is_ok());
    }
}

#[test]
fn baseline_read_role_limits_are_utf8_bytes_not_characters() {
    for valid in ["x".to_owned(), "x".repeat(63), "\u{00e9}".repeat(31)] {
        assert_eq!(validate_role(&valid), Ok(()));
    }
    for invalid in [
        String::new(),
        "x".repeat(64),
        "\u{00e9}".repeat(32),
        "\0".to_owned(),
    ] {
        assert_eq!(validate_role(&invalid), Err(()));
    }
}

#[test]
fn baseline_read_errors_expose_only_closed_codes_and_sqlstate() {
    let reasons = [
        (
            BaselineReadReason::InvalidConfiguration,
            "invalid_configuration",
        ),
        (
            BaselineReadReason::PostgresIdentityUnsupported,
            "postgres_identity_unsupported",
        ),
        (
            BaselineReadReason::BaselineShapeInvalid,
            "baseline_shape_invalid",
        ),
        (
            BaselineReadReason::BaselineContractUnsupported,
            "baseline_contract_unsupported",
        ),
        (
            BaselineReadReason::BaselineDigestMismatch,
            "baseline_digest_mismatch",
        ),
        (
            BaselineReadReason::BaselineRuntimeRoleMismatch,
            "baseline_runtime_role_mismatch",
        ),
        (
            BaselineReadReason::BaselineReadDenied,
            "baseline_read_denied",
        ),
        (BaselineReadReason::StatementFailed, "statement_failed"),
        (
            BaselineReadReason::PoolAcquireTimeout,
            "pool_acquire_timeout",
        ),
        (BaselineReadReason::StatementTimeout, "statement_timeout"),
        (BaselineReadReason::CleanupFailed, "cleanup_failed"),
    ];
    for (reason, code) in reasons {
        let error = BaselineReadError::new(reason);
        assert_eq!(reason.code(), code);
        assert_eq!(error.to_string(), code);
        assert_eq!(error.reason(), reason);
        assert_eq!(error.sqlstate(), None);
        assert!(error.source().is_none());
    }
    let private_url = format!("postgres://{}:{}@host/db", "private", "secret");
    for detail in [private_url.as_str(), "private_owner", "SELECT secret"] {
        let error = BaselineReadError::from_database(Some("P0001"), Some(detail));
        assert_eq!(error.reason(), BaselineReadReason::StatementFailed);
        assert_eq!(error.sqlstate(), Some("P0001"));
        assert!(!format!("{error:?}").contains(detail));
        assert!(!error.to_string().contains(detail));
        assert!(error.source().is_none());
    }
}

#[test]
fn baseline_read_maps_only_known_server_failure_details() {
    for reason in [
        BaselineReadReason::PostgresIdentityUnsupported,
        BaselineReadReason::BaselineShapeInvalid,
        BaselineReadReason::BaselineContractUnsupported,
        BaselineReadReason::BaselineDigestMismatch,
        BaselineReadReason::BaselineRuntimeRoleMismatch,
        BaselineReadReason::BaselineReadDenied,
    ] {
        assert_eq!(
            BaselineReadError::from_database(Some("P0001"), Some(reason.code())).reason(),
            reason
        );
        assert_eq!(
            BaselineReadError::from_database(Some("XX000"), Some(reason.code())).reason(),
            BaselineReadReason::StatementFailed
        );
    }
    for code in ["42883", "42P01", "42703", "3F000", "42704"] {
        let error = BaselineReadError::from_database(Some(code), None);
        assert_eq!(error.reason(), BaselineReadReason::BaselineShapeInvalid);
        assert_eq!(error.sqlstate(), Some(code));
    }
    assert_eq!(
        BaselineReadError::from_database(Some("42501"), None).reason(),
        BaselineReadReason::BaselineReadDenied
    );
    // Query cancellation and statement timeout share SQLSTATE; only the owning
    // lifecycle can classify the cause from its cancellation/deadline state.
    assert_eq!(
        BaselineReadError::from_database(Some("57014"), None).reason(),
        BaselineReadReason::StatementFailed
    );
}

#[test]
fn baseline_read_drops_malformed_sqlstate_without_accepting_failure() {
    for code in [
        None,
        Some(""),
        Some("ABCDE\n"),
        Some("abcde"),
        Some("secret-sqlstate"),
    ] {
        let error = BaselineReadError::from_database(code, None);
        assert_eq!(error.reason(), BaselineReadReason::StatementFailed);
        assert_eq!(error.sqlstate(), None);
    }
    assert_eq!(
        BaselineReadError::from_database(Some("P0001"), None).reason(),
        BaselineReadReason::StatementFailed
    );
}

#[test]
fn baseline_read_classifies_projection_and_transport_errors_without_raw_sources() {
    let errors = [
        sqlx::Error::RowNotFound,
        sqlx::Error::ColumnNotFound("private_column".to_owned()),
        sqlx::Error::ColumnIndexOutOfBounds { index: 1, len: 0 },
        sqlx::Error::ColumnDecode {
            index: "private_column".to_owned(),
            source: std::io::Error::other("private_value").into(),
        },
        sqlx::Error::Decode(std::io::Error::other("private_value").into()),
        sqlx::Error::TypeNotFound {
            type_name: "private_type".to_owned(),
        },
    ];
    for source in errors {
        let error = BaselineReadError::from_query(&source);
        assert_eq!(error.reason(), BaselineReadReason::BaselineShapeInvalid);
        assert!(!format!("{error:?}").contains("private_"));
        assert!(error.source().is_none());
    }
    let private_url = format!("postgres://{}:{}@host/db", "private", "secret");
    let error = BaselineReadError::from_query(&sqlx::Error::Io(std::io::Error::other(private_url)));
    assert_eq!(error.reason(), BaselineReadReason::StatementFailed);
    assert!(!format!("{error:?}").contains("private"));
    assert!(error.source().is_none());
}

#[tokio::test]
async fn baseline_read_refuses_an_unmanaged_database_and_bad_expectation() -> anyhow::Result<()> {
    let database = revaer_test_support::postgres::start_postgres()?;
    let mut connection = sqlx::PgConnection::connect(database.connection_string()).await?;
    let invalid = read_runtime_baseline(&mut connection, &DIGEST, "").await;
    let missing = read_runtime_baseline(&mut connection, &DIGEST, "fixture_runtime").await;
    sqlx::raw_sql(include_str!(
        "../../../../scripts/tests/database-baseline-malformed-projection.sql"
    ))
    .execute(&mut connection)
    .await?;
    let malformed = read_runtime_baseline(&mut connection, &DIGEST, "2").await;
    connection.close().await?;

    assert_eq!(
        invalid.map_err(|error| error.reason()),
        Err(BaselineReadReason::InvalidConfiguration)
    );
    let missing = missing.map_err(|error| (error.reason(), error.sqlstate().map(str::to_owned)));
    assert_eq!(
        missing,
        Err((
            BaselineReadReason::BaselineShapeInvalid,
            Some("3F000".to_owned())
        ))
    );
    assert_eq!(
        malformed.map_err(|error| error.reason()),
        Err(BaselineReadReason::BaselineShapeInvalid)
    );
    Ok(())
}
