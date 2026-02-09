use hdi::prelude::*;

/// Information about a storage bundle created by the progenitor.
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct StorageBundleInfo {
    /// Unique identifier for the bundle (typically the network seed of the storage DNA)
    pub network_seed: String,
    /// Progenitor agent pubkey
    pub progenitor: AgentPubKey,
    /// Network pubkey of the storage DNA
    pub network_pub_key: Vec<u8>,
}

pub fn validate_create_storage_bundle_info(
    _action: EntryCreationAction,
    bundle_info: StorageBundleInfo,
) -> ExternResult<ValidateCallbackResult> {
    if bundle_info.network_seed.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "network_seed cannot be empty".to_string(),
        ));
    }
    if bundle_info.network_pub_key.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "network_pub_key cannot be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
