//! Authenticated administrative root catalog, the sole path-bearing root response.

use serde_json::{Value, json};

pub(super) fn path() -> (&'static str, Value) {
    let (_, mut value) = super::media_single_path(
        "/v1/media/root-catalog",
        super::media_op(
            "get",
            "Read root catalog",
            "200",
            "Persisted root catalog page",
            Some("RootCatalogPageResponse"),
            None,
            super::MediaParameterSet::None,
        ),
    );
    value["get"]["parameters"] = json!([
        {"name": "limit", "in": "query", "schema": {"type": "integer", "minimum": 1, "maximum": 200, "default": 50}},
        {"name": "cursor", "in": "query", "schema": {"type": "string", "maxLength": 110},
         "description": "Canonical unpadded base64url cursor identifying an active slot."}
    ]);
    value["get"]["responses"]["200"]["headers"] = json!({
        "Cache-Control": {"schema": {"type": "string", "enum": ["no-store"]}}
    });
    ("/v1/media/root-catalog", value)
}

pub(super) fn schemas() -> Vec<(&'static str, Value)> {
    let mut states = super::root_readiness::states();
    for state in &mut states {
        let ready = state["properties"]["generation"].is_object();
        if let Some(properties) = state["properties"].as_object_mut() {
            properties.remove("kinds");
        }
        let mut required = vec![
            "format_version",
            "source_state",
            "attestation_state",
            "slots",
        ];
        for reason in ["source_reason", "attestation_reason"] {
            if state["properties"].get(reason).is_some() {
                required.push(reason);
            }
        }
        state["properties"]["slots"] = json!({
            "type": "array", "maxItems": if ready { 200 } else { 0 },
            "items": {"$ref": "#/components/schemas/RootCatalogSlot"}
        });
        if ready {
            required.push("generation");
            state["properties"]["generation"] =
                json!({"$ref": "#/components/schemas/RootCatalogGeneration"});
            state["properties"]["next_cursor"] = json!({"type": "string", "maxLength": 110});
        }
        state["required"] = json!(required);
    }
    vec![
        ("RootCatalogPageResponse", json!({"oneOf": states})),
        ("RootCatalogGeneration", generation()),
        ("RootCatalogSlot", slot()),
        ("RootCatalogAllowedKind", allowed_kind()),
    ]
}

fn closed_object(properties: &Value) -> Value {
    let required: Vec<_> = properties
        .as_object()
        .into_iter()
        .flat_map(|map| map.keys())
        .collect();
    json!({"type": "object", "additionalProperties": false, "required": required, "properties": properties})
}

fn generation() -> Value {
    let digest = json!({"type": "string", "pattern": "^[0-9a-f]{64}$"});
    closed_object(&json!({
        "media_root_catalog_generation_public_id": {"type": "string", "format": "uuid"},
        "generation": {"type": "string", "pattern": "^[1-9][0-9]{0,18}$"},
        "source_sha256": digest, "generation_sha256": digest,
        "slot_count": {"type": "integer", "minimum": 0, "maximum": 256},
        "activated_at": {"type": "string", "format": "date-time"},
        "reconciled_at": {"type": "string", "format": "date-time"}
    }))
}

fn slot() -> Value {
    let path = json!({"type": "string", "minLength": 2, "maxLength": 4096});
    let identity = json!({"type": "string", "pattern": "^[0-9a-f]{16}$"});
    let owner = json!({"type": "integer", "minimum": 0, "maximum": 4_294_967_295_u64});
    closed_object(&json!({
        "media_root_catalog_slot_public_id": {"type": "string", "format": "uuid"},
        "logical_key": {"type": "string", "minLength": 1, "maxLength": 64},
        "requested_path": path, "canonical_path": path,
        "filesystem_device": identity, "filesystem_inode": identity,
        "mount_id": {"type": "string", "pattern": "^(0|[1-9][0-9]{0,18})$"},
        "filesystem_type": {"type": "string", "minLength": 1, "maxLength": 64},
        "read_capable": {"type": "boolean"}, "write_capable": {"type": "boolean"},
        "create_new_capable": {"type": "boolean"}, "fsync_capable": {"type": "boolean"},
        "rename_capable": {"type": "boolean"}, "delete_capable": {"type": "boolean"},
        "capacity_probe_capable": {"type": "boolean"},
        "durability_class": {"type": "string", "enum": ["disposable", "restart_persistent"]},
        "durability_evidence": {"type": "string", "enum": ["none", "linux_dedicated_mount", "kubernetes_persistent_volume_claim"]},
        "sole_writer_class": {"type": "string", "enum": ["uncontrolled", "revaer_exclusive"]},
        "sole_writer_evidence": {"type": "string", "enum": ["none", "linux_dedicated_service", "kubernetes_read_write_once_pod"]},
        "owner_uid": owner, "owner_gid": owner,
        "mode_bits": {"type": "string", "pattern": "^[0-7]{4}$"},
        "validated_at": {"type": "string", "format": "date-time"},
        "root_identity_sha256": {"type": "string", "pattern": "^[0-9a-f]{64}$"},
        "allowed_kinds": {"type": "array", "minItems": 1, "maxItems": 5,
            "items": {"$ref": "#/components/schemas/RootCatalogAllowedKind"}}
    }))
}

fn allowed_kind() -> Value {
    let reasons = [
        "media_configuration_root_unmapped",
        "media_root_slot_unknown",
        "media_root_kind_forbidden",
        "media_root_binding_incomplete",
        "media_root_attestation_stale",
        "media_root_attestation_invalid",
        "media_root_overlap",
        "media_root_unsafe_ancestry",
        "media_root_durability_unproven",
        "media_root_writer_control_unproven",
        "media_root_identity_mismatch",
    ];
    json!({
        "type": "object", "additionalProperties": false,
        "required": ["kind", "binding_ready", "destructive_ready"],
        "properties": {
            "kind": {"type": "string", "enum": ["source", "output", "workspace", "backup", "quarantine"]},
            "binding_ready": {"type": "boolean"}, "destructive_ready": {"type": "boolean"},
            "binding_reason": {"type": "string", "enum": reasons},
            "destructive_reason": {"type": "string", "enum": reasons}
        }
    })
}
