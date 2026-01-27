use crate::*;

/// Input for generating an invite code from the storage zome
#[derive(Serialize, Deserialize, Debug, SerializedBytes, Clone)]
pub struct GenerateInviteCodeInput {
    /// Optional target agent's public key (39-byte raw format).
    /// If provided, a membrane proof will be generated and included in the invite code.
    #[serde(with = "serde_bytes")]
    pub for_agent_pub_key: Option<Vec<u8>>,
}

/// An invite code containing all data needed to join a network.
/// Can be serialized to/from a base64 string via MessagePack for easy sharing.
#[derive(Serialize, Deserialize, Debug, SerializedBytes, Clone)]
pub struct InviteCodeData {
    /// The network seed string that identifies this network
    pub network_seed: String,
    /// The progenitor's public key (39-byte raw format)
    #[serde(with = "serde_bytes")]
    pub progenitor_pub_key: Vec<u8>,
    /// The public key of the network, used to verify membrane proofs
    #[serde(with = "serde_bytes")]
    pub network_pub_key: Vec<u8>,
    /// The membrane proof bytes (MessagePack-encoded MembraneProofEnvelope)
    #[serde(with = "serde_bytes")]
    pub membrane_proof: Option<Vec<u8>>,
}
