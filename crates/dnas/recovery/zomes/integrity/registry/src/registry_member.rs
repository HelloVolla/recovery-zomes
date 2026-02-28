use hdi::prelude::*;

/// Represents an agent's membership in the registry (for backup hosting).
///
/// The member's *status* is tracked separately via `MemberStatusEntry` entries
/// linked from the same agent pub key, so that it can be updated independently
/// by a different peer without needing to read or modify this entry.
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct RegistryMember {
    /// The agent public key of the member
    pub agent: AgentPubKey,
    /// Optional display name for the member
    pub display_name: Option<String>,
}
