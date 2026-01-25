use crate::*;

#[derive(Serialize, Deserialize, Debug, SerializedBytes, Clone)]
pub struct Properties {
    pub progenitor_pub_key: AgentPubKey,
    #[serde(with = "serde_bytes")]
    pub network_pub_key: Vec<u8>,
}
