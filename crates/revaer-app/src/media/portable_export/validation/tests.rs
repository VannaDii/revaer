use super::*;

const HEADER: &str =
    "format_version: 1\nkind: revaer.media.profile_bundle\nmetadata: {name: Native}\n";
const PROFILE: &str = "profiles:\n- version: 1\n  profile_key: library\n  display_name: Library\n  description: ''\n  enabled: false\n  dry_run_only: true\n  desired_target_key: output\n  desired_target_version: 1\n  policy_key: preserve\n  policy_version: 2\n  output_root_key: output\n  workspace_root_key: workspace\n  quarantine_root_key: quarantine\n";

#[test]
fn requires_exact_explicit_authority_for_each_resource_key() -> anyhow::Result<()> {
    let input = format!("{HEADER}{PROFILE}");
    let create = MediaYamlResourcePrecondition::Create {
        kind: MediaYamlResourceKind::Profiles,
        key: "library".into(),
    };
    let matched = MediaYamlResourcePrecondition::Match {
        kind: MediaYamlResourceKind::Profiles,
        key: "library".into(),
        expected_version: 7,
    };
    validate_preconditions(&input, std::slice::from_ref(&create))?;
    validate_preconditions(&input, std::slice::from_ref(&matched))?;
    let Err(error) = validate_preconditions(&input, &[]) else {
        anyhow::bail!("YAML body version was treated as write authority");
    };
    assert_eq!(
        error.code(),
        Some("media_configuration_precondition_required")
    );
    assert!(validate_preconditions(&input, &[create.clone(), create.clone()]).is_err());
    assert!(validate_preconditions(&input, &[create, matched]).is_err());
    for (kind, key, expected_version) in [
        (MediaYamlResourceKind::Targets, "library", 1),
        (MediaYamlResourceKind::Profiles, "another", 1),
        (MediaYamlResourceKind::Profiles, "/physical/path", 1),
        (MediaYamlResourceKind::Profiles, "library", 0),
        (MediaYamlResourceKind::Profiles, "library", -1),
    ] {
        assert!(
            validate_preconditions(
                &input,
                &[MediaYamlResourcePrecondition::Match {
                    kind,
                    key: key.into(),
                    expected_version,
                }]
            )
            .is_err()
        );
    }
    let input = format!(
        "{HEADER}{PROFILE}{}",
        PROFILE
            .strip_prefix("profiles:\n")
            .ok_or_else(|| anyhow::anyhow!("missing fixture prefix"))?
            .replacen("version: 1", "version: 2", 1)
    );
    validate_preconditions(
        &input,
        &[MediaYamlResourcePrecondition::Match {
            kind: MediaYamlResourceKind::Profiles,
            key: "library".into(),
            expected_version: 7,
        }],
    )?;
    validate_preconditions(HEADER, &[])?;
    Ok(())
}

#[test]
fn rejects_ambiguous_or_unbounded_precondition_envelopes() -> anyhow::Result<()> {
    use revaer_api::models::MediaYamlImportRequest;
    for preconditions in [
        "null",
        "[{\"intent\":\"create\",\"kind\":\"profiles\",\"key\":\"library\",\"expected_version\":1}]",
        "[{\"intent\":\"match\",\"kind\":\"profiles\",\"key\":\"library\"}]",
        "[{\"intent\":\"create\",\"kind\":\"unknown\",\"key\":\"library\"}]",
    ] {
        let input = format!("{{\"yaml_payload\":\"test\",\"preconditions\":{preconditions}}}");
        assert!(serde_json::from_str::<MediaYamlImportRequest>(&input).is_err());
    }
    let row = MediaYamlResourcePrecondition::Create {
        kind: MediaYamlResourceKind::Profiles,
        key: "library".into(),
    };
    let Err(error) = validate_preconditions(&format!("{HEADER}{PROFILE}"), &vec![row; 129]) else {
        anyhow::bail!("precondition overflow accepted");
    };
    assert_eq!(error.code(), Some("media_configuration_bound_exceeded"));
    Ok(())
}

#[test]
fn decodes_complete_native_profile_without_reinterpreting_its_version() -> anyhow::Result<()> {
    let bundle = parse(&format!("{HEADER}{PROFILE}"))?;
    assert_eq!(bundle.profiles.len(), 1);
    assert_eq!(bundle.profiles[0].version, 1);
    assert_eq!(bundle.profiles[0].profile.fields().policy_version, 2);
    assert_eq!(
        bundle.profiles[0].profile.fields().output_root_key,
        "output"
    );
    for (original, replacement) in [
        ("version: 1", "version: 0"),
        ("version: 1", "version: '1'"),
        ("  enabled: false\n", ""),
        (
            "  workspace_root_key: workspace",
            "  workspace_root_key: null",
        ),
        (
            "  output_root_key: output",
            "  output_root_key: /physical/output",
        ),
        ("  description: ''", "  source_root: /physical/source"),
    ] {
        assert!(
            parse(&format!(
                "{HEADER}{}",
                PROFILE.replacen(original, replacement, 1)
            ))
            .is_err()
        );
    }
    Ok(())
}

#[test]
fn rejects_yaml_syntax_abuse_before_native_decoding() {
    for input in [
        format!("{HEADER}metadata: {{name: Duplicate}}\n"),
        HEADER.replace("{name: Native}", "{name: Native, name: Duplicate}"),
        HEADER.replace("{name: Native}", "{1: Native}"),
        HEADER.replace("{name: Native}", "!private {name: Native}"),
        HEADER.replace("{name: Native}", "&recursive [*recursive]"),
        format!("{HEADER}targets: &targets []\npolicies: *targets\n"),
        format!("{HEADER}---\n{HEADER}"),
        format!("{HEADER}unknown_field: true\n"),
        String::new(),
    ] {
        assert!(parse(&input).is_err());
    }
}

#[test]
fn punctuation_in_text_is_content_not_alias_or_tag_syntax() -> anyhow::Result<()> {
    for metadata in [
        "metadata: {name: Native, description: 'Use * and & and ! literally'}",
        "metadata:\n  name: Native\n  description: |\n    * not an alias\n    & not an anchor\n    ! not a tag",
        "metadata: {name: !!str Native}",
    ] {
        let input = format!("format_version: 1\nkind: revaer.media.profile_bundle\n{metadata}\n");
        let result = validate(&input, &[], &[], &[])?;
        assert!(result.valid);
        assert_eq!(result.profile_count, 0);
    }
    Ok(())
}

#[test]
fn enforces_bundle_bytes_resources_and_nesting_bounds() -> anyhow::Result<()> {
    let mut oversized = HEADER.to_owned();
    oversized.push_str(&" ".repeat(4 * 1_024 * 1_024));
    let Err(error) = parse(&oversized) else {
        anyhow::bail!("byte overflow accepted");
    };
    assert_eq!(error.code(), Some("media_configuration_bound_exceeded"));
    let body = PROFILE
        .strip_prefix("profiles:\n")
        .ok_or_else(|| anyhow::anyhow!("missing fixture prefix"))?;
    let oversized = format!("{HEADER}profiles:\n{}", body.repeat(129));
    assert!(parse(&oversized).is_err());
    let nested = format!("{HEADER}targets: {}{}\n", "[".repeat(65), "]".repeat(65));
    assert!(parse(&nested).is_err());
    Ok(())
}

#[test]
fn missing_exact_catalog_references_are_blocking_and_pointer_addressed() -> anyhow::Result<()> {
    let result = validate(&format!("{HEADER}{PROFILE}"), &[], &[], &[])?;
    assert!(!result.valid);
    assert_eq!(result.profile_count, 1);
    assert!(result.issues.iter().any(|issue| issue.blocking
        && issue.code == "media_yaml_desired_target_not_found"
        && issue.pointer == "/profiles/0/desired_target_key"));
    assert!(result.issues.iter().any(|issue| issue.blocking
        && issue.code == "media_yaml_policy_profile_not_found"
        && issue.pointer == "/profiles/0/policy_key"));
    Ok(())
}

#[test]
fn association_prefix_is_required_and_exact_profile_pin_is_checked() -> anyhow::Result<()> {
    let association = "discovery_associations:\n- association_key: movies\n  profile_key: library\n  profile_version: 1\n  source_root_key: source\n  root_relative_path: ''\n  manual_enabled: false\n  watcher_enabled: false\n  schedule_enabled: false\n";
    let bundle = parse(&format!("{HEADER}{PROFILE}{association}"))?;
    assert_eq!(bundle.discovery_associations[0].root_relative_path, "");
    for replacement in ["", "  root_relative_path: null\n"] {
        assert!(
            parse(&format!(
                "{HEADER}{PROFILE}{}",
                association.replace("  root_relative_path: ''\n", replacement)
            ))
            .is_err()
        );
    }
    let result = validate(
        &format!(
            "{HEADER}{PROFILE}{}",
            association.replace("profile_version: 1", "profile_version: 2")
        ),
        &[],
        &[],
        &[],
    )?;
    assert!(result.issues.iter().any(|issue| issue.blocking
        && issue.code == "media_configuration_reference_missing"
        && issue.pointer == "/discovery_associations/0/profile_key"));
    Ok(())
}
