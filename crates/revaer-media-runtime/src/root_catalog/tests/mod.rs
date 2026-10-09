mod parse;
mod source;

pub(super) const VALID_SLOT_JSON: &str = r#"{
    "key":"media-library",
    "allowed_kinds":["source","output"],
    "path":"/data/media-library",
    "durability_class":"restart_persistent",
    "durability_evidence":"linux_dedicated_mount",
    "sole_writer_class":"revaer_exclusive",
    "sole_writer_evidence":"linux_dedicated_service"
}"#;

pub(super) fn valid_document() -> String {
    format!(r#"{{"format_version":1,"slots":[{VALID_SLOT_JSON}]}}"#)
}

pub(super) fn digest_hex(bytes: [u8; 32]) -> String {
    use std::fmt::Write as _;

    let mut hex = String::with_capacity(64);
    for byte in bytes {
        write!(&mut hex, "{byte:02x}").expect("writing to a String cannot fail");
    }
    hex
}
