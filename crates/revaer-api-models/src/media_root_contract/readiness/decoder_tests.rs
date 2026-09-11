use std::error::Error;

use super::{RootCatalogReadinessResponse, RootKindReadiness};

const ROW: &str = r#"{"kind":"source","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0}"#;

fn document(source_row: &str, extra: &str) -> String {
    let other_rows = ["output", "workspace", "backup", "quarantine"]
        .map(|kind| ROW.replace("source", kind))
        .join(",");
    format!(
        r#"{{"format_version":1,"source_state":"ready","attestation_state":"ready","generation":"1","kinds":[{source_row},{other_rows}]{extra}}}"#
    )
}

#[test]
fn json_decoder_rejects_duplicate_fields_without_value_map_normalization()
-> Result<(), Box<dyn Error>> {
    for duplicate in [
        r#", "format_version":1"#,
        r#", "source_state":"ready"#,
        r#", "attestation_state":"ready"#,
        r#", "generation":"1"#,
        r#", "kinds":[]"#,
        r#", "source_reason":null, "source_reason":null"#,
        r#", "attestation_reason":null, "attestation_reason":null"#,
    ] {
        let error = serde_json::from_str::<RootCatalogReadinessResponse>(&document(ROW, duplicate))
            .err()
            .ok_or("duplicate envelope field accepted")?;
        assert!(
            error
                .to_string()
                .starts_with("media root readiness is invalid")
        );
    }
    for duplicate in [
        r#", "kind":"source"#,
        r#", "attested_slot_count":0"#,
        r#", "binding_ready_slot_count":0"#,
        r#", "destructive_ready_slot_count":0"#,
    ] {
        let row = format!(
            "{}{duplicate}}}",
            ROW.strip_suffix('}').ok_or("row fixture suffix")?
        );
        assert!(serde_json::from_str::<RootCatalogReadinessResponse>(&document(&row, "")).is_err());
    }
    Ok(())
}

#[test]
fn standalone_kind_rows_require_an_object_and_coherent_counts() -> Result<(), Box<dyn Error>> {
    let row = serde_json::from_str::<RootKindReadiness>(ROW)?;
    assert_eq!(row.attested_slot_count, 0);
    for invalid in [
        r#"["source",0,0,0]"#.to_owned(),
        ROW.replace("\"attested_slot_count\":0", "\"attested_slot_count\":257"),
        ROW.replace(
            "\"binding_ready_slot_count\":0",
            "\"binding_ready_slot_count\":1",
        ),
        ROW.replace(
            "\"destructive_ready_slot_count\":0",
            "\"destructive_ready_slot_count\":1",
        ),
    ] {
        assert!(serde_json::from_str::<RootKindReadiness>(&invalid).is_err());
    }
    Ok(())
}
