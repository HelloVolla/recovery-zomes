use std::path::PathBuf;

/// Returns the path to the packaged DNA for testing
#[cfg(test)]
pub fn dna_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("crates/dnas/recovery/workdir/volla_recovery.dna")
}
