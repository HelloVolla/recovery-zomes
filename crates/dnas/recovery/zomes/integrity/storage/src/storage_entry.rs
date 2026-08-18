use hdi::prelude::*;
use integrity_types::{ContentType, EncryptedMetadata};


/// Main storage entry containing metadata and references to data chunks
/// All actual data is stored in DataChunk entries, referenced by EntryHash
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct StorageEntry {
    /// Content type
    pub content_type: ContentType,
    /// Total size in bytes
    pub size: usize,
    /// Last modified timestamp
    pub last_modified: Timestamp,
    /// Timestamp when the data was collected
    pub timestamp: Timestamp,
    /// Optional strongly-typed metadata for specific content types.
    /// Encrypted with XChaCha20-Poly1305 using the owner's Android Keystore key.
    pub metadata: Option<EncryptedMetadata>,

    /// Ordered list of chunk hashes that make up the complete data
    pub chunk_hashes: Vec<EntryHash>,
}

pub fn validate_create_storage_entry(
    _action: TypedAction<EntryCreationData>,
    _storage_entry: StorageEntry,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_storage_entry(
    _action: TypedAction<UpdateData>,
    _storage_entry: StorageEntry,
    _original_action: TypedAction<EntryCreationData>,
    _original_storage_entry: StorageEntry,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_storage_entry(
    _action: TypedAction<DeleteData>,
    _original_action: TypedAction<EntryCreationData>,
    _original_storage_entry: StorageEntry,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_chunk_to_storage_entries(
    _action: TypedAction<CreateLinkData>,
    _base_address: AnyLinkableHash,
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_chunk_to_storage_entries(
    _action: TypedAction<DeleteLinkData>,
    _original_action: TypedAction<CreateLinkData>,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_storage_entry_updates(
    _action: TypedAction<CreateLinkData>,
    _base_address: AnyLinkableHash,
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_storage_entry_updates(
    _action: TypedAction<DeleteLinkData>,
    _original_action: TypedAction<CreateLinkData>,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "StorageEntryUpdates links cannot be deleted".to_string(),
    ))
}

pub fn validate_create_link_all_storage_entries(
    _action: TypedAction<CreateLinkData>,
    _base_address: AnyLinkableHash,
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_all_storage_entries(
    _action: TypedAction<DeleteLinkData>,
    _original_action: TypedAction<CreateLinkData>,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_storage_entry_by_type(
    _action: TypedAction<CreateLinkData>,
    _base_address: AnyLinkableHash,
    _target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_storage_entry_by_type(
    _action: TypedAction<DeleteLinkData>,
    _original_action: TypedAction<CreateLinkData>,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}
