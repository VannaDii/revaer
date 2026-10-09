//! Exact scalar spellings for catalog transport, without normalization.

use chrono::DateTime;

use super::RootCatalogError;

pub(super) fn decimal(value: &str, positive: bool) -> Result<u64, RootCatalogError> {
    if value.is_empty() || value.len() > 19 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(RootCatalogError);
    }
    let parsed = value.parse::<u64>().map_err(|_| RootCatalogError)?;
    if parsed > i64::MAX as u64 || (positive && parsed == 0) || parsed.to_string() != value {
        return Err(RootCatalogError);
    }
    Ok(parsed)
}

pub(super) fn hex(value: &str, length: usize) -> Result<(), RootCatalogError> {
    if value.len() != length
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(RootCatalogError);
    }
    Ok(())
}

pub(super) fn timestamp(value: &str) -> Result<(), RootCatalogError> {
    if !(value.ends_with('Z') || value.ends_with("+00:00")) {
        return Err(RootCatalogError);
    }
    DateTime::parse_from_rfc3339(value).map_err(|_| RootCatalogError)?;
    Ok(())
}

pub(super) fn path(value: &str) -> Result<(), RootCatalogError> {
    if !value.starts_with('/') || value == "/" || value.len() > 4_096 || value.contains('\0') {
        return Err(RootCatalogError);
    }
    Ok(())
}
