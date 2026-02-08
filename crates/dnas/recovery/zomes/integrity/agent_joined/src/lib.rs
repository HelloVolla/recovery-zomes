//! Agent Joined Integrity Zome
//!
//! A minimal zome that defines an entry type for signaling when an agent has joined
//! a network. When an agent commits this entry to their source chain, it signals
//! that they have successfully joined the cell.

use hdi::prelude::*;

/// Entry that is committed to the source chain when an agent joins a network.
/// The presence of this entry signals that the agent has successfully joined.
#[hdk_entry_helper]
#[derive(Clone, PartialEq)]
pub struct AgentJoined {
    /// The agent's public key
    pub agent: AgentPubKey,
    /// Timestamp when the agent joined
    pub joined_at: Timestamp,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    AgentJoined(AgentJoined),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    AllAgents,
}

#[hdk_extern]
pub fn validate(_op: Op) -> ExternResult<ValidateCallbackResult> {
    // All AgentJoined entries are valid
    Ok(ValidateCallbackResult::Valid)
}
