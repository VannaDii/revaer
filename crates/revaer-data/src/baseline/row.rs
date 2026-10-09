use chrono::{DateTime, Utc};
use sqlx::FromRow;

use super::BaselineReadReason;

const CONTRACT_VERSION: i16 = 1;
const POSTGRES_VERSION_NUM: i32 = 160_014;

#[derive(FromRow)]
pub(super) struct BaselineRow {
    pub(super) contract_version: i16,
    pub(super) init_sha256: Vec<u8>,
    pub(super) postgres_version_num: i32,
    pub(super) schema_owner_role: String,
    pub(super) runtime_role: String,
    pub(super) sealed_at: DateTime<Utc>,
}

/// A single baseline row whose digest, version, and runtime login were verified.
///
/// This value contains no credentials or role names. It is not a substitute for
/// the bootstrap lifecycle's connection identity, catalog, privilege, deadline,
/// and cancellation checks, and does not attest arbitrary later schema changes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedBaseline {
    contract_version: i16,
    init_sha256: [u8; 32],
    postgres_version_num: i32,
    sealed_at: DateTime<Utc>,
}

impl VerifiedBaseline {
    /// Return the verified contract version.
    #[must_use]
    pub const fn contract_version(&self) -> i16 {
        self.contract_version
    }

    /// Return the exact sealed init-byte digest.
    #[must_use]
    pub const fn init_sha256(&self) -> &[u8; 32] {
        &self.init_sha256
    }

    /// Return the admitted `PostgreSQL` version number recorded by the baseline.
    #[must_use]
    pub const fn postgres_version_num(&self) -> i32 {
        self.postgres_version_num
    }

    /// Return the baseline transaction's seal time.
    #[must_use]
    pub const fn sealed_at(&self) -> DateTime<Utc> {
        self.sealed_at
    }
}

pub(super) fn validate_role(role: &str) -> Result<(), ()> {
    if (1..=63).contains(&role.len()) && !role.contains('\0') {
        Ok(())
    } else {
        Err(())
    }
}

pub(super) fn verify(
    rows: Vec<BaselineRow>,
    expected_digest: &[u8; 32],
    runtime_login: &str,
) -> Result<VerifiedBaseline, BaselineReadReason> {
    let [row]: [BaselineRow; 1] = rows
        .try_into()
        .map_err(|_| BaselineReadReason::BaselineShapeInvalid)?;
    if row.contract_version != CONTRACT_VERSION {
        return Err(BaselineReadReason::BaselineContractUnsupported);
    }
    let init_sha256: [u8; 32] = row
        .init_sha256
        .try_into()
        .map_err(|_| BaselineReadReason::BaselineShapeInvalid)?;
    if row.postgres_version_num != POSTGRES_VERSION_NUM {
        return Err(BaselineReadReason::PostgresIdentityUnsupported);
    }
    validate_role(&row.schema_owner_role)
        .and_then(|()| validate_role(&row.runtime_role))
        .map_err(|()| BaselineReadReason::BaselineShapeInvalid)?;
    if row.schema_owner_role == row.runtime_role {
        return Err(BaselineReadReason::BaselineShapeInvalid);
    }
    if &init_sha256 != expected_digest {
        return Err(BaselineReadReason::BaselineDigestMismatch);
    }
    if row.runtime_role != runtime_login {
        return Err(BaselineReadReason::BaselineRuntimeRoleMismatch);
    }
    Ok(VerifiedBaseline {
        contract_version: row.contract_version,
        init_sha256,
        postgres_version_num: row.postgres_version_num,
        sealed_at: row.sealed_at,
    })
}
