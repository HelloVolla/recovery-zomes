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
    let seed_array = get_signing_key_seed()?;
    input.get_signed_membrane_proof_bytes(&seed_array)
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

/// Helper to retrieve the 32-byte signing key seed from the source chain.
fn get_signing_key_seed() -> ExternResult<[u8; 32]> {
    let private_key_entry = get_private_signing_key(())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Private signing key not found on source chain".to_string())))?;

    let seed_bytes: Vec<u8> = private_key_entry.seed.bytes().to_vec();
    if seed_bytes.len() != 32 {
        return Err(wasm_error!(WasmErrorInner::Guest("Private signing key seed must be 32 bytes".to_string())));
    }
    let seed_array: [u8; 32] = seed_bytes.try_into()
        .map_err(|_| wasm_error!(WasmErrorInner::Guest("Failed to convert seed to array".to_string())))?;
    Ok(seed_array)
}

/// Wrapper around `InviteCode::generate` that gathers cell info from the HDK
/// and delegates to the pure function in shared-types.
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
    // Gather cell info from the HDK
    let dna_info = dna_info()?;
    let network_seed = dna_info.modifiers.network_seed.clone();
    let dna_hash = dna_info.hash.get_raw_39().to_vec();

    let properties = Properties::try_from(dna_info.modifiers.properties.clone())
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(format!("Failed to deserialize DNA properties: {:?}", e))))?;
    let network_pub_key = properties.network_pub_key.clone();
    let progenitor_pub_key = properties.progenitor_pub_key.clone().get_raw_39().to_vec();

    info!(
        "generate_invite_code: progenitor_pub_key raw39={:?}, network_pub_key_len={}",
        progenitor_pub_key,
        network_pub_key.len()
    );

    // Get the signing key seed from the source chain, only if needed
    let seed_array = if input.for_agent_pub_key.is_some() {
        Some(get_signing_key_seed()?)
    } else {
        None
    };

    // Delegate to the pure function in shared-types
    InviteCode::generate(
        network_seed,
        progenitor_pub_key,
        network_pub_key,
        dna_hash,
        seed_array.as_ref(),
        input.for_agent_pub_key,
        input.display_name,
    )
}
