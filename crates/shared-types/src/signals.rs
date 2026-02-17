use crate::*;

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Signal {
    AgentJoined {
        agent: AgentPubKey,
    },
}
