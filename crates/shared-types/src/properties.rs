use crate::*;

#[derive(Serialize, Deserialize, Debug, SerializedBytes, Clone)]
pub struct Properties {
    pub progenitor_pub_key: AgentPubKey,
}
