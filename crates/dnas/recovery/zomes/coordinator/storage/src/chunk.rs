use hdk::prelude::*;
use storage_integrity::*;

/// Create chunk from data
#[hdk_extern]
pub fn create_chunk_from_data(data: Vec<u8>) -> ExternResult<Record> {
    let chunk = Chunk(SerializedBytes::from(UnsafeBytes::from(data)));
    let chunk_hash = create_entry(&EntryTypes::Chunk(chunk.clone()))?;
    let record = get(chunk_hash.clone(), GetOptions::local())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly created Chunk".to_string())
    ))?;
    Ok(record)
}

#[hdk_extern]
pub fn create_chunk(chunk: Chunk) -> ExternResult<Record> {
    let chunk_hash = create_entry(&EntryTypes::Chunk(chunk.clone()))?;
    let record = get(chunk_hash.clone(), GetOptions::local())?.ok_or(wasm_error!(
        WasmErrorInner::Guest("Could not find the newly created Chunk".to_string())
    ))?;
    Ok(record)
}

#[hdk_extern]
pub fn get_chunk_from_entry_hash(entry_hash: EntryHash) -> ExternResult<Option<Chunk>> {
    let Some(details) = get_details(entry_hash, GetOptions::local())? else {
        return Ok(None);
    };
    match details {
        Details::Entry(details) => Ok(Some(
            Chunk::try_from(&details.entry).map_err(|_| {
                wasm_error!(WasmErrorInner::Guest("Malformed entry".to_string()))
            })?,
        )),
        _ => Err(wasm_error!(WasmErrorInner::Guest(
            "Malformed get details response".to_string()
        ))),
    }
}

#[hdk_extern]
pub fn get_chunk(chunk_hash: ActionHash) -> ExternResult<Option<Record>> {
    let Some(details) = get_details(chunk_hash, GetOptions::local())? else {
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
pub fn delete_chunk(original_chunk_hash: ActionHash) -> ExternResult<ActionHash> {
    delete_entry(original_chunk_hash)
}

#[hdk_extern]
pub fn get_all_deletes_for_chunk(
    original_chunk_hash: ActionHash,
) -> ExternResult<Option<Vec<SignedActionHashed>>> {
    let Some(details) = get_details(original_chunk_hash, GetOptions::local())? else {
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
pub fn get_oldest_delete_for_chunk(
    original_chunk_hash: ActionHash,
) -> ExternResult<Option<SignedActionHashed>> {
    let Some(mut deletes) = get_all_deletes_for_chunk(original_chunk_hash)? else {
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
