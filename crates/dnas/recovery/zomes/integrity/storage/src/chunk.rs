use hdi::prelude::*;

#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct Chunk(pub SerializedBytes);

pub fn validate_create_chunk(
    _action: EntryCreationAction,
    _chunk: Chunk,
) -> ExternResult<ValidateCallbackResult> {
    let chunk = _chunk.0;
    if chunk.bytes().is_empty() {
        return Ok(ValidateCallbackResult::Invalid(String::from(
            "Chunk cannot be empty",
        )));
    }

    Ok(ValidateCallbackResult::Valid)
}

pub fn validate_update_chunk(
    _action: Update,
    _chunk: Chunk,
    _original_action: EntryCreationAction,
    _original_chunk: Chunk,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Invalid(
        "Chunks cannot be updated".to_string(),
    ))
}

pub fn validate_delete_chunk(
    _action: Delete,
    _original_action: EntryCreationAction,
    _original_chunk: Chunk,
) -> ExternResult<ValidateCallbackResult> {
    Ok(ValidateCallbackResult::Valid)
}
