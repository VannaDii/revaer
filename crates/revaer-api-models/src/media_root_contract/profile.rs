//! Complete, exact profile create/replace inputs without filesystem authority.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, de};

use super::{RootInputError, object, validate_root_logical_key};

/// Explicit profile inputs, validated by [`ProfileVersionRequest::new`].
///
/// No version, path, enabled state or dry-run setting is inferred. Optional
/// roots represent absence only; policy-dependent requirements are checked by
/// the caller against the explicitly selected immutable policy version.
#[derive(Clone, PartialEq, Eq, Serialize)]
pub struct ProfileVersionRequestFields {
    /// Immutable lowercase ASCII profile key, 1-64 bytes.
    pub profile_key: String,
    /// Exact display name, 1-128 UTF-8 bytes, without trimming.
    pub display_name: String,
    /// Exact description, 0-1024 UTF-8 bytes, without trimming.
    pub description: String,
    /// Requested operational eligibility; false does not invalidate the body.
    pub enabled: bool,
    /// Restricts mutation; false never overrides a dry-run policy.
    pub dry_run_only: bool,
    /// Exact logical desired-target key, 1-64 bytes.
    pub desired_target_key: String,
    /// Explicit desired-target version, a positive `PostgreSQL` integer.
    pub desired_target_version: i32,
    /// Exact logical policy key, 1-64 bytes.
    pub policy_key: String,
    /// Explicit policy version, a positive `PostgreSQL` integer.
    pub policy_version: i32,
    /// Required logical output root, never a path.
    pub output_root_key: String,
    /// Required logical workspace root, never a path.
    pub workspace_root_key: String,
    /// Logical backup root, omitted from JSON when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub backup_root_key: Option<String>,
    /// Logical quarantine root, omitted from JSON when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quarantine_root_key: Option<String>,
}

impl fmt::Debug for ProfileVersionRequestFields {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("ProfileVersionRequestFields")
    }
}

/// Validated, immutable complete create/replace body specified by ADR 590.
///
/// All fields except the two optional root keys are required. Explicit null,
/// unknown/duplicate fields, positional arrays and nonpositive versions fail.
/// Keys use the exact 1-64 byte lowercase ASCII alphanumeric/internal-hyphen
/// grammar. Text and keys are never trimmed, normalized or silently truncated.
/// Debug output and validation errors omit submitted values.
///
/// HTTP must enforce ADR 521's 1 MiB decompressed request limit before decoding;
/// Serde validation cannot bound whitespace or the original encoded body.
/// Conditional headers, target/policy membership, policy-required optional root
/// bindings, catalog readiness and operational admission belong to callers.
/// This type does not resolve roots, advance heads or grant mutation permission.
///
/// ```
/// use revaer_api_models::media_root_contract::{
///     ProfileVersionRequest, ProfileVersionRequestFields,
/// };
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let request = ProfileVersionRequest::new(ProfileVersionRequestFields {
///     profile_key: "library".into(),
///     display_name: "Library".into(),
///     description: String::new(),
///     enabled: false,
///     dry_run_only: true,
///     desired_target_key: "archive".into(),
///     desired_target_version: 1,
///     policy_key: "preserve".into(),
///     policy_version: 2,
///     output_root_key: "output".into(),
///     workspace_root_key: "workspace".into(),
///     backup_root_key: None,
///     quarantine_root_key: None,
/// })?;
/// assert_eq!(request.fields().policy_version, 2);
/// assert!(!request.fields().enabled);
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ProfileVersionRequest {
    fields: ProfileVersionRequestFields,
}

impl ProfileVersionRequest {
    /// Validate all explicit fields without applying defaults or resolving keys.
    ///
    /// # Errors
    /// Rejects invalid required or present optional keys, an empty/over-128-byte
    /// display name, an over-1024-byte description, or nonpositive versions.
    /// Disabled and non-dry-run requests are structurally valid, not admitted.
    pub fn new(fields: ProfileVersionRequestFields) -> Result<Self, RootInputError> {
        for key in [
            fields.profile_key.as_str(),
            fields.desired_target_key.as_str(),
            fields.policy_key.as_str(),
            fields.output_root_key.as_str(),
            fields.workspace_root_key.as_str(),
        ]
        .into_iter()
        .chain(fields.backup_root_key.as_deref())
        .chain(fields.quarantine_root_key.as_deref())
        {
            validate_root_logical_key(key)?;
        }
        if fields.display_name.is_empty()
            || fields.display_name.len() > 128
            || fields.description.len() > 1_024
            || fields.desired_target_version <= 0
            || fields.policy_version <= 0
        {
            return Err(RootInputError);
        }
        Ok(Self { fields })
    }

    /// Return every immutable validated field using its exact wire name.
    #[must_use]
    pub const fn fields(&self) -> &ProfileVersionRequestFields {
        &self.fields
    }
}

impl<'de> Deserialize<'de> for ProfileVersionRequest {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let input: ProfileInput =
            object::deserialize(deserializer).map_err(|_| de::Error::custom(RootInputError))?;
        Self::new(input.0).map_err(de::Error::custom)
    }
}

#[derive(Deserialize)]
#[serde(transparent)]
struct ProfileInput(#[serde(with = "ProfileWire")] ProfileVersionRequestFields);

#[derive(Deserialize)]
#[serde(remote = "ProfileVersionRequestFields", deny_unknown_fields)]
struct ProfileWire {
    profile_key: String,
    display_name: String,
    description: String,
    enabled: bool,
    dry_run_only: bool,
    desired_target_key: String,
    desired_target_version: i32,
    policy_key: String,
    policy_version: i32,
    output_root_key: String,
    workspace_root_key: String,
    #[serde(default, deserialize_with = "present_root")]
    backup_root_key: Option<String>,
    #[serde(default, deserialize_with = "present_root")]
    quarantine_root_key: Option<String>,
}

fn present_root<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    String::deserialize(deserializer).map(Some)
}

#[cfg(test)]
mod tests;
