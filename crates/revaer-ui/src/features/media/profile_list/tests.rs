use super::ProfilePage;
use revaer_api_models::media_root_contract::{ProfileCollectionCursor, ProfileVersionPageResponse};

fn profile() -> serde_json::Value {
    serde_json::json!({
        "media_profile_public_id": uuid::Uuid::nil(), "profile_key": "movies",
        "display_name": " Exact name ", "description": "Exact description",
        "enabled": false, "dry_run_only": true, "desired_target_key": "target", "desired_target_version": 2,
        "policy_key": "safe-dry-run", "policy_version": 3, "output_root_key": "output", "workspace_root_key": "workspace",
        "latest_version": 4, "active_version": 4, "lifecycle_state": "active",
        "created_at": "2026-10-01T00:00:00Z", "updated_at": "2026-10-01T00:00:00Z",
        "root_bindings": [
            {"kind": "output", "logical_key": "output", "resolution_state": "resolved",
                "binding_ready": true, "destructive_ready": true},
            {"kind": "workspace", "logical_key": "workspace", "resolution_state": "resolved",
                "binding_ready": true, "destructive_ready": false, "destructive_reason": "media_root_durability_unproven"}
        ]
    })
}

#[test]
fn page_preserves_disabled_heads_exact_values_and_readiness()
-> Result<(), Box<dyn std::error::Error>> {
    let token = ProfileCollectionCursor::new("movies", uuid::Uuid::nil())?.encode()?;
    let response: ProfileVersionPageResponse = serde_json::from_value(serde_json::json!({
        "profiles": [profile()], "next_cursor": token,
    }))?;
    let page = ProfilePage::from(response);
    assert_eq!(page.profiles.len(), 1);
    assert_eq!(page.next_cursor.as_deref(), Some(token.as_str()));
    let summary = &page.profiles[0];
    assert_eq!(summary.name, " Exact name ");
    assert_eq!(summary.description, "Exact description");
    assert_eq!(summary.latest, 4);
    assert_eq!(summary.active, Some(4));
    assert!(!summary.enabled);
    assert!(summary.dry_run_only);
    assert_eq!(summary.target, "target v2");
    assert_eq!(summary.policy, "safe-dry-run v3");
    assert_eq!(summary.roots[0].kind, "Output");
    assert_eq!(summary.roots[1].key, "workspace");
    assert!(summary.roots[1].binding);
    assert!(!summary.roots[1].destructive);
    assert_eq!(
        summary.roots[1].reason.as_deref(),
        Some("media_root_durability_unproven")
    );
    Ok(())
}

#[test]
fn page_rejects_legacy_partial_and_incoherent_transport() -> Result<(), Box<dyn std::error::Error>>
{
    for invalid in [
        serde_json::json!({"profiles": [{"profile_key": "movies", "source_root": "/input", "output_root": "/output"}]}),
        serde_json::json!({"profiles": [profile()], "next_cursor": "bad-cursor"}),
        serde_json::json!({"profiles": [profile(), profile()]}),
        serde_json::json!({"profiles": [], "offset": 0}),
    ] {
        assert!(serde_json::from_value::<ProfileVersionPageResponse>(invalid).is_err());
    }
    let response: ProfileVersionPageResponse =
        serde_json::from_value(serde_json::json!({"profiles": []}))?;
    let page = ProfilePage::from(response);
    assert!(page.profiles.is_empty());
    assert!(page.next_cursor.is_none());
    Ok(())
}
