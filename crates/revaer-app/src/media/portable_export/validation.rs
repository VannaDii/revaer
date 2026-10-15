//! Native YAML compilation without writes or filesystem authority.

use std::collections::BTreeSet;

use revaer_api::models::{MediaYamlResourceKind, MediaYamlResourcePrecondition};
use serde_yaml::Value;
use yaml_rust2::parser::{Event, Parser, Tag};

use super::super::{
    AppMediaCompatibilityTargetResponse, AppMediaDesiredTargetResponse, AppMediaPolicyResponse,
    MediaYamlIssue, MediaYamlValidationResult, push_yaml_issue, validate_unique_catalog_keys,
    validate_yaml_compatibility_rows, validate_yaml_desired_rows, validate_yaml_policy_rows,
};
use super::{MediaServiceError, MediaServiceErrorKind, PortableBundle, bound_error};

pub(in crate::media) fn validate(
    input: &str,
    compatibility: &[AppMediaCompatibilityTargetResponse],
    targets: &[AppMediaDesiredTargetResponse],
    policies: &[AppMediaPolicyResponse],
) -> Result<MediaYamlValidationResult, MediaServiceError> {
    let bundle = parse(input)?;
    let mut issues = Vec::new();
    validate_catalogs(&bundle, compatibility, targets, policies, &mut issues);
    validate_profiles(&bundle, targets, policies, &mut issues);
    validate_associations(&bundle, &mut issues);
    Ok(MediaYamlValidationResult {
        version: bundle.format_version.to_string(),
        valid: !issues.iter().any(|issue| issue.blocking),
        profile_count: bundle.profiles.len(),
        issues,
    })
}

pub(super) fn parse(input: &str) -> Result<PortableBundle, MediaServiceError> {
    if input.len() > revaer_api::models::MEDIA_YAML_BUNDLE_MAX_BYTES {
        return Err(bound_error());
    }
    syntax_preflight(input)?;
    let value: Value = serde_yaml::from_str(input).map_err(|_| invalid())?;
    string_mapping_keys(&value)?;
    let bundle: PortableBundle = serde_yaml::from_value(value).map_err(|_| invalid())?;
    if bundle.format_version != 1
        || bundle.kind != "revaer.media.profile_bundle"
        || bundle.metadata.name.is_empty()
        || bundle.metadata.name.len() > 128
        || bundle
            .metadata
            .description
            .as_ref()
            .is_some_and(|v| v.len() > 1_024)
    {
        return Err(invalid());
    }
    let resource_count = bundle.profiles.len()
        + bundle.discovery_associations.len()
        + bundle.compatibility_targets.len()
        + bundle.targets.len()
        + bundle.policies.len();
    let child_count = bundle.discovery_associations.len()
        + bundle
            .profiles
            .iter()
            .map(|p| {
                let fields = p.profile.fields();
                2 + usize::from(fields.backup_root_key.is_some())
                    + usize::from(fields.quarantine_root_key.is_some())
            })
            .sum::<usize>()
        + bundle
            .targets
            .iter()
            .map(|t| t.streams.len())
            .sum::<usize>();
    if resource_count > 128 || child_count > 4_096 {
        return Err(bound_error());
    }
    Ok(bundle)
}

pub(in crate::media) fn validate_preconditions(
    input: &str,
    preconditions: &[MediaYamlResourcePrecondition],
) -> Result<(), MediaServiceError> {
    if preconditions.len() > revaer_api::models::MEDIA_YAML_PRECONDITIONS_MAX {
        return Err(bound_error());
    }
    let bundle = parse(input)?;
    let mut resources = BTreeSet::new();
    resources.extend(bundle.compatibility_targets.iter().map(|row| {
        (
            MediaYamlResourceKind::CompatibilityTargets,
            row.compatibility_target_key.as_str(),
        )
    }));
    resources.extend(
        bundle
            .targets
            .iter()
            .map(|row| (MediaYamlResourceKind::Targets, row.target_key.as_str())),
    );
    resources.extend(
        bundle
            .policies
            .iter()
            .map(|row| (MediaYamlResourceKind::Policies, row.policy_key.as_str())),
    );
    resources.extend(bundle.profiles.iter().map(|row| {
        (
            MediaYamlResourceKind::Profiles,
            row.profile.fields().profile_key.as_str(),
        )
    }));
    resources.extend(bundle.discovery_associations.iter().map(|row| {
        (
            MediaYamlResourceKind::DiscoveryAssociations,
            row.association_key.as_str(),
        )
    }));
    let mut supplied = BTreeSet::new();
    for precondition in preconditions {
        let identity = precondition.resource();
        let valid_key = match identity.0 {
            MediaYamlResourceKind::Profiles | MediaYamlResourceKind::DiscoveryAssociations => {
                super::validate_root_logical_key(identity.1).is_ok()
            }
            _ => revaer_api::models::validate_media_key(identity.1)
                .is_ok_and(|key| key == identity.1),
        };
        if !valid_key
            || !supplied.insert(identity)
            || !resources.contains(&identity)
            || matches!(precondition, MediaYamlResourcePrecondition::Match { expected_version, .. }
                if *expected_version <= 0)
        {
            return Err(invalid());
        }
    }
    if resources != supplied {
        return Err(MediaServiceError::new(MediaServiceErrorKind::Invalid)
            .with_code("media_configuration_precondition_required"));
    }
    Ok(())
}

fn syntax_preflight(input: &str) -> Result<(), MediaServiceError> {
    let mut parser = Parser::new_from_str(input);
    let mut documents = 0;
    let mut depth = 0_usize;
    loop {
        let (event, _) = parser.next_token().map_err(|_| invalid())?;
        match event {
            Event::DocumentStart => {
                documents += 1;
                if documents > 1 {
                    return Err(invalid());
                }
            }
            Event::Alias(_) => return Err(invalid()),
            Event::Scalar(_, _, _, tag) => check_tag(tag)?,
            Event::MappingStart(_, tag) | Event::SequenceStart(_, tag) => {
                check_tag(tag)?;
                depth += 1;
                if depth > 64 {
                    return Err(bound_error());
                }
            }
            Event::MappingEnd | Event::SequenceEnd => {
                depth = depth.checked_sub(1).ok_or_else(invalid)?;
            }
            Event::StreamEnd => break,
            Event::Nothing | Event::StreamStart | Event::DocumentEnd => {}
        }
    }
    if documents != 1 || depth != 0 {
        return Err(invalid());
    }
    Ok(())
}

fn check_tag(tag: Option<Tag>) -> Result<(), MediaServiceError> {
    if tag.is_some_and(|tag| {
        tag.handle != "tag:yaml.org,2002:"
            || !["str", "bool", "int", "float", "null", "seq", "map"].contains(&tag.suffix.as_str())
    }) {
        return Err(invalid());
    }
    Ok(())
}

fn string_mapping_keys(root: &Value) -> Result<(), MediaServiceError> {
    let mut pending = vec![root];
    while let Some(value) = pending.pop() {
        match value {
            Value::Mapping(mapping) => {
                for (key, value) in mapping {
                    if !matches!(key, Value::String(_)) {
                        return Err(invalid());
                    }
                    pending.push(value);
                }
            }
            Value::Sequence(values) => pending.extend(values),
            Value::Tagged(_) => return Err(invalid()),
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
    Ok(())
}

fn validate_catalogs(
    bundle: &PortableBundle,
    compatibility: &[AppMediaCompatibilityTargetResponse],
    targets: &[AppMediaDesiredTargetResponse],
    policies: &[AppMediaPolicyResponse],
    issues: &mut Vec<MediaYamlIssue>,
) {
    validate_unique_catalog_keys(
        issues,
        "/compatibility_targets",
        bundle
            .compatibility_targets
            .iter()
            .map(|t| (t.compatibility_target_key.clone(), t.version)),
    );
    validate_unique_catalog_keys(
        issues,
        "/targets",
        bundle
            .targets
            .iter()
            .map(|t| (t.target_key.clone(), t.version)),
    );
    validate_unique_catalog_keys(
        issues,
        "/policies",
        bundle
            .policies
            .iter()
            .map(|p| (p.policy_key.clone(), p.version)),
    );
    validate_yaml_compatibility_rows(issues, &bundle.compatibility_targets, compatibility);
    validate_yaml_desired_rows(issues, &bundle.targets, targets);
    validate_yaml_policy_rows(issues, &bundle.policies, policies);
}

fn validate_profiles(
    bundle: &PortableBundle,
    targets: &[AppMediaDesiredTargetResponse],
    policies: &[AppMediaPolicyResponse],
    issues: &mut Vec<MediaYamlIssue>,
) {
    let mut seen = BTreeSet::new();
    for (index, profile) in bundle.profiles.iter().enumerate() {
        let fields = profile.profile.fields();
        if !seen.insert((&fields.profile_key, profile.version)) {
            issue(
                issues,
                "media_configuration_invalid",
                &format!("/profiles/{index}"),
            );
        }
        if !bundle.targets.iter().any(|t| {
            (&t.target_key, t.version)
                == (&fields.desired_target_key, fields.desired_target_version)
        }) && !targets.iter().any(|t| {
            (&t.target_key, t.version)
                == (&fields.desired_target_key, fields.desired_target_version)
        }) {
            issue(
                issues,
                "media_yaml_desired_target_not_found",
                &format!("/profiles/{index}/desired_target_key"),
            );
        }
        let output = bundle
            .policies
            .iter()
            .find(|p| (&p.policy_key, p.version) == (&fields.policy_key, fields.policy_version))
            .map(|p| &p.output)
            .or_else(|| {
                policies
                    .iter()
                    .find(|p| {
                        (&p.policy_key, p.version) == (&fields.policy_key, fields.policy_version)
                    })
                    .map(|p| &p.output)
            });
        match output {
            None => issue(
                issues,
                "media_yaml_policy_profile_not_found",
                &format!("/profiles/{index}/policy_key"),
            ),
            Some(output) if output.quarantine_enabled && fields.quarantine_root_key.is_none() => {
                issue(
                    issues,
                    "media_root_binding_incomplete",
                    &format!("/profiles/{index}/quarantine_root_key"),
                );
            }
            Some(_) => {}
        }
    }
}

fn validate_associations(bundle: &PortableBundle, issues: &mut Vec<MediaYamlIssue>) {
    let mut seen = BTreeSet::new();
    for (index, association) in bundle.discovery_associations.iter().enumerate() {
        let pointer = format!("/discovery_associations/{index}");
        if !seen.insert(&association.association_key)
            || [
                association.association_key.as_str(),
                association.profile_key.as_str(),
                association.source_root_key.as_str(),
            ]
            .iter()
            .any(|key| super::validate_root_logical_key(key).is_err())
            || super::validate_root_association_prefix(&association.root_relative_path).is_err()
            || association.profile_version <= 0
        {
            issue(issues, "media_configuration_invalid", &pointer);
        }
        if !bundle.profiles.iter().any(|p| {
            (&p.profile.fields().profile_key, p.version)
                == (&association.profile_key, association.profile_version)
        }) {
            issue(
                issues,
                "media_configuration_reference_missing",
                &format!("{pointer}/profile_key"),
            );
        }
    }
}

fn issue(issues: &mut Vec<MediaYamlIssue>, code: &str, pointer: &str) {
    push_yaml_issue(issues, code, pointer, true);
}

fn invalid() -> MediaServiceError {
    MediaServiceError::new(MediaServiceErrorKind::Invalid).with_code("media_yaml_invalid")
}

#[cfg(test)]
mod tests;
