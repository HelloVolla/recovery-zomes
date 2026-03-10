use crate::*;

/// Input for setting a bundle member's status (creates a new BundleMember)
#[derive(Serialize, Deserialize, Debug)]
pub struct SetBundleMemberStatusInput {
    pub agent: AgentPubKey,
    pub network_seed: String,
    pub status: MemberStatus,
}

/// Input for registering a new storage bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterBundleInput {
    pub network_seed: String,
    pub progenitor: AgentPubKey,
    pub network_pub_key: Vec<u8>,
    pub dna_hash: Vec<u8>,
}
