use crate::*;

#[derive(Clone, PartialEq, serde::Serialize, serde::Deserialize, Debug)]
pub enum MemberStatus {
    /// Agent has been invited but hasn't joined the registry yet
    Invited,
    /// Agent has joined and is actively a member
    Active,
    /// Membership has been revoked
    Revoked,
}

/// Input for inviting a registry member
#[derive(Serialize, Deserialize, Debug)]
pub struct InviteMemberInput {
    pub agent: AgentPubKey,
    pub display_name: Option<String>,
}

/// Update member status by agent pub key
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateMemberByAgentInput {
    pub agent: AgentPubKey,
    pub new_status: MemberStatus,
}
