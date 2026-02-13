use hdi::prelude::*;

/// Private entry for storing the signing private key (seed) on the progenitor's source chain.
/// This key is used to sign membrane proofs for joining agents.
///
/// This is a PRIVATE entry - it stays on the local source chain and is never shared with the network.
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct PrivateSigningKeyEntry {
    /// Raw 32-byte ed25519 seed (private key)
    pub seed: SerializedBytes,
}
