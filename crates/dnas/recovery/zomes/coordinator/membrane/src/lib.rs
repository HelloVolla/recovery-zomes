use hdk::prelude::*;
use shared_types::{GenerateInviteCodeInput, InviteCode, MembraneProofData, Properties};
use membrane_integrity::*;

// Called the first time a zome call is made to the cell containing this zome
#[hdk_extern]
pub fn init() -> ExternResult<InitCallbackResult> {
    // Grant unrestricted access to membrane proof generation for remote calls
    let mut functions = HashSet::new();
    functions.insert((zome_info()?.name, "generate_membrane_proof".into()));

    create_cap_grant(ZomeCallCapGrant {
        tag: "membrane_remote".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(functions),
    })?;

    Ok(InitCallbackResult::Pass)
}

#[hdk_extern]
pub fn generate_membrane_proof(input: MembraneProofData) -> ExternResult<SerializedBytes> {
    use ed25519_dalek::SigningKey;

    let private_key_entry = get_private_signing_key(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Private signing key not found on source chain".to_string())))?;

    let seed_bytes: Vec<u8> = private_key_entry.seed.bytes().to_vec();
    if seed_bytes.len() != 32 {
        return Err(wasm_error!(WasmErrorInner::Guest("Private signing key seed must be 32 bytes".to_string())));
    }
    let seed_array: [u8; 32] = seed_bytes.try_into()
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Failed to convert seed to array".to_string())))?;
    let signing_key = SigningKey::from_bytes(&seed_array);

    let proof = input.get_signed_membrane_proof_bytes(&signing_key)?;
    Ok(proof)
}

/// Commit the private signing key (seed) to the source chain as a private entry.
/// This should be called by the progenitor after creating the storage bundle.
/// The private key is stored locally and never published to the DHT.
#[hdk_extern]
pub fn commit_private_signing_key(seed: Vec<u8>) -> ExternResult<ActionHash> {
    use membrane_integrity::PrivateSigningKeyEntry;

    if seed.len() != 32 {
        return Err(wasm_error!(WasmErrorInner::Guest("Private signing key seed must be exactly 32 bytes".to_string())));
    }

    let entry = PrivateSigningKeyEntry {
        seed: SerializedBytes::try_from(UnsafeBytes::from(seed))
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to serialize private signing key: {:?}", e))))?,
    };

    create_entry(&EntryTypes::PrivateSigningKeyEntry(entry))
}

/// Retrieve the private signing key (seed) from the source chain.
#[hdk_extern]
pub fn get_private_signing_key(_: ()) -> ExternResult<Option<membrane_integrity::PrivateSigningKeyEntry>> {
    use membrane_integrity::PrivateSigningKeyEntry;

    // Query all entries on the local source chain and filter for PrivateSigningKeyEntry
    let storage_entry_type: EntryType = UnitEntryTypes::PrivateSigningKeyEntry.try_into()?;
    let filter = ChainQueryFilter::new().entry_type(storage_entry_type).include_entries(true);
    let records = query(filter)?;

    for record in records {
        if let Some(entry) = record.entry().as_option() {
            // Try to deserialize as PrivateSigningKeyEntry
            if let Ok(private_key) = PrivateSigningKeyEntry::try_from(entry.clone()) {
                return Ok(Some(private_key));
            }
        }
    }

    Ok(None)
}

/// Generates an invite code for another agent to join this cell's network.
///
/// This function creates an invite code that contains:
/// - The network seed identifying the storage bundle
/// - The progenitor's public key
/// - The network signing public key for signature verification
/// - An optional membrane proof containing the signature authorizing the specified agent to join
///   (generated using the private key stored on the source chain)
///
/// # Arguments
///
/// * `input` - GenerateInviteCodeInput containing an optional target agent public key.
///   If provided, a membrane proof will be generated and included in the invite code.
///
/// # Returns
///
/// An InviteCode instance containing all information needed to join this network.
#[hdk_extern]
pub fn generate_invite_code(input: GenerateInviteCodeInput) -> ExternResult<InviteCode> {
    // Get the DNA info to extract network seed and properties
    let dna_info = dna_info()?;
    let network_seed = dna_info.modifiers.network_seed.clone();

    // Get the current agent's public key (the progenitor calling this function)
    let agent_info = agent_info()?;
    let progenitor_pub_key: Vec<u8> = agent_info.agent_initial_pubkey.get_raw_39().to_vec();

    // Deserialize the properties to get the network_pub_key
    let properties = Properties::try_from(dna_info.modifiers.properties.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to deserialize DNA properties: {:?}", e))))?;

    let network_pub_key = properties.network_pub_key.clone();

    let mut membrane_proof_bytes = None;

    if let Some(for_agent_pub_key) = input.for_agent_pub_key {
        // Parse the target agent's public key
        let for_agent = AgentPubKey::try_from_raw_39(for_agent_pub_key)
            .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Invalid target agent pub key format: {:?}", e))))?;

        // Create membrane proof data
        let membrane_proof_data = shared_types::MembraneProofData {
            network_seed: network_seed.clone(),
            for_agent,
            network_pub_key: network_pub_key.clone(),
        };

        // Generate the membrane proof using the stored private signing key
        // If this fails, leave membrane_proof as None
        if let Ok(proof) = generate_membrane_proof(membrane_proof_data) {
            membrane_proof_bytes = Some(proof.bytes().to_vec());
        }
    }

    Ok(InviteCode {
        network_seed,
        progenitor_pub_key,
        network_pub_key,
        membrane_proof: membrane_proof_bytes,
    })
}
