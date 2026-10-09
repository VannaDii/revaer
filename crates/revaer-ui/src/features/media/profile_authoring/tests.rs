use super::*;
use revaer_api_models::media_root_contract::{
    ProfileLifecycle, ProfileRootBinding, ProfileRootResolution, ProfileVersionResponseFields,
};
use uuid::Uuid;

#[test]
fn save_failures_preserve_drafts_and_never_suggest_automatic_retry() {
    for status in [0, 400, 401, 403, 404, 405, 409, 412, 413, 428, 500] {
        let message = mutation_failure(status);
        assert!(message.contains("preserved"));
        assert!(!message.contains("association"));
    }
}

fn draft() -> ProfileDraft {
    ProfileDraft {
        key: "library".into(),
        name: " Library ".into(),
        description: "Exact description".into(),
        target_key: "archive".into(),
        target_version: "2".into(),
        policy_key: "preserve".into(),
        policy_version: "3".into(),
        ..ProfileDraft::default()
    }
}

fn roots() -> Result<(ProfileRootDraft, Vec<RootChoice>), &'static str> {
    let mut roots = ProfileRootDraft::default();
    let choices: Vec<_> = [ProfileRootKind::Output, ProfileRootKind::Workspace]
        .into_iter()
        .map(|kind| RootChoice {
            key: kind
                .label()
                .split(' ')
                .next()
                .unwrap_or_default()
                .to_lowercase(),
            kind: kind.kind(),
            binding_ready: true,
            destructive_ready: false,
        })
        .collect();
    roots.select(ProfileRootKind::Output, "output", &choices)?;
    roots.select(ProfileRootKind::Workspace, "workspace", &choices)?;
    Ok((roots, choices))
}

fn response(
    request: ProfileVersionRequest,
) -> Result<ProfileVersionResponse, Box<dyn std::error::Error>> {
    let root_bindings = [ProfileRootKind::Output, ProfileRootKind::Workspace]
        .into_iter()
        .map(|kind| ProfileRootBinding {
            kind: kind.kind(),
            logical_key: if kind == ProfileRootKind::Output {
                "output"
            } else {
                "workspace"
            }
            .into(),
            resolution_state: ProfileRootResolution::Resolved,
            binding_ready: true,
            binding_reason: None,
            destructive_ready: false,
            destructive_reason: Some("media_root_durability_unproven".into()),
        })
        .collect();
    Ok(ProfileVersionResponse::new(ProfileVersionResponseFields {
        profile: request,
        media_profile_public_id: Uuid::from_u128(1),
        latest_version: 1,
        active_version: Some(1),
        lifecycle_state: ProfileLifecycle::Active,
        root_bindings,
        created_at: "2026-09-21T00:00:00Z".into(),
        updated_at: "2026-09-21T00:00:00Z".into(),
    })?)
}

#[test]
fn authoring_defaults_are_disabled_dry_run_with_no_implicit_versions() {
    let draft = ProfileDraft::default();
    assert!(!draft.enabled);
    assert!(draft.dry_run_only);
    assert!(draft.target_version.is_empty());
    assert!(draft.policy_version.is_empty());
}

#[test]
fn loaded_profile_preserves_exact_draft_and_requires_its_strong_fence()
-> Result<(), Box<dyn std::error::Error>> {
    let (roots, choices) = roots()?;
    let request = draft().request(&roots, &choices)?;
    let response = response(request.clone())?;
    let id = response.fields().media_profile_public_id;
    let tag = format!("\"media-profile:{id}:v1\"");
    let edit = ProfileEdit::loaded(id, &response, tag.clone())?;
    let restored = ProfileDraft::from_request(&edit.profile);
    let restored_roots = ProfileRootDraft::from_request(&edit.profile);
    assert_eq!(restored.request(&restored_roots, &choices)?, request);
    assert!(restored.request(&restored_roots, &[]).is_err());
    assert_eq!(restored_roots.selected(ProfileRootKind::Output), "output");
    assert!(ProfileEdit::loaded(Uuid::nil(), &response, tag.clone()).is_err());
    for invalid in [
        String::new(),
        format!("W/{tag}"),
        format!("\"media-profile:{id}:v2\""),
    ] {
        assert!(ProfileEdit::loaded(id, &response, invalid).is_err());
    }
    Ok(())
}

#[test]
fn replacement_confirmation_requires_exact_body_identity_next_head_and_tag()
-> Result<(), Box<dyn std::error::Error>> {
    let (roots, choices) = roots()?;
    let request = draft().request(&roots, &choices)?;
    let original = response(request.clone())?;
    let id = original.fields().media_profile_public_id;
    let edit = ProfileEdit::loaded(id, &original, format!("\"media-profile:{id}:v1\""))?;
    let mut fields = original.fields().clone();
    fields.latest_version = 2;
    fields.active_version = Some(2);
    let replacement = ProfileVersionResponse::new(fields.clone())?;
    let tag = format!("\"media-profile:{id}:v2\"");
    assert_eq!(edit.confirm_replacement(&request, &replacement, &tag)?, 2);
    assert!(edit.confirm_replacement(&request, &original, &tag).is_err());
    assert!(
        edit.confirm_replacement(&request, &replacement, &edit.etag)
            .is_err()
    );
    fields.media_profile_public_id = Uuid::nil();
    assert!(
        edit.confirm_replacement(
            &request,
            &ProfileVersionResponse::new(fields.clone())?,
            &tag
        )
        .is_err()
    );
    fields.media_profile_public_id = id;
    let mut changed = request.fields().clone();
    changed.description = "Unexpected server change".into();
    fields.profile = ProfileVersionRequest::new(changed)?;
    assert!(
        edit.confirm_replacement(&request, &ProfileVersionResponse::new(fields)?, &tag)
            .is_err()
    );
    assert_eq!(edit.profile, request);
    Ok(())
}

#[test]
fn complete_request_preserves_exact_inputs_and_requires_current_roots()
-> Result<(), Box<dyn std::error::Error>> {
    let (roots, mut choices) = roots()?;
    let draft = draft();
    let request = draft.request(&roots, &choices)?;
    assert_eq!(request.fields().display_name, " Library ");
    assert_eq!(request.fields().desired_target_version, 2);
    assert_eq!(request.fields().policy_version, 3);
    assert_eq!(request.fields().backup_root_key, None);
    choices[0].binding_ready = false;
    assert!(draft.request(&roots, &choices).is_err());
    assert!(draft.request(&roots, &[]).is_err());
    assert_eq!(roots.selected(ProfileRootKind::Output), "output");
    Ok(())
}

#[test]
fn explicit_versions_and_safety_flags_are_not_inferred() -> Result<(), Box<dyn std::error::Error>> {
    let (roots, choices) = roots()?;
    for value in ["", "0", "-1", "2147483648", "1.5", " 2"] {
        let mut draft = draft();
        draft.target_version = value.into();
        assert!(draft.request(&roots, &choices).is_err());
        draft.target_version = "2".into();
        draft.policy_version = value.into();
        assert!(draft.request(&roots, &choices).is_err());
    }
    for enabled in [false, true] {
        for dry_run_only in [false, true] {
            let request = ProfileDraft {
                enabled,
                dry_run_only,
                ..draft()
            }
            .request(&roots, &choices)?;
            assert_eq!(request.fields().enabled, enabled);
            assert_eq!(request.fields().dry_run_only, dry_run_only);
        }
    }
    Ok(())
}

#[test]
fn confirmation_requires_exact_body_initial_heads_and_strong_tag()
-> Result<(), Box<dyn std::error::Error>> {
    let (roots, choices) = roots()?;
    let request = draft().request(&roots, &choices)?;
    let saved = response(request.clone())?;
    let expected_tag = "\"media-profile:00000000-0000-0000-0000-000000000001:v1\"";
    assert!(confirm_creation(&saved, expected_tag, &request).is_ok());
    for tag in [
        "",
        "\"\"",
        "W/\"weak\"",
        "unquoted",
        "\"bad\nline\"",
        "\"profile-v1\"",
        "\"media-profile:00000000-0000-0000-0000-000000000002:v1\"",
        "\"media-profile:00000000-0000-0000-0000-000000000001:v2\"",
        "\"media-policy:00000000-0000-0000-0000-000000000001:v1\"",
    ] {
        assert!(confirm_creation(&saved, tag, &request).is_err());
    }
    let changed = ProfileDraft {
        enabled: true,
        ..draft()
    }
    .request(&roots, &choices)?;
    assert!(confirm_creation(&saved, expected_tag, &changed).is_err());
    let mut fields = saved.fields().clone();
    fields.latest_version = 2;
    fields.active_version = Some(2);
    assert!(
        confirm_creation(
            &ProfileVersionResponse::new(fields)?,
            "\"profile-v2\"",
            &request
        )
        .is_err()
    );
    Ok(())
}
