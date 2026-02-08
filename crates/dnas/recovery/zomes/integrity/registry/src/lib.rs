pub mod registry_member;
pub use registry_member::*;
pub mod storage_bundle_info;
pub use storage_bundle_info::*;
pub mod bundle_membership;
pub use bundle_membership::*;
pub mod invite_code;
pub use invite_code::*;

use hdi::prelude::*;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    RegistryMember(RegistryMember),
    StorageBundleInfo(StorageBundleInfo),
    BundleMembership(BundleMembership),
    InviteCode(InviteCode),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    /// Link from progenitor's agent pub key to registry members
    ProgenitorToMembers,
    /// Link from member's agent pub key to their RegistryMember entry
    AgentToMember,
    /// Link from progenitor to their storage bundles
    ProgenitorToBundles,
    /// Link from bundle entry hash to its member agents
    BundleToMembers,
    /// Link from agent to bundles they are a member of
    AgentToBundles,
    /// Link from agent pub key to their invite codes
    AgentToInviteCodes,
    /// Link from invite code entry to target agent (for reverse lookup)
    InviteCodeToAgent,
    /// Link from bundle ID hash to invite codes for that bundle
    BundleToInviteCodes,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::RegistryMember(_) => Ok(ValidateCallbackResult::Valid),
                EntryTypes::StorageBundleInfo(bundle_info) => {
                    validate_create_storage_bundle_info(EntryCreationAction::Create(action), bundle_info)
                }
                EntryTypes::BundleMembership(membership) => {
                    validate_create_bundle_membership(EntryCreationAction::Create(action), membership)
                }
                EntryTypes::InviteCode(invite_code) => {
                    validate_create_invite_code(EntryCreationAction::Create(action), invite_code)
                }
            },
            OpEntry::UpdateEntry { app_entry, action, .. } => match app_entry {
                EntryTypes::RegistryMember(_) => Ok(ValidateCallbackResult::Valid),
                EntryTypes::StorageBundleInfo(bundle_info) => {
                    validate_create_storage_bundle_info(EntryCreationAction::Update(action), bundle_info)
                }
                EntryTypes::BundleMembership(membership) => {
                    validate_create_bundle_membership(EntryCreationAction::Update(action), membership)
                }
                EntryTypes::InviteCode(invite_code) => {
                    validate_create_invite_code(EntryCreationAction::Update(action), invite_code)
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}
