use crate::*;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};

/// Input for generating an invite code from the storage zome
#[derive(Serialize, Deserialize, Debug, SerializedBytes, Clone)]
pub struct GenerateInviteCodeInput {
    /// Optional target agent's public key (39-byte raw format).
    /// If provided, a membrane proof will be generated and included in the invite code.
    #[serde(with = "serde_bytes")]
    pub for_agent_pub_key: Option<Vec<u8>>,
}

/// Input for creating an invite code in the registry zome
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateInviteCodeInput {
    pub code: String,
    pub target_agent: AgentPubKey,
    pub bundle_id: String,
}

/// An invite code containing all data needed to join a network.
///
/// This struct can be serialized to a base64 string via MessagePack for easy sharing.
/// The base64 string representation is also used as the `code` field when storing
/// invite codes in the registry DHT for tracking purposes.
///
/// # Serialization
///
/// Use `to_base64()` to convert to a shareable string, and `from_base64()` to parse it back.
/// The base64 encoding uses URL-safe characters without padding for maximum compatibility.
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

impl InviteCodeData {
    /// Serializes the invite code to a base64-encoded string.
    ///
    /// Uses MessagePack for efficient binary serialization followed by URL-safe base64 encoding.
    /// The resulting string can be shared with other users and parsed back using `from_base64()`.
    ///
    /// This base64 string is also used as the `code` field when storing invite codes
    /// in the registry DHT for status tracking.
    pub fn to_base64(&self) -> Result<String, String> {
        let msgpack_bytes = rmp_serde::to_vec(self)
            .map_err(|e| format!("Failed to serialize invite code: {}", e))?;
        Ok(URL_SAFE_NO_PAD.encode(&msgpack_bytes))
    }

    /// Deserializes an invite code from a base64-encoded string.
    ///
    /// Expects the string to contain URL-safe base64-encoded MessagePack data,
    /// as produced by `to_base64()`.
    pub fn from_base64(encoded: &str) -> Result<Self, String> {
        let msgpack_bytes = URL_SAFE_NO_PAD
            .decode(encoded.as_bytes())
            .map_err(|e| format!("Failed to decode base64: {}", e))?;
        rmp_serde::from_slice(&msgpack_bytes)
            .map_err(|e| format!("Failed to deserialize invite code: {}", e))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GetInviteCodeByCodeInput {
    /// The base64-encoded invite code string to search for
    pub code: String,
    /// The agent whose invite codes to search
    pub agent: AgentPubKey,
}
