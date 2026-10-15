use serde_json::{Value, json};

use super::{VALID_SLOT_JSON, digest_hex, valid_document};
use crate::root_catalog::{
    DurabilityClass, DurabilityEvidence, MAX_ROOT_CATALOG_DOCUMENT_BYTES,
    MAX_ROOT_CATALOG_KEY_BYTES, MAX_ROOT_CATALOG_PATH_BYTES, MAX_ROOT_CATALOG_SLOTS,
    RootCatalogParseError, RootKind, SoleWriterClass, SoleWriterEvidence, parse_root_catalog_v1,
};

fn default_slot(key: &str, path: &str) -> Value {
    json!({
        "key": key,
        "allowed_kinds": ["source"],
        "path": path,
        "durability_class": "disposable",
        "durability_evidence": "none",
        "sole_writer_class": "uncontrolled",
        "sole_writer_evidence": "none"
    })
}

fn document(slots: &[Value]) -> Vec<u8> {
    serde_json::to_vec(&json!({"format_version": 1, "slots": slots}))
        .expect("test catalog serializes")
}

fn parse_slot(slot: Value) -> Result<crate::root_catalog::RootCatalog, RootCatalogParseError> {
    parse_root_catalog_v1(&document(&[slot]))
}

#[test]
fn valid_catalog_exposes_exact_typed_values_and_fixed_digest() {
    let catalog = parse_root_catalog_v1(valid_document().as_bytes()).expect("valid catalog");
    assert_eq!(catalog.format_version(), 1);
    assert_eq!(catalog.slots().len(), 1);
    let slot = &catalog.slots()[0];
    assert_eq!(slot.key(), "media-library");
    assert_eq!(slot.path(), "/data/media-library");
    assert_eq!(slot.allowed_kinds(), &[RootKind::Source, RootKind::Output]);
    assert_eq!(slot.durability_class(), DurabilityClass::RestartPersistent);
    assert_eq!(
        slot.durability_evidence(),
        DurabilityEvidence::LinuxDedicatedMount
    );
    assert_eq!(slot.sole_writer_class(), SoleWriterClass::RevaerExclusive);
    assert_eq!(
        slot.sole_writer_evidence(),
        SoleWriterEvidence::LinuxDedicatedService
    );
    assert_eq!(
        digest_hex(catalog.semantic_sha256()),
        "ae95414063ad0e6b65bdfdbef7acbed01ab7b6466a8782d759c0483c7ee4c3c8"
    );
}

#[test]
fn empty_catalog_has_independent_fixed_digest() {
    let catalog =
        parse_root_catalog_v1(br#"{"format_version":1,"slots":[]}"#).expect("empty catalog");
    assert!(catalog.is_empty());
    assert_eq!(
        digest_hex(catalog.semantic_sha256()),
        "8f0b256ad26c139e8c51bd426ba510eb43aefca9cb1d58fb0737b6518e6bc867"
    );
}

#[test]
fn object_slot_kind_and_whitespace_order_do_not_change_identity() {
    let first = format!(
        r#"{{"format_version":1,"slots":[{VALID_SLOT_JSON},{}]}}"#,
        default_slot("archive", "/archive")
    );
    let second = r#"
      {
        "slots": [
          {
            "sole_writer_evidence": "none",
            "durability_evidence": "none",
            "path": "/archive",
            "allowed_kinds": ["source"],
            "sole_writer_class": "uncontrolled",
            "key": "archive",
            "durability_class": "disposable"
          },
          {
            "sole_writer_class": "revaer_exclusive",
            "path": "/data/media-library",
            "allowed_kinds": ["output", "source"],
            "durability_evidence": "linux_dedicated_mount",
            "key": "media-library",
            "sole_writer_evidence": "linux_dedicated_service",
            "durability_class": "restart_persistent"
          }
        ],
        "format_version": 1
      }
    "#;
    let first = parse_root_catalog_v1(first.as_bytes()).expect("first ordering");
    let second = parse_root_catalog_v1(second.as_bytes()).expect("second ordering");
    assert_eq!(first, second);
    assert_eq!(first.slots()[0].key(), "archive");
    assert_eq!(first.slots()[1].key(), "media-library");
}

#[test]
fn raw_document_bound_accepts_one_below_and_at_then_rejects_one_above() {
    let base = br#"{"format_version":1,"slots":[]}"#;
    for byte_len in [
        MAX_ROOT_CATALOG_DOCUMENT_BYTES - 1,
        MAX_ROOT_CATALOG_DOCUMENT_BYTES,
    ] {
        let mut raw = base.to_vec();
        raw.resize(byte_len, b' ');
        assert!(parse_root_catalog_v1(&raw).is_ok(), "length {byte_len}");
    }
    let mut above = base.to_vec();
    above.resize(MAX_ROOT_CATALOG_DOCUMENT_BYTES + 1, b' ');
    assert_eq!(
        parse_root_catalog_v1(&above),
        Err(RootCatalogParseError::DocumentBoundExceeded {
            maximum_bytes: MAX_ROOT_CATALOG_DOCUMENT_BYTES
        })
    );
}

#[test]
fn slot_bound_accepts_empty_one_below_and_at_then_rejects_one_above() {
    assert!(parse_root_catalog_v1(&document(&[])).is_ok());
    for count in [MAX_ROOT_CATALOG_SLOTS - 1, MAX_ROOT_CATALOG_SLOTS] {
        let slots: Vec<Value> = (0..count)
            .map(|index| default_slot(&format!("slot-{index}"), &format!("/root/{index}")))
            .collect();
        assert!(
            parse_root_catalog_v1(&document(&slots)).is_ok(),
            "count {count}"
        );
    }
    let slots: Vec<Value> = (0..=MAX_ROOT_CATALOG_SLOTS)
        .map(|index| default_slot(&format!("slot-{index}"), &format!("/root/{index}")))
        .collect();
    assert_eq!(
        parse_root_catalog_v1(&document(&slots)),
        Err(RootCatalogParseError::SlotBoundExceeded {
            maximum_slots: MAX_ROOT_CATALOG_SLOTS
        })
    );
}

#[test]
fn key_bounds_and_closed_ascii_grammar_are_exact() {
    for key_len in [
        1,
        MAX_ROOT_CATALOG_KEY_BYTES - 1,
        MAX_ROOT_CATALOG_KEY_BYTES,
    ] {
        let key = "a".repeat(key_len);
        assert!(
            parse_slot(default_slot(&key, "/root")).is_ok(),
            "key length {key_len}"
        );
    }
    for invalid in [
        String::new(),
        "a".repeat(MAX_ROOT_CATALOG_KEY_BYTES + 1),
        "Upper".to_owned(),
        "under_score".to_owned(),
        "with/slash".to_owned(),
        "white space".to_owned(),
        "unicodé".to_owned(),
        "-".to_owned(),
        "-slot".to_owned(),
        "slot-".to_owned(),
        "0-a--9-".to_owned(),
    ] {
        assert_eq!(
            parse_slot(default_slot(&invalid, "/root")),
            Err(RootCatalogParseError::InvalidKey { slot_index: 0 }),
            "key {invalid:?}"
        );
    }
    for valid in ["0", "a0", "0-a--9"] {
        assert!(parse_slot(default_slot(valid, "/root")).is_ok());
    }
}

#[test]
fn decoded_path_bounds_absolute_rule_and_nul_rejection_are_exact() {
    for path_len in [
        2,
        MAX_ROOT_CATALOG_PATH_BYTES - 1,
        MAX_ROOT_CATALOG_PATH_BYTES,
    ] {
        let path = format!("/{}", "a".repeat(path_len - 1));
        assert!(
            parse_slot(default_slot("slot", &path)).is_ok(),
            "path length {path_len}"
        );
    }
    for invalid in [
        String::new(),
        "/".to_owned(),
        "relative/path".to_owned(),
        format!("/{}", "a".repeat(MAX_ROOT_CATALOG_PATH_BYTES)),
        "/root\0child".to_owned(),
    ] {
        assert_eq!(
            parse_slot(default_slot("slot", &invalid)),
            Err(RootCatalogParseError::InvalidPath { slot_index: 0 })
        );
    }
    assert!(parse_slot(default_slot("slot", "/média/世界")).is_ok());
}

#[test]
fn escape_amplification_cannot_bypass_decoded_path_bound() {
    let escaped = "\\u0061".repeat(MAX_ROOT_CATALOG_PATH_BYTES);
    let raw = format!(
        r#"{{"format_version":1,"slots":[{{"key":"slot","allowed_kinds":["source"],"path":"/{escaped}","durability_class":"disposable","durability_evidence":"none","sole_writer_class":"uncontrolled","sole_writer_evidence":"none"}}]}}"#
    );
    assert_eq!(
        parse_root_catalog_v1(raw.as_bytes()),
        Err(RootCatalogParseError::InvalidPath { slot_index: 0 })
    );
}

#[test]
fn duplicate_slot_keys_and_kind_values_are_rejected() {
    assert_eq!(
        parse_root_catalog_v1(&document(&[
            default_slot("same", "/one"),
            default_slot("same", "/two")
        ])),
        Err(RootCatalogParseError::DuplicateKey { slot_index: 1 })
    );
    let mut duplicate_kind = default_slot("slot", "/root");
    duplicate_kind["allowed_kinds"] = json!(["source", "source"]);
    assert_eq!(
        parse_slot(duplicate_kind),
        Err(RootCatalogParseError::DuplicateKind { slot_index: 0 })
    );
    let mut empty = default_slot("slot", "/root");
    empty["allowed_kinds"] = json!([]);
    assert_eq!(
        parse_slot(empty),
        Err(RootCatalogParseError::EmptyKindSet { slot_index: 0 })
    );
}

#[test]
fn version_one_containers_require_objects_and_enums_require_strings() -> anyhow::Result<()> {
    assert_eq!(
        parse_root_catalog_v1(b"[1,[]]"),
        Err(RootCatalogParseError::MalformedDocument)
    );
    assert_eq!(
        parse_slot(json!([
            "slot",
            ["source"],
            "/root",
            "disposable",
            "none",
            "uncontrolled",
            "none"
        ])),
        Err(RootCatalogParseError::MalformedDocument)
    );
    for field in [
        "durability_class",
        "durability_evidence",
        "sole_writer_class",
        "sole_writer_evidence",
    ] {
        let mut slot = default_slot("slot", "/root");
        let variant = slot[field]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("string enum"))?;
        slot[field] = json!({variant: null});
        assert_eq!(
            parse_slot(slot),
            Err(RootCatalogParseError::MalformedDocument),
            "field {field}"
        );
    }
    let mut slot = default_slot("slot", "/root");
    slot["allowed_kinds"] = json!([{"source": null}]);
    assert_eq!(
        parse_slot(slot),
        Err(RootCatalogParseError::MalformedDocument)
    );
    Ok(())
}

#[test]
fn every_unknown_enum_value_is_rejected() {
    for field in [
        "durability_class",
        "durability_evidence",
        "sole_writer_class",
        "sole_writer_evidence",
    ] {
        let mut slot = default_slot("slot", "/root");
        slot[field] = json!("future_value");
        assert_eq!(
            parse_slot(slot),
            Err(RootCatalogParseError::MalformedDocument),
            "field {field}"
        );
    }
    let mut slot = default_slot("slot", "/root");
    slot["allowed_kinds"] = json!(["future_kind"]);
    assert_eq!(
        parse_slot(slot),
        Err(RootCatalogParseError::MalformedDocument)
    );
}

#[test]
fn only_accepted_durability_pairs_validate() {
    let accepted = [
        ("disposable", "none"),
        ("restart_persistent", "linux_dedicated_mount"),
        ("restart_persistent", "kubernetes_persistent_volume_claim"),
    ];
    for (class, evidence) in accepted {
        let mut slot = default_slot("slot", "/root");
        slot["durability_class"] = json!(class);
        slot["durability_evidence"] = json!(evidence);
        assert!(parse_slot(slot).is_ok(), "{class}/{evidence}");
    }
    for (class, evidence) in [
        ("disposable", "linux_dedicated_mount"),
        ("disposable", "kubernetes_persistent_volume_claim"),
        ("restart_persistent", "none"),
    ] {
        let mut slot = default_slot("slot", "/root");
        slot["durability_class"] = json!(class);
        slot["durability_evidence"] = json!(evidence);
        assert_eq!(
            parse_slot(slot),
            Err(RootCatalogParseError::InvalidDurabilityMapping { slot_index: 0 })
        );
    }
}

#[test]
fn only_accepted_sole_writer_pairs_validate() {
    let accepted = [
        ("uncontrolled", "none"),
        ("revaer_exclusive", "linux_dedicated_service"),
        ("revaer_exclusive", "kubernetes_read_write_once_pod"),
    ];
    for (class, evidence) in accepted {
        let mut slot = default_slot("slot", "/root");
        slot["sole_writer_class"] = json!(class);
        slot["sole_writer_evidence"] = json!(evidence);
        assert!(parse_slot(slot).is_ok(), "{class}/{evidence}");
    }
    for (class, evidence) in [
        ("uncontrolled", "linux_dedicated_service"),
        ("uncontrolled", "kubernetes_read_write_once_pod"),
        ("revaer_exclusive", "none"),
    ] {
        let mut slot = default_slot("slot", "/root");
        slot["sole_writer_class"] = json!(class);
        slot["sole_writer_evidence"] = json!(evidence);
        assert_eq!(
            parse_slot(slot),
            Err(RootCatalogParseError::InvalidSoleWriterMapping { slot_index: 0 })
        );
    }
}

#[test]
fn combined_source_output_requires_persistent_exclusive_declarations() {
    let mut uncontrolled = default_slot("slot", "/root");
    uncontrolled["allowed_kinds"] = json!(["source", "output"]);
    assert_eq!(
        parse_slot(uncontrolled),
        Err(RootCatalogParseError::InvalidSourceOutputDeclaration { slot_index: 0 })
    );

    let mut persistent = default_slot("slot", "/root");
    persistent["allowed_kinds"] = json!(["source", "output"]);
    persistent["durability_class"] = json!("restart_persistent");
    persistent["durability_evidence"] = json!("linux_dedicated_mount");
    assert_eq!(
        parse_slot(persistent),
        Err(RootCatalogParseError::InvalidSourceOutputDeclaration { slot_index: 0 })
    );
    assert!(parse_root_catalog_v1(valid_document().as_bytes()).is_ok());
}

#[test]
fn exact_top_level_and_slot_field_sets_are_required() {
    for missing in ["format_version", "slots"] {
        let mut value = json!({"format_version": 1, "slots": []});
        value.as_object_mut().expect("object").remove(missing);
        assert_eq!(
            parse_root_catalog_v1(&serde_json::to_vec(&value).expect("serialize")),
            Err(RootCatalogParseError::MalformedDocument)
        );
    }
    let mut top_extra = json!({"format_version": 1, "slots": []});
    top_extra["future"] = json!(true);
    assert_eq!(
        parse_root_catalog_v1(&serde_json::to_vec(&top_extra).expect("serialize")),
        Err(RootCatalogParseError::MalformedDocument)
    );

    for missing in [
        "key",
        "allowed_kinds",
        "path",
        "durability_class",
        "durability_evidence",
        "sole_writer_class",
        "sole_writer_evidence",
    ] {
        let mut slot = default_slot("slot", "/root");
        slot.as_object_mut().expect("object").remove(missing);
        assert_eq!(
            parse_slot(slot),
            Err(RootCatalogParseError::MalformedDocument),
            "missing {missing}"
        );
    }
    let mut slot_extra = default_slot("slot", "/root");
    slot_extra["future"] = json!(true);
    assert_eq!(
        parse_slot(slot_extra),
        Err(RootCatalogParseError::MalformedDocument)
    );
}

#[test]
fn every_duplicate_object_field_is_rejected() {
    for duplicate in [r#","format_version":1"#, r#","slots":[]"#] {
        let raw = format!(r#"{{"format_version":1,"slots":[]{duplicate}}}"#);
        assert_eq!(
            parse_root_catalog_v1(raw.as_bytes()),
            Err(RootCatalogParseError::MalformedDocument)
        );
    }

    for duplicate in [
        r#","key":"other""#,
        r#","allowed_kinds":["source"]"#,
        r#","path":"/other""#,
        r#","durability_class":"disposable""#,
        r#","durability_evidence":"none""#,
        r#","sole_writer_class":"uncontrolled""#,
        r#","sole_writer_evidence":"none""#,
    ] {
        let raw = format!(
            r#"{{"format_version":1,"slots":[{{"key":"slot","allowed_kinds":["source"],"path":"/root","durability_class":"disposable","durability_evidence":"none","sole_writer_class":"uncontrolled","sole_writer_evidence":"none"{duplicate}}}]}}"#
        );
        assert_eq!(
            parse_root_catalog_v1(raw.as_bytes()),
            Err(RootCatalogParseError::MalformedDocument)
        );
    }
}

#[test]
fn malformed_versions_values_and_trailing_documents_fail_completely() {
    for version in ["1.0", "1e0", "-1", "\"1\"", "null", "4294967296"] {
        let raw = format!(r#"{{"format_version":{version},"slots":[]}}"#);
        assert_eq!(
            parse_root_catalog_v1(raw.as_bytes()),
            Err(RootCatalogParseError::MalformedDocument),
            "version {version}"
        );
    }
    for version in [0, 2, u32::MAX] {
        let raw = format!(r#"{{"format_version":{version},"slots":[]}}"#);
        assert_eq!(
            parse_root_catalog_v1(raw.as_bytes()),
            Err(RootCatalogParseError::UnsupportedVersion { actual: version })
        );
    }
    for invalid in [
        Vec::new(),
        b"   \n\t".to_vec(),
        b"{not-json}".to_vec(),
        b"{\"format_version\":1,\"slots\":[]} true".to_vec(),
        b"{\"format_version\":1,\"slots\":[]} x".to_vec(),
        vec![b'{', 0xff, b'}'],
    ] {
        assert_eq!(
            parse_root_catalog_v1(&invalid),
            Err(RootCatalogParseError::MalformedDocument)
        );
    }
    assert!(parse_root_catalog_v1(b"{\"format_version\":1,\"slots\":[]} \n\t").is_ok());
}

#[test]
fn all_five_kinds_normalize_to_the_fixed_bit_order() {
    let mut slot = default_slot("slot", "/root");
    slot["allowed_kinds"] = json!(["quarantine", "backup", "workspace", "output"]);
    slot["sole_writer_class"] = json!("revaer_exclusive");
    slot["sole_writer_evidence"] = json!("linux_dedicated_service");
    let catalog = parse_slot(slot).expect("valid kind set");
    assert_eq!(
        catalog.slots()[0].allowed_kinds(),
        &[
            RootKind::Output,
            RootKind::Workspace,
            RootKind::Backup,
            RootKind::Quarantine
        ]
    );
}

#[test]
fn parse_reason_codes_are_stable() {
    let bound = RootCatalogParseError::DocumentBoundExceeded {
        maximum_bytes: MAX_ROOT_CATALOG_DOCUMENT_BYTES,
    };
    assert_eq!(bound.reason_code(), "media_root_catalog_bound_exceeded");
    assert_eq!(
        RootCatalogParseError::MalformedDocument.reason_code(),
        "media_root_catalog_format_invalid"
    );
}
