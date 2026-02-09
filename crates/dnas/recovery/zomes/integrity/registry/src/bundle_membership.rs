use hdi::prelude::*;

/// Represents an agent's membership in a storage bundle.
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct BundleMembership {
    /// The network seed of the bundle this membership is for
    pub network_seed: String,
    /// The agent who is a member of this bundle
    pub member_agent: AgentPubKey,
    /// Timestamp when the agent was added to the bundle
    pub added_at: Timestamp,
    /// Status of the membership
    pub status: MembershipStatus,
}

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize, Debug)]
pub enum MembershipStatus {
    /// Agent has been invited but hasn't cloned the bundle yet
    Pending,
    /// Agent has confirmed they've cloned the bundle
    Active,
    /// Membership has been revoked
    Revoked,
}

pub fn validate_create_bundle_membership(
    _action: EntryCreationAction,
    membership: BundleMembership,
) -> ExternResult<ValidateCallbackResult> {
    if membership.network_seed.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "network_seed cannot be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
