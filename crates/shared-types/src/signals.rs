use crate::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Signal {
    AgentJoined {
        agent: AgentPubKey,
    },
    RegistryMemberCreated {
        agent: AgentPubKey,
        display_name: Option<String>,
        timestamp: Timestamp,
    },
    RegistryMemberStatusSet {
        agent: AgentPubKey,
        status: MemberStatus,
        timestamp: Timestamp,
    },
    InviteAgentToBundle {
        agent: AgentPubKey,
        network_seed: String,
        invite_code_base64: String,
    },
}
