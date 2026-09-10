use super::{
    RootCatalogCursor, RootInputError, validate_root_association_prefix,
    validate_root_candidate_path, validate_root_catalog_limit, validate_root_logical_key,
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use std::{error::Error, io};
use uuid::Uuid;

// Ruby stdlib: Base64.urlsafe_encode64([key.bytesize].pack("n") + key.b +
// [uuid_hex].pack("H*"), padding: false). No parent encoder produced these literals.
const CURSOR_GOLDENS: [(&str, &str, &str); 5] = [
    (
        "a",
        "00000000-0000-0000-0000-000000000000",
        "AAFhAAAAAAAAAAAAAAAAAAAAAA",
    ),
    (
        "ab",
        "00010203-0405-0607-0809-0a0b0c0d0e0f",
        "AAJhYgABAgMEBQYHCAkKCwwNDg8",
    ),
    (
        "0-9",
        "00112233-4455-6677-8899-aabbccddeeff",
        "AAMwLTkAESIzRFVmd4iZqrvM3e7_",
    ),
    (
        "root",
        "fbfffbff-fbff-fbff-fbff-fbfffbfffbff",
        "AARyb290-__7__v_-__7__v_-__7_w",
    ),
    (
        "abcdefghijklmnopqrstuvwxyz0123456789--abcdefghijklmnopqrstuvwxyz",
        "ffffffff-ffff-ffff-ffff-ffffffffffff",
        concat!(
            "AEBhYmNkZWZnaGlqa2xtbm9wcXJzdHV2d3h5ejAxMjM0NTY3ODkt",
            "LWFiY2RlZmdoaWprbG1ub3BxcnN0dXZ3eHl6_____________________w"
        ),
    ),
];

fn check(condition: bool, message: &'static str) -> Result<(), Box<dyn Error>> {
    if condition {
        Ok(())
    } else {
        Err(io::Error::other(message).into())
    }
}

fn rejects<T>(result: &Result<T, RootInputError>) -> Result<(), Box<dyn Error>> {
    check(result.is_err(), "invalid root input was accepted")
}

// Only malformed-input tests use this envelope builder; success uses literals.
fn malformed_key_cursor(key: &[u8]) -> Result<String, Box<dyn Error>> {
    let mut payload = u16::try_from(key.len())?.to_be_bytes().to_vec();
    payload.extend_from_slice(key);
    payload.extend_from_slice(Uuid::nil().as_bytes());
    Ok(URL_SAFE_NO_PAD.encode(payload))
}

#[test]
fn logical_key_ascii_grammar_is_exact_at_every_position() -> Result<(), Box<dyn Error>> {
    for byte in 0_u8..=127 {
        let character = char::from(byte);
        let edge_allowed = byte.is_ascii_lowercase() || byte.is_ascii_digit();
        let interior_allowed = edge_allowed || byte == b'-';
        for value in [
            character.to_string(),
            format!("{character}a"),
            format!("a{character}"),
        ] {
            if edge_allowed {
                validate_root_logical_key(&value)?;
            } else {
                rejects(&validate_root_logical_key(&value))?;
            }
        }
        let interior = format!("a{character}z");
        if interior_allowed {
            validate_root_logical_key(&interior)?;
        } else {
            rejects(&validate_root_logical_key(&interior))?;
        }
    }
    Ok(())
}

#[test]
fn logical_key_lengths_and_internal_hyphens_are_exact() -> Result<(), Box<dyn Error>> {
    for length in 1..=64 {
        validate_root_logical_key(&"a".repeat(length))?;
        validate_root_logical_key(&"0".repeat(length))?;
    }
    for value in ["a--b", "0-9", "a-b-c", "9--0"] {
        validate_root_logical_key(value)?;
    }
    validate_root_logical_key(&format!("a{}z", "-".repeat(62)))?;
    for value in ["", "-", "--", "-a", "a-"] {
        rejects(&validate_root_logical_key(value))?;
    }
    rejects(&validate_root_logical_key(&"a".repeat(65)))?;
    rejects(&validate_root_logical_key(&"a".repeat(4_096)))?;
    Ok(())
}

#[test]
fn logical_keys_reject_whitespace_and_non_ascii() -> Result<(), Box<dyn Error>> {
    for value in [
        " a",
        "a ",
        "\ta",
        "a\n",
        "a\r\n",
        "a_b",
        "\u{e9}",
        "e\u{301}",
        "\u{ff41}",
        "a\u{a0}b",
        "a\u{200b}b",
    ] {
        rejects(&validate_root_logical_key(value))?;
    }
    Ok(())
}

#[test]
fn only_empty_association_prefix_selects_the_whole_root() -> Result<(), Box<dyn Error>> {
    validate_root_association_prefix("")?;
    rejects(&validate_root_candidate_path(""))?;
    for value in ["/", "//", ".", "..", "./", "../"] {
        rejects(&validate_root_association_prefix(value))?;
        rejects(&validate_root_candidate_path(value))?;
    }
    Ok(())
}

#[test]
fn relative_paths_reject_only_the_prohibited_component_forms() -> Result<(), Box<dyn Error>> {
    for value in [
        "/name", "name/", "a//b", "a///b", "a/./b", "a/../b", "./a", "../a", "a/.", "a/..",
        "a/b/./c", "a/b/../c", "\\", "\\name", "a\\b", "a/b\\c", "\0", "\0name", "name\0",
        "a/\0/b",
    ] {
        rejects(&validate_root_candidate_path(value))?;
        rejects(&validate_root_association_prefix(value))?;
    }
    Ok(())
}

#[test]
fn relative_paths_do_not_invent_ascii_character_restrictions() -> Result<(), Box<dyn Error>> {
    for byte in 0_u8..=127 {
        let value = format!("a{}b", char::from(byte));
        if matches!(byte, 0 | b'\\') {
            rejects(&validate_root_candidate_path(&value))?;
            rejects(&validate_root_association_prefix(&value))?;
        } else {
            validate_root_candidate_path(&value)?;
            validate_root_association_prefix(&value)?;
        }
    }
    Ok(())
}

#[test]
fn relative_paths_accept_non_special_unicode_and_literal_text() -> Result<(), Box<dyn Error>> {
    for value in [
        " ",
        "\t",
        "\n",
        "\r",
        " leading and trailing ",
        ".hidden",
        "...",
        "a/.../b",
        "a/..name",
        "a/name..",
        "C:relative",
        "~",
        "%00",
        "%2f",
        "%2e%2e/file",
        "a?query#fragment",
        "caf\u{e9}/film",
        "cafe\u{301}/film",
        "\u{ff0e}/\u{ff0e}\u{ff0e}",
        "\u{2215}/\u{ff0f}",
        "\u{1f680}/file",
    ] {
        validate_root_candidate_path(value)?;
        validate_root_association_prefix(value)?;
    }
    Ok(())
}

#[test]
fn path_budget_counts_utf8_bytes_not_characters_or_components() -> Result<(), Box<dyn Error>> {
    let boundary_paths = [
        "x".repeat(4_096),
        "\u{e9}".repeat(2_048),
        "\u{1f680}".repeat(1_024),
        format!("{}/a", "\u{e9}".repeat(2_047)),
    ];
    for value in boundary_paths {
        check(value.len() == 4_096, "incorrect byte-boundary fixture")?;
        validate_root_candidate_path(&value)?;
        validate_root_association_prefix(&value)?;
        let oversized = format!("{value}a");
        check(oversized.len() == 4_097, "incorrect oversized fixture")?;
        rejects(&validate_root_candidate_path(&oversized))?;
        rejects(&validate_root_association_prefix(&oversized))?;
    }
    Ok(())
}

#[test]
fn cursor_encodes_independent_literal_goldens() -> Result<(), Box<dyn Error>> {
    for (key, uuid_text, literal) in CURSOR_GOLDENS {
        let slot_id = Uuid::parse_str(uuid_text)?;
        let cursor = RootCatalogCursor::new(key, slot_id)?;
        check(cursor.logical_key() == key, "constructor changed the key")?;
        check(
            cursor.slot_public_id() == slot_id,
            "constructor changed UUID",
        )?;
        check(
            cursor.encode()? == literal,
            "cursor encoding differs from golden",
        )?;
    }
    Ok(())
}

#[test]
fn cursor_decodes_independent_literal_goldens() -> Result<(), Box<dyn Error>> {
    for (key, uuid_text, literal) in CURSOR_GOLDENS {
        let cursor = RootCatalogCursor::decode(literal)?;
        check(
            cursor.logical_key() == key,
            "decoded key differs from golden",
        )?;
        check(
            cursor.slot_public_id() == Uuid::parse_str(uuid_text)?,
            "decoded UUID byte order differs from golden",
        )?;
        check(
            cursor.encode()? == literal,
            "decoded cursor is not canonical",
        )?;
    }
    Ok(())
}

#[test]
fn cursor_maximum_is_82_raw_bytes_and_110_encoded_bytes() -> Result<(), Box<dyn Error>> {
    let (key, _, literal) = CURSOR_GOLDENS
        .last()
        .ok_or_else(|| io::Error::other("maximum cursor fixture missing"))?;
    check(key.len() == 64, "maximum key fixture length")?;
    check(literal.len() == 110, "maximum encoded fixture length")?;
    let raw = URL_SAFE_NO_PAD.decode(literal)?;
    check(raw.len() == 82, "maximum raw fixture length")?;
    let cursor = RootCatalogCursor::decode(literal)?;
    check(cursor.logical_key() == *key, "maximum key was changed")?;
    rejects(&RootCatalogCursor::decode(&"A".repeat(111)))?;
    rejects(&RootCatalogCursor::decode(&"A".repeat(112)))?;
    rejects(&RootCatalogCursor::decode(&"A".repeat(4_096)))?;
    Ok(())
}

#[test]
fn cursor_nil_uuid_and_unknown_membership_are_structurally_valid() -> Result<(), Box<dyn Error>> {
    let cursor = RootCatalogCursor::new("not-in-a-catalog", Uuid::nil())?;
    check(cursor.slot_public_id().is_nil(), "nil UUID was changed")?;
    let decoded = RootCatalogCursor::decode(&cursor.encode()?)?;
    check(
        decoded.logical_key() == "not-in-a-catalog" && decoded.slot_public_id().is_nil(),
        "codec imposed catalog membership or UUID-version authority",
    )?;
    Ok(())
}

#[test]
fn cursor_constructor_and_decoder_share_exact_key_grammar() -> Result<(), Box<dyn Error>> {
    for value in [
        String::new(),
        "-a".to_owned(),
        "a-".to_owned(),
        "A".to_owned(),
        "a_b".to_owned(),
        " a".to_owned(),
        "a\n".to_owned(),
        "a\0b".to_owned(),
        "\u{e9}".to_owned(),
        "a".repeat(65),
    ] {
        rejects(&RootCatalogCursor::new(&value, Uuid::nil()))?;
        let malformed = malformed_key_cursor(value.as_bytes())?;
        rejects(&RootCatalogCursor::decode(&malformed))?;
    }
    for bytes in [&[0xff][..], &[0xc3, 0x28][..]] {
        rejects(&RootCatalogCursor::decode(&malformed_key_cursor(bytes)?))?;
    }
    Ok(())
}

#[test]
fn cursor_rejects_padding_whitespace_and_alternate_alphabets() -> Result<(), Box<dyn Error>> {
    for (_, _, literal) in CURSOR_GOLDENS {
        for malformed in [
            format!("{literal}="),
            format!("{literal}=="),
            format!("={literal}"),
            format!(" {literal}"),
            format!("{literal} "),
            format!("{literal}\n"),
            format!("\t{literal}"),
            format!("{literal}%3D"),
        ] {
            rejects(&RootCatalogCursor::decode(&malformed))?;
        }
    }
    let url_literal = "AARyb290-__7__v_-__7__v_-__7_w";
    for malformed in [
        url_literal.replace('-', "+"),
        url_literal.replace('_', "/"),
        url_literal.replace('-', "%2D"),
        "AA\nFhAAAAAAAAAAAAAAAAAAAAAA".to_owned(),
        "AA=FhAAAAAAAAAAAAAAAAAAAAAA".to_owned(),
        "AAFhAAAAAAAAAAAAAAAAAAAAA\u{e9}".to_owned(),
    ] {
        rejects(&RootCatalogCursor::decode(&malformed))?;
    }
    Ok(())
}

#[test]
fn cursor_rejects_nonzero_unused_final_bits() -> Result<(), Box<dyn Error>> {
    for malformed in [
        "AAFhAAAAAAAAAAAAAAAAAAAAAB",
        "AAFhAAAAAAAAAAAAAAAAAAAAAP",
        "AAJhYgABAgMEBQYHCAkKCwwNDg9",
        "AAJhYgABAgMEBQYHCAkKCwwNDg-",
        "AAJhYgABAgMEBQYHCAkKCwwNDg_",
        "AARyb290-__7__v_-__7__v_-__7_x",
    ] {
        rejects(&RootCatalogCursor::decode(malformed))?;
    }
    Ok(())
}

#[test]
fn cursor_rejects_every_truncated_golden() -> Result<(), Box<dyn Error>> {
    for (_, _, literal) in CURSOR_GOLDENS {
        for length in 0..literal.len() {
            let truncated = literal
                .get(..length)
                .ok_or_else(|| io::Error::other("non-ASCII cursor fixture"))?;
            rejects(&RootCatalogCursor::decode(truncated))?;
        }
        let raw = URL_SAFE_NO_PAD.decode(literal)?;
        for length in 0..raw.len() {
            let truncated = raw
                .get(..length)
                .ok_or_else(|| io::Error::other("invalid raw fixture boundary"))?;
            rejects(&RootCatalogCursor::decode(
                &URL_SAFE_NO_PAD.encode(truncated),
            ))?;
        }
    }
    Ok(())
}

#[test]
fn cursor_rejects_trailing_bytes_instead_of_ignoring_them() -> Result<(), Box<dyn Error>> {
    for (_, _, literal) in CURSOR_GOLDENS {
        let raw = URL_SAFE_NO_PAD.decode(literal)?;
        for trailing in [vec![0], vec![255], vec![0; 16]] {
            let mut malformed = raw.clone();
            malformed.extend_from_slice(&trailing);
            rejects(&RootCatalogCursor::decode(
                &URL_SAFE_NO_PAD.encode(malformed),
            ))?;
        }
        rejects(&RootCatalogCursor::decode(&format!("{literal}A")))?;
    }
    Ok(())
}

#[test]
fn cursor_rejects_wrong_big_endian_lengths_and_short_envelopes() -> Result<(), Box<dyn Error>> {
    for declared_length in [0_u16, 2, 64, 65, 256, u16::MAX] {
        let mut payload = declared_length.to_be_bytes().to_vec();
        payload.push(b'a');
        payload.extend_from_slice(Uuid::nil().as_bytes());
        rejects(&RootCatalogCursor::decode(&URL_SAFE_NO_PAD.encode(payload)))?;
    }
    for length in 0..19 {
        rejects(&RootCatalogCursor::decode(
            &URL_SAFE_NO_PAD.encode(vec![0; length]),
        ))?;
    }
    Ok(())
}

#[test]
fn cursor_rejects_json_shaped_legacy_payloads() -> Result<(), Box<dyn Error>> {
    for legacy in [
        r#"{"logical_key":"a","slot_public_id":"00000000-0000-0000-0000-000000000000"}"#,
        r#"["a","00000000-0000-0000-0000-000000000000"]"#,
        r#"{"version":1,"key":"a","id":null}"#,
    ] {
        rejects(&RootCatalogCursor::decode(legacy))?;
        rejects(&RootCatalogCursor::decode(
            &URL_SAFE_NO_PAD.encode(legacy.as_bytes()),
        ))?;
    }
    Ok(())
}

#[test]
fn catalog_limit_matches_the_entire_u16_domain() -> Result<(), Box<dyn Error>> {
    check(
        validate_root_catalog_limit(None)? == 50,
        "default root catalog limit is not 50",
    )?;
    for limit in 0..=u16::MAX {
        if (1..=200).contains(&limit) {
            check(
                validate_root_catalog_limit(Some(limit))? == limit,
                "valid root catalog limit was changed",
            )?;
        } else {
            rejects(&validate_root_catalog_limit(Some(limit)))?;
        }
    }
    Ok(())
}

#[test]
fn root_input_errors_are_closed_and_do_not_echo_values() -> Result<(), Box<dyn Error>> {
    let expected_display = RootInputError.to_string();
    let expected_debug = format!("{RootInputError:?}");
    let errors = [
        validate_root_logical_key("sensitive_key").err(),
        validate_root_candidate_path("/private/candidate").err(),
        validate_root_association_prefix("../private/prefix").err(),
        RootCatalogCursor::new("private_key", Uuid::nil()).err(),
        RootCatalogCursor::decode("private-cursor-value").err(),
        validate_root_catalog_limit(Some(201)).err(),
    ];
    for error in errors {
        let error = error.ok_or_else(|| io::Error::other("invalid input had no error"))?;
        check(error.to_string() == expected_display, "error exposed input")?;
        check(
            format!("{error:?}") == expected_debug,
            "debug exposed input",
        )?;
        check(error.source().is_none(), "root input error exposed a cause")?;
    }
    Ok(())
}

#[test]
fn cursor_debug_omits_key_and_identity() -> Result<(), Box<dyn Error>> {
    let cursor = RootCatalogCursor::new("private-library", Uuid::from_u128(1))?;
    check(cursor.clone() == cursor, "cursor clone changed identity")?;
    check(
        format!("{cursor:?}") == "RootCatalogCursor",
        "cursor debug exposed its key or identity",
    )?;
    Ok(())
}
