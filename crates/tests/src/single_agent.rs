use crate::dna_path::*;
use hdk::prelude::*;
use holochain::sweettest::{SweetConductor, SweetDnaFile};
use shared_types::ContentType;
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
        content_type: ContentType::Other("text/plain".to_string()),
        size: 6,
        last_modified: Timestamp::now(),
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

/// Test retrieving all storage entries
#[tokio::test(flavor = "multi_thread")]
async fn test_get_all_storage_entries() {
    // Set up conductor
    let mut conductor = SweetConductor::from_standard_config().await;

    // Load DNA from bundle
    let dna = SweetDnaFile::from_bundle(&dna_path()).await.unwrap();

    // Install app
    let app = conductor.setup_app("test_app", &[dna]).await.unwrap();
    let cell = &app.cells()[0];

    // Create chunks for first storage entry
    let chunk1_data = vec![1u8, 2, 3];
    let chunk1_record: Record = conductor
        .call(&cell.zome("storage"), "create_chunk_from_data", chunk1_data)
        .await;
    let chunk1_entry_hash: EntryHash = chunk1_record.action().entry_hash().unwrap().clone();

    // Create first storage entry
    let storage_entry1 = StorageEntry {
        label: "file1.txt".to_string(),
        content_type: ContentType::Other("text/plain".to_string()),
        size: 3,
        last_modified: Timestamp::now(),
        timestamp: Timestamp::now(),
        custom_metadata: None,
        chunk_hashes: vec![chunk1_entry_hash],
    };

    let _record1: Record = conductor
        .call(&cell.zome("storage"), "create_storage_entry", storage_entry1.clone())
        .await;

    // Create chunks for second storage entry
    let chunk2_data = vec![4u8, 5, 6, 7];
    let chunk2_record: Record = conductor
        .call(&cell.zome("storage"), "create_chunk_from_data", chunk2_data)
        .await;
    let chunk2_entry_hash: EntryHash = chunk2_record.action().entry_hash().unwrap().clone();

    // Create second storage entry
    let storage_entry2 = StorageEntry {
        label: "file2.bin".to_string(),
        content_type: ContentType::Other("application/octet-stream".to_string()),
        size: 4,
        last_modified: Timestamp::now(),
        timestamp: Timestamp::now(),
        custom_metadata: Some(SerializedBytes::from(UnsafeBytes::from("custom data".as_bytes().to_vec()))),
        chunk_hashes: vec![chunk2_entry_hash],
    };

    let _record2: Record = conductor
        .call(&cell.zome("storage"), "create_storage_entry", storage_entry2.clone())
        .await;

    // Retrieve all storage entries
    let all_entries: Vec<Record> = conductor
        .call(&cell.zome("storage"), "get_all_storage_entries", ())
        .await;

    // Verify we got both entries
    assert_eq!(all_entries.len(), 2, "Expected 2 storage entries");

    // Extract storage entries directly
    let storage_entries: Vec<StorageEntry> = all_entries
        .iter()
        .filter_map(|record| {
            record
                .entry()
                .to_app_option::<StorageEntry>()
                .ok()
                .flatten()
        })
        .collect();

    // Print entries to console
    println!("Retrieved storage entries:");
    for entry in &storage_entries {
        println!("  - {:?}", entry);
    }

    assert!(storage_entries.iter().any(|e| e.label == "file1.txt"), "Should contain file1.txt");
    assert!(storage_entries.iter().any(|e| e.label == "file2.bin"), "Should contain file2.bin");
}

/// Test file upload and download roundtrip using get_file
#[tokio::test(flavor = "multi_thread")]
async fn test_file_upload_download_roundtrip() {
    // Set up conductor
    let mut conductor = SweetConductor::from_standard_config().await;

    // Load DNA from bundle
    let dna = SweetDnaFile::from_bundle(&dna_path()).await.unwrap();

    // Install app
    let app = conductor.setup_app("test_app", &[dna]).await.unwrap();
    let cell = &app.cells()[0];

    // Create test data - a larger file split across multiple chunks
    let test_data: Vec<u8> = (0..255u8).cycle().take(1500).collect();

    // Create chunks from the test data (simulating chunking like upload_file does)
    let chunk_size = 500;
    let mut chunk_entry_hashes: Vec<EntryHash> = Vec::new();

    for chunk_data in test_data.chunks(chunk_size) {
        let chunk_record: Record = conductor
            .call(&cell.zome("storage"), "create_chunk_from_data", chunk_data.to_vec())
            .await;
        chunk_entry_hashes.push(chunk_record.action().entry_hash().unwrap().clone());
    }

    // Create storage entry with chunk references
    let storage_entry = StorageEntry {
        label: "test_roundtrip.bin".to_string(),
        content_type: ContentType::Other("application/octet-stream".to_string()),
        size: test_data.len(),
        last_modified: Timestamp::now(),
        timestamp: Timestamp::now(),
        custom_metadata: None,
        chunk_hashes: chunk_entry_hashes,
    };

    let storage_record: Record = conductor
        .call(&cell.zome("storage"), "create_storage_entry", storage_entry)
        .await;

    let storage_entry_hash = storage_record.action_address().clone();

    // Now use get_file to retrieve the complete file data
    let retrieved_data: Option<Vec<u8>> = conductor
        .call(&cell.zome("storage"), "get_file", storage_entry_hash)
        .await;

    // Verify the retrieved data matches the original
    assert!(retrieved_data.is_some(), "Should retrieve file data");
    let retrieved = retrieved_data.unwrap();
    assert_eq!(retrieved.len(), test_data.len(), "Data length should match");
    assert_eq!(retrieved, test_data, "Data content should match exactly");

    println!("Successfully uploaded and downloaded {} bytes across 3 chunks", test_data.len());
}
