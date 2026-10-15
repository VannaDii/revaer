use sha2::{Digest, Sha256};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=init.sql");
    let digest: [u8; 32] = Sha256::digest(std::fs::read("init.sql")?).into();
    let output = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    std::fs::write(output.join("init-sha256.bin"), digest)?;
    Ok(())
}
