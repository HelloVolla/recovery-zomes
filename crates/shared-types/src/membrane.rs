use ed25519_dalek::{SigningKey, Signer};

use crate::*;

#[derive(Serialize, Deserialize, Debug, SerializedBytes, Clone)]
pub struct MembraneProofData {
    pub network_seed: String,
    pub for_agent: AgentPubKey,
    #[serde(with = "serde_bytes")]
    pub network_pub_key: Vec<u8>,
}

impl MembraneProofData {
    pub fn get_signed_membrane_proof_bytes(&self, signing_key: &SigningKey) -> ExternResult<SerializedBytes> {
        // Serialize and sign the membrane proof data
        let data_bytes = holochain_serialized_bytes::encode(&self)
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Serialization error".to_string())))?;
        let signature = signing_key.sign(&data_bytes);

        // Create the membrane proof envelope with the action hash for DHT lookup
        let envelope = MembraneProofEnvelope {
            signature: Signature(signature.to_bytes()),
            data: self.clone(),
        };

        // Convert envelope to SerializedBytes using the derived trait
        envelope.try_into()
            .map_err(|_| wasm_error!(WasmErrorInner::Guest("Serialization error".to_string())))
    }
}

#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub struct MembraneProofEnvelope {
    pub signature: Signature,
    pub data: MembraneProofData,
}
