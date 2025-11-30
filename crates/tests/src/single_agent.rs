use crate::dna_path::*;
use hdk::prelude::*;
use holochain::sweettest::{SweetConductor, SweetDnaFile};
use storage_integrity::{Chunk, StorageEntry};

/// Test chunk creation and retrieval lifecycle
#[tokio::test(flavor = "multi_thread")]
async fn test_chunk_lifecycle() {
    // Set up conductor
    let mut conductor = SweetConductor::from_standard_config().await;

    // Load DNA from bundle
    let dna = SweetDnaFile::from_bundle(&dna_path()).await.unwrap();

    // Install app
    let app = conductor.setup_app("test_app", &[dna]).await.unwrap();
    let cell = &app.cells()[0];

    // Create test data
    let test_data = vec![1u8, 2, 3, 4, 5];

    // Create chunk from data
    let chunk_record: Record = conductor
        .call(&cell.zome("storage"), "create_chunk_from_data", test_data.clone())
        .await;

    // Retrieve the chunk
    let entry_hash: EntryHash = chunk_record.action().entry_hash().unwrap().clone();
    let retrieved_chunk: Option<Chunk> = conductor
        .call(&cell.zome("storage"), "get_chunk_from_entry_hash", entry_hash)
        .await;

    // Verify chunk was retrieved
    assert!(retrieved_chunk.is_some());
    let chunk = retrieved_chunk.unwrap();

    // Verify chunk data matches
    let chunk_bytes: Vec<u8> = chunk.0.bytes().to_vec();
    assert_eq!(chunk_bytes, test_data);
}
