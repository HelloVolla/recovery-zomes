use hdi::prelude::*;
use integrity_types::MemberStatus;

/// Represents an agent's membership in a storage bundle.
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct BundleMember {
    /// The agent who is a member of this bundle
    pub agent: AgentPubKey,
    /// The network seed of the bundle this member is in
    pub network_seed: String,
    /// Status of the membership
    pub status: MemberStatus,
}

pub fn validate_create_bundle_member(
    _action: EntryCreationAction,
    member: BundleMember,
) -> ExternResult<ValidateCallbackResult> {
    if member.network_seed.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "network_seed cannot be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
