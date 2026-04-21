pub mod private_signing_key;
pub use private_signing_key::*;
use integrity_types::{MembraneProofEnvelope, Properties};
use hdi::prelude::*;
use ed25519_dalek::{Signature as DalekSignature, Verifier, VerifyingKey};

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    #[entry_type(visibility = "private")]
    PrivateSigningKeyEntry(PrivateSigningKeyEntry),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    _Placeholder,
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

    info!(
        "check_agent: agent_pub_key={:?}, props.progenitor_pub_key={:?}",
        agent_pub_key,
        props.progenitor_pub_key
    );

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

            info!(
                "check_agent: membrane_proof network_seed={}, expected={}, for_agent={:?}, expected_agent={:?}, network_pub_key_len={}",
                envelope.data.network_seed,
                info.modifiers.network_seed,
                envelope.data.for_agent,
                agent_pub_key,
                envelope.data.network_pub_key.len()
            );

            if envelope.data.network_seed != info.modifiers.network_seed {
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

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, .. } => match app_entry {
                EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
            },
            OpEntry::UpdateEntry { app_entry, .. } => match app_entry {
                EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterUpdate(update_entry) => match update_entry {
            OpUpdate::Entry { app_entry, .. } => match app_entry {
                EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        FlatOp::RegisterDelete(_) => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterCreateLink { .. } => Ok(ValidateCallbackResult::Valid),
        FlatOp::RegisterDeleteLink { .. } => Ok(ValidateCallbackResult::Valid),
        FlatOp::StoreRecord(store_record) => {
            match store_record {
                OpRecord::CreateEntry { app_entry, .. } => match app_entry {
                    EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
                },
                OpRecord::UpdateEntry { app_entry, .. } => match app_entry {
                    EntryTypes::PrivateSigningKeyEntry(_) => Ok(ValidateCallbackResult::Valid),
                },
                OpRecord::DeleteEntry { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::CreateLink { .. } => Ok(ValidateCallbackResult::Valid),
                OpRecord::DeleteLink { .. } => Ok(ValidateCallbackResult::Valid),
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
