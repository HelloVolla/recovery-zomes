pub mod storage_entry;
use shared_types::{MembraneProofEnvelope, Properties};
pub use storage_entry::*;
pub mod chunk;
pub use chunk::*;
pub mod private_signing_key;
pub use private_signing_key::*;
use hdi::prelude::*;
use ed25519_dalek::{Signature as DalekSignature, Verifier, VerifyingKey};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Chunk(Chunk),
    StorageEntry(StorageEntry),
    #[entry_type(visibility = "private")]
    PrivateSigningKeyEntry(PrivateSigningKeyEntry),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    ChunkToStorageEntries,
    StorageEntryUpdates,
}

/// Verifies an ed25519 signature using raw public key bytes.
///
/// # Arguments
/// * `signing_pub_key` - Raw 32-byte ed25519 public key
/// * `signature` - Holochain Signature (64-byte ed25519 signature)
/// * `data` - The data that was signed (will be serialized)
fn verify_ed25519_signature<T: Serialize + std::fmt::Debug>(
    signing_pub_key: &[u8],
    signature: &Signature,
    data: &T,
) -> ExternResult<bool> {
    // Ensure the public key is exactly 32 bytes
    if signing_pub_key.len() != 32 {
        return Ok(false);
    }

    // Parse the public key
    let pub_key_bytes: [u8; 32] = signing_pub_key.try_into().map_err(|_| {
        wasm_error!(WasmErrorInner::Guest("Invalid public key length".to_string()))
    })?;
    let verifying_key = VerifyingKey::from_bytes(&pub_key_bytes).map_err(|_| {
        wasm_error!(WasmErrorInner::Guest("Invalid ed25519 public key".to_string()))
    })?;

    // Parse the signature (Holochain Signature is 64 bytes)
    let sig_bytes: [u8; 64] = signature.0.clone().try_into().map_err(|_| {
        wasm_error!(WasmErrorInner::Guest("Invalid signature length".to_string()))
    })?;
    let dalek_sig = DalekSignature::from_bytes(&sig_bytes);

    // Serialize the data the same way Holochain does for signing
    let data_bytes = holochain_serialized_bytes::encode(data)
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Failed to serialize data for signature verification".to_string())))?;

    // Verify the signature
    Ok(verifying_key.verify(&data_bytes, &dalek_sig).is_ok())
}

fn check_agent(
    agent_pub_key: AgentPubKey,
    membrane_proof: Option<MembraneProof>,
) -> ExternResult<ValidateCallbackResult> {
    let info = dna_info()?;

    // Always validate provisioned cells
    if info.modifiers.properties.bytes().len() == 1 {
        return Ok(ValidateCallbackResult::Valid);
    }

    let props = Properties::try_from(info.modifiers.properties.clone())
        .map_err(|e| wasm_error!(e))?;

    // Progenitor is always valid
    if agent_pub_key == props.progenitor_pub_key {
        return Ok(ValidateCallbackResult::Valid);
    }

    match membrane_proof {
        None => {
            Ok(ValidateCallbackResult::Invalid(
                "Membrane proof must be provided".to_string(),
            ))
        }
        Some(serialized_proof) => {
            let envelope = MembraneProofEnvelope::try_from((*serialized_proof).clone())
                .map_err(|e| wasm_error!(e))?;

            if envelope.data.bundle_id != info.modifiers.network_seed {
                return Ok(ValidateCallbackResult::Invalid(
                    "Membrane proof is not for this conversation".to_string(),
                ));
            }

            if envelope.data.for_agent != agent_pub_key {
                return Ok(ValidateCallbackResult::Invalid(
                    "Membrane proof is not for this agent".to_string(),
                ));
            }

            // Verify signature using the key fetched from DHT
            if verify_ed25519_signature(&envelope.data.network_pub_key, &envelope.signature, &envelope.data)? {
                return Ok(ValidateCallbackResult::Valid);
            }

            Ok(ValidateCallbackResult::Invalid(
                "Membrane proof signature invalid".to_string(),
            ))
        }
    }
}

// Validation you perform during the genesis process. Nobody else on the network performs it, only you.
// There *is no* access to network calls in this callback
#[hdk_extern]
pub fn genesis_self_check(
    data: GenesisSelfCheckData,
) -> ExternResult<ValidateCallbackResult> {
    check_agent(data.agent_key, data.membrane_proof)
}

// Validation the network performs when you try to join, you can't perform this validation yourself as you are not a member yet.
// There *is* access to network calls in this function
pub fn validate_agent_joining(
    agent_pub_key: AgentPubKey,
    membrane_proof: &Option<MembraneProof>,
) -> ExternResult<ValidateCallbackResult> {
    check_agent(agent_pub_key, (*membrane_proof).clone())
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
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::Chunk(chunk) => {
                    validate_create_chunk(EntryCreationAction::Create(action), chunk)
                }
                EntryTypes::StorageEntry(storage_entry) => validate_create_storage_entry(
                    EntryCreationAction::Create(action),
                    storage_entry,
                ),
                EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
            },
            OpEntry::UpdateEntry {
                app_entry, action, ..
            } => match app_entry {
                EntryTypes::Chunk(chunk) => {
                    validate_create_chunk(EntryCreationAction::Update(action), chunk)
                }
                EntryTypes::StorageEntry(storage_entry) => validate_create_storage_entry(
                    EntryCreationAction::Update(action),
                    storage_entry,
                ),
                EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(update_entry) => match update_entry {
            OpUpdate::Entry { app_entry, action } => {
                let original_action = must_get_action(action.clone().original_action_address)?
                    .action()
                    .to_owned();
                let original_create_action = match EntryCreationAction::try_from(original_action) {
                    Ok(action) => action,
                    Err(e) => {
                        return Ok(ValidateCallbackResult::Invalid(format!(
                            "Expected to get EntryCreationAction from Action: {e:?}"
                        )));
                    }
                };
                match app_entry {
                    EntryTypes::StorageEntry(storage_entry) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_storage_entry =
                            match StorageEntry::try_from(original_app_entry) {
                                Ok(entry) => entry,
                                Err(e) => {
                                    return Ok(ValidateCallbackResult::Invalid(format!(
                                        "Expected to get StorageEntry from Record: {e:?}"
                                    )));
                                }
                            };
                        validate_update_storage_entry(
                            action,
                            storage_entry,
                            original_create_action,
                            original_storage_entry,
                        )
                    }
                    EntryTypes::Chunk(chunk) => {
                        let original_app_entry =
                            must_get_valid_record(action.clone().original_action_address)?;
                        let original_chunk = match Chunk::try_from(original_app_entry) {
                            Ok(entry) => entry,
                            Err(e) => {
                                return Ok(ValidateCallbackResult::Invalid(format!(
                                    "Expected to get Chunk from Record: {e:?}"
                                )));
                            }
                        };
                        validate_update_chunk(action, chunk, original_create_action, original_chunk)
                    }
                    EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
                }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDelete(delete_entry) => {
            let original_action_hash = delete_entry.clone().action.deletes_address;
            let original_record = must_get_valid_record(original_action_hash)?;
            let original_record_action = original_record.action().clone();
            let original_action = match EntryCreationAction::try_from(original_record_action) {
                Ok(action) => action,
                Err(e) => {
                    return Ok(ValidateCallbackResult::Invalid(format!(
                        "Expected to get EntryCreationAction from Action: {e:?}"
                    )));
                }
            };
            let app_entry_type = match original_action.entry_type() {
                EntryType::App(app_entry_type) => app_entry_type,
                _ => {
                    return Ok(ValidateCallbackResult::Valid);
                }
            };
            let entry = match original_record.entry().as_option() {
                Some(entry) => entry,
                None => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original record for a delete must contain an entry".to_string(),
                    ));
                }
            };
            let original_app_entry = match EntryTypes::deserialize_from_type(
                app_entry_type.zome_index,
                app_entry_type.entry_index,
                entry,
            )? {
                Some(app_entry) => app_entry,
                None => {
                    return Ok(ValidateCallbackResult::Invalid(
                        "Original app entry must be one of the defined entry types for this zome"
                            .to_string(),
                    ));
                }
            };
            match original_app_entry {
                EntryTypes::StorageEntry(original_storage_entry) => validate_delete_storage_entry(
                    delete_entry.clone().action,
                    original_action,
                    original_storage_entry,
                ),
                EntryTypes::Chunk(original_chunk) => validate_delete_chunk(
                    delete_entry.clone().action,
                    original_action,
                    original_chunk,
                ),
                EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
            }
        }
        FlatOp::RegisterCreateLink {
            link_type,
            base_address,
            target_address,
            tag,
            action,
        } => match link_type {
            LinkTypes::ChunkToStorageEntries => validate_create_link_chunk_to_storage_entries(
                action,
                base_address,
                target_address,
                tag,
            ),
            LinkTypes::StorageEntryUpdates => validate_create_link_storage_entry_updates(
                action,
                base_address,
                target_address,
                tag,
            ),
        },
        FlatOp::RegisterDeleteLink {
            link_type,
            base_address,
            target_address,
            tag,
            original_action,
            action,
        } => match link_type {
            LinkTypes::ChunkToStorageEntries => validate_delete_link_chunk_to_storage_entries(
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
            LinkTypes::StorageEntryUpdates => validate_delete_link_storage_entry_updates(
                action,
                original_action,
                base_address,
                target_address,
                tag,
            ),
        },
        FlatOp::StoreRecord(store_record) => {
            match store_record {
                // Complementary validation to the `StoreEntry` Op, in which the record itself is validated
                // If you want to optimize performance, you can remove the validation for an entry type here and keep it in `StoreEntry`
                // Notice that doing so will cause `must_get_valid_record` for this record to return a valid record even if the `StoreEntry` validation failed
                OpRecord::CreateEntry { app_entry, action } => match app_entry {
                    EntryTypes::Chunk(chunk) => {
                        validate_create_chunk(EntryCreationAction::Create(action), chunk)
                    }
                    EntryTypes::StorageEntry(storage_entry) => validate_create_storage_entry(
                        EntryCreationAction::Create(action),
                        storage_entry,
                    ),
                    EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
                },
                // Complementary validation to the `RegisterUpdate` Op, in which the record itself is validated
                // If you want to optimize performance, you can remove the validation for an entry type here and keep it in `StoreEntry` and in `RegisterUpdate`
                // Notice that doing so will cause `must_get_valid_record` for this record to return a valid record even if the other validations failed
                OpRecord::UpdateEntry {
                    original_action_hash,
                    app_entry,
                    action,
                    ..
                } => {
                    let original_record = must_get_valid_record(original_action_hash)?;
                    let original_action = original_record.action().clone();
                    let original_action = match original_action {
                        Action::Create(create) => EntryCreationAction::Create(create),
                        Action::Update(update) => EntryCreationAction::Update(update),
                        _ => {
                            return Ok(ValidateCallbackResult::Invalid(
                                "Original action for an update must be a Create or Update action"
                                    .to_string(),
                            ));
                        }
                    };
                    match app_entry {
                        EntryTypes::Chunk(chunk) => {
                            let result = validate_create_chunk(
                                EntryCreationAction::Update(action.clone()),
                                chunk.clone(),
                            )?;
                            if let ValidateCallbackResult::Valid = result {
                                let original_chunk: Option<Chunk> = original_record
                                    .entry()
                                    .to_app_option()
                                    .map_err(|e| wasm_error!(e))?;
                                let original_chunk = match original_chunk {
                                    Some(chunk) => chunk,
                                    None => {
                                        return Ok(
                                            ValidateCallbackResult::Invalid(
                                                "The updated entry type must be the same as the original entry type"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                };
                                validate_update_chunk(
                                    action,
                                    chunk,
                                    original_action,
                                    original_chunk,
                                )
                            } else {
                                Ok(result)
                            }
                        }
                        EntryTypes::StorageEntry(storage_entry) => {
                            let result = validate_create_storage_entry(
                                EntryCreationAction::Update(action.clone()),
                                storage_entry.clone(),
                            )?;
                            if let ValidateCallbackResult::Valid = result {
                                let original_storage_entry: Option<StorageEntry> = original_record
                                    .entry()
                                    .to_app_option()
                                    .map_err(|e| wasm_error!(e))?;
                                let original_storage_entry = match original_storage_entry {
                                    Some(storage_entry) => storage_entry,
                                    None => {
                                        return Ok(
                                            ValidateCallbackResult::Invalid(
                                                "The updated entry type must be the same as the original entry type"
                                                    .to_string(),
                                            ),
                                        );
                                    }
                                };
                                validate_update_storage_entry(
                                    action,
                                    storage_entry,
                                    original_action,
                                    original_storage_entry,
                                )
                            } else {
                                Ok(result)
                            }
                        }
                        EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
                    }
                }
                // Complementary validation to the `RegisterDelete` Op, in which the record itself is validated
                // If you want to optimize performance, you can remove the validation for an entry type here and keep it in `RegisterDelete`
                // Notice that doing so will cause `must_get_valid_record` for this record to return a valid record even if the `RegisterDelete` validation failed
                OpRecord::DeleteEntry {
                    original_action_hash,
                    action,
                    ..
                } => {
                    let original_record = must_get_valid_record(original_action_hash)?;
                    let original_action = original_record.action().clone();
                    let original_action = match original_action {
                        Action::Create(create) => EntryCreationAction::Create(create),
                        Action::Update(update) => EntryCreationAction::Update(update),
                        _ => {
                            return Ok(ValidateCallbackResult::Invalid(
                                "Original action for a delete must be a Create or Update action"
                                    .to_string(),
                            ));
                        }
                    };
                    let app_entry_type = match original_action.entry_type() {
                        EntryType::App(app_entry_type) => app_entry_type,
                        _ => {
                            return Ok(ValidateCallbackResult::Valid);
                        }
                    };
                    let entry = match original_record.entry().as_option() {
                        Some(entry) => entry,
                        None => {
                            return Ok(ValidateCallbackResult::Invalid(
                                "Original record for a delete must contain an entry".to_string(),
                            ));
                        }
                    };
                    let original_app_entry = match EntryTypes::deserialize_from_type(
                        app_entry_type.zome_index,
                        app_entry_type.entry_index,
                        entry,
                    )? {
                        Some(app_entry) => app_entry,
                        None => {
                            return Ok(
                                ValidateCallbackResult::Invalid(
                                    "Original app entry must be one of the defined entry types for this zome"
                                        .to_string(),
                                ),
                            );
                        }
                    };
                    match original_app_entry {
                        EntryTypes::Chunk(original_chunk) => {
                            validate_delete_chunk(action, original_action, original_chunk)
                        }
                        EntryTypes::StorageEntry(original_storage_entry) => {
                            validate_delete_storage_entry(
                                action,
                                original_action,
                                original_storage_entry,
                            )
                        }
                        EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
                    }
                }
                // Complementary validation to the `RegisterCreateLink` Op, in which the record itself is validated
                // If you want to optimize performance, you can remove the validation for an entry type here and keep it in `RegisterCreateLink`
                // Notice that doing so will cause `must_get_valid_record` for this record to return a valid record even if the `RegisterCreateLink` validation failed
                OpRecord::CreateLink {
                    base_address,
                    target_address,
                    tag,
                    link_type,
                    action,
                } => match link_type {
                    LinkTypes::ChunkToStorageEntries => {
                        validate_create_link_chunk_to_storage_entries(
                            action,
                            base_address,
                            target_address,
                            tag,
                        )
                    }
                    LinkTypes::StorageEntryUpdates => validate_create_link_storage_entry_updates(
                        action,
                        base_address,
                        target_address,
                        tag,
                    ),
                },
                // Complementary validation to the `RegisterDeleteLink` Op, in which the record itself is validated
                // If you want to optimize performance, you can remove the validation for an entry type here and keep it in `RegisterDeleteLink`
                // Notice that doing so will cause `must_get_valid_record` for this record to return a valid record even if the `RegisterDeleteLink` validation failed
                OpRecord::DeleteLink {
                    original_action_hash,
                    base_address,
                    action,
                } => {
                    let record = must_get_valid_record(original_action_hash)?;
                    let create_link = match record.action() {
                        Action::CreateLink(create_link) => create_link.clone(),
                        _ => {
                            return Ok(ValidateCallbackResult::Invalid(
                                "The action that a DeleteLink deletes must be a CreateLink"
                                    .to_string(),
                            ));
                        }
                    };
                    let link_type = match LinkTypes::from_type(
                        create_link.zome_index,
                        create_link.link_type,
                    )? {
                        Some(lt) => lt,
                        None => {
                            return Ok(ValidateCallbackResult::Valid);
                        }
                    };
                    match link_type {
                        LinkTypes::ChunkToStorageEntries => {
                            validate_delete_link_chunk_to_storage_entries(
                                action,
                                create_link.clone(),
                                base_address,
                                create_link.target_address,
                                create_link.tag,
                            )
                        }
                        LinkTypes::StorageEntryUpdates => {
                            validate_delete_link_storage_entry_updates(
                                action,
                                create_link.clone(),
                                base_address,
                                create_link.target_address,
                                create_link.tag,
                            )
                        }
                    }
                }
                OpRecord::CreatePrivateEntry { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::UpdatePrivateEntry { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::CreateCapClaim { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::CreateCapGrant { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::UpdateCapClaim { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::UpdateCapGrant { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::Dna { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::OpenChain { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::CloseChain { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::InitZomesComplete { .. } => Ok(ValidateCallbackResult::Valid),
                _ => Ok(ValidateCallbackResult::Valid),
            }
        }
        FlatOp::RegisterAgentActivity(agent_activity) => match agent_activity {
            OpActivity::CreateAgent { agent, action } => {
                let previous_action = must_get_action(action.prev_action)?;
                match previous_action.action() {
                        Action::AgentValidationPkg(
                            AgentValidationPkg { membrane_proof, .. },
                        ) => validate_agent_joining(agent, membrane_proof),
                        _ => {
                            Ok(
                                ValidateCallbackResult::Invalid(
                                    "The previous action for a `CreateAgent` action must be an `AgentValidationPkg`"
                                        .to_string(),
                                ),
                            )
                        }
                    }
            }
            _ => Ok(ValidateCallbackResult::Valid),
        },
    }
}
