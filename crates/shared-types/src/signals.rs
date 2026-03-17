use crate::*;

/// Signals emitted locally via `emit_signal` to the connected client/UI.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum Signal {
    RegistryMemberCreated {
        agent: AgentPubKey,
        display_name: Option<String>,
        timestamp: Timestamp,
    },
    RegistryMemberStatusSet {
        agent: AgentPubKey,
        status: MemberStatus,
        timestamp: Timestamp,
    },
    InviteAgentToBundle {
        agent: AgentPubKey,
        network_seed: String,
        invite_code_base64: String,
    },
    BundleMemberCreated {
        agent: AgentPubKey,
        network_seed: String,
        status: MemberStatus,
    },
}

/// Top-level signal envelope emitted via `emit_signal` (both local and remote paths).
///
/// The `origin` tag distinguishes whether the signal was produced locally by a
/// `post_commit` hook (`Local`) or forwarded from a remote peer (`Remote`).
/// This makes the wire bytes self-describing so the client can detect origin
/// without any ambiguity.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "origin", content = "signal")]
pub enum SignalWithOrigin {
    Local(Signal),
    Remote(Signal),
}
