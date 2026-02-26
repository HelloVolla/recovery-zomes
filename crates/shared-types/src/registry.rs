use crate::*;

/// A registry member entry with the status included
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub struct RegistryMemberWithStatus {
    pub agent: AgentPubKey,
    pub display_name: Option<String>,
    pub status: MemberStatus,
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

/// Input for inviting an agent to a bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct InviteAgentToBundleInput {
    pub network_seed: String,
    pub agent: AgentPubKey,
    pub invite_code_base64: String,
}
