use crate::*;

#[derive(Serialize, Deserialize, Debug, SerializedBytes, Clone)]
pub struct MembraneProofData {
    pub bundle_id: String,
    pub for_agent: AgentPubKey,
}

#[derive(Serialize, Deserialize, Debug, SerializedBytes)]
pub struct MembraneProofEnvelope {
    pub signature: Signature,
    pub data: MembraneProofData,
    /// Action hash of the SigningKeyEntry creation on the progenitor's source chain.
    /// Validators fetch this record to get the public key for signature verification.
    /// Using ActionHash instead of EntryHash prevents offline brute-force attacks
    /// on password-derived keys since ActionHash includes unpredictable data.
    pub signing_key_action_hash: ActionHash,
}
