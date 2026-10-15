use serde_json::{Value, json};

use super::{ProfileVersionRequest, ProfileVersionRequestFields, RootInputError};

const KEY_FIELDS: [&str; 7] = [
    "profile_key",
    "desired_target_key",
    "policy_key",
    "output_root_key",
    "workspace_root_key",
    "backup_root_key",
    "quarantine_root_key",
];

fn body() -> Value {
    json!({
        "profile_key": "library-1",
        "display_name": " Library \u{e9} ",
        "description": " Exact description.\n",
        "enabled": false,
        "dry_run_only": true,
        "desired_target_key": "archive-1",
        "desired_target_version": 2,
        "policy_key": "preserve-1",
        "policy_version": 3,
        "output_root_key": "output-1",
        "workspace_root_key": "workspace-1"
    })
}

fn complete_body() -> Value {
    let mut value = body();
    value["backup_root_key"] = json!("backup-1");
    value["quarantine_root_key"] = json!("quarantine-1");
    value
}

fn decode(value: Value) -> serde_json::Result<ProfileVersionRequest> {
    serde_json::from_value(value)
}

fn rejected(value: Value) -> Result<(), Box<dyn std::error::Error>> {
    let error = decode(value).err().ok_or("invalid input was accepted")?;
    assert!(error.to_string().starts_with("media root input is invalid"));
    assert!(!error.to_string().contains("private-sentinel"));
    Ok(())
}

#[test]
fn exact_body_and_constructor_round_trip() -> Result<(), Box<dyn std::error::Error>> {
    let fields = ProfileVersionRequestFields {
        profile_key: "library-1".into(),
        display_name: " Library \u{e9} ".into(),
        description: " Exact description.\n".into(),
        enabled: false,
        dry_run_only: true,
        desired_target_key: "archive-1".into(),
        desired_target_version: 2,
        policy_key: "preserve-1".into(),
        policy_version: 3,
        output_root_key: "output-1".into(),
        workspace_root_key: "workspace-1".into(),
        backup_root_key: None,
        quarantine_root_key: None,
    };
    let request = ProfileVersionRequest::new(fields.clone())?;
    assert_eq!(request.fields(), &fields);
    assert_eq!(decode(body())?, request);
    assert_eq!(serde_json::to_value(&request)?, body());
    assert_eq!(decode(serde_json::to_value(&request)?)?, request);
    Ok(())
}

#[test]
fn every_required_field_must_be_present() -> Result<(), Box<dyn std::error::Error>> {
    let original = body();
    for key in original.as_object().ok_or("object")?.keys() {
        let mut value = original.clone();
        assert!(value.as_object_mut().ok_or("object")?.remove(key).is_some());
        rejected(value)?;
    }
    Ok(())
}

#[test]
fn optional_root_combinations_preserve_absence() -> Result<(), Box<dyn std::error::Error>> {
    for backup in [false, true] {
        for quarantine in [false, true] {
            let mut value = body();
            if backup {
                value["backup_root_key"] = json!("backup-1");
            }
            if quarantine {
                value["quarantine_root_key"] = json!("quarantine-1");
            }
            let request = decode(value.clone())?;
            assert_eq!(request.fields().backup_root_key.is_some(), backup);
            assert_eq!(request.fields().quarantine_root_key.is_some(), quarantine);
            assert_eq!(serde_json::to_value(&request)?, value);
            assert_eq!(decode(serde_json::to_value(request)?)?, decode(value)?);
        }
    }
    Ok(())
}

#[test]
fn all_fields_reject_null_and_incorrect_types() -> Result<(), Box<dyn std::error::Error>> {
    let original = complete_body();
    for (key, valid) in original.as_object().ok_or("object")? {
        for invalid in [
            Value::Null,
            json!([]),
            json!({}),
            json!("private-sentinel"),
            json!(true),
            json!(1),
            json!(1.0),
        ] {
            let same_type = (valid.is_string() && invalid.is_string())
                || (valid.is_boolean() && invalid.is_boolean())
                || (valid.is_i64() && invalid.is_i64());
            if !same_type {
                let mut value = original.clone();
                value[key] = invalid;
                rejected(value)?;
            }
        }
    }
    Ok(())
}

#[test]
fn duplicates_are_rejected_for_every_field() -> Result<(), Box<dyn std::error::Error>> {
    let original = complete_body();
    let encoded = serde_json::to_string(&original)?;
    let prefix = encoded.strip_suffix('}').ok_or("object terminator")?;
    for (key, value) in original.as_object().ok_or("object")? {
        for repeated in [value.clone(), Value::Null] {
            let duplicate = format!("{prefix},{}:{repeated}}}", serde_json::to_string(key)?);
            let error = serde_json::from_str::<ProfileVersionRequest>(&duplicate)
                .err()
                .ok_or("duplicate field was accepted")?;
            assert!(error.to_string().starts_with("media root input is invalid"));
        }
    }
    Ok(())
}

#[test]
fn unapproved_fields_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    for field in [
        "source_root_key",
        "output_root",
        "workspace_root",
        "backup_root",
        "quarantine_root",
        "path",
        "root_bindings",
        "manual_enabled",
        "watcher_enabled",
        "schedule_enabled",
        "retention_override",
        "file_rules",
        "media_profile_public_id",
        "latest_version",
        "active_version",
        "lifecycle_state",
        "created_at",
        "updated_at",
        "etag",
        "private-sentinel",
    ] {
        let mut value = body();
        value[field] = json!("/private-sentinel/media");
        rejected(value)?;
    }
    Ok(())
}

#[test]
fn only_object_requests_are_accepted() -> Result<(), Box<dyn std::error::Error>> {
    for value in [
        Value::Null,
        json!(true),
        json!(1),
        json!("private-sentinel"),
        json!([]),
        json!([
            "library",
            "Library",
            "",
            false,
            true,
            "archive",
            1,
            "preserve",
            1,
            "output",
            "workspace",
            "backup",
            "quarantine"
        ]),
    ] {
        rejected(value)?;
    }
    Ok(())
}

#[test]
fn every_key_uses_exact_bounded_grammar() -> Result<(), Box<dyn std::error::Error>> {
    for field in KEY_FIELDS {
        for invalid in [
            "",
            "UPPER",
            "under_score",
            "-edge",
            "edge-",
            "a/b",
            "a\\b",
            "a.b",
            "with space",
            " key",
            "key ",
            "\u{e9}",
            "a\0b",
            "/private-sentinel",
        ] {
            let mut value = body();
            value[field] = json!(invalid);
            rejected(value)?;
        }
        let mut oversized = body();
        oversized[field] = json!("a".repeat(65));
        rejected(oversized)?;
        for key in ["1".to_owned(), "a--9".to_owned(), "a".repeat(64)] {
            let mut value = body();
            value[field] = json!(key);
            assert_eq!(serde_json::to_value(decode(value.clone())?)?, value);
        }
    }
    Ok(())
}

#[test]
fn text_bounds_count_utf8_bytes_without_normalization() -> Result<(), Box<dyn std::error::Error>> {
    for (field, limit) in [("display_name", 128), ("description", 1_024)] {
        for text in ["a".repeat(limit), "\u{e9}".repeat(limit / 2), " ".into()] {
            let mut value = body();
            value[field] = json!(text);
            assert_eq!(serde_json::to_value(decode(value.clone())?)?, value);
        }
        for text in ["a".repeat(limit + 1), "\u{e9}".repeat(limit / 2 + 1)] {
            let mut value = body();
            value[field] = json!(text);
            rejected(value)?;
        }
    }
    let mut value = body();
    value["description"] = json!("");
    assert_eq!(decode(value.clone())?.fields().description, "");
    value["display_name"] = json!("");
    rejected(value)?;
    Ok(())
}

#[test]
fn both_versions_are_explicit_positive_postgres_integers() -> Result<(), Box<dyn std::error::Error>>
{
    for field in ["desired_target_version", "policy_version"] {
        for invalid in [
            json!(0),
            json!(-1),
            json!(i32::MIN),
            json!(2_147_483_648_u64),
            json!(u64::MAX),
            json!(1.0),
            json!("1"),
            json!(true),
        ] {
            let mut value = body();
            value[field] = invalid;
            rejected(value)?;
        }
        for version in [1, i32::MAX] {
            let mut value = body();
            value[field] = json!(version);
            assert_eq!(serde_json::to_value(decode(value.clone())?)?, value);
        }
    }
    Ok(())
}

#[test]
fn enabled_and_dry_run_flags_are_preserved() -> Result<(), Box<dyn std::error::Error>> {
    for enabled in [false, true] {
        for dry_run_only in [false, true] {
            let mut value = body();
            value["enabled"] = json!(enabled);
            value["dry_run_only"] = json!(dry_run_only);
            let request = decode(value.clone())?;
            assert_eq!(request.fields().enabled, enabled);
            assert_eq!(request.fields().dry_run_only, dry_run_only);
            assert_eq!(serde_json::to_value(request)?, value);
        }
    }
    Ok(())
}

#[test]
fn constructor_cannot_bypass_validation() -> Result<(), Box<dyn std::error::Error>> {
    let original = decode(complete_body())?;
    let mutations: [fn(&mut ProfileVersionRequestFields); 13] = [
        |f| f.profile_key = " key".into(),
        |f| f.desired_target_key = "under_score".into(),
        |f| f.policy_key = "UPPER".into(),
        |f| f.output_root_key = "/private-sentinel".into(),
        |f| f.workspace_root_key = String::new(),
        |f| f.backup_root_key = Some("a".repeat(65)),
        |f| f.quarantine_root_key = Some("bad-".into()),
        |f| f.display_name = String::new(),
        |f| f.display_name = "\u{e9}".repeat(65),
        |f| f.description = "a".repeat(1_025),
        |f| f.desired_target_version = 0,
        |f| f.policy_version = 0,
        |f| f.policy_version = -1,
    ];
    for mutate in mutations {
        let mut fields = original.fields().clone();
        mutate(&mut fields);
        assert_eq!(ProfileVersionRequest::new(fields), Err(RootInputError));
    }
    assert_eq!(
        format!("{original:?}"),
        "ProfileVersionRequest { fields: ProfileVersionRequestFields }"
    );
    Ok(())
}
