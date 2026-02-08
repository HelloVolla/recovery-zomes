use hdi::prelude::*;

/// Main storage entry containing metadata and references to data chunks
/// All actual data is stored in DataChunk entries, referenced by EntryHash
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct StorageEntry {
    /// Human-readable label for this data
    pub label: String,
    /// Content type
    pub content_type: String,
    /// Total size in bytes
    pub size: usize,
    /// Timestamp when the data was collected
    pub timestamp: Timestamp,
    /// Optional custom metadata as JSON
    pub custom_metadata: Option<SerializedBytes>,
    /// Ordered list of chunk hashes that make up the complete data
    pub chunk_hashes: Vec<EntryHash>,
}

pub fn validate_create_storage_entry(
    _action: EntryCreationAction,
    storage_entry: StorageEntry,
) -> ExternResult<ValidateCallbackResult> {
    for entry_hash in storage_entry.chunk_hashes.clone() {
        let entry = must_get_entry(entry_hash)?;
        let _chunk = crate::Chunk::try_from(entry)?;
    }
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_storage_entry(
    _action: Update,
    _storage_entry: StorageEntry,
    _original_action: EntryCreationAction,
    _original_storage_entry: StorageEntry,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_storage_entry(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original_storage_entry: StorageEntry,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_chunk_to_storage_entries(
    _action: CreateLink,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    let entry_hash = base_address
        .into_entry_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No entry hash associated with link".to_string()
        )))?;
    let entry = must_get_entry(entry_hash)?.content;
    let _chunk = crate::Chunk::try_from(entry)?;
    let action_hash =
        target_address
            .into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "No action hash associated with link".to_string()
            )))?;
    let record = must_get_valid_record(action_hash)?;
    let _storage_entry: crate::StorageEntry = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_chunk_to_storage_entries(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_create_link_storage_entry_updates(
    _action: CreateLink,
    base_address: AnyLinkableHash,
    target_address: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    let action_hash = base_address
        .into_action_hash()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "No action hash associated with link".to_string()
        )))?;
    let record = must_get_valid_record(action_hash)?;
    let _storage_entry: crate::StorageEntry = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    let action_hash =
        target_address
            .into_action_hash()
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "No action hash associated with link".to_string()
            )))?;
    let record = must_get_valid_record(action_hash)?;
    let _storage_entry: crate::StorageEntry = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Linked action must reference an entry".to_string()
        )))?;
    // TODO: add the appropriate validation rules
    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_delete_link_storage_entry_updates(
    _action: DeleteLink,
    _original_action: CreateLink,
    _base: AnyLinkableHash,
    _target: AnyLinkableHash,
    _tag: LinkTag,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "StorageEntryUpdates links cannot be deleted".to_string(),
    ))
}
