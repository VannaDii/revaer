use super::super::{RootIdentityEncodingError as Error, encode_root_catalog_identity_v1};
use super::{TestResult, claim, golden_claims, loaded, single, source};
use crate::root_catalog::{MAX_ROOT_CATALOG_SLOTS, parse_root_catalog_v1};

#[test]
fn duplicates_missing_extra_and_excess_claims_fail_closed() -> TestResult {
    let source = source()?;
    let claims = golden_claims(&source);
    assert_eq!(
        encode_root_catalog_identity_v1(&source, &claims[..1]),
        Err(Error::MissingSlot)
    );
    assert_eq!(
        encode_root_catalog_identity_v1(&source, &[claims[0], claims[0]]),
        Err(Error::DuplicateSlot)
    );
    let other = single()?;
    assert_eq!(
        encode_root_catalog_identity_v1(&source, &[claim(&other.catalog().slots()[0])]),
        Err(Error::UnexpectedSlot)
    );
    assert_eq!(
        encode_root_catalog_identity_v1(&source, &vec![claims[0]; MAX_ROOT_CATALOG_SLOTS + 1]),
        Err(Error::SlotBoundExceeded)
    );
    Ok(())
}

#[test]
fn maximum_catalog_is_complete_ordered_and_bounded() -> TestResult {
    let slot = crate::root_catalog::tests::VALID_SLOT_JSON;
    let slots = (0..MAX_ROOT_CATALOG_SLOTS)
        .rev()
        .map(|index| slot.replace("media-library", &format!("slot-{index:03}")))
        .collect::<Vec<_>>();
    let document = format!(r#"{{"format_version":1,"slots":[{}]}}"#, slots.join(","));
    let source = loaded(parse_root_catalog_v1(document.as_bytes())?);
    let mut claims = Vec::new();
    for (index, slot) in source.catalog().slots().iter().enumerate() {
        let mut fields = claim(slot);
        fields.filesystem_inode = u64::try_from(index)?;
        claims.push(fields);
    }
    let encoded = encode_root_catalog_identity_v1(&source, &claims)?;
    assert_eq!(encoded.slots().len(), 256);
    assert_eq!(encoded.slots()[0].logical_key(), "slot-000");
    assert_eq!(encoded.slots()[255].logical_key(), "slot-255");
    assert_eq!(
        encoded.slots().iter().map(|slot| slot.frame().len()).max(),
        Some(137)
    );
    Ok(())
}

#[test]
fn scalar_boundaries_preserve_unsigned_bytes_and_reject_overflow() -> TestResult {
    let source = single()?;
    let baseline = claim(&source.catalog().slots()[0]);
    for value in [0, i64::MAX.unsigned_abs(), 1_u64 << 63, u64::MAX] {
        let mut fields = baseline;
        fields.filesystem_device = value;
        fields.filesystem_inode = value;
        let encoded = encode_root_catalog_identity_v1(&source, &[fields])?;
        assert_eq!(&encoded.slots()[0].frame()[103..111], &value.to_be_bytes());
        assert_eq!(&encoded.slots()[0].frame()[111..119], &value.to_be_bytes());
    }
    for value in [0, i64::MAX.unsigned_abs()] {
        let mut fields = baseline;
        fields.mount_id = value;
        let encoded = encode_root_catalog_identity_v1(&source, &[fields])?;
        assert_eq!(&encoded.slots()[0].frame()[119..127], &value.to_be_bytes());
    }
    for value in [1_u64 << 63, u64::MAX] {
        let mut fields = baseline;
        fields.mount_id = value;
        assert_eq!(
            encode_root_catalog_identity_v1(&source, &[fields]),
            Err(Error::InvalidMountId)
        );
    }
    for value in [0, 1 << 31, u32::MAX] {
        let mut fields = baseline;
        fields.owner_uid = value;
        fields.owner_gid = value;
        let encoded = encode_root_catalog_identity_v1(&source, &[fields])?;
        let frame = encoded.slots()[0].frame();
        assert_eq!(
            &frame[frame.len() - 12..frame.len() - 8],
            &value.to_be_bytes()
        );
        assert_eq!(
            &frame[frame.len() - 8..frame.len() - 4],
            &value.to_be_bytes()
        );
    }
    for value in [0, 4094, 4095, 4096, u32::MAX] {
        let mut fields = baseline;
        fields.mode_bits = value;
        let result = encode_root_catalog_identity_v1(&source, &[fields]);
        if value <= 4095 {
            assert!(result.is_ok());
        } else {
            assert_eq!(result, Err(Error::InvalidModeBits));
        }
    }
    Ok(())
}

#[test]
fn text_bounds_use_exact_utf8_bytes_without_normalizing() -> TestResult {
    let source = single()?;
    let baseline = claim(&source.catalog().slots()[0]);
    for path in [
        String::new(),
        "/".into(),
        "relative".into(),
        "/nul\0path".into(),
        format!("/{}", "x".repeat(4096)),
    ] {
        let mut fields = baseline;
        fields.canonical_path = &path;
        assert_eq!(
            encode_root_catalog_identity_v1(&source, &[fields]),
            Err(Error::InvalidCanonicalPath)
        );
    }
    for length in [2, 4095, 4096] {
        let path = format!("/{}", "x".repeat(length - 1));
        let mut fields = baseline;
        fields.canonical_path = &path;
        assert!(encode_root_catalog_identity_v1(&source, &[fields]).is_ok());
    }
    for text in [
        String::new(),
        "x".repeat(65),
        "ext\0four".into(),
        "\u{e9}".repeat(33),
    ] {
        let mut fields = baseline;
        fields.filesystem_type = &text;
        assert_eq!(
            encode_root_catalog_identity_v1(&source, &[fields]),
            Err(Error::InvalidFilesystemType)
        );
    }
    for text in [
        "x".into(),
        "x".repeat(63),
        "x".repeat(64),
        "\u{e9}".repeat(32),
    ] {
        let mut fields = baseline;
        fields.filesystem_type = &text;
        assert!(encode_root_catalog_identity_v1(&source, &[fields]).is_ok());
    }
    Ok(())
}

#[test]
fn maximum_text_fields_have_exact_bounded_frame_and_utf8_lengths() -> TestResult {
    let key = "k".repeat(64);
    let path = format!("/{}x", "\u{e9}".repeat(2047));
    assert_eq!(path.len(), 4096);
    let document = crate::root_catalog::tests::valid_document()
        .replace("/data/media-library", &path)
        .replace("media-library", &key);
    let source = loaded(parse_root_catalog_v1(document.as_bytes())?);
    let filesystem_type = "f".repeat(64);
    let mut fields = claim(&source.catalog().slots()[0]);
    fields.filesystem_type = &filesystem_type;
    let encoded = encode_root_catalog_identity_v1(&source, &[fields])?;
    let bytes = encoded.slots()[0].frame();
    // 39-byte header; four text prefixes; 64+4096+4096+64 text bytes;
    // kind(1), device/inode/mount(24), capabilities/enums(5), owner/mode(12).
    assert_eq!(bytes.len(), 8417);
    assert_eq!(&bytes[39..43], &64_u32.to_be_bytes());
    assert_eq!(&bytes[107..111], &4096_u32.to_be_bytes());
    assert_eq!(&bytes[4207..4211], &4096_u32.to_be_bytes());
    Ok(())
}

#[test]
fn path_and_inode_overlap_rejected_but_shared_mount_siblings_allowed() -> TestResult {
    let source = source()?;
    let baseline = golden_claims(&source);
    for (left, right) in [
        ("/same", "/same"),
        ("/same", "/same/child"),
        ("/same/child", "/same"),
    ] {
        let mut claims = baseline;
        claims[0].canonical_path = left;
        claims[1].canonical_path = right;
        assert_eq!(
            encode_root_catalog_identity_v1(&source, &claims),
            Err(Error::Overlap)
        );
    }
    let mut claims = baseline;
    claims[0].filesystem_device = claims[1].filesystem_device;
    claims[0].filesystem_inode = claims[1].filesystem_inode;
    assert_eq!(
        encode_root_catalog_identity_v1(&source, &claims),
        Err(Error::Overlap)
    );
    claims = baseline;
    claims[0].mount_id = claims[1].mount_id;
    claims[0].canonical_path = "/prefix";
    claims[1].canonical_path = "/prefix-sibling";
    assert!(encode_root_catalog_identity_v1(&source, &claims).is_ok());
    claims[0].filesystem_device = claims[1].filesystem_device;
    assert!(encode_root_catalog_identity_v1(&source, &claims).is_ok());
    Ok(())
}

#[test]
fn writer_control_required_without_inventing_destructive_readiness() -> TestResult {
    let document = crate::root_catalog::tests::valid_document()
        .replace("[\"source\",\"output\"]", "[\"workspace\"]")
        .replace("restart_persistent", "disposable")
        .replace("linux_dedicated_mount", "none");
    let source = loaded(parse_root_catalog_v1(document.as_bytes())?);
    // Disposable writer claims can be encoded; this is not destructive readiness.
    assert!(super::encode_single(&source).is_ok());
    let unowned = document
        .replace("revaer_exclusive", "uncontrolled")
        .replace("linux_dedicated_service", "none");
    let source = loaded(parse_root_catalog_v1(unowned.as_bytes())?);
    assert_eq!(
        super::encode_single(&source),
        Err(Error::WriterControlMismatch)
    );
    Ok(())
}
