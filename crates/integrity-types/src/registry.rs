use crate::*;

/// Status of a registry member
#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum MemberStatus {
    /// Agent has been invited with an invite code
    Invited(String),
    /// Agent has joined and is actively a member
    Active,
    /// Membership has been revoked
    Revoked,
    /// Membership has been rejected
    Rejected(RejectionReason),
}

#[derive(Clone, PartialEq, Serialize, Deserialize, Debug)]
pub enum RejectionReason {
    /// Unknown failure reason
    Unknown,
    /// The invite code was for another agent
    ForAnotherAgent,
    /// The agent does not have enough storage space
    NotEnoughStorage,
}
