pub mod storage_entry;
pub use storage_entry::*;
pub mod chunk;
pub use chunk::*;
use hdi::prelude::*;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Chunk(Chunk),
    StorageEntry(StorageEntry),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    ChunkToStorageEntries,
    StorageEntryUpdates,
    AllStorageEntries,
    StorageEntryByType,
}

// This is the unified validation callback for all entries and link types in this integrity zome
// Below is a match template for all of the variants of `DHT Ops` and entry and link types
// Holochain has already performed the following validation for you:
// - The action signature matches on the hash of its content and is signed by its author
// - The previous action exists, has a lower timestamp than the new action, and incremented sequence number
// - The previous action author is the same as the new action author
// - The timestamp of each action is after the DNA's origin time
// - AgentActivity authorities check that the agent hasn't forked their chain
// - The entry hash in the action matches the entry content
// - The entry type in the action matches the entry content
// - The entry size doesn't exceed the maximum entry size (currently 4MB)
// - Private entry types are not included in the Op content, and public entry types are
// - If the `Op` is an update or a delete, the original action exists and is a `Create` or `Update` action
// - If the `Op` is an update, the original entry exists and is of the same type as the new one
// - If the `Op` is a delete link, the original action exists and is a `CreateLink` action
// - Link tags don't exceed the maximum tag size (currently 1KB)
// - Countersigned entries include an action from each required signer
// You can read more about validation here: https://docs.rs/hdi/latest/hdi/index.html#data-validation
#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::CreateEntry(op_entry) => match op_entry {
            OpEntry::CreateEntry { app_entry, action } => {
                let action = TypedAction {
                    header: action.header,
                    data: EntryCreationData::Create(action.data),
                };
                match app_entry {
                    EntryTypes::Chunk(chunk) => validate_create_chunk(action, chunk),
                    EntryTypes::StorageEntry(storage_entry) => {
                        validate_create_storage_entry(action, storage_entry)
                    }
                }
            }
            OpEntry::UpdateEntry { app_entry, action } => {
                let action = TypedAction {
                    header: action.header,
                    data: EntryCreationData::Update(action.data),
                };
                match app_entry {
                    EntryTypes::Chunk(chunk) => validate_create_chunk(action, chunk),
                    EntryTypes::StorageEntry(storage_entry) => {
                        validate_create_storage_entry(action, storage_entry)
                    }
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::Update(update_entry) => match update_entry {
            OpUpdate::Entry { app_entry, .. } => match app_entry {
                EntryTypes::Chunk(_) => {
                    Ok(ValidateCallbackResult::Invalid(
                        "Chunks cannot be updated".to_string(),
                    ))
                }
                _ => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::Delete(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::Link(OpLink::CreateLink { link_type, action }) => {
            let base_address = action.base_address.clone();
            let target_address = action.target_address.clone();
            let tag = action.tag.clone();
            match link_type {
                LinkTypes::ChunkToStorageEntries => validate_create_link_chunk_to_storage_entries(
                    action, base_address, target_address, tag,
                ),
                LinkTypes::StorageEntryUpdates => validate_create_link_storage_entry_updates(
                    action, base_address, target_address, tag,
                ),
                LinkTypes::AllStorageEntries => validate_create_link_all_storage_entries(
                    action, base_address, target_address, tag,
                ),
                LinkTypes::StorageEntryByType => validate_create_link_storage_entry_by_type(
                    action, base_address, target_address, tag,
                ),
            }
        }
        FlatOp::Link(OpLink::DeleteLink {
            original_action,
            link_type,
            action,
        }) => {
            let base_address = original_action.base_address.clone();
            let target_address = original_action.target_address.clone();
            let tag = original_action.tag.clone();
            match link_type {
                LinkTypes::ChunkToStorageEntries => validate_delete_link_chunk_to_storage_entries(
                    action, original_action, base_address, target_address, tag,
                ),
                LinkTypes::StorageEntryUpdates => validate_delete_link_storage_entry_updates(
                    action, original_action, base_address, target_address, tag,
                ),
                LinkTypes::AllStorageEntries => validate_delete_link_all_storage_entries(
                    action, original_action, base_address, target_address, tag,
                ),
                LinkTypes::StorageEntryByType => validate_delete_link_storage_entry_by_type(
                    action, original_action, base_address, target_address, tag,
                ),
            }
        }
        FlatOp::CreateRecord(store_record) => {
            match store_record {
                OpRecord::CreateEntry { app_entry, action } => {
                    let action = TypedAction {
                        header: action.header,
                        data: EntryCreationData::Create(action.data),
                    };
                    match app_entry {
                        EntryTypes::Chunk(chunk) => validate_create_chunk(action, chunk),
                        EntryTypes::StorageEntry(storage_entry) => {
                            validate_create_storage_entry(action, storage_entry)
                        }
                    }
                }
                OpRecord::UpdateEntry { app_entry, .. } => match app_entry {
                    EntryTypes::Chunk(_) => {
                        Ok(ValidateCallbackResult::Invalid(
                            "Chunks cannot be updated".to_string(),
                        ))
                    }
                    _ => Ok(ValidateCallbackResult::Valid),
                },
                OpRecord::DeleteEntry { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::CreateLink { link_type, action } => {
                    let base_address = action.base_address.clone();
                    let target_address = action.target_address.clone();
                    let tag = action.tag.clone();
                    match link_type {
                        LinkTypes::ChunkToStorageEntries => {
                            validate_create_link_chunk_to_storage_entries(
                                action, base_address, target_address, tag,
                            )
                        }
                        LinkTypes::StorageEntryUpdates => {
                            validate_create_link_storage_entry_updates(
                                action, base_address, target_address, tag,
                            )
                        }
                        LinkTypes::AllStorageEntries => {
                            validate_create_link_all_storage_entries(
                                action, base_address, target_address, tag,
                            )
                        }
                        LinkTypes::StorageEntryByType => {
                            validate_create_link_storage_entry_by_type(
                                action, base_address, target_address, tag,
                            )
                        }
                    }
                }
                OpRecord::DeleteLink { .. } => Ok(ValidateCallbackResult::Valid),
                _ => Ok(ValidateCallbackResult::Valid),
            }
        }
        FlatOp::AgentActivity(_agent_activity) => Ok(ValidateCallbackResult::Valid),
    }
}
