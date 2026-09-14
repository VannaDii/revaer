use serde_json::{Value, json};
use uuid::Uuid;

use super::{DiscoveryAssociationRequest, DiscoveryModes, RootInputError};

fn body() -> Value {
    json!({
        "association_key": "library-1",
        "media_profile_public_id": "00000000-0000-0000-0000-000000000001",
        "profile_version": 1,
        "source_root_key": "source-1",
        "root_relative_path": "",
        "manual_enabled": true,
        "watcher_enabled": false,
        "schedule_enabled": false
    })
}

fn decode(value: Value) -> serde_json::Result<DiscoveryAssociationRequest> {
    serde_json::from_value(value)
}

fn rejected(value: Value) -> Result<(), Box<dyn std::error::Error>> {
    let error = decode(value).err().ok_or("invalid input was accepted")?;
    assert!(error.to_string().starts_with("media root input is invalid"));
    assert!(!error.to_string().contains("private-sentinel"));
    Ok(())
}

#[test]
fn exact_whole_root_body_round_trips() -> Result<(), Box<dyn std::error::Error>> {
    let request = decode(body())?;
    assert_eq!(serde_json::to_value(&request)?, body());
    assert_eq!(request.association_key(), "library-1");
    assert_eq!(request.media_profile_public_id(), Uuid::from_u128(1));
    assert_eq!(request.profile_version(), 1);
    assert_eq!(request.source_root_key(), "source-1");
    assert_eq!(request.root_relative_path(), "");
    assert_eq!(
        request.modes(),
        DiscoveryModes {
            manual_enabled: true,
            watcher_enabled: false,
            schedule_enabled: false,
        }
    );
    assert_eq!(decode(serde_json::to_value(&request)?)?, request);
    Ok(())
}

#[test]
fn every_field_is_required_and_nonnull() -> Result<(), Box<dyn std::error::Error>> {
    let original = body();
    for key in original
        .as_object()
        .ok_or("test body must be an object")?
        .keys()
    {
        let mut omitted = original.clone();
        let removed = omitted.as_object_mut().ok_or("object")?.remove(key);
        assert!(removed.is_some());
        rejected(omitted)?;
        let mut null = original.clone();
        null[key] = Value::Null;
        rejected(null)?;
    }
    Ok(())
}

#[test]
fn raw_path_fields_and_unknown_fields_are_rejected() -> Result<(), Box<dyn std::error::Error>> {
    for field in [
        "source_root",
        "output_root",
        "source_path",
        "source_paths",
        "slot_public_id",
        "generation",
        "schedule_interval_minutes",
        "private-sentinel",
    ] {
        let mut value = body();
        value[field] = json!("/private-sentinel/media");
        rejected(value)?;
    }
    Ok(())
}

#[test]
fn only_an_object_is_a_request() -> Result<(), Box<dyn std::error::Error>> {
    for value in [
        Value::Null,
        json!(true),
        json!(1),
        json!("private-sentinel"),
        json!([]),
        json!([
            "library-1",
            "00000000-0000-0000-0000-000000000001",
            1,
            "source-1",
            "",
            true,
            false,
            false
        ]),
    ] {
        rejected(value)?;
    }
    Ok(())
}

#[test]
fn every_duplicate_field_is_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let original = body();
    let encoded = serde_json::to_string(&original)?;
    let prefix = encoded.strip_suffix('}').ok_or("object terminator")?;
    for (key, value) in original.as_object().ok_or("object")? {
        let duplicate = format!("{prefix},{}:{value}}}", serde_json::to_string(key)?);
        let error = serde_json::from_str::<DiscoveryAssociationRequest>(&duplicate)
            .err()
            .ok_or("duplicate field was accepted")?;
        assert!(error.to_string().starts_with("media root input is invalid"));
    }
    Ok(())
}

#[test]
fn logical_key_grammar_is_enforced_without_coercion() -> Result<(), Box<dyn std::error::Error>> {
    for field in ["association_key", "source_root_key"] {
        for key in [
            "",
            "UPPER",
            "under_score",
            "-edge",
            "edge-",
            "a/b",
            "a\\b",
            "with space",
            " key",
            "key ",
            "/private-sentinel",
            "a\0b",
            "a.b",
        ] {
            let mut value = body();
            value[field] = json!(key);
            rejected(value)?;
        }
        let mut value = body();
        value[field] = json!("a".repeat(65));
        rejected(value)?;
        for key in ["1".to_owned(), "a--9".to_owned(), "a".repeat(64)] {
            let mut value = body();
            value[field] = json!(key);
            assert_eq!(serde_json::to_value(decode(value.clone())?)?, value);
        }
    }
    Ok(())
}

#[test]
fn prefixes_are_relative_bounded_and_exact() -> Result<(), Box<dyn std::error::Error>> {
    for prefix in [
        "/absolute",
        "trailing/",
        "a//b",
        ".",
        "..",
        "a/../b",
        "a/./b",
        "a\\b",
        "a\0b",
    ] {
        let mut value = body();
        value["root_relative_path"] = json!(prefix);
        rejected(value)?;
    }
    let mut oversized = body();
    oversized["root_relative_path"] = json!("a".repeat(4_097));
    rejected(oversized)?;
    for prefix in [
        String::new(),
        "season-1/episode".to_owned(),
        " space /\u{e9}".to_owned(),
        "a".repeat(4_096),
    ] {
        let mut value = body();
        value["root_relative_path"] = json!(prefix);
        assert_eq!(decode(value.clone())?.root_relative_path(), prefix);
        assert_eq!(serde_json::to_value(decode(value.clone())?)?, value);
    }
    Ok(())
}

#[test]
fn version_is_a_positive_postgres_integer() -> Result<(), Box<dyn std::error::Error>> {
    for version in [
        json!(0),
        json!(-1),
        json!(2_147_483_648_u64),
        json!(u64::MAX),
        json!(1.0),
        json!("1"),
        json!(true),
    ] {
        let mut value = body();
        value["profile_version"] = version;
        rejected(value)?;
    }
    for version in [1, i32::MAX] {
        let mut value = body();
        value["profile_version"] = json!(version);
        assert_eq!(decode(value)?.profile_version(), version);
    }
    Ok(())
}

#[test]
fn fields_have_strict_json_types() -> Result<(), Box<dyn std::error::Error>> {
    for field in [
        "association_key",
        "media_profile_public_id",
        "source_root_key",
        "root_relative_path",
    ] {
        for value in [json!(0), json!(true), json!([]), json!({})] {
            let mut request = body();
            request[field] = value;
            rejected(request)?;
        }
    }
    let mut malformed_uuid = body();
    malformed_uuid["media_profile_public_id"] = json!("private-sentinel");
    rejected(malformed_uuid)?;
    let mut positional_uuid = body();
    positional_uuid["media_profile_public_id"] =
        json!([0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
    rejected(positional_uuid)?;
    for field in ["manual_enabled", "watcher_enabled", "schedule_enabled"] {
        for value in [json!(0), json!(1), json!("false"), json!([]), json!({})] {
            let mut request = body();
            request[field] = value;
            rejected(request)?;
        }
    }
    Ok(())
}

#[test]
fn mode_combinations_are_preserved_not_admitted() -> Result<(), Box<dyn std::error::Error>> {
    for manual in [false, true] {
        for watcher in [false, true] {
            for schedule in [false, true] {
                let mut value = body();
                value["manual_enabled"] = json!(manual);
                value["watcher_enabled"] = json!(watcher);
                value["schedule_enabled"] = json!(schedule);
                let request = decode(value.clone())?;
                assert_eq!(
                    request.modes(),
                    DiscoveryModes {
                        manual_enabled: manual,
                        watcher_enabled: watcher,
                        schedule_enabled: schedule,
                    }
                );
                assert_eq!(serde_json::to_value(request)?, value);
            }
        }
    }
    Ok(())
}

#[test]
fn constructor_and_diagnostics_preserve_the_boundary() -> Result<(), Box<dyn std::error::Error>> {
    let modes = DiscoveryModes {
        manual_enabled: false,
        watcher_enabled: true,
        schedule_enabled: true,
    };
    for (key, version, source, prefix) in [
        ("/private-sentinel", 1, "source", ""),
        ("library", 0, "source", ""),
        ("library", -1, "source", ""),
        ("library", 1, "/private-sentinel", ""),
        ("library", 1, "source", "../private-sentinel"),
    ] {
        assert_eq!(
            DiscoveryAssociationRequest::new(
                key,
                Uuid::from_u128(1),
                version,
                source,
                prefix,
                modes,
            ),
            Err(RootInputError)
        );
    }
    let request = DiscoveryAssociationRequest::new(
        "private-sentinel",
        Uuid::nil(),
        i32::MAX,
        "source",
        "private-sentinel",
        modes,
    )?;
    assert_eq!(request.media_profile_public_id(), Uuid::nil());
    assert_eq!(request.modes(), modes);
    assert_eq!(request.profile_version(), i32::MAX);
    assert_eq!(format!("{request:?}"), "DiscoveryAssociationRequest { .. }");
    assert_eq!(RootInputError.to_string(), "media root input is invalid");
    Ok(())
}
