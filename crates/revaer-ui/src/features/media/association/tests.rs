use super::*;

fn profile() -> ActiveProfile {
    ActiveProfile {
        id: Uuid::from_u128(1),
        key: "balanced".to_owned(),
        version: 3,
    }
}

fn source() -> SourceChoice {
    SourceChoice {
        key: "library".to_owned(),
        binding_ready: true,
        destructive_ready: false,
    }
}

fn draft() -> AssociationDraft {
    AssociationDraft {
        key: "scan-library".to_owned(),
        profile: Some((profile().id, 3)),
        source: source().key,
        ..AssociationDraft::default()
    }
}

#[test]
fn empty_prefix_does_not_implicitly_select_whole_root() {
    assert_eq!(
        draft().request(&[profile()], &[source()]).err(),
        Some("Select whole-root or relative-prefix scope.")
    );
    let mut draft = draft();
    draft.scope = SourceScope::Prefix;
    assert_eq!(
        draft.request(&[profile()], &[source()]).err(),
        Some("Enter a nonempty relative prefix.")
    );
}

#[test]
fn whole_root_and_explicit_modes_are_preserved() -> Result<(), &'static str> {
    let mut draft = draft();
    draft.scope = SourceScope::WholeRoot;
    draft.prefix = "ignored-only-after-explicit-whole-root-choice".to_owned();
    draft.modes = DiscoveryModes {
        manual_enabled: true,
        watcher_enabled: true,
        schedule_enabled: true,
    };
    let request = draft.request(&[profile()], &[source()])?;
    assert_eq!(request.root_relative_path(), "");
    assert_eq!(request.profile_version(), 3);
    assert_eq!(request.source_root_key(), "library");
    assert_eq!(request.modes(), draft.modes);
    Ok(())
}

#[test]
fn relative_prefix_preserves_exact_bytes() -> Result<(), &'static str> {
    let mut draft = draft();
    draft.scope = SourceScope::Prefix;
    draft.prefix = "Series/Season 1".to_owned();
    let request = draft.request(&[profile()], &[source()])?;
    assert_eq!(request.root_relative_path(), "Series/Season 1");
    assert_eq!(
        request.modes(),
        DiscoveryModes {
            manual_enabled: false,
            watcher_enabled: false,
            schedule_enabled: false
        }
    );
    Ok(())
}

#[test]
fn rejects_paths_and_stale_or_unready_bindings_without_mutating_draft() {
    let mut draft = draft();
    draft.scope = SourceScope::Prefix;
    for prefix in ["/private/library", "../library", "a/../b"] {
        draft.prefix = prefix.to_owned();
        assert!(draft.request(&[profile()], &[source()]).is_err());
        assert_eq!(draft.prefix, prefix);
    }
    draft.scope = SourceScope::WholeRoot;
    assert!(draft.request(&[], &[source()]).is_err());
    assert!(
        draft
            .request(
                &[ActiveProfile {
                    version: 4,
                    ..profile()
                }],
                &[source()]
            )
            .is_err()
    );
    assert!(
        draft
            .request(
                &[profile()],
                &[SourceChoice {
                    binding_ready: false,
                    ..source()
                }]
            )
            .is_err()
    );
    assert!(draft.request(&[profile()], &[]).is_err());
    draft.profile = None;
    assert_eq!(
        draft.request(&[profile()], &[source()]).err(),
        Some("Select an active profile version.")
    );
}

#[test]
fn mutation_errors_are_bounded_and_never_authorize_automatic_retry() {
    for status in [0, 400, 401, 403, 404, 405, 409, 412, 413, 428, 500, 503] {
        assert!(!mutation_failure(status).is_empty());
    }
    assert!(mutation_failure(412).contains("preserved draft"));
    assert!(mutation_failure(0).contains("Check existing associations"));
}

#[test]
fn profile_projection_requires_immutable_versions_and_valid_heads()
-> Result<(), Box<dyn std::error::Error>> {
    use crate::models::media_configuration::{ProfileHead, ProfileHeadPage};
    let legacy = serde_json::json!({"profiles": [{"media_profile_public_id": profile().id, "profile_key": "balanced", "configuration_version": 3}]});
    assert!(serde_json::from_value::<ProfileHeadPage>(legacy).is_err());
    for version in [0, 4] {
        let page = ProfileHeadPage {
            profiles: vec![ProfileHead {
                media_profile_public_id: profile().id,
                profile_key: "balanced".to_owned(),
                latest_version: 3,
                active_version: Some(version),
            }],
            next_cursor: None,
        };
        assert!(ProfileChoices::try_from(page).is_err());
    }
    let page: ProfileHeadPage = serde_json::from_value(serde_json::json!({
        "profiles": [
            {"media_profile_public_id": profile().id, "profile_key": "balanced", "latest_version": 4, "active_version": 3},
            {"media_profile_public_id": Uuid::from_u128(2), "profile_key": "draft", "latest_version": 1, "active_version": null}
        ], "next_cursor": "next-profile-page"
    }))?;
    let choices = ProfileChoices::try_from(page)?;
    assert_eq!(choices.active, vec![profile()]);
    assert_eq!(choices.next_cursor.as_deref(), Some("next-profile-page"));
    Ok(())
}

#[test]
fn creation_confirmation_requires_matching_key_version_and_strong_etag() -> Result<(), &'static str>
{
    let draft = AssociationDraft {
        scope: SourceScope::WholeRoot,
        ..draft()
    };
    let request = draft.request(&[profile()], &[source()])?;
    let mut response = AssociationCreated {
        media_discovery_association_public_id: Uuid::from_u128(5),
        association_key: draft.key,
        latest_version: 1,
        active_version: Some(1),
        media_profile_public_id: request.media_profile_public_id(),
        profile_version: request.profile_version(),
        source_root_key: request.source_root_key().to_owned(),
        root_relative_path: request.root_relative_path().to_owned(),
        manual_enabled: request.modes().manual_enabled,
        watcher_enabled: request.modes().watcher_enabled,
        schedule_enabled: request.modes().schedule_enabled,
    };
    assert!(
        confirm_creation(&response, "\"association-version-1\"", &request)?
            .contains("active version 1")
    );
    for field in 0..7 {
        let mut changed = response.clone();
        match field {
            0 => changed.media_profile_public_id = Uuid::from_u128(999),
            1 => changed.profile_version += 1,
            2 => changed.source_root_key = "other-source".to_owned(),
            3 => changed.root_relative_path = "other-scope".to_owned(),
            4 => changed.manual_enabled = !changed.manual_enabled,
            5 => changed.watcher_enabled = !changed.watcher_enabled,
            _ => changed.schedule_enabled = !changed.schedule_enabled,
        }
        assert!(confirm_creation(&changed, "\"association-version-1\"", &request).is_err());
    }
    response.active_version = None;
    assert!(
        confirm_creation(&response, "\"association-version-1\"", &request)?
            .contains("draft version 1")
    );
    for tag in [
        "",
        "W/\"weak\"",
        "unquoted",
        "\"bad\"quote\"",
        "\"bad\nline\"",
    ] {
        assert!(confirm_creation(&response, tag, &request).is_err());
    }
    response.latest_version = 2;
    assert!(confirm_creation(&response, "\"tag\"", &request).is_err());
    response.latest_version = 1;
    response.active_version = Some(2);
    assert!(confirm_creation(&response, "\"tag\"", &request).is_err());
    response.active_version = Some(1);
    response.association_key = "another-association".to_owned();
    assert!(confirm_creation(&response, "\"tag\"", &request).is_err());
    Ok(())
}
