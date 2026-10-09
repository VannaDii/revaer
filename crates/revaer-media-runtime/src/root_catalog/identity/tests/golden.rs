use super::{TestResult, golden_claims, hex, loaded, source};
use crate::root_catalog::{RootCatalogLoad, parse_root_catalog_v1};

use super::super::{
    RootIdentityEncodingError, aggregate_frame, encode_root_catalog_identity_v1, generation_frame,
};

// Independently transcribed ADR 517/550/557 fields, not encoder-produced fixtures.
// Ruby OpenSSL SHA256 over these literal bytes yielded the fixed digests below.
// Text lengths count UTF-8 bytes; no presence markers appear for NOT NULL fields.
const SLOT_A: &str = concat!(
    "7265766165722d6d656469612d726f6f742d736c6f742d6174746573746174696f6e00",
    "00000001",
    "0000000161",
    "000000052f6d2fc3a9",
    "000000082f7265616c2fc3a9",
    "01",
    "0102030405060708",
    "ffffffffffffffff",
    "0000000000000000",
    "0000000465787434",
    "01",
    "00000000",
    "00000000",
    "ffffffff",
    "00000fff",
);
const SLOT_Z: &str = concat!(
    "7265766165722d6d656469612d726f6f742d736c6f742d6174746573746174696f6e00",
    "00000001",
    "000000027a39",
    "000000052f776f726b",
    "000000052f776f726b",
    "04",
    "8000000000000000",
    "0000000000000000",
    "7fffffffffffffff",
    "00000003786673",
    "7e",
    "01020102",
    "ffffffff",
    "80000000",
    "00000000",
);
const SOURCE_SHA: &str = "121ea684c73474764ce8f45fe5bc9294977d17bc540cc485dcf433fe5e4a2945";
const A_SHA: &str = "22595f1988e86623ad0b03c3ce791a0792a721808dd3d57e81585f1955e2bc4f";
const Z_SHA: &str = "cfdfe871f1d7a72b088aef25782ea79eef3ae26df61d9848c08a663b25ee4fde";
const ATTESTATION_SHA: &str = "d46abdc7f552b03a9e41bbcfdddd8e8cd17e7462c515fc801e144701456918d2";

#[test]
fn independently_specified_slot_aggregate_and_generation_vectors() -> TestResult {
    let source = source()?;
    let claims = golden_claims(&source);
    let encoded = encode_root_catalog_identity_v1(&source, &claims)?;
    assert_eq!(hex(&encoded.source_sha256())?, SOURCE_SHA);
    assert_eq!(hex(encoded.slots()[0].frame())?, SLOT_A);
    assert_eq!(hex(encoded.slots()[1].frame())?, SLOT_Z);
    assert_eq!(encoded.slots()[0].frame().len(), 115);
    assert_eq!(encoded.slots()[1].frame().len(), 112);
    assert_eq!(hex(&encoded.slots()[0].root_identity_sha256())?, A_SHA);
    assert_eq!(hex(&encoded.slots()[1].root_identity_sha256())?, Z_SHA);
    let aggregate = aggregate_frame(encoded.source_sha256(), encoded.slots())?;
    assert_eq!(aggregate.len(), 145);
    assert_eq!(
        hex(&aggregate)?,
        format!(
            "7265766165722d6d656469612d726f6f742d6174746573746174696f6e0000000001\
         {SOURCE_SHA}000000020000000161{A_SHA}000000027a39{Z_SHA}"
        )
    );
    assert_eq!(hex(&encoded.attestation_sha256())?, ATTESTATION_SHA);
    let generation = generation_frame(encoded.source_sha256(), encoded.attestation_sha256());
    assert_eq!(generation.len(), 97);
    assert_eq!(
        hex(&generation)?,
        format!(
            "7265766165722d6d656469612d726f6f742d67656e65726174696f6e0000000001\
         {SOURCE_SHA}{ATTESTATION_SHA}"
        )
    );
    assert_eq!(
        hex(&encoded.generation_sha256())?,
        "c03388a6b0e9600eee95bf451ddd9f58c4f75f2c2700827dc06a5821c1e7b60f"
    );
    Ok(())
}

#[test]
fn loaded_empty_catalog_vectors_are_not_missing_source() -> TestResult {
    let source = loaded(parse_root_catalog_v1(
        br#"{"format_version":1,"slots":[]}"#,
    )?);
    let encoded = encode_root_catalog_identity_v1(&source, &[])?;
    assert!(encoded.slots().is_empty());
    assert_eq!(
        hex(&encoded.source_sha256())?,
        "8f0b256ad26c139e8c51bd426ba510eb43aefca9cb1d58fb0737b6518e6bc867"
    );
    assert_eq!(
        hex(&encoded.attestation_sha256())?,
        "8f02f1258c4d32e0a9919e632d0ab9ec9c17a96f888ee26ee90fb8b9007568cd"
    );
    assert_eq!(
        hex(&encoded.generation_sha256())?,
        "c69e6c6c3b5624cb119e895a8dac30a7989143139f9bde93199b8268295e2bc4"
    );
    assert_eq!(
        encode_root_catalog_identity_v1(&RootCatalogLoad::missing(), &[]),
        Err(RootIdentityEncodingError::SourceNotLoaded)
    );
    Ok(())
}
