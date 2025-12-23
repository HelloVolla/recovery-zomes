use crate::*;

#[derive(Serialize, Deserialize, Debug, SerializedBytes, Clone)]
pub struct Properties {
    pub progenitor_pub_key: AgentPubKey,
    /// Raw 32-byte ed25519 public key used for verifying membrane proof signatures.
    /// This key is provided externally when creating the network.
    #[serde(with = "serde_bytes")]
    pub signing_pub_key: Vec<u8>,
}
