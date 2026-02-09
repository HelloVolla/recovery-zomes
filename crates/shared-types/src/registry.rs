use crate::*;

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

/// Update member status by agent pub key
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateMemberByAgentInput {
    pub agent: AgentPubKey,
    pub new_status: MemberStatus,
}

/// Input for inviting an agent to a bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct InviteAgentToBundleInput {
    pub network_seed: String,
    pub agent: AgentPubKey,
    pub invite_code_base64: String,
}
