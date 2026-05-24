//! Profile compilation and semantic validation.

use thiserror::Error;

/// Compile-time media profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MediaProfile {
    /// Human-readable profile key.
    pub key: String,
    /// Source root path.
    pub source_root: String,
    /// Output root path.
    pub output_root: String,
    /// Whether this profile is dry-run only.
    pub dry_run_only: bool,
}

/// Semantic validation error.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Source and output roots overlap.
    #[error("source and output roots must not overlap")]
    OverlappingRoots,
    /// Empty profile key.
    #[error("profile key must not be empty")]
    EmptyProfileKey,
    /// Source root path is empty.
    #[error("source root must not be empty")]
    EmptySourceRoot,
    /// Output root path is empty.
    #[error("output root must not be empty")]
    EmptyOutputRoot,
    /// Duplicate profile key in the same compile set.
    #[error("duplicate profile key")]
    DuplicateProfileKey,
    /// Two profile source roots overlap.
    #[error("overlapping profile roots")]
    OverlappingProfileRoots,
}

/// Validate profile semantics.
///
/// # Errors
///
/// Returns [`ValidationError::EmptyProfileKey`] when the profile key is blank.
/// Returns [`ValidationError::OverlappingRoots`] when source and output roots overlap.
pub fn validate_profile(profile: &MediaProfile) -> Result<(), ValidationError> {
    if profile.key.trim().is_empty() {
        return Err(ValidationError::EmptyProfileKey);
    }
    if profile.source_root.trim().is_empty() {
        return Err(ValidationError::EmptySourceRoot);
    }
    if profile.output_root.trim().is_empty() {
        return Err(ValidationError::EmptyOutputRoot);
    }

    let source = normalize_path(&profile.source_root);
    let output = normalize_path(&profile.output_root);
    if source.starts_with(&output) || output.starts_with(&source) {
        return Err(ValidationError::OverlappingRoots);
    }

    Ok(())
}

/// Validate a profile set for key and path collisions.
///
/// # Errors
///
/// Returns semantic validation errors for any invalid profile row or overlap.
pub fn validate_profiles(profiles: &[MediaProfile]) -> Result<(), ValidationError> {
    let mut seen_keys = std::collections::BTreeSet::new();
    let mut seen_roots: Vec<String> = Vec::new();

    for profile in profiles {
        validate_profile(profile)?;

        let lowered_key = profile.key.trim().to_ascii_lowercase();
        if !seen_keys.insert(lowered_key) {
            return Err(ValidationError::DuplicateProfileKey);
        }

        let source = normalize_path(&profile.source_root);
        for other in &seen_roots {
            if source.starts_with(other) || other.starts_with(&source) {
                return Err(ValidationError::OverlappingProfileRoots);
            }
        }
        seen_roots.push(source);
    }

    Ok(())
}

fn normalize_path(path: &str) -> String {
    let mut normalized = path.trim().to_string();
    while normalized.ends_with('/') {
        normalized.pop();
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::{MediaProfile, ValidationError, validate_profile, validate_profiles};

    #[test]
    fn reject_overlapping_paths() {
        let profile = MediaProfile {
            key: "tv".to_string(),
            source_root: "/media/tv".to_string(),
            output_root: "/media/tv/out".to_string(),
            dry_run_only: false,
        };

        assert_eq!(
            validate_profile(&profile),
            Err(ValidationError::OverlappingRoots)
        );
    }

    #[test]
    fn accept_disjoint_paths() {
        let profile = MediaProfile {
            key: "tv".to_string(),
            source_root: "/input/tv".to_string(),
            output_root: "/output/tv".to_string(),
            dry_run_only: true,
        };

        assert!(validate_profile(&profile).is_ok());
    }

    #[test]
    fn reject_duplicate_profile_key_in_set() {
        let profiles = vec![
            MediaProfile {
                key: "movies".to_string(),
                source_root: "/input/movies".to_string(),
                output_root: "/output/movies".to_string(),
                dry_run_only: true,
            },
            MediaProfile {
                key: "Movies".to_string(),
                source_root: "/input/movies2".to_string(),
                output_root: "/output/movies2".to_string(),
                dry_run_only: true,
            },
        ];

        assert_eq!(
            validate_profiles(&profiles),
            Err(ValidationError::DuplicateProfileKey)
        );
    }

    #[test]
    fn reject_overlapping_profile_roots_in_set() {
        let profiles = vec![
            MediaProfile {
                key: "movies".to_string(),
                source_root: "/media".to_string(),
                output_root: "/output/movies".to_string(),
                dry_run_only: true,
            },
            MediaProfile {
                key: "tv".to_string(),
                source_root: "/media/tv".to_string(),
                output_root: "/output/tv".to_string(),
                dry_run_only: true,
            },
        ];

        assert_eq!(
            validate_profiles(&profiles),
            Err(ValidationError::OverlappingProfileRoots)
        );
    }
}
