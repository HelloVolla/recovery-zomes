use hdi::prelude::*;

/// Status of an invite code
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize, Debug)]
pub enum InviteCodeStatus {
    /// Invite is pending - agent hasn't joined yet
    Pending,
    /// Invite was fulfilled - agent joined the cell
    Fulfilled,
    /// Invite was revoked/cancelled
    Revoked,
    /// Failed to join the cell
    Failed(InviteCodeFailureReason),
}

/// Failure reason for an invite code
#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize, Debug)]
pub enum InviteCodeFailureReason {
    /// Unknown failure reason
    Unknown,
    /// The invite code was for another agent
    ForAnotherAgent,
    /// The agent does not have enough storage space
    NotEnoughStorage,
}

/// Represents an invite code for a specific agent to join a cell.
/// These are committed to the DHT so invited agents can scan for their pending invites.
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct InviteCode {
    /// The invite code string (unique identifier)
    pub code: String,
    /// The agent this invite code is for
    pub target_agent: AgentPubKey,
    /// The network seed this invite is for
    pub network_seed: String,
    /// Status of the invite
    pub status: InviteCodeStatus,
}

pub fn validate_create_invite_code(
    _action: EntryCreationAction,
    invite_code: InviteCode,
) -> ExternResult<ValidateCallbackResult> {
    if invite_code.code.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "invite code cannot be empty".to_string(),
        ));
    }
    if invite_code.network_seed.is_empty() {
        return Ok(ValidateCallbackResult::Invalid(
            "network_seed cannot be empty".to_string(),
        ));
    }
    Ok(ValidateCallbackResult::Valid)
}
