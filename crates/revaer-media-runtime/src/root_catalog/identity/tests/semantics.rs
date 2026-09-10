use super::super::{
    RootIdentityEncodingError, RootSlotIdentityClaims, capability_mask,
    encode_root_catalog_identity_v1,
};
use super::{SOURCE, TestResult, claim, encode_single, golden_claims, loaded, single, source};
use crate::root_catalog::{
    RootCatalogFileEvidence, RootCatalogLoad, RootCatalogSourceTrust, parse_root_catalog_v1,
};

#[test]
fn input_order_and_source_metadata_do_not_change_identity() -> TestResult {
    let source = source()?;
    let claims = golden_claims(&source);
    let expected = encode_root_catalog_identity_v1(&source, &claims)?;
    let reversed = [claims[1], claims[0]];
    assert_eq!(
        encode_root_catalog_identity_v1(&source, &reversed)?,
        expected
    );
    assert_eq!(expected.slots()[0].logical_key(), "a");
    assert_eq!(expected.slots()[1].logical_key(), "z9");
    let alternate_metadata = RootCatalogLoad::loaded(
        source.catalog().clone(),
        RootCatalogFileEvidence {
            trust: RootCatalogSourceTrust::Packaged,
            owner_uid: 0,
            mode: 0o444,
            document_bytes: 999,
        },
    );
    assert_eq!(
        encode_root_catalog_identity_v1(&alternate_metadata, &claims)?,
        expected
    );
    // No timestamp, DB id, activation state, or generation occurrence is an
    // encoder input. Re-encoding after any such occurrence yields the same bytes.
    assert_eq!(encode_root_catalog_identity_v1(&source, &claims)?, expected);
    Ok(())
}

#[test]
fn input_schema_excludes_timestamps_and_database_occurrence_ids() -> TestResult {
    let source = single()?;
    let fields = claim(&source.catalog().slots()[0]);
    // Exhaustive destructuring is a compile-time fence: adding a timestamp or
    // database occurrence field cannot silently extend the encoding inputs.
    let RootSlotIdentityClaims {
        declaration,
        canonical_path,
        filesystem_device,
        filesystem_inode,
        mount_id,
        filesystem_type,
        capabilities,
        owner_uid,
        owner_gid,
        mode_bits,
    } = fields;
    let reconstructed = RootSlotIdentityClaims {
        declaration,
        canonical_path,
        filesystem_device,
        filesystem_inode,
        mount_id,
        filesystem_type,
        capabilities,
        owner_uid,
        owner_gid,
        mode_bits,
    };
    assert_eq!(
        encode_root_catalog_identity_v1(&source, &[fields])?,
        encode_root_catalog_identity_v1(&source, &[reconstructed])?
    );
    Ok(())
}

#[test]
fn each_observed_scalar_participates_in_every_downstream_digest() -> TestResult {
    let source = source()?;
    let claims = golden_claims(&source);
    let expected = encode_root_catalog_identity_v1(&source, &claims)?;
    for field in 0..14 {
        let mut changed = claims;
        match field {
            0 => changed[0].canonical_path = "/different",
            1 => changed[0].filesystem_device ^= 1,
            2 => changed[0].filesystem_inode ^= 1,
            3 => changed[0].mount_id ^= 1,
            4 => changed[0].filesystem_type = "xfs",
            5 => changed[0].owner_uid ^= 1,
            6 => changed[0].owner_gid ^= 1,
            7 => changed[0].mode_bits ^= 1,
            bit => changed[0].capabilities[bit - 7] = true,
        }
        let actual = encode_root_catalog_identity_v1(&source, &changed)?;
        assert_eq!(actual.source_sha256(), expected.source_sha256());
        assert_ne!(
            actual.slots()[0].root_identity_sha256(),
            expected.slots()[0].root_identity_sha256()
        );
        assert_eq!(actual.slots()[1], expected.slots()[1]);
        assert_ne!(actual.attestation_sha256(), expected.attestation_sha256());
        assert_ne!(actual.generation_sha256(), expected.generation_sha256());
    }
    Ok(())
}

#[test]
fn source_only_changes_also_fence_the_entire_catalog() -> TestResult {
    let source = source()?;
    let expected = encode_root_catalog_identity_v1(&source, &golden_claims(&source))?;
    for (before, after) in [
        ("\"key\":\"a\"", "\"key\":\"b\""),
        (r"/m/\u00e9", "/other"),
        ("[\"workspace\"]", "[\"backup\"]"),
        (
            "kubernetes_persistent_volume_claim",
            "linux_dedicated_mount",
        ),
        ("kubernetes_read_write_once_pod", "linux_dedicated_service"),
    ] {
        let changed = loaded(parse_root_catalog_v1(
            SOURCE.replace(before, after).as_bytes(),
        )?);
        let actual = encode_root_catalog_identity_v1(&changed, &golden_claims(&changed))?;
        assert_ne!(actual.source_sha256(), expected.source_sha256());
        assert_ne!(actual.attestation_sha256(), expected.attestation_sha256());
        assert_ne!(actual.generation_sha256(), expected.generation_sha256());
    }
    Ok(())
}

#[test]
fn kind_order_is_canonical_but_declaration_mutation_is_rejected() -> TestResult {
    let source = single()?;
    let expected = encode_single(&source)?;
    let document = crate::root_catalog::tests::valid_document();
    let reordered = loaded(parse_root_catalog_v1(
        document
            .replace("[\"source\",\"output\"]", "[\"output\",\"source\"]")
            .as_bytes(),
    )?);
    assert_eq!(
        encode_root_catalog_identity_v1(&source, &[claim(&reordered.catalog().slots()[0])])?,
        expected
    );
    for (before, after) in [
        ("/data/media-library", "/different"),
        ("[\"source\",\"output\"]", "[\"source\"]"),
        (
            "linux_dedicated_mount",
            "kubernetes_persistent_volume_claim",
        ),
        ("linux_dedicated_service", "kubernetes_read_write_once_pod"),
    ] {
        let other = loaded(parse_root_catalog_v1(
            document.replace(before, after).as_bytes(),
        )?);
        assert_eq!(
            encode_root_catalog_identity_v1(&source, &[claim(&other.catalog().slots()[0])]),
            Err(RootIdentityEncodingError::DeclarationMismatch)
        );
    }
    Ok(())
}

#[test]
fn all_seven_capability_bits_are_exact_and_never_set_reserved_bit() -> TestResult {
    for mask in 0_u8..128 {
        let booleans = std::array::from_fn(|bit| mask & (1 << bit) != 0);
        assert_eq!(capability_mask(booleans), mask);
        assert_eq!(capability_mask(booleans) & 128, 0);
    }
    let source = source()?;
    let baseline = golden_claims(&source);
    for mask in 0_u8..128 {
        let mut claims = baseline;
        claims[0].capabilities = std::array::from_fn(|bit| mask & (1 << bit) != 0);
        let result = encode_root_catalog_identity_v1(&source, &claims);
        if mask & 1 == 0 {
            assert_eq!(result, Err(RootIdentityEncodingError::CapabilityMismatch));
        } else {
            let encoding = result?;
            let frame = encoding.slots()[0].frame();
            assert_eq!(frame[frame.len() - 17], mask);
        }
        claims = baseline;
        claims[1].capabilities = std::array::from_fn(|bit| mask & (1 << bit) != 0);
        let result = encode_root_catalog_identity_v1(&source, &claims);
        if mask & 0b0111_1110 == 0b0111_1110 {
            let encoding = result?;
            let frame = encoding.slots()[1].frame();
            assert_eq!(frame[frame.len() - 17], mask);
        } else {
            assert_eq!(result, Err(RootIdentityEncodingError::CapabilityMismatch));
        }
    }
    Ok(())
}

#[test]
fn all_kind_masks_and_exact_enum_assignments() -> TestResult {
    let document = crate::root_catalog::tests::valid_document();
    let names = ["source", "output", "workspace", "backup", "quarantine"];
    for mask in 1_u8..32 {
        let kinds: Vec<_> = names
            .iter()
            .enumerate()
            .filter(|(bit, _)| mask & (1 << bit) != 0)
            .map(|(_, name)| format!("\"{name}\""))
            .collect();
        let source = loaded(parse_root_catalog_v1(
            document
                .replace("[\"source\",\"output\"]", &format!("[{}]", kinds.join(",")))
                .as_bytes(),
        )?);
        let encoded = encode_single(&source)?;
        let bytes = encoded.slots()[0].frame();
        // 39-byte domain/version, then key(13), requested(19), canonical(19),
        // each with its own 4-byte prefix.
        assert_eq!(bytes[102], mask);
        assert_eq!(&bytes[bytes.len() - 16..bytes.len() - 12], &[1, 1, 1, 1]);
    }
    for (class, evidence, expected) in [
        ("disposable", "none", [0, 0]),
        ("restart_persistent", "linux_dedicated_mount", [1, 1]),
        (
            "restart_persistent",
            "kubernetes_persistent_volume_claim",
            [1, 2],
        ),
    ] {
        for (writer, proof, writer_bytes) in [
            ("uncontrolled", "none", [0, 0]),
            ("revaer_exclusive", "linux_dedicated_service", [1, 1]),
            ("revaer_exclusive", "kubernetes_read_write_once_pod", [1, 2]),
        ] {
            let doc = document
                .replace("[\"source\",\"output\"]", "[\"source\"]")
                .replace("restart_persistent", class)
                .replace("linux_dedicated_mount", evidence)
                .replace("revaer_exclusive", writer)
                .replace("linux_dedicated_service", proof);
            let source = loaded(parse_root_catalog_v1(doc.as_bytes())?);
            let encoding = encode_single(&source)?;
            let bytes = encoding.slots()[0].frame();
            assert_eq!(
                &bytes[bytes.len() - 16..bytes.len() - 12],
                &[expected[0], expected[1], writer_bytes[0], writer_bytes[1]]
            );
        }
    }
    Ok(())
}
