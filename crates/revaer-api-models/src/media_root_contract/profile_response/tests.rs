use super::*;
use serde_json::{Value, json};

fn response() -> Value {
    json!({
        "profile_key": "library", "display_name": " Library ", "description": "",
        "enabled": false, "dry_run_only": true,
        "desired_target_key": "archive", "desired_target_version": 2,
        "policy_key": "preserve", "policy_version": 3,
        "output_root_key": "output", "workspace_root_key": "workspace",
        "media_profile_public_id": "00000000-0000-0000-0000-000000000011",
        "latest_version": 2, "active_version": 2, "lifecycle_state": "active",
        "root_bindings": [
            { "kind": "output", "logical_key": "output", "resolution_state": "resolved",
              "binding_ready": true, "destructive_ready": true },
            { "kind": "workspace", "logical_key": "workspace", "resolution_state": "resolved",
              "binding_ready": true, "destructive_ready": false,
              "destructive_reason": "media_root_durability_unproven" }
        ],
        "created_at": "2026-09-21T00:00:00Z", "updated_at": "2026-09-21T01:00:00+00:00"
    })
}

#[test]
fn complete_response_round_trips_disabled_active_version() -> Result<(), Box<dyn std::error::Error>>
{
    let value = response();
    let decoded: ProfileVersionResponse = serde_json::from_value(value.clone())?;
    assert!(!decoded.fields().profile.fields().enabled);
    assert_eq!(serde_json::to_value(&decoded)?, value);
    assert_eq!(
        format!("{decoded:?}"),
        "ProfileVersionResponse(ProfileVersionResponseFields)"
    );
    Ok(())
}

#[test]
fn lifecycle_heads_are_checked_independently_of_enablement() {
    for (lifecycle, active, accepted) in [
        ("active", Some(2), true),
        ("active", Some(1), false),
        ("active", None, false),
        ("draft", Some(1), true),
        ("draft", None, true),
        ("draft", Some(2), false),
        ("archived", None, true),
        ("archived", Some(1), false),
        ("draft", Some(0), false),
        ("draft", Some(3), false),
    ] {
        let mut value = response();
        value["lifecycle_state"] = json!(lifecycle);
        if let Some(active) = active {
            value["active_version"] = json!(active);
        } else if let Some(object) = value.as_object_mut() {
            object.remove("active_version");
        }
        assert_eq!(
            serde_json::from_value::<ProfileVersionResponse>(value).is_ok(),
            accepted
        );
    }
}

#[test]
fn optional_roots_require_exact_bindings_and_allow_unmapped_drafts()
-> Result<(), Box<dyn std::error::Error>> {
    let mut input = response();
    input["backup_root_key"] = json!("backup");
    assert!(serde_json::from_value::<ProfileVersionResponse>(input.clone()).is_err());
    input["lifecycle_state"] = json!("draft");
    input["active_version"] = json!(1);
    let binding = json!({
        "kind": "backup", "logical_key": "backup", "resolution_state": "unmapped",
        "binding_ready": false, "binding_reason": "media_configuration_root_unmapped",
        "destructive_ready": false, "destructive_reason": "media_configuration_root_unmapped"
    });
    input["root_bindings"]
        .as_array_mut()
        .ok_or("bindings missing")?
        .push(binding.clone());
    let parsed: ProfileVersionResponse = serde_json::from_value(input.clone())?;
    assert_eq!(serde_json::to_value(&parsed)?, input);
    let mut fields = parsed.fields().clone();
    fields.root_bindings.push(fields.root_bindings[0].clone());
    assert!(ProfileVersionResponse::new(fields).is_err());
    input["root_bindings"]
        .as_array_mut()
        .ok_or("bindings missing")?
        .push(binding);
    assert!(serde_json::from_value::<ProfileVersionResponse>(input).is_err());
    Ok(())
}

#[test]
fn bindings_must_match_exact_keys_order_and_readiness() {
    for (field, value) in [
        ("kind", json!("source")),
        ("logical_key", json!("other")),
        ("resolution_state", json!("unmapped")),
        ("resolution_state", json!("unknown")),
        ("binding_ready", json!(false)),
        ("binding_reason", json!("private/path")),
        ("canonical_path", json!("/private/path")),
    ] {
        let mut input = response();
        input["root_bindings"][0][field] = value;
        assert!(serde_json::from_value::<ProfileVersionResponse>(input).is_err());
    }
    for bindings in [
        json!([]),
        json!([response()["root_bindings"][0]]),
        json!([
            response()["root_bindings"][1],
            response()["root_bindings"][0]
        ]),
    ] {
        let mut input = response();
        input["root_bindings"] = bindings;
        assert!(serde_json::from_value::<ProfileVersionResponse>(input).is_err());
    }
}

#[test]
fn decoder_rejects_null_unknown_duplicate_and_invalid_scalars()
-> Result<(), Box<dyn std::error::Error>> {
    for (key, value) in [
        ("active_version", Value::Null),
        ("backup_root_key", Value::Null),
        ("etag", json!("tag")),
        ("latest_version", json!(0)),
        ("updated_at", json!("2026-09-21T00:00:00+01:00")),
        ("created_at", json!("invalidZ")),
        ("policy_version", json!(0)),
    ] {
        let mut input = response();
        input[key] = value;
        assert!(serde_json::from_value::<ProfileVersionResponse>(input).is_err());
    }
    let encoded = serde_json::to_string(&response())?;
    let mut positional = response();
    positional["root_bindings"][0] =
        json!(["output", "output", "resolved", true, null, true, null]);
    assert!(serde_json::from_value::<ProfileVersionResponse>(positional).is_err());
    let duplicate = encoded.replacen('{', "{\"enabled\":true,", 1);
    assert!(serde_json::from_str::<ProfileVersionResponse>(&duplicate).is_err());
    assert!(serde_json::from_str::<ProfileVersionResponse>("[]").is_err());
    Ok(())
}
