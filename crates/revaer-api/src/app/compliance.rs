//! Required source-compliance metadata, not authenticity or package certification.

use std::path::Path;

/// Existing packaged metadata location. Production startup has no override.
pub const SOURCE_COMPLIANCE_BUNDLE_PATH: &str =
    "/app/compliance/final-image-compliance-bundle.json";

/// A successfully loaded manifest digest, checked only for the existing syntax.
#[derive(Debug)]
pub struct SourceComplianceMetadata {
    digest: String,
}

/// Bounded classifications of required metadata failures.
#[derive(Debug)]
pub enum ComplianceMetadataError {
    /// The required file could not be read as UTF-8.
    Read {
        /// Original filesystem failure.
        source: std::io::Error,
    },
    /// The document was not valid JSON.
    MalformedJson {
        /// Original parser failure, never included in the startup diagnostic.
        source: serde_json::Error,
    },
    /// The existing digest field was absent.
    MissingDigest,
    /// The digest field was not a string.
    WrongDigestType,
    /// The trimmed digest was not 64 ASCII hexadecimal bytes.
    InvalidDigest,
}

impl ComplianceMetadataError {
    /// Stable diagnostic category that cannot include document or path contents.
    #[must_use]
    pub fn category(&self) -> &'static str {
        match self {
            Self::Read { source } if matches!(source.kind(), std::io::ErrorKind::NotFound) => {
                "missing_file"
            }
            Self::Read { .. } => "unreadable_file",
            Self::MalformedJson { .. } => "malformed_json",
            Self::MissingDigest => "missing_digest",
            Self::WrongDigestType => "wrong_digest_type",
            Self::InvalidDigest => "invalid_digest",
        }
    }
}

impl std::fmt::Display for ComplianceMetadataError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "compliance metadata failed: {}", self.category())
    }
}

impl std::error::Error for ComplianceMetadataError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source } => Some(source),
            Self::MalformedJson { source } => Some(source),
            Self::MissingDigest | Self::WrongDigestType | Self::InvalidDigest => None,
        }
    }
}

impl SourceComplianceMetadata {
    /// Read the existing digest field, preserving JSON, trim and case behavior.
    ///
    /// This is not an integrity, authenticity, symlink or current-package check.
    /// Callers must report failures once at their startup origin before creating
    /// infrastructure. No logging or background work is started here.
    ///
    /// # Errors
    /// Returns a typed read, parse, field or digest-syntax failure.
    pub fn load(path: &Path) -> Result<Self, ComplianceMetadataError> {
        let contents = std::fs::read_to_string(path)
            .map_err(|source| ComplianceMetadataError::Read { source })?;
        let manifest: serde_json::Value = serde_json::from_str(&contents)
            .map_err(|source| ComplianceMetadataError::MalformedJson { source })?;
        let value = manifest
            .get("source_compliance_sha256")
            .ok_or(ComplianceMetadataError::MissingDigest)?;
        let digest = value
            .as_str()
            .ok_or(ComplianceMetadataError::WrongDigestType)?
            .trim();
        if digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(ComplianceMetadataError::InvalidDigest);
        }
        Ok(Self {
            digest: format!("sha256:{}", digest.to_ascii_lowercase()),
        })
    }

    /// The existing normalized successful wire value.
    #[must_use]
    pub fn digest(&self) -> &str {
        &self.digest
    }

    #[cfg(test)]
    pub(crate) fn fixture() -> Self {
        Self {
            digest: format!("sha256:{}", "ab".repeat(32)),
        }
    }
}
