use crate::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RemoteCallInput {
    pub target_agent: AgentPubKey,
    pub zome_name: String,
    pub fn_name: String,
    pub cap_secret: Option<CapSecret>,
    pub payload: ExternIO,
}
