//! Profile compilation and semantic validation.

use serde::de::{IgnoredAny, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::fmt;
use thiserror::Error;

/// Maximum accepted Revaer media-profile YAML payload size.
pub const MAX_PROFILE_YAML_BYTES: usize = 1_048_576;
/// Maximum number of profiles accepted in one compile or YAML import request.
pub const MAX_PROFILE_COUNT: usize = 1_024;
const MAX_PROFILE_KEY_BYTES: usize = 128;
const MAX_PROFILE_ROOT_BYTES: usize = 4_096;

/// Compile-time media profile.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
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

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MediaProfileYamlBundle {
    profiles: BoundedProfiles,
}

#[derive(Debug)]
struct BoundedProfiles {
    values: Vec<MediaProfile>,
    too_many: bool,
}

impl<'de> Deserialize<'de> for BoundedProfiles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_seq(BoundedProfilesVisitor)
    }
}

struct BoundedProfilesVisitor;

impl<'de> Visitor<'de> for BoundedProfilesVisitor {
    type Value = BoundedProfiles;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded media profile sequence")
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values =
            Vec::with_capacity(sequence.size_hint().unwrap_or(0).min(MAX_PROFILE_COUNT));
        while values.len() < MAX_PROFILE_COUNT {
            let Some(profile) = sequence.next_element::<MediaProfile>()? else {
                return Ok(BoundedProfiles {
                    values,
                    too_many: false,
                });
            };
            values.push(profile);
        }

        let too_many = sequence.next_element::<IgnoredAny>()?.is_some();
        while sequence.next_element::<IgnoredAny>()?.is_some() {}
        Ok(BoundedProfiles { values, too_many })
    }
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
    /// Profile key exceeds the accepted byte limit.
    #[error("profile key exceeds the accepted byte limit")]
    ProfileKeyTooLong,
    /// Source root path is empty.
    #[error("source root must not be empty")]
    EmptySourceRoot,
    /// Output root path is empty.
    #[error("output root must not be empty")]
    EmptyOutputRoot,
    /// A profile root exceeds the accepted byte limit.
    #[error("profile root exceeds the accepted byte limit")]
    ProfileRootTooLong,
    /// A profile root is not a canonical lexical absolute path.
    #[error("profile root must be a canonical lexical absolute path")]
    InvalidProfileRoot,
    /// Duplicate profile key in the same compile set.
    #[error("duplicate profile key")]
    DuplicateProfileKey,
    /// Two roots owned by different profiles overlap.
    #[error("overlapping profile roots")]
    OverlappingProfileRoots,
    /// YAML input exceeds the accepted byte limit.
    #[error("media profile YAML payload exceeds the accepted byte limit")]
    YamlPayloadTooLarge,
    /// YAML input cannot be decoded as the Revaer profile shape.
    #[error("media profile YAML payload is invalid")]
    InvalidYaml,
    /// A profile collection exceeds the accepted cardinality.
    #[error("media profile count exceeds the accepted limit")]
    TooManyProfiles,
}

/// Parse and validate one bounded media-profile YAML bundle.
///
/// The byte limit is enforced before YAML decoding. The collection limit is enforced before
/// profile semantics or root-overlap work.
///
/// # Errors
///
/// Returns a [`ValidationError`] when the payload exceeds a bound, YAML decoding fails, or the
/// decoded profile set violates semantic invariants.
pub fn parse_and_validate_profiles_yaml(
    yaml_payload: &str,
) -> Result<Vec<MediaProfile>, ValidationError> {
    if yaml_payload.len() > MAX_PROFILE_YAML_BYTES {
        return Err(ValidationError::YamlPayloadTooLarge);
    }
    let bundle: MediaProfileYamlBundle =
        serde_yaml::from_str(yaml_payload).map_err(|_| ValidationError::InvalidYaml)?;
    if bundle.profiles.too_many {
        return Err(ValidationError::TooManyProfiles);
    }
    validate_profiles(&bundle.profiles.values)?;
    Ok(bundle.profiles.values)
}

/// Validate profile semantics.
///
/// # Errors
///
/// Returns a semantic validation error when identity, root bounds, or root isolation is invalid.
pub fn validate_profile(profile: &MediaProfile) -> Result<(), ValidationError> {
    let key = profile.key.trim();
    if key.is_empty() {
        return Err(ValidationError::EmptyProfileKey);
    }
    if key.len() > MAX_PROFILE_KEY_BYTES {
        return Err(ValidationError::ProfileKeyTooLong);
    }
    let source = profile.source_root.trim();
    if source.is_empty() {
        return Err(ValidationError::EmptySourceRoot);
    }
    let output = profile.output_root.trim();
    if output.is_empty() {
        return Err(ValidationError::EmptyOutputRoot);
    }
    if source.len() > MAX_PROFILE_ROOT_BYTES || output.len() > MAX_PROFILE_ROOT_BYTES {
        return Err(ValidationError::ProfileRootTooLong);
    }

    let source = normalize_path(source)?;
    let output = normalize_path(output)?;
    if paths_overlap(&source, &output) {
        return Err(ValidationError::OverlappingRoots);
    }

    Ok(())
}

/// Validate a profile set for key and path collisions.
///
/// The collection bound is checked before per-profile semantic work. Normalized roots are sorted
/// once by path components, then only adjacent pairs are compared; an ancestor's component
/// sequence is immediately before its descendant subtree.
///
/// # Errors
///
/// Returns semantic validation errors for any invalid profile row, cardinality, or overlap.
pub fn validate_profiles(profiles: &[MediaProfile]) -> Result<(), ValidationError> {
    if profiles.len() > MAX_PROFILE_COUNT {
        return Err(ValidationError::TooManyProfiles);
    }

    let mut seen_keys = std::collections::BTreeSet::new();
    let mut roots = Vec::with_capacity(profiles.len().saturating_mul(2));
    for profile in profiles {
        validate_profile(profile)?;

        let lowered_key = profile.key.trim().to_ascii_lowercase();
        if !seen_keys.insert(lowered_key) {
            return Err(ValidationError::DuplicateProfileKey);
        }
        roots.push(normalize_path(&profile.source_root)?);
        roots.push(normalize_path(&profile.output_root)?);
    }

    sort_roots_by_components(&mut roots);
    if scan_adjacent_roots(&roots).overlap {
        return Err(ValidationError::OverlappingProfileRoots);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct RootOverlapScan {
    overlap: bool,
    comparisons: usize,
}

fn scan_adjacent_roots(sorted_roots: &[String]) -> RootOverlapScan {
    let mut comparisons = 0;
    for pair in sorted_roots.windows(2) {
        comparisons += 1;
        if paths_overlap(&pair[0], &pair[1]) {
            return RootOverlapScan {
                overlap: true,
                comparisons,
            };
        }
    }
    RootOverlapScan {
        overlap: false,
        comparisons,
    }
}

fn sort_roots_by_components(roots: &mut [String]) {
    roots.sort_unstable_by(|left, right| {
        left.split('/')
            .filter(|component| !component.is_empty())
            .cmp(right.split('/').filter(|component| !component.is_empty()))
    });
}

fn normalize_path(path: &str) -> Result<String, ValidationError> {
    let trimmed = path.trim();
    if !trimmed.starts_with('/')
        || trimmed.contains("//")
        || trimmed.contains('\\')
        || trimmed.chars().any(char::is_control)
    {
        return Err(ValidationError::InvalidProfileRoot);
    }
    let normalized = if trimmed.len() > 1 {
        trimmed.strip_suffix('/').unwrap_or(trimmed)
    } else {
        trimmed
    };
    if normalized != "/"
        && normalized[1..]
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(ValidationError::InvalidProfileRoot);
    }
    Ok(normalized.to_string())
}

fn paths_overlap(left: &str, right: &str) -> bool {
    left == right
        || left == "/"
        || right == "/"
        || left
            .strip_prefix(right)
            .is_some_and(|suffix| suffix.starts_with('/'))
        || right
            .strip_prefix(left)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_PROFILE_COUNT, MAX_PROFILE_ROOT_BYTES, MAX_PROFILE_YAML_BYTES, MediaProfile,
        ValidationError, parse_and_validate_profiles_yaml, scan_adjacent_roots,
        sort_roots_by_components, validate_profile, validate_profiles,
    };
    use std::fmt::Write as _;

    fn profile(index: usize) -> MediaProfile {
        MediaProfile {
            key: format!("profile-{index:04}"),
            source_root: format!("/input/{index:04}"),
            output_root: format!("/output/{index:04}"),
            dry_run_only: true,
        }
    }

    fn profiles_yaml(count: usize) -> Result<String, std::fmt::Error> {
        let mut yaml = "profiles:\n".to_string();
        for index in 0..count {
            writeln!(
                yaml,
                "  - key: profile-{index:04}\n    source_root: /input/{index:04}\n    output_root: /output/{index:04}\n    dry_run_only: true"
            )?;
        }
        Ok(yaml)
    }

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
        assert!(validate_profile(&profile(0)).is_ok());
    }

    #[test]
    fn reject_duplicate_profile_key_in_set() {
        let mut profiles = vec![profile(0), profile(1)];
        profiles[1].key = profiles[0].key.to_ascii_uppercase();
        assert_eq!(
            validate_profiles(&profiles),
            Err(ValidationError::DuplicateProfileKey)
        );
    }

    #[test]
    fn reject_overlapping_profile_roots_in_set() {
        let mut profiles = vec![profile(0), profile(1)];
        profiles[1].source_root = format!("{}/nested", profiles[0].source_root);
        assert_eq!(
            validate_profiles(&profiles),
            Err(ValidationError::OverlappingProfileRoots)
        );
    }

    #[test]
    fn accept_sibling_paths_with_shared_prefix() {
        let profiles = vec![profile(1), profile(10)];
        assert!(validate_profiles(&profiles).is_ok());
    }

    #[test]
    fn reject_cross_profile_source_output_overlap() {
        let mut profiles = vec![profile(0), profile(1)];
        profiles[1].source_root = format!("{}/television", profiles[0].output_root);
        assert_eq!(
            validate_profiles(&profiles),
            Err(ValidationError::OverlappingProfileRoots)
        );
    }

    #[test]
    fn component_order_detects_ancestry_across_punctuation_siblings() {
        let profiles = vec![
            MediaProfile {
                key: "ancestor".to_string(),
                source_root: "/a".to_string(),
                output_root: "/z/ancestor".to_string(),
                dry_run_only: true,
            },
            MediaProfile {
                key: "punctuation-sibling".to_string(),
                source_root: "/a-0".to_string(),
                output_root: "/z/punctuation-sibling".to_string(),
                dry_run_only: true,
            },
            MediaProfile {
                key: "descendant".to_string(),
                source_root: "/a/b".to_string(),
                output_root: "/z/descendant".to_string(),
                dry_run_only: true,
            },
        ];

        assert_eq!(
            validate_profiles(&profiles),
            Err(ValidationError::OverlappingProfileRoots)
        );
    }

    #[test]
    fn root_overlaps_every_absolute_descendant() {
        let profile = MediaProfile {
            key: "root".to_string(),
            source_root: "/".to_string(),
            output_root: "/output".to_string(),
            dry_run_only: true,
        };
        assert_eq!(
            validate_profile(&profile),
            Err(ValidationError::OverlappingRoots)
        );
    }

    #[test]
    fn lexical_absolute_roots_reject_alias_and_traversal_components() {
        for invalid_root in [
            "relative/path",
            "/input/./tv",
            "/input/../tv",
            "/input//tv",
            "//input/tv",
            r"/input\tv",
            "/input/\u{0}tv",
        ] {
            let invalid = MediaProfile {
                key: "invalid".to_string(),
                source_root: invalid_root.to_string(),
                output_root: "/output/tv".to_string(),
                dry_run_only: true,
            };
            assert_eq!(
                validate_profile(&invalid),
                Err(ValidationError::InvalidProfileRoot),
                "accepted {invalid_root:?}"
            );
        }
    }

    #[test]
    fn root_byte_limit_accepts_maximum_and_rejects_maximum_plus_one() {
        let maximum = format!("/{}", "a".repeat(MAX_PROFILE_ROOT_BYTES - 1));
        let maximum_profile = MediaProfile {
            key: "maximum".to_string(),
            source_root: maximum.clone(),
            output_root: "/output".to_string(),
            dry_run_only: true,
        };
        assert!(validate_profile(&maximum_profile).is_ok());

        let overlong_profile = MediaProfile {
            source_root: format!("{maximum}a"),
            ..maximum_profile
        };
        assert_eq!(
            validate_profile(&overlong_profile),
            Err(ValidationError::ProfileRootTooLong)
        );
    }

    #[test]
    fn yaml_roots_enforce_root_ancestry_and_lexical_canonical_form() {
        let root_yaml = "profiles:\n  - key: root\n    source_root: /\n    output_root: /output\n    dry_run_only: true\n";
        assert_eq!(
            parse_and_validate_profiles_yaml(root_yaml),
            Err(ValidationError::OverlappingRoots)
        );

        for invalid_root in [
            "relative/path",
            "/input/./tv",
            "/input/../tv",
            "/input//tv",
            "//input/tv",
            r"/input\tv",
        ] {
            let yaml = format!(
                "profiles:\n  - key: invalid\n    source_root: '{invalid_root}'\n    output_root: /output/tv\n    dry_run_only: true\n"
            );
            assert_eq!(
                parse_and_validate_profiles_yaml(&yaml),
                Err(ValidationError::InvalidProfileRoot),
                "YAML accepted {invalid_root:?}"
            );
        }
    }

    #[test]
    fn yaml_root_byte_limit_accepts_maximum_and_rejects_maximum_plus_one() {
        let maximum = format!("/{}", "a".repeat(MAX_PROFILE_ROOT_BYTES - 1));
        let maximum_yaml = format!(
            "profiles:\n  - key: maximum\n    source_root: '{maximum}'\n    output_root: /output\n    dry_run_only: true\n"
        );
        assert_eq!(
            parse_and_validate_profiles_yaml(&maximum_yaml).map(|profiles| profiles.len()),
            Ok(1)
        );

        let overlong_yaml = format!(
            "profiles:\n  - key: overlong\n    source_root: '{maximum}a'\n    output_root: /output\n    dry_run_only: true\n"
        );
        assert_eq!(
            parse_and_validate_profiles_yaml(&overlong_yaml),
            Err(ValidationError::ProfileRootTooLong)
        );
    }

    #[test]
    fn profile_count_limit_accepts_maximum_and_rejects_maximum_plus_one() {
        let profiles: Vec<_> = (0..MAX_PROFILE_COUNT).map(profile).collect();
        assert!(validate_profiles(&profiles).is_ok());

        let profiles: Vec<_> = (0..=MAX_PROFILE_COUNT).map(profile).collect();
        assert_eq!(
            validate_profiles(&profiles),
            Err(ValidationError::TooManyProfiles)
        );
    }

    #[test]
    fn sorted_overlap_scan_scales_linearly_after_sorting() {
        let mut roots: Vec<_> = (0..MAX_PROFILE_COUNT)
            .flat_map(|index| [format!("/input/{index:04}"), format!("/output/{index:04}")])
            .collect();
        sort_roots_by_components(&mut roots);

        let scan = scan_adjacent_roots(&roots);
        assert!(!scan.overlap);
        assert_eq!(scan.comparisons, roots.len() - 1);
    }

    #[test]
    fn yaml_limits_are_checked_before_profile_semantics() -> Result<(), std::fmt::Error> {
        let oversized = "x".repeat(MAX_PROFILE_YAML_BYTES + 1);
        assert_eq!(
            parse_and_validate_profiles_yaml(&oversized),
            Err(ValidationError::YamlPayloadTooLarge)
        );

        let yaml = profiles_yaml(MAX_PROFILE_COUNT + 1)?;
        assert_eq!(
            parse_and_validate_profiles_yaml(&yaml),
            Err(ValidationError::TooManyProfiles)
        );

        let mut malformed_extra = profiles_yaml(MAX_PROFILE_COUNT)?;
        malformed_extra.push_str("  - not-a-profile\n");
        assert_eq!(
            parse_and_validate_profiles_yaml(&malformed_extra),
            Err(ValidationError::TooManyProfiles)
        );
        Ok(())
    }

    #[test]
    fn yaml_byte_limit_accepts_exact_maximum_and_rejects_one_more() -> Result<(), std::fmt::Error> {
        let mut yaml = profiles_yaml(1)?;
        yaml.push_str(&" ".repeat(MAX_PROFILE_YAML_BYTES - yaml.len()));
        assert_eq!(yaml.len(), MAX_PROFILE_YAML_BYTES);
        assert_eq!(
            parse_and_validate_profiles_yaml(&yaml).map(|profiles| profiles.len()),
            Ok(1)
        );

        yaml.push(' ');
        assert_eq!(
            parse_and_validate_profiles_yaml(&yaml),
            Err(ValidationError::YamlPayloadTooLarge)
        );
        Ok(())
    }

    #[test]
    fn yaml_maximum_profile_set_is_valid() -> Result<(), std::fmt::Error> {
        let yaml = profiles_yaml(MAX_PROFILE_COUNT)?;
        assert_eq!(
            parse_and_validate_profiles_yaml(&yaml).map(|profiles| profiles.len()),
            Ok(MAX_PROFILE_COUNT)
        );
        Ok(())
    }
}
