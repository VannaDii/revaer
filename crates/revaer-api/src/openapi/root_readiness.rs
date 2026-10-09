//! Public path-free catalog readiness documentation.

use serde_json::{Value, json};

pub(super) fn path() -> (&'static str, Value) {
    let (_, mut value) = super::media_single_path(
        "/v1/media/root-catalog/readiness",
        super::media_op(
            "get",
            "Read root catalog readiness",
            "200",
            "Path-free root readiness",
            Some("RootCatalogReadinessResponse"),
            None,
            super::MediaParameterSet::None,
        ),
    );
    value["get"]["responses"]["200"]["headers"] = json!({
        "Cache-Control": {"schema": {"type": "string", "enum": ["no-store"]}}
    });
    ("/v1/media/root-catalog/readiness", value)
}

pub(super) fn states() -> Vec<Value> {
    let mut states = vec![state("ready", "ready", None, None, true)];
    for (source, reason) in [
        ("missing", "media_root_catalog_source_missing"),
        ("untrusted", "media_root_catalog_source_untrusted"),
        ("invalid", "media_root_catalog_format_invalid"),
        ("bound_exceeded", "media_root_catalog_bound_exceeded"),
        ("unsupported", "media_root_platform_unsupported"),
    ] {
        states.push(state(source, "not_evaluated", Some(reason), None, false));
    }
    for reason in [
        "media_root_attestation_invalid",
        "media_root_overlap",
        "media_root_unsafe_ancestry",
        "media_root_durability_unproven",
        "media_root_writer_control_unproven",
        "media_root_identity_mismatch",
    ] {
        states.push(state("ready", "invalid", None, Some(reason), false));
    }
    states
}

pub(super) fn schemas() -> Vec<(&'static str, Value)> {
    let count = json!({"type": "integer", "minimum": 0, "maximum": 256});
    vec![
        ("RootCatalogReadinessResponse", json!({"oneOf": states()})),
        (
            "RootKindReadiness",
            json!({
                "type": "object", "additionalProperties": false,
                "required": ["kind", "attested_slot_count", "binding_ready_slot_count", "destructive_ready_slot_count"],
                "properties": {
                    "kind": {"type": "string", "enum": ["source", "output", "workspace", "backup", "quarantine"]},
                    "attested_slot_count": count,
                    "binding_ready_slot_count": count,
                    "destructive_ready_slot_count": count
                },
                "description": "Counts satisfy destructive <= binding <= attested. All counts are zero without an active generation."
            }),
        ),
    ]
}

fn state(
    source: &str,
    attestation: &str,
    source_reason: Option<&str>,
    attestation_reason: Option<&str>,
    generation: bool,
) -> Value {
    let mut required = vec![
        "format_version",
        "source_state",
        "attestation_state",
        "kinds",
    ];
    let mut properties = json!({
        "format_version": {"type": "integer", "enum": [1]},
        "source_state": {"type": "string", "enum": [source]},
        "attestation_state": {"type": "string", "enum": [attestation]},
        "kinds": {
            "type": "array", "minItems": 5, "maxItems": 5,
            "items": {"$ref": "#/components/schemas/RootKindReadiness"},
            "description": "Exactly source, output, workspace, backup, quarantine in that order."
        }
    });
    for (field, reason) in [
        ("source_reason", source_reason),
        ("attestation_reason", attestation_reason),
    ] {
        if let Some(reason) = reason {
            required.push(field);
            properties[field] = json!({"type": "string", "enum": [reason]});
        }
    }
    if generation {
        required.push("generation");
        properties["generation"] = json!({
            "type": "string", "pattern": "^[1-9][0-9]{0,18}$",
            "description": "Canonical decimal generation, at most 9223372036854775807."
        });
    }
    json!({"type": "object", "additionalProperties": false, "required": required, "properties": properties})
}
