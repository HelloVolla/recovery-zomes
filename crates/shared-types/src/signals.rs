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
    },
    RegistryMemberStatusSet {
        agent: AgentPubKey,
        status: MemberStatus,
    },
}
