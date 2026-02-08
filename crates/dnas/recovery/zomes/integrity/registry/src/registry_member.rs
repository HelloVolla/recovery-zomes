use hdi::prelude::*;
use shared_types::MemberStatus;

/// Represents an agent's membership in the registry (for backup hosting).
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct RegistryMember {
    /// The agent public key of the member
    pub agent: AgentPubKey,
    /// Optional display name for the member
    pub display_name: Option<String>,
    /// Status of the membership
    pub status: MemberStatus,
}
