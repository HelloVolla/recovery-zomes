use crate::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Signal {
    AgentJoined {
        agent: AgentPubKey,
    },
}
