use hdi::prelude::*;
use integrity_types::MemberStatus;

/// Standalone entry representing the status of a member,
/// decoupled from the RegistryMember entry.
///
/// Multiple status entries may exist for the same agent; callers should
/// select the one with the greatest `timestamp` to get the most recent status.
/// The timestamp to use is the timestamp of the link to this entry.
#[derive(Clone, PartialEq)]
#[hdk_entry_helper]
pub struct MemberStatusEntry {
    /// The agent public key this status applies to
    pub agent: AgentPubKey,
    /// The membership status
    pub status: MemberStatus,
}
