use crate::*;

/// Request for creating a new storage entry
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug, SerializedBytes, Clone)]
pub struct CreateStorageEntryRequest {
    pub label: String,
    pub content_type: String,
    pub size: u64,
    pub timestamp: u64,
    #[serde(with = "serde_bytes")]
    pub custom_metadata: Option<Vec<u8>>,
    /// Serialize the EntryHash as bytes
    #[serde_as(as = "Vec<serde_with::Bytes>")]
    pub chunk_hashes: Vec<Vec<u8>>,
}
