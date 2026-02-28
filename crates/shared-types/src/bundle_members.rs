use crate::*;

/// Input for setting a bundle member's status (creates a new BundleMember)
#[derive(Serialize, Deserialize, Debug)]
pub struct SetBundleMemberStatusInput {
    pub agent: AgentPubKey,
    pub network_seed: String,
    pub status: MemberStatus,
}
