use hdk::prelude::*;
use storage_integrity::*;

fn get_storage_entry_anchor() -> ExternResult<AnyLinkableHash> {
    let path = Path::from("all_storage_entries");
    let entry_hash = path.path_entry_hash()?;
    Ok(entry_hash.into())
}

#[hdk_extern]
pub fn create_storage_entry(storage_entry: StorageEntry) -> ExternResult<Record> {
    let storage_entry_hash = create_entry(&EntryTypes::StorageEntry(storage_entry.clone()))?;
    for base in storage_entry.chunk_hashes.clone() {
        create_link(
            base,
            storage_entry_hash.clone(),
            LinkTypes::ChunkToStorageEntries,
            (),
        )?;
    }
    create_link(
        get_storage_entry_anchor()?,
        storage_entry_hash.clone(),
        LinkTypes::AllStorageEntries,
        (),
    )?;
    let record = get(storage_entry_hash.clone(), GetOptions::local())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly created StorageEntry".to_string())
    ))?;
    Ok(record)
}

#[hdk_extern]
pub fn get_latest_storage_entry(
    original_storage_entry_hash: ActionHash,
) -> ExternResult<Option<Record>> {
    let links = get_links(
        LinkQuery {
            base: original_storage_entry_hash.clone().into(),
            link_type: LinkTypes::StorageEntryUpdates.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;
    let latest_link = links
        .into_iter()
        .max_by(|link_a, link_b| link_a.timestamp.cmp(&link_b.timestamp));
    let latest_storage_entry_hash = match latest_link {
        Some(link) => {
            link.target
                .clone()
                .into_action_hash()
                .ok_or(wasm_error!(WasmErrorInner::Guest(
                    "No action hash associated with link".to_string()
                )))?
        }
        None => original_storage_entry_hash.clone(),
    };
    get(latest_storage_entry_hash, GetOptions::local())
}

#[hdk_extern]
pub fn get_original_storage_entry(
    original_storage_entry_hash: ActionHash,
) -> ExternResult<Option<Record>> {
    let Some(details) = get_details(original_storage_entry_hash, GetOptions::local())? else {
        return Ok(None);
    };
    match details {
        Details::Record(details) => Ok(Some(details.record)),
        _ => Err(wasm_error!(WasmErrorInner::Guest(
            "Malformed get details response".to_string()
        ))),
    }
}

#[hdk_extern]
pub fn get_all_revisions_for_storage_entry(
    original_storage_entry_hash: ActionHash,
) -> ExternResult<Vec<Record>> {
    let Some(original_record) = get_original_storage_entry(original_storage_entry_hash.clone())?
    else {
        return Ok(vec![]);
    };
    let links = get_links(
        LinkQuery {
            base: original_storage_entry_hash.clone().into(),
            link_type: LinkTypes::StorageEntryUpdates.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;
    let get_input: Vec<GetInput> = links
        .into_iter()
        .map(|link| {
            Ok(GetInput::new(
                link.target
                    .into_action_hash()
                    .ok_or(wasm_error!(WasmErrorInner::Guest(
                        "No action hash associated with link".to_string()
                    )))?
                    .into(),
                GetOptions::local(),
            ))
        })
        .collect::<ExternResult<Vec<GetInput>>>()?;
    let records = HDK.with(|hdk| hdk.borrow().get(get_input))?;
    let mut records: Vec<Record> = records.into_iter().flatten().collect();
    records.insert(0, original_record);
    Ok(records)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateStorageEntryInput {
    pub original_storage_entry_hash: ActionHash,
    pub previous_storage_entry_hash: ActionHash,
    pub updated_storage_entry: StorageEntry,
}

#[hdk_extern]
pub fn update_storage_entry(input: UpdateStorageEntryInput) -> ExternResult<Record> {
    let updated_storage_entry_hash = update_entry(
        input.previous_storage_entry_hash.clone(),
        &input.updated_storage_entry,
    )?;
    create_link(
        input.original_storage_entry_hash.clone(),
        updated_storage_entry_hash.clone(),
        LinkTypes::StorageEntryUpdates,
        (),
    )?;
    let record =
        get(updated_storage_entry_hash.clone(), GetOptions::local())?.ok_or(wasm_error!(
            WasmErrorInner::Guest("Could not find the newly updated StorageEntry".to_string())
        ))?;
    Ok(record)
}

#[hdk_extern]
pub fn delete_storage_entry(original_storage_entry_hash: ActionHash) -> ExternResult<ActionHash> {
    let details = get_details(original_storage_entry_hash.clone(), GetOptions::local())?.ok_or(
        wasm_error!(WasmErrorInner::Guest("StorageEntry not found".to_string())),
    )?;
    let record = match details {
        Details::Record(details) => Ok(details.record),
        _ => Err(wasm_error!(WasmErrorInner::Guest(
            "Malformed get details response".to_string()
        ))),
    }?;
    let entry = record
        .entry()
        .as_option()
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "StorageEntry record has no entry".to_string()
        )))?;
    let storage_entry = <StorageEntry>::try_from(entry)?;
    for base_address in storage_entry.chunk_hashes {
        let links = get_links(
            LinkQuery {
                base: base_address.clone().into(),
                link_type: LinkTypes::ChunkToStorageEntries.try_into_filter()?,
                tag_prefix: None,
                before: None,
                after: None,
                author: None,
            },
            GetStrategy::Local,
        )?;
        for link in links {
            if let Some(action_hash) = link.target.into_action_hash() {
                if action_hash == original_storage_entry_hash {
                    delete_link(link.create_link_hash, GetOptions::local())?;
                }
            }
        }
    }
    let anchor_links = get_links(
        LinkQuery {
            base: get_storage_entry_anchor()?,
            link_type: LinkTypes::AllStorageEntries.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;
    for link in anchor_links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if action_hash == original_storage_entry_hash {
                delete_link(link.create_link_hash, GetOptions::local())?;
            }
        }
    }
    delete_entry(original_storage_entry_hash)
}

#[hdk_extern]
pub fn get_all_deletes_for_storage_entry(
    original_storage_entry_hash: ActionHash,
) -> ExternResult<Option<Vec<SignedActionHashed>>> {
    let Some(details) = get_details(original_storage_entry_hash, GetOptions::local())? else {
        return Ok(None);
    };
    match details {
        Details::Entry(_) => Err(wasm_error!(WasmErrorInner::Guest(
            "Malformed details".into()
        ))),
        Details::Record(record_details) => Ok(Some(record_details.deletes)),
    }
}

#[hdk_extern]
pub fn get_oldest_delete_for_storage_entry(
    original_storage_entry_hash: ActionHash,
) -> ExternResult<Option<SignedActionHashed>> {
    let Some(mut deletes) = get_all_deletes_for_storage_entry(original_storage_entry_hash)? else {
        return Ok(None);
    };
    deletes.sort_by(|delete_a, delete_b| {
        delete_a
            .action()
            .timestamp()
            .cmp(&delete_b.action().timestamp())
    });
    Ok(deletes.first().cloned())
}

#[hdk_extern]
pub fn get_all_storage_entries(_: ()) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery {
            base: get_storage_entry_anchor()?,
            link_type: LinkTypes::AllStorageEntries.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;
    let mut records = Vec::with_capacity(links.len());
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::local())? {
                records.push(record);
            }
        }
    }
    Ok(records)
}

#[hdk_extern]
pub fn get_storage_entries_for_chunk(chunk_hash: EntryHash) -> ExternResult<Vec<Link>> {
    get_links(
        LinkQuery {
            base: chunk_hash.clone().into(),
            link_type: LinkTypes::ChunkToStorageEntries.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )
}

#[hdk_extern]
pub fn get_deleted_storage_entries_for_chunk(
    chunk_hash: EntryHash,
) -> ExternResult<Vec<(SignedActionHashed, Vec<SignedActionHashed>)>> {
    let details = get_links_details(
        LinkQuery {
            base: chunk_hash.clone().into(),
            link_type: LinkTypes::ChunkToStorageEntries.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;
    Ok(details
        .into_inner()
        .into_iter()
        .filter(|(_link, deletes)| !deletes.is_empty())
        .collect())
}

/// Retrieves a complete file from the DHT by fetching the StorageEntry and all its chunks.
///
/// This function:
/// 1. Fetches the latest version of the StorageEntry using the original action hash
/// 2. Retrieves all chunks referenced in the StorageEntry
/// 3. Reassembles the chunks in order into the complete file data
///
/// # Arguments
///
/// * `storage_entry_hash` - The action hash of the original StorageEntry
///
/// # Returns
///
/// The complete file data as a `Vec<u8>`, or `None` if the StorageEntry is not found.
/// Returns an error if any chunk cannot be retrieved.
#[hdk_extern]
pub fn get_file(storage_entry_hash: ActionHash) -> ExternResult<Option<Vec<u8>>> {
    // Get the latest storage entry
    let Some(record) = get_latest_storage_entry(storage_entry_hash)? else {
        return Ok(None);
    };

    let storage_entry: StorageEntry = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Record has no entry".to_string()
        )))?;

    // Collect all chunk data in order
    let mut data = Vec::with_capacity(storage_entry.size);
    for chunk_hash in storage_entry.chunk_hashes {
        let chunk = crate::chunk::get_chunk_from_entry_hash(chunk_hash)?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Chunk not found".to_string()
            )))?;
        data.extend(chunk.0.bytes());
    }

    Ok(Some(data))
}

/// Finds a storage entry by its label.
///
/// This function queries all storage entries via the anchor link pattern
/// and returns the first one matching the given label.
///
/// # Arguments
///
/// * `label` - The label to search for
///
/// # Returns
///
/// The matching StorageEntry record, or `None` if no entry with that label exists.
#[hdk_extern]
pub fn get_storage_entry_by_label(label: String) -> ExternResult<Option<Record>> {
    let all_entries = get_all_storage_entries(())?;

    for record in all_entries {
        if let Some(entry) = record.entry().to_app_option::<StorageEntry>().ok().flatten() {
            if entry.label == label {
                return Ok(Some(record));
            }
        }
    }

    Ok(None)
}

/// Retrieves a complete file from the DHT by searching for a StorageEntry with the given label.
///
/// This function:
/// 1. Searches all storage entries for one with the matching label
/// 2. Retrieves all chunks referenced in the StorageEntry
/// 3. Reassembles the chunks in order into the complete file data
///
/// # Arguments
///
/// * `label` - The label of the file to retrieve
///
/// # Returns
///
/// The complete file data as a `Vec<u8>`, or `None` if no entry with that label exists.
/// Returns an error if any chunk cannot be retrieved.
#[hdk_extern]
pub fn get_file_by_label(label: String) -> ExternResult<Option<Vec<u8>>> {
    let Some(record) = get_storage_entry_by_label(label)? else {
        return Ok(None);
    };

    let storage_entry: StorageEntry = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Record has no entry".to_string()
        )))?;

    // Collect all chunk data in order
    let mut data = Vec::with_capacity(storage_entry.size);
    for chunk_hash in storage_entry.chunk_hashes {
        let chunk = crate::chunk::get_chunk_from_entry_hash(chunk_hash)?
            .ok_or(wasm_error!(WasmErrorInner::Guest(
                "Chunk not found".to_string()
            )))?;
        data.extend(chunk.0.bytes());
    }

    Ok(Some(data))
}
