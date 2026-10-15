use super::*;

pub(in crate::media) fn rows() -> Vec<RootCatalogReadinessRow> {
    ["source", "output", "workspace", "backup", "quarantine"]
        .into_iter()
        .map(|kind| RootCatalogReadinessRow {
            source_state: "ready".to_owned(),
            source_reason_code: None,
            attestation_state: "ready".to_owned(),
            attestation_reason_code: None,
            attestation_generation: Some(7),
            root_kind: kind.to_owned(),
            attested_slot_count: 2,
            binding_ready_slot_count: 1,
            destructive_ready_slot_count: 0,
        })
        .collect()
}

#[test]
fn preserves_complete_ready_snapshot() -> anyhow::Result<()> {
    let result = response(rows())?;
    assert_eq!(result.kinds()[0].attested_slot_count, 2);
    assert_eq!(result.kinds()[0].binding_ready_slot_count, 1);
    assert_eq!(result.kinds()[4].destructive_ready_slot_count, 0);
    let value = serde_json::to_value(result)?;
    assert_eq!(value["generation"], "7");
    assert_eq!(value.as_object().map(serde_json::Map::len), Some(5));
    Ok(())
}

#[test]
fn preserves_missing_source_without_fabricating_generation() -> anyhow::Result<()> {
    let mut snapshot = rows();
    for row in &mut snapshot {
        row.source_state = "missing".to_owned();
        row.source_reason_code = Some("media_root_catalog_source_missing".to_owned());
        row.attestation_state = "not_evaluated".to_owned();
        row.attestation_generation = None;
        row.attested_slot_count = 0;
        row.binding_ready_slot_count = 0;
    }
    let result = serde_json::to_value(response(snapshot)?)?;
    assert_eq!(result["source_state"], "missing");
    assert!(result.get("generation").is_none());
    Ok(())
}

#[test]
fn rejects_missing_extra_reordered_or_unknown_kind_rows() {
    let mut snapshot = rows();
    snapshot.truncate(4);
    assert!(response(snapshot).is_err());
    let mut snapshot = rows();
    snapshot.extend(rows());
    assert!(response(snapshot).is_err());
    let mut snapshot = rows();
    snapshot.swap(0, 1);
    assert!(response(snapshot).is_err());
    let mut snapshot = rows();
    snapshot[0].root_kind = "private-path".to_owned();
    assert!(response(snapshot).is_err());
}

#[test]
fn rejects_mixed_generations_and_incoherent_state() {
    let mut snapshot = rows();
    snapshot[4].attestation_generation = Some(8);
    assert!(response(snapshot).is_err());
    let mut snapshot = rows();
    snapshot[1].source_reason_code = Some("private-sql-detail".to_owned());
    assert!(response(snapshot).is_err());
    for generation in [None, Some(0), Some(-1)] {
        let mut snapshot = rows();
        for row in &mut snapshot {
            row.attestation_generation = generation;
        }
        assert!(response(snapshot).is_err());
    }
}

#[test]
fn rejects_negative_out_of_bounds_and_inverted_counts() {
    for count in [-1, 257, i32::MAX] {
        let mut snapshot = rows();
        snapshot[0].attested_slot_count = count;
        assert!(response(snapshot).is_err());
    }
    let mut snapshot = rows();
    snapshot[0].binding_ready_slot_count = 3;
    assert!(response(snapshot).is_err());
    let mut snapshot = rows();
    snapshot[0].destructive_ready_slot_count = 2;
    assert!(response(snapshot).is_err());
}
