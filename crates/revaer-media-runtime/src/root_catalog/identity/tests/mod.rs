use crate::root_catalog::{
    RootCatalog, RootCatalogFileEvidence, RootCatalogLoad, RootCatalogSlot, RootCatalogSourceTrust,
    parse_root_catalog_v1,
};

use super::{RootCatalogIdentityEncoding, RootSlotIdentityClaims, encode_root_catalog_identity_v1};

mod bounds;
mod golden;
mod semantics;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SOURCE: &str = r#"{"format_version":1,"slots":[
    {"key":"z9","path":"/work","allowed_kinds":["workspace"],
     "durability_class":"restart_persistent","durability_evidence":"kubernetes_persistent_volume_claim",
     "sole_writer_class":"revaer_exclusive","sole_writer_evidence":"kubernetes_read_write_once_pod"},
    {"key":"a","path":"/m/\u00e9","allowed_kinds":["source"],
     "durability_class":"disposable","durability_evidence":"none",
     "sole_writer_class":"uncontrolled","sole_writer_evidence":"none"}
]}"#;

// Synthetic source metadata is confined to unit fixtures. No descriptor proof
// or package validation is asserted by constructing a loaded test source.
fn loaded(catalog: RootCatalog) -> RootCatalogLoad {
    RootCatalogLoad::loaded_for_encoding_test(
        catalog,
        RootCatalogFileEvidence {
            trust: RootCatalogSourceTrust::NativeOverride,
            owner_uid: 1000,
            mode: 0o400,
            document_bytes: SOURCE.len(),
        },
    )
}

fn source() -> Result<RootCatalogLoad, Box<dyn std::error::Error>> {
    Ok(loaded(parse_root_catalog_v1(SOURCE.as_bytes())?))
}

fn claim(slot: &RootCatalogSlot) -> RootSlotIdentityClaims<'_> {
    RootSlotIdentityClaims {
        declaration: slot,
        canonical_path: slot.path(),
        filesystem_device: 1,
        filesystem_inode: 2,
        mount_id: 3,
        filesystem_type: "ext4",
        capabilities: [true; 7],
        owner_uid: 4,
        owner_gid: 5,
        mode_bits: 0o700,
    }
}

fn golden_claims(source: &RootCatalogLoad) -> [RootSlotIdentityClaims<'_>; 2] {
    let a = RootSlotIdentityClaims {
        declaration: &source.catalog().slots()[0],
        canonical_path: "/real/\u{e9}",
        filesystem_device: 0x0102_0304_0506_0708,
        filesystem_inode: u64::MAX,
        mount_id: 0,
        filesystem_type: "ext4",
        capabilities: [true, false, false, false, false, false, false],
        owner_uid: 0,
        owner_gid: u32::MAX,
        mode_bits: 4095,
    };
    let z = RootSlotIdentityClaims {
        declaration: &source.catalog().slots()[1],
        canonical_path: "/work",
        filesystem_device: 1 << 63,
        filesystem_inode: 0,
        mount_id: i64::MAX.unsigned_abs(),
        filesystem_type: "xfs",
        capabilities: [false, true, true, true, true, true, true],
        owner_uid: u32::MAX,
        owner_gid: 1 << 31,
        mode_bits: 0,
    };
    [a, z]
}

fn encode_single(
    source: &RootCatalogLoad,
) -> Result<RootCatalogIdentityEncoding, super::RootIdentityEncodingError> {
    encode_root_catalog_identity_v1(source, &[claim(&source.catalog().slots()[0])])
}

fn single() -> Result<RootCatalogLoad, Box<dyn std::error::Error>> {
    Ok(loaded(parse_root_catalog_v1(
        crate::root_catalog::tests::valid_document().as_bytes(),
    )?))
}

fn hex(bytes: &[u8]) -> Result<String, std::fmt::Error> {
    use std::fmt::Write;
    let mut result = String::new();
    for byte in bytes {
        write!(result, "{byte:02x}")?;
    }
    Ok(result)
}
