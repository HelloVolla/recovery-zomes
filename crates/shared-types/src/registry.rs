use crate::*;

/// Input for inviting a registry member
#[derive(Serialize, Deserialize, Debug)]
pub struct InviteMemberInput {
    pub agent: AgentPubKey,
    pub display_name: Option<String>,
}
