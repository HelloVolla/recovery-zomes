use crate::*;

/// Entry type for storing the signing public key on the progenitor's source chain.
/// This key is used to verify membrane proof signatures for joining agents.
#[derive(Serialize, Deserialize, Debug, Clone, SerializedBytes)]
pub struct SigningKeyEntry {
    /// Raw 32-byte ed25519 public key
    #[serde(with = "serde_bytes")]
    pub pub_key: Vec<u8>,
}
