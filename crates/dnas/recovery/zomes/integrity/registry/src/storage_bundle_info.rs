use hdi::prelude::*;

/// Information about a storage bundle created by the progenitor.
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct StorageBundleInfo {
    /// Unique identifier for the bundle (typically the network seed of the storage DNA)
    pub bundle_id: String,
    /// Human-readable name for the bundle
    pub name: String,
    /// Timestamp when the bundle was created
    pub created_at: Timestamp,
    /// Optional description of what this bundle contains
    pub description: Option<String>,
}

pub fn validate_create_storage_bundle_info(
    _action: EntryCreationAction,
    bundle_info: StorageBundleInfo,
) -> ExternResult<ValidateCallbackResult> {
    if bundle_info.bundle_id.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "bundle_id cannot be empty".to_string(),
        ));
    }
    if bundle_info.name.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "name cannot be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
