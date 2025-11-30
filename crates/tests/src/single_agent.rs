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

/// Test storage entry creation with chunk references
#[tokio::test(flavor = "multi_thread")]
async fn test_storage_entry_creation() {
    // Set up conductor
    let mut conductor = SweetConductor::from_standard_config().await;

    // Load DNA from bundle
    let dna = SweetDnaFile::from_bundle(&dna_path()).await.unwrap();

    // Install app
    let app = conductor.setup_app("test_app", &[dna]).await.unwrap();
    let cell = &app.cells()[0];

    // Create test chunks
    let chunk1_data = vec![1u8, 2, 3];
    let chunk2_data = vec![4u8, 5, 6];

    let chunk1_record: Record = conductor
        .call(&cell.zome("storage"), "create_chunk_from_data", chunk1_data)
        .await;

    let chunk2_record: Record = conductor
        .call(&cell.zome("storage"), "create_chunk_from_data", chunk2_data)
        .await;

    let chunk1_entry_hash: EntryHash = chunk1_record.action().entry_hash().unwrap().clone();
    let chunk2_entry_hash: EntryHash = chunk2_record.action().entry_hash().unwrap().clone();

    // Create storage entry with chunk references
    let storage_entry = StorageEntry {
        label: "test_file.txt".to_string(),
        content_type: "text/plain".to_string(),
        size: 6,
        timestamp: Timestamp::now(),
        custom_metadata: None,
        chunk_hashes: vec![chunk1_entry_hash.clone(), chunk2_entry_hash.clone()],
    };

    let record: Record = conductor
        .call(&cell.zome("storage"), "create_storage_entry", storage_entry.clone())
        .await;

    // Verify record was created
    assert!(record.action().author().clone().into_inner().len() > 0);

    // Verify storage entry can be retrieved
    let entry = record.entry().to_app_option::<StorageEntry>().unwrap();
    assert!(entry.is_some());

    let retrieved_entry = entry.unwrap();
    assert_eq!(retrieved_entry.label, "test_file.txt");
    assert_eq!(retrieved_entry.size, 6);
    assert_eq!(retrieved_entry.chunk_hashes.len(), 2);

    // Verify links were created from chunks to storage entry
    let links_from_chunk1: Vec<Link> = conductor
        .call(&cell.zome("storage"), "get_storage_entries_for_chunk", chunk1_entry_hash.clone())
        .await;

    assert_eq!(links_from_chunk1.len(), 1);

    let links_from_chunk2: Vec<Link> = conductor
        .call(&cell.zome("storage"), "get_storage_entries_for_chunk", chunk2_entry_hash.clone())
        .await;

    assert_eq!(links_from_chunk2.len(), 1);
}
