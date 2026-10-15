use std::{error::Error, num::NonZeroU64};

use serde_json::{Value, json};
use uuid::Uuid;

use super::super::{
    RootAttestationFailure, RootCatalogAllowedKind, RootCatalogCapability, RootCatalogCursor,
    RootCatalogGeneration, RootCatalogPageResponse, RootCatalogReadinessState, RootCatalogSlot,
    RootKind, RootSourceFailure,
};

type TestResult = Result<(), Box<dyn Error>>;

fn generation(count: u16) -> Value {
    json!({
        "media_root_catalog_generation_public_id": Uuid::from_u128(1),
        "generation": "9223372036854775807",
        "source_sha256": "a".repeat(64), "generation_sha256": "b".repeat(64),
        "slot_count": count, "activated_at": "2026-09-14T01:02:03Z",
        "reconciled_at": "2026-09-14T01:02:03.123456+00:00"
    })
}

fn slot(index: u16) -> Value {
    json!({
        "media_root_catalog_slot_public_id": Uuid::from_u128(u128::from(index)),
        "logical_key": format!("root-{index:03}"),
        "requested_path": format!("/private/requested-{index}"),
        "canonical_path": format!("/private/canonical-{index}"),
        "filesystem_device": "ffffffffffffffff", "filesystem_inode": format!("{index:016x}"),
        "mount_id": "0", "filesystem_type": "ext4",
        "read_capable": true, "write_capable": true, "create_new_capable": true,
        "fsync_capable": true, "rename_capable": true, "delete_capable": true,
        "capacity_probe_capable": true,
        "durability_class": "restart_persistent", "durability_evidence": "linux_dedicated_mount",
        "sole_writer_class": "revaer_exclusive", "sole_writer_evidence": "linux_dedicated_service",
        "owner_uid": u32::MAX, "owner_gid": 0, "mode_bits": "7777",
        "validated_at": "2026-09-14T01:02:03Z", "root_identity_sha256": format!("{index:064x}"),
        "allowed_kinds": [
            { "kind": "source", "binding_ready": true, "destructive_ready": true },
            { "kind": "output", "binding_ready": true, "destructive_ready": true }
        ]
    })
}

fn page(rows: u16, total: u16) -> Value {
    json!({
        "format_version": 1, "source_state": "ready", "attestation_state": "ready",
        "generation": generation(total), "slots": (1..=rows).map(slot).collect::<Vec<_>>()
    })
}

#[test]
fn complete_page_round_trip_and_public_construction() -> TestResult {
    let mut wire = page(1, 256);
    let cursor = RootCatalogCursor::new("root-001", Uuid::from_u128(1))?.encode()?;
    wire["next_cursor"] = json!(cursor);
    let response: RootCatalogPageResponse = serde_json::from_value(wire.clone())?;
    assert_eq!(serde_json::to_value(&response)?, wire);
    assert_eq!(response.next_cursor(), Some(cursor.as_str()));
    let metadata = response.generation().ok_or("missing metadata")?;
    assert_eq!(metadata.fields().slot_count, 256);
    let first = response.slots().first().ok_or("missing slot")?;
    assert_eq!(first.logical_key(), "root-001");
    assert_eq!(first.requested_path(), "/private/requested-1");
    assert_eq!(first.canonical_path(), "/private/canonical-1");
    assert!(
        first.read_capable()
            && first.write_capable()
            && first.create_new_capable()
            && first.fsync_capable()
            && first.rename_capable()
            && first.delete_capable()
            && first.capacity_probe_capable()
    );
    assert_eq!(
        first.fields().read_capable,
        RootCatalogCapability::new(true)
    );
    assert_eq!(
        serde_json::to_value(RootCatalogCapability::new(false))?,
        json!(false)
    );
    assert_eq!(first.allowed_kinds()[0].kind, RootKind::Source);
    assert!(first.allowed_kinds()[0].binding_ready);
    assert!(first.allowed_kinds()[0].destructive_reason.is_none());
    let rebuilt = RootCatalogPageResponse::new(
        response.state(),
        Some(RootCatalogGeneration::new(metadata.fields().clone())?),
        vec![RootCatalogSlot::new(first.fields().clone())?],
        Some(cursor),
    )?;
    assert_eq!(rebuilt, response);
    for debug in [
        format!("{response:?}"),
        format!("{:?}", first.fields()),
        format!("{:?}", metadata.fields()),
    ] {
        assert!(!debug.contains("/private"));
    }
    assert!(wire["generation"].get("attestation_sha256").is_none());
    Ok(())
}

#[test]
fn unavailable_invalid_and_ready_empty_remain_distinct() -> TestResult {
    for state in [
        RootCatalogReadinessState::SourceUnavailable(RootSourceFailure::Missing),
        RootCatalogReadinessState::SourceUnavailable(RootSourceFailure::Untrusted),
        RootCatalogReadinessState::SourceUnavailable(RootSourceFailure::Invalid),
        RootCatalogReadinessState::SourceUnavailable(RootSourceFailure::BoundExceeded),
        RootCatalogReadinessState::SourceUnavailable(RootSourceFailure::Unsupported),
        RootCatalogReadinessState::AttestationInvalid(RootAttestationFailure::Invalid),
        RootCatalogReadinessState::AttestationInvalid(RootAttestationFailure::Overlap),
        RootCatalogReadinessState::AttestationInvalid(RootAttestationFailure::UnsafeAncestry),
        RootCatalogReadinessState::AttestationInvalid(RootAttestationFailure::DurabilityUnproven),
        RootCatalogReadinessState::AttestationInvalid(
            RootAttestationFailure::WriterControlUnproven,
        ),
        RootCatalogReadinessState::AttestationInvalid(RootAttestationFailure::IdentityMismatch),
    ] {
        let response = RootCatalogPageResponse::new(state, None, Vec::new(), None)?;
        let wire = serde_json::to_value(&response)?;
        assert!(wire.get("generation").is_none());
        assert_eq!(
            serde_json::from_value::<RootCatalogPageResponse>(wire)?,
            response
        );
    }
    let ready: RootCatalogPageResponse = serde_json::from_value(page(0, 0))?;
    assert!(ready.generation().is_some());
    assert!(ready.slots().is_empty());
    for (field, value) in [
        ("source_state", json!("missing")),
        ("source_reason", json!("media_root_catalog_source_missing")),
        ("attestation_state", json!("not_evaluated")),
        ("attestation_reason", json!("media_root_overlap")),
        ("generation", Value::Null),
        ("format_version", json!(2)),
    ] {
        let mut wire = page(0, 0);
        wire[field] = value;
        assert!(serde_json::from_value::<RootCatalogPageResponse>(wire).is_err());
    }
    assert!(
        RootCatalogPageResponse::new(
            RootCatalogReadinessState::Ready {
                generation: NonZeroU64::MIN
            },
            Some(serde_json::from_value(generation(0))?),
            Vec::new(),
            None,
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn exact_scalar_bounds_and_spellings_are_required() {
    for (field, value) in [
        ("generation", json!("01")),
        ("generation", json!("0")),
        ("generation", json!("+1")),
        ("generation", json!(1)),
        ("generation", json!("9223372036854775808")),
        ("slot_count", json!(257)),
        ("source_sha256", json!("A".repeat(64))),
        ("generation_sha256", json!("a".repeat(63))),
        ("activated_at", json!("2026-09-14T01:02:03-07:00")),
        ("reconciled_at", json!("2026-02-30T00:00:00Z")),
        ("attestation_sha256", json!("a".repeat(64))),
    ] {
        let mut wire = generation(1);
        wire[field] = value;
        assert!(
            serde_json::from_value::<RootCatalogGeneration>(wire).is_err(),
            "{field}"
        );
    }
    for (field, value) in [
        ("logical_key", json!("Bad-Key")),
        ("requested_path", json!("relative")),
        ("requested_path", json!("/")),
        ("canonical_path", json!("/a\0b")),
        ("requested_path", json!(format!("/{}", "a".repeat(4096)))),
        ("filesystem_device", json!("F".repeat(16))),
        ("filesystem_inode", json!("a".repeat(15))),
        ("mount_id", json!("00")),
        ("mount_id", json!("-1")),
        ("mount_id", json!("9223372036854775808")),
        ("mount_id", json!(0)),
        ("filesystem_type", json!("")),
        ("filesystem_type", json!("a".repeat(65))),
        ("owner_uid", json!(4_294_967_296_u64)),
        ("owner_gid", json!(-1)),
        ("owner_gid", json!(1.5)),
        ("mode_bits", json!("8888")),
        ("mode_bits", json!("755")),
        ("mode_bits", json!(755)),
        ("root_identity_sha256", json!("g".repeat(64))),
        ("validated_at", json!("2026-09-14T01:02:03-00:00")),
    ] {
        let mut wire = slot(1);
        wire[field] = value;
        assert!(
            serde_json::from_value::<RootCatalogSlot>(wire).is_err(),
            "{field}"
        );
    }
}

#[test]
fn evidence_pairs_and_reported_kind_coherence() -> TestResult {
    let mut kubernetes = slot(1);
    kubernetes["durability_evidence"] = json!("kubernetes_persistent_volume_claim");
    kubernetes["sole_writer_evidence"] = json!("kubernetes_read_write_once_pod");
    assert!(serde_json::from_value::<RootCatalogSlot>(kubernetes).is_ok());
    let mut source = slot(1);
    source["durability_class"] = json!("disposable");
    source["durability_evidence"] = json!("none");
    source["sole_writer_class"] = json!("uncontrolled");
    source["sole_writer_evidence"] = json!("none");
    source["allowed_kinds"] = json!([{
        "kind": "source", "binding_ready": true, "destructive_ready": false,
        "destructive_reason": "media_root_durability_unproven"
    }]);
    let decoded: RootCatalogSlot = serde_json::from_value(source.clone())?;
    assert_eq!(serde_json::to_value(decoded)?, source);
    for (field, value) in [
        ("durability_evidence", json!("none")),
        ("sole_writer_evidence", json!("none")),
        ("durability_class", json!("unknown")),
        ("sole_writer_class", json!("unknown")),
        ("read_capable", json!(false)),
        ("fsync_capable", json!(false)),
        ("allowed_kinds", json!([])),
    ] {
        let mut wire = slot(1);
        wire[field] = value;
        assert!(
            serde_json::from_value::<RootCatalogSlot>(wire).is_err(),
            "{field}"
        );
    }
    for kind in [
        json!({"kind":"source", "binding_ready":false, "destructive_ready":true}),
        json!({"kind":"source", "binding_ready":true, "binding_reason":"media_root_overlap", "destructive_ready":false}),
        json!({"kind":"source", "binding_ready":false, "binding_reason":"/secret", "destructive_ready":false}),
        json!({"kind":"unknown", "binding_ready":true, "destructive_ready":true}),
    ] {
        assert!(serde_json::from_value::<RootCatalogAllowedKind>(kind).is_err());
    }
    let mut reversed = slot(1);
    reversed["allowed_kinds"]
        .as_array_mut()
        .ok_or("missing kinds")?
        .reverse();
    assert!(serde_json::from_value::<RootCatalogSlot>(reversed).is_err());
    let mut duplicate = slot(1);
    duplicate["allowed_kinds"][1] = duplicate["allowed_kinds"][0].clone();
    assert!(serde_json::from_value::<RootCatalogSlot>(duplicate).is_err());
    Ok(())
}

#[test]
fn page_bounds_order_uniqueness_and_cursor_are_enforced() -> TestResult {
    assert_eq!(
        serde_json::from_value::<RootCatalogPageResponse>(page(200, 256))?
            .slots()
            .len(),
        200
    );
    assert!(serde_json::from_value::<RootCatalogPageResponse>(page(201, 256)).is_err());
    assert!(serde_json::from_value::<RootCatalogPageResponse>(page(2, 1)).is_err());
    for field in [
        "logical_key",
        "media_root_catalog_slot_public_id",
        "canonical_path",
        "filesystem_inode",
        "root_identity_sha256",
    ] {
        let mut wire = page(2, 2);
        wire["slots"][1][field] = wire["slots"][0][field].clone();
        assert!(
            serde_json::from_value::<RootCatalogPageResponse>(wire).is_err(),
            "{field}"
        );
    }
    let mut reversed = page(2, 2);
    reversed["slots"]
        .as_array_mut()
        .ok_or("missing slots")?
        .reverse();
    assert!(serde_json::from_value::<RootCatalogPageResponse>(reversed).is_err());
    for (rows, total, cursor) in [
        (1, 2, "not-a-cursor".to_owned()),
        (
            1,
            2,
            RootCatalogCursor::new("root-002", Uuid::from_u128(2))?.encode()?,
        ),
        (
            1,
            2,
            RootCatalogCursor::new("root-001", Uuid::from_u128(2))?.encode()?,
        ),
        (
            1,
            1,
            RootCatalogCursor::new("root-001", Uuid::from_u128(1))?.encode()?,
        ),
        (
            0,
            0,
            RootCatalogCursor::new("root-001", Uuid::from_u128(1))?.encode()?,
        ),
    ] {
        let mut wire = page(rows, total);
        wire["next_cursor"] = json!(cursor);
        assert!(serde_json::from_value::<RootCatalogPageResponse>(wire).is_err());
    }
    Ok(())
}

#[test]
fn object_only_strict_decoding_and_path_free_errors() -> TestResult {
    let original = page(1, 1);
    for pointer in ["", "/generation", "/slots/0", "/slots/0/allowed_kinds/0"] {
        for replacement in [json!([]), Value::Null, json!("/secret/rejected")] {
            let mut wire = original.clone();
            *wire.pointer_mut(pointer).ok_or("missing pointer")? = replacement;
            let error = serde_json::from_value::<RootCatalogPageResponse>(wire)
                .err()
                .ok_or("accepted invalid object")?;
            assert!(!format!("{error:?} {error}").contains("/secret"));
        }
        let mut wire = original.clone();
        wire.pointer_mut(pointer).ok_or("missing object")?["/secret/unknown"] = json!(true);
        let error = serde_json::from_value::<RootCatalogPageResponse>(wire)
            .err()
            .ok_or("accepted unknown field")?;
        assert!(!format!("{error:?} {error}").contains("/secret"));
    }
    let encoded = serde_json::to_string(&original)?;
    for field in ["format_version", "slot_count", "read_capable", "kind"] {
        let repeated = encoded.replace(
            &format!("\"{field}\":"),
            &format!("\"{field}\":null,\"{field}\":"),
        );
        assert!(serde_json::from_str::<RootCatalogPageResponse>(&repeated).is_err());
    }
    let mut missing = original;
    missing["slots"][0]
        .as_object_mut()
        .ok_or("missing slot")?
        .remove("read_capable");
    assert!(serde_json::from_value::<RootCatalogPageResponse>(missing).is_err());
    Ok(())
}
