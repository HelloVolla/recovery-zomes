use crate::*;

/// A registry member entry with the status included
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct RegistryMemberWithStatus {
    pub agent: AgentPubKey,
    pub display_name: Option<String>,
    pub status: MemberStatus,
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum MemberStatus {
    /// Agent has been invited with an invite code
    Invited(String),
    /// Agent has joined and is actively a member
    Active,
    /// Membership has been revoked
    Revoked,
    /// Membership has been rejected
    Rejected(RejectionReason),
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum RejectionReason {
    /// Unknown failure reason
    Unknown,
    /// The invite code was for another agent
    ForAnotherAgent,
    /// The agent does not have enough storage space
    NotEnoughStorage,
}

/// Input for inviting a registry member
#[derive(Serialize, Deserialize, Debug)]
pub struct InviteMemberInput {
    pub agent: AgentPubKey,
    pub display_name: Option<String>,
    pub invite_code_base64: String,
}

/// Input for setting a member's status (creates a new MemberStatusEntry)
#[derive(Serialize, Deserialize, Debug)]
pub struct SetMemberStatusInput {
    pub agent: AgentPubKey,
    pub new_status: MemberStatus,
}

/// Combined result of a member entry lookup and its most recent status.
///
/// Both fields are `Option` because a peer may have written only one of the
/// two entries by the time the query runs.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MemberAndStatus<M, S> {
    /// The RegistryMember entry together with its action hash, if found.
    pub member: Option<(Vec<u8>, M)>,
    /// The most recent MemberStatusEntry together with its action hash, if found.
    pub status: Option<(Vec<u8>, S)>,
}

/// Input for inviting an agent to a bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct InviteAgentToBundleInput {
    pub network_seed: String,
    pub agent: AgentPubKey,
    pub invite_code_base64: String,
}
