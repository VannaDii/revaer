use crate::media_root_contract::RootCatalogReadinessResponse;
use serde_json::{Value, json};
use std::{error::Error, io};

type TestResult = Result<(), Box<dyn Error>>;

// Independent ADR 557 HTTP literals, not output from the implementation's codec.
const READY_JSON: &str = r#"{
    "format_version": 1,
    "source_state": "ready",
    "attestation_state": "ready",
    "generation": "9223372036854775807",
    "kinds": [
        {"kind":"source","attested_slot_count":256,"binding_ready_slot_count":128,"destructive_ready_slot_count":64},
        {"kind":"output","attested_slot_count":4,"binding_ready_slot_count":3,"destructive_ready_slot_count":1},
        {"kind":"workspace","attested_slot_count":1,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"backup","attested_slot_count":2,"binding_ready_slot_count":2,"destructive_ready_slot_count":2},
        {"kind":"quarantine","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0}
    ]
}"#;

const EMPTY_READY_JSON: &str = r#"{
    "format_version": 1,
    "source_state": "ready",
    "attestation_state": "ready",
    "generation": "1",
    "kinds": [
        {"kind":"source","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"output","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"workspace","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"backup","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"quarantine","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0}
    ]
}"#;

const MISSING_JSON: &str = r#"{
    "format_version": 1,
    "source_state": "missing",
    "source_reason": "media_root_catalog_source_missing",
    "attestation_state": "not_evaluated",
    "kinds": [
        {"kind":"source","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"output","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"workspace","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"backup","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"quarantine","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0}
    ]
}"#;

const INVALID_ATTESTATION_JSON: &str = r#"{
    "format_version": 1,
    "source_state": "ready",
    "attestation_state": "invalid",
    "attestation_reason": "media_root_attestation_invalid",
    "kinds": [
        {"kind":"source","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"output","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"workspace","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"backup","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0},
        {"kind":"quarantine","attested_slot_count":0,"binding_ready_slot_count":0,"destructive_ready_slot_count":0}
    ]
}"#;

const SOURCE_STATES: [(&str, Option<&str>); 6] = [
    ("ready", None),
    ("missing", Some("media_root_catalog_source_missing")),
    ("untrusted", Some("media_root_catalog_source_untrusted")),
    ("invalid", Some("media_root_catalog_format_invalid")),
    ("bound_exceeded", Some("media_root_catalog_bound_exceeded")),
    ("unsupported", Some("media_root_platform_unsupported")),
];

const ATTESTATION_REASONS: [&str; 6] = [
    "media_root_attestation_invalid",
    "media_root_overlap",
    "media_root_unsafe_ancestry",
    "media_root_durability_unproven",
    "media_root_writer_control_unproven",
    "media_root_identity_mismatch",
];

const KIND_NAMES: [&str; 5] = ["source", "output", "workspace", "backup", "quarantine"];
const COUNT_FIELDS: [&str; 3] = [
    "attested_slot_count",
    "binding_ready_slot_count",
    "destructive_ready_slot_count",
];
const KIND_POINTERS: [&str; 5] = ["/kinds/0", "/kinds/1", "/kinds/2", "/kinds/3", "/kinds/4"];

fn round_trip(expected: &Value) -> TestResult {
    let response = serde_json::from_value::<RootCatalogReadinessResponse>(expected.clone())?;
    assert_eq!(serde_json::to_value(response)?, *expected);
    Ok(())
}

fn rejected_error(input: Value) -> Result<serde_json::Error, Box<dyn Error>> {
    match serde_json::from_value::<RootCatalogReadinessResponse>(input) {
        Ok(_) => Err(io::Error::other("invalid readiness JSON was accepted").into()),
        Err(error) => {
            assert_eq!(error.to_string(), "media root readiness is invalid");
            Ok(error)
        }
    }
}

fn remove_field(value: &mut Value, field: &str) -> TestResult {
    let object = value
        .as_object_mut()
        .ok_or_else(|| io::Error::other("fixture must be an object"))?;
    assert!(object.remove(field).is_some(), "fixture lacks {field}");
    Ok(())
}

fn kind_rows(value: &mut Value) -> Result<&mut Vec<Value>, Box<dyn Error>> {
    value
        .get_mut("kinds")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| io::Error::other("fixture lacks kinds array").into())
}

fn set_counts(value: &mut Value, row: usize, counts: [u16; 3]) {
    for (field, count) in COUNT_FIELDS.into_iter().zip(counts) {
        value["kinds"][row][field] = json!(count);
    }
}

fn null_fields_serialize_as_omitted(expected: &Value, fields: [&str; 2]) -> TestResult {
    for mask in 0..4 {
        let mut input = expected.clone();
        for (index, field) in fields.into_iter().enumerate() {
            if mask & (1 << index) != 0 {
                input[field] = Value::Null;
            }
        }
        let response = serde_json::from_value::<RootCatalogReadinessResponse>(input)?;
        assert_eq!(serde_json::to_value(response)?, *expected);
    }
    Ok(())
}

#[test]
fn literal_responses_round_trip_with_exact_fields_and_types() -> TestResult {
    for literal in [
        READY_JSON,
        EMPTY_READY_JSON,
        MISSING_JSON,
        INVALID_ATTESTATION_JSON,
    ] {
        round_trip(&serde_json::from_str::<Value>(literal)?)?;
    }
    Ok(())
}

#[test]
fn a_ready_empty_catalog_is_not_a_missing_source() -> TestResult {
    let empty = serde_json::from_value::<RootCatalogReadinessResponse>(serde_json::from_str(
        EMPTY_READY_JSON,
    )?)?;
    let missing = serde_json::from_value::<RootCatalogReadinessResponse>(serde_json::from_str(
        MISSING_JSON,
    )?)?;
    let empty = serde_json::to_value(empty)?;
    let missing = serde_json::to_value(missing)?;
    assert_eq!(empty["kinds"], missing["kinds"]);
    assert_ne!(empty, missing);
    assert_eq!(empty["generation"], "1");
    assert!(missing.get("generation").is_none());
    assert!(empty.get("source_reason").is_none());
    assert_eq!(
        missing["source_reason"],
        "media_root_catalog_source_missing"
    );
    Ok(())
}

#[test]
fn every_unavailable_source_has_its_exact_reason() -> TestResult {
    for (state, reason) in SOURCE_STATES {
        let Some(reason) = reason else { continue };
        let mut input = serde_json::from_str::<Value>(MISSING_JSON)?;
        input["source_state"] = json!(state);
        input["source_reason"] = json!(reason);
        round_trip(&input)?;
        null_fields_serialize_as_omitted(&input, ["generation", "attestation_reason"])?;
    }
    Ok(())
}

#[test]
fn source_reasons_cannot_be_omitted_or_exchanged_between_states() -> TestResult {
    for (state, expected_reason) in SOURCE_STATES {
        let base = if state == "ready" {
            EMPTY_READY_JSON
        } else {
            MISSING_JSON
        };
        for (_, candidate_reason) in SOURCE_STATES {
            let mut input = serde_json::from_str::<Value>(base)?;
            input["source_state"] = json!(state);
            input["source_reason"] = json!(candidate_reason);
            let result = serde_json::from_value::<RootCatalogReadinessResponse>(input);
            assert_eq!(
                result.is_ok(),
                candidate_reason == expected_reason,
                "{state}: {candidate_reason:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn invalid_attestation_accepts_exactly_the_six_catalog_state_reasons() -> TestResult {
    for reason in ATTESTATION_REASONS {
        let mut input = serde_json::from_str::<Value>(INVALID_ATTESTATION_JSON)?;
        input["attestation_reason"] = json!(reason);
        round_trip(&input)?;
        null_fields_serialize_as_omitted(&input, ["source_reason", "generation"])?;
    }
    Ok(())
}

#[test]
fn source_attestation_and_generation_combinations_are_coherent() -> TestResult {
    let empty = serde_json::from_str::<Value>(EMPTY_READY_JSON)?;
    for (source, reason) in SOURCE_STATES {
        for attestation in ["ready", "not_evaluated", "invalid"] {
            for generation in [None, Some("1")] {
                let input = json!({
                    "format_version": 1,
                    "source_state": source,
                    "source_reason": reason,
                    "attestation_state": attestation,
                    "attestation_reason": (attestation == "invalid").then_some("media_root_attestation_invalid"),
                    "generation": generation,
                    "kinds": empty["kinds"]
                });
                let coherent = if source == "ready" {
                    matches!(
                        (attestation, generation.is_some()),
                        ("ready", true) | ("invalid", false)
                    )
                } else {
                    attestation == "not_evaluated" && generation.is_none()
                };
                let result = serde_json::from_value::<RootCatalogReadinessResponse>(input);
                assert_eq!(
                    result.is_ok(),
                    coherent,
                    "{source}/{attestation}/{generation:?}"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn reasons_are_forbidden_outside_their_corresponding_failure_state() -> TestResult {
    for reason in ATTESTATION_REASONS {
        for literal in [READY_JSON, MISSING_JSON] {
            let mut input = serde_json::from_str::<Value>(literal)?;
            input["attestation_reason"] = json!(reason);
            rejected_error(input)?;
        }
    }
    for (_, reason) in SOURCE_STATES {
        let Some(reason) = reason else { continue };
        let mut input = serde_json::from_str::<Value>(INVALID_ATTESTATION_JSON)?;
        input["source_reason"] = json!(reason);
        rejected_error(input)?;
    }
    Ok(())
}

#[test]
fn ready_null_reasons_are_omitted_and_state_required_values_stay_required() -> TestResult {
    null_fields_serialize_as_omitted(
        &serde_json::from_str::<Value>(READY_JSON)?,
        ["source_reason", "attestation_reason"],
    )?;
    for (literal, required) in [
        (READY_JSON, "generation"),
        (MISSING_JSON, "source_reason"),
        (INVALID_ATTESTATION_JSON, "attestation_reason"),
    ] {
        let mut input = serde_json::from_str::<Value>(literal)?;
        input[required] = Value::Null;
        rejected_error(input.clone())?;
        remove_field(&mut input, required)?;
        rejected_error(input)?;
    }
    Ok(())
}

#[test]
fn source_and_attestation_spellings_are_closed_and_untrimmed() -> TestResult {
    for (field, invalid) in [
        ("source_state", "Ready"),
        ("source_state", "READY"),
        ("source_state", "ready "),
        ("source_state", " ready"),
        ("source_state", "bound-exceeded"),
        ("source_state", "platform_unsupported"),
        ("source_state", "not_evaluated"),
        ("source_state", ""),
        ("attestation_state", "READY"),
        ("attestation_state", "Invalid"),
        ("attestation_state", "ready\n"),
        ("attestation_state", "not-evaluated"),
        ("attestation_state", "missing"),
        ("attestation_state", "untrusted"),
        ("attestation_state", ""),
    ] {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        input[field] = json!(invalid);
        rejected_error(input)?;
    }
    Ok(())
}

#[test]
fn reason_spellings_are_closed_and_not_interchangeable() -> TestResult {
    for (literal, field) in [
        (MISSING_JSON, "source_reason"),
        (INVALID_ATTESTATION_JSON, "attestation_reason"),
    ] {
        for reason in [
            "",
            "unknown",
            "media_root_attestation_stale",
            "media_root_slot_unknown",
        ] {
            let mut input = serde_json::from_str::<Value>(literal)?;
            input[field] = json!(reason);
            rejected_error(input)?;
        }
        let expected = serde_json::from_str::<Value>(literal)?;
        let reason = expected[field]
            .as_str()
            .ok_or_else(|| io::Error::other("fixture lacks reason"))?;
        for invalid in [
            reason.to_uppercase(),
            format!(" {reason}"),
            format!("{reason} "),
        ] {
            let mut input = expected.clone();
            input[field] = json!(invalid);
            rejected_error(input)?;
        }
    }
    for (_, reason) in SOURCE_STATES {
        let Some(reason) = reason else { continue };
        let mut input = serde_json::from_str::<Value>(INVALID_ATTESTATION_JSON)?;
        input["attestation_reason"] = json!(reason);
        rejected_error(input)?;
    }
    for reason in ATTESTATION_REASONS {
        let mut input = serde_json::from_str::<Value>(MISSING_JSON)?;
        input["source_reason"] = json!(reason);
        rejected_error(input)?;
    }
    Ok(())
}

#[test]
fn required_fields_are_not_defaulted_at_either_level() -> TestResult {
    for field in [
        "format_version",
        "source_state",
        "attestation_state",
        "kinds",
    ] {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        remove_field(&mut input, field)?;
        rejected_error(input)?;
    }
    for row in 0..5 {
        for field in std::iter::once("kind").chain(COUNT_FIELDS) {
            let mut input = serde_json::from_str::<Value>(READY_JSON)?;
            remove_field(&mut input["kinds"][row], field)?;
            rejected_error(input)?;
        }
    }
    Ok(())
}

#[test]
fn envelopes_kinds_and_rows_require_their_json_container_types() -> TestResult {
    for invalid in [
        Value::Null,
        json!(true),
        json!(1),
        json!("ready"),
        json!([]),
    ] {
        rejected_error(invalid)?;
    }
    rejected_error(json!({}))?;
    let empty = serde_json::from_str::<Value>(EMPTY_READY_JSON)?;
    rejected_error(json!([
        1,
        "ready",
        null,
        "ready",
        null,
        "1",
        empty["kinds"]
    ]))?;
    for invalid in [
        Value::Null,
        json!(false),
        json!(5),
        json!("source"),
        json!({}),
    ] {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        input["kinds"] = invalid;
        rejected_error(input)?;
    }
    for (row, kind) in KIND_NAMES.into_iter().enumerate() {
        for invalid in [
            Value::Null,
            json!(true),
            json!(1),
            json!("source"),
            json!([]),
            json!({}),
        ] {
            let mut input = serde_json::from_str::<Value>(READY_JSON)?;
            input["kinds"][row] = invalid;
            rejected_error(input)?;
        }
        let mut input = empty.clone();
        input["kinds"][row] = json!([kind, 0, 0, 0]);
        rejected_error(input)?;
    }
    Ok(())
}

#[test]
fn states_and_present_reasons_require_strings() -> TestResult {
    for (literal, field) in [
        (READY_JSON, "source_state"),
        (READY_JSON, "attestation_state"),
        (MISSING_JSON, "source_reason"),
        (INVALID_ATTESTATION_JSON, "attestation_reason"),
    ] {
        for invalid in [
            Value::Null,
            json!(true),
            json!(0),
            json!(1.0),
            json!([]),
            json!({}),
        ] {
            let mut input = serde_json::from_str::<Value>(literal)?;
            input[field] = invalid;
            rejected_error(input)?;
        }
    }
    Ok(())
}

#[test]
fn format_version_is_only_json_integer_one() -> TestResult {
    for invalid in [
        Value::Null,
        json!(-1),
        json!(0),
        json!(2),
        json!(256),
        json!(u64::MAX),
        json!(1.0),
        json!(1.5),
        json!("1"),
        json!(true),
        json!([]),
        json!({}),
    ] {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        input["format_version"] = invalid;
        rejected_error(input)?;
    }
    Ok(())
}

#[test]
fn generation_preserves_canonical_strings_above_javascript_integer_precision() -> TestResult {
    for generation in [
        "1",
        "2",
        "9007199254740991",
        "9007199254740992",
        "9007199254740993",
        "9223372036854775806",
        "9223372036854775807",
    ] {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        input["generation"] = json!(generation);
        round_trip(&input)?;
    }
    Ok(())
}

#[test]
fn generation_rejects_noncanonical_zero_negative_and_overflow_strings() -> TestResult {
    for generation in [
        "",
        "0",
        "00",
        "01",
        "+1",
        "-0",
        "-1",
        " 1",
        "1 ",
        "\t1",
        "1\n",
        "1\r\n",
        "1.0",
        "1e0",
        "1E3",
        "0x1",
        "1_000",
        "1\0",
        "\u{661}",
        "\u{ff11}",
        "9223372036854775808",
        "18446744073709551615",
        "999999999999999999999999999999",
    ] {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        input["generation"] = json!(generation);
        rejected_error(input)?;
    }
    Ok(())
}

#[test]
fn generation_does_not_accept_numbers_or_administration_objects() -> TestResult {
    for generation in [
        Value::Null,
        json!(1),
        json!(i64::MAX),
        json!(u64::MAX),
        json!(1.0),
        json!(true),
        json!([]),
        json!({}),
        json!({"attestation_generation": "1"}),
    ] {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        input["generation"] = generation;
        rejected_error(input)?;
    }
    Ok(())
}

#[test]
fn kinds_reject_unknown_spellings_and_non_string_values_at_every_position() -> TestResult {
    for (row, kind) in KIND_NAMES.into_iter().enumerate() {
        for invalid in [
            json!(kind.to_uppercase()),
            json!(format!(" {kind}")),
            json!(format!("{kind} ")),
            json!(format!("{kind}s")),
            json!("scratch"),
            json!(""),
            Value::Null,
            json!(row),
            json!(true),
            json!([]),
            json!({}),
        ] {
            let mut input = serde_json::from_str::<Value>(READY_JSON)?;
            input["kinds"][row]["kind"] = invalid;
            rejected_error(input)?;
        }
    }
    Ok(())
}

#[test]
fn kind_order_and_uniqueness_are_checked_without_sorting_or_deduplication() -> TestResult {
    for left in 0..5 {
        for right in 0..5 {
            if left == right {
                continue;
            }
            let mut swapped = serde_json::from_str::<Value>(READY_JSON)?;
            kind_rows(&mut swapped)?.swap(left, right);
            rejected_error(swapped)?;
            let mut duplicate = serde_json::from_str::<Value>(READY_JSON)?;
            duplicate["kinds"][left] = duplicate["kinds"][right].clone();
            rejected_error(duplicate)?;
        }
    }
    Ok(())
}

#[test]
fn kinds_require_exactly_five_rows() -> TestResult {
    for length in 0..5 {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        kind_rows(&mut input)?.truncate(length);
        rejected_error(input)?;
    }
    for row in 0..5 {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        kind_rows(&mut input)?.remove(row);
        rejected_error(input)?;
    }
    for length in [6, 10, 256] {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        let extra = input["kinds"][0].clone();
        kind_rows(&mut input)?.resize(length, extra);
        rejected_error(input)?;
    }
    Ok(())
}

#[test]
fn counts_accept_zero_equality_strict_subsets_and_the_256_boundary() -> TestResult {
    for row in 0..5 {
        for counts in [
            [0, 0, 0],
            [1, 0, 0],
            [1, 1, 0],
            [1, 1, 1],
            [256, 0, 0],
            [256, 256, 0],
            [256, 256, 256],
            [256, 255, 254],
        ] {
            let mut input = serde_json::from_str::<Value>(READY_JSON)?;
            set_counts(&mut input, row, counts);
            round_trip(&input)?;
        }
    }
    Ok(())
}

#[test]
fn each_count_requires_a_bounded_nonnegative_json_integer() -> TestResult {
    for row in 0..5 {
        for field in COUNT_FIELDS {
            for invalid in [
                json!(-1),
                json!(257),
                json!(65_536),
                json!(u64::MAX),
                json!(0.0),
                json!(1.0),
                json!(256.0),
                json!(1.5),
                json!("256"),
                json!(true),
                Value::Null,
                json!([]),
                json!({}),
            ] {
                let mut input = serde_json::from_str::<Value>(READY_JSON)?;
                set_counts(&mut input, row, [256, 256, 256]);
                input["kinds"][row][field] = invalid;
                rejected_error(input)?;
            }
        }
    }
    Ok(())
}

#[test]
fn destructive_binding_and_attested_counts_obey_subset_order() -> TestResult {
    for row in 0..5 {
        for counts in [
            [0, 1, 0],
            [0, 0, 1],
            [1, 0, 1],
            [1, 2, 0],
            [2, 1, 2],
            [255, 256, 0],
            [256, 255, 256],
        ] {
            let mut input = serde_json::from_str::<Value>(READY_JSON)?;
            set_counts(&mut input, row, counts);
            rejected_error(input)?;
        }
    }
    Ok(())
}

#[test]
fn unavailable_sources_require_zero_counts_for_every_kind() -> TestResult {
    for (state, reason) in SOURCE_STATES {
        let Some(reason) = reason else { continue };
        for row in 0..5 {
            for counts in [[1, 0, 0], [1, 1, 0], [1, 1, 1]] {
                let mut input = serde_json::from_str::<Value>(MISSING_JSON)?;
                input["source_state"] = json!(state);
                input["source_reason"] = json!(reason);
                set_counts(&mut input, row, counts);
                rejected_error(input)?;
            }
        }
    }
    Ok(())
}

#[test]
fn invalid_attestations_require_zero_counts_for_every_kind() -> TestResult {
    for reason in ATTESTATION_REASONS {
        for row in 0..5 {
            for counts in [[1, 0, 0], [1, 1, 0], [1, 1, 1]] {
                let mut input = serde_json::from_str::<Value>(INVALID_ATTESTATION_JSON)?;
                input["attestation_reason"] = json!(reason);
                set_counts(&mut input, row, counts);
                rejected_error(input)?;
            }
        }
    }
    Ok(())
}

#[test]
fn unknown_and_private_fields_are_rejected_at_both_levels() -> TestResult {
    for field in [
        "unexpected",
        "path",
        "root_path",
        "source_root",
        "output_root",
        "requested_path",
        "canonical_path",
        "key",
        "logical_key",
        "id",
        "public_id",
        "slot_public_id",
        "generation_public_id",
        "identity",
        "device",
        "inode",
        "mount_id",
        "filesystem_type",
        "owner",
        "owner_uid",
        "owner_gid",
        "mode",
        "digest",
        "source_sha256",
        "generation_sha256",
        "attestation_sha256",
        "root_identity_sha256",
        "slots",
        "next_cursor",
        "source_reason_code",
        "attestation_reason_code",
    ] {
        for pointer in std::iter::once("").chain(KIND_POINTERS) {
            let mut input = serde_json::from_str::<Value>(READY_JSON)?;
            let object = input
                .pointer_mut(pointer)
                .and_then(Value::as_object_mut)
                .ok_or_else(|| io::Error::other("fixture object missing"))?;
            assert!(
                object
                    .insert(field.to_owned(), json!("/private/readiness-fixture"))
                    .is_none()
            );
            rejected_error(input)?;
        }
    }
    Ok(())
}

#[test]
fn unknown_null_or_container_fields_are_not_ignored_in_any_state() -> TestResult {
    for literal in [
        READY_JSON,
        EMPTY_READY_JSON,
        MISSING_JSON,
        INVALID_ATTESTATION_JSON,
    ] {
        for pointer in std::iter::once("").chain(KIND_POINTERS) {
            for extra in [Value::Null, json!(0), json!(true), json!([]), json!({})] {
                let mut input = serde_json::from_str::<Value>(literal)?;
                let object = input
                    .pointer_mut(pointer)
                    .and_then(Value::as_object_mut)
                    .ok_or_else(|| io::Error::other("fixture object missing"))?;
                assert!(object.insert("unexpected".to_owned(), extra).is_none());
                rejected_error(input)?;
            }
        }
    }
    Ok(())
}

fn assert_path_free_error(input: Value) -> TestResult {
    let error = rejected_error(input)?;
    for rendered in [error.to_string(), format!("{error:?}")] {
        assert!(
            !rendered.contains("readiness-redaction-sentinel"),
            "serde error echoed a rejected path"
        );
    }
    Ok(())
}

#[test]
fn serde_error_display_and_debug_do_not_echo_rejected_path_values() -> TestResult {
    for path in [
        "/private/readiness-redaction-sentinel/root",
        "C:\\private\\readiness-redaction-sentinel\\root",
    ] {
        for field in [
            "format_version",
            "source_state",
            "source_reason",
            "attestation_state",
            "attestation_reason",
            "generation",
            "kinds",
        ] {
            let mut input = serde_json::from_str::<Value>(READY_JSON)?;
            input[field] = json!(path);
            assert_path_free_error(input)?;
        }
        for row in 0..5 {
            for field in std::iter::once("kind").chain(COUNT_FIELDS) {
                let mut input = serde_json::from_str::<Value>(READY_JSON)?;
                input["kinds"][row][field] = json!(path);
                assert_path_free_error(input)?;
            }
        }
    }
    Ok(())
}

#[test]
fn serde_error_display_and_debug_do_not_echo_path_shaped_unknown_keys() -> TestResult {
    for pointer in std::iter::once("").chain(KIND_POINTERS) {
        let mut input = serde_json::from_str::<Value>(READY_JSON)?;
        let object = input
            .pointer_mut(pointer)
            .and_then(Value::as_object_mut)
            .ok_or_else(|| io::Error::other("fixture object missing"))?;
        assert!(
            object
                .insert(
                    "/private/readiness-redaction-sentinel/root".to_owned(),
                    Value::Null
                )
                .is_none()
        );
        assert_path_free_error(input)?;
    }
    Ok(())
}

#[test]
fn accepted_response_debug_has_no_filesystem_path_or_private_field() -> TestResult {
    for literal in [
        READY_JSON,
        EMPTY_READY_JSON,
        MISSING_JSON,
        INVALID_ATTESTATION_JSON,
    ] {
        let response =
            serde_json::from_value::<RootCatalogReadinessResponse>(serde_json::from_str(literal)?)?;
        let debug = format!("{response:?}");
        for forbidden in [
            "/",
            "\\",
            "requested_path",
            "canonical_path",
            "logical_key",
            "owner_uid",
            "root_identity_sha256",
        ] {
            assert!(
                !debug.contains(forbidden),
                "response debug exposed {forbidden}"
            );
        }
    }
    Ok(())
}
