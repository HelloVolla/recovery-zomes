pub mod registry_member;
pub use registry_member::*;
pub mod storage_bundle_info;
pub use storage_bundle_info::*;
pub mod bundle_member;
pub use bundle_member::*;

use hdi::prelude::*;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    RegistryMember(RegistryMember),
    StorageBundleInfo(StorageBundleInfo),
    BundleMember(BundleMember),
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
                EntryTypes::BundleMember(membership) => {
                    validate_create_bundle_member(EntryCreationAction::Create(action), membership)
                }
            },
            OpEntry::UpdateEntry { app_entry, action, .. } => match app_entry {
                EntryTypes::RegistryMember(_) => Ok(ValidateCallbackResult::Valid),
                EntryTypes::StorageBundleInfo(bundle_info) => {
                    validate_create_storage_bundle_info(EntryCreationAction::Update(action), bundle_info)
                }
                EntryTypes::BundleMember(membership) => {
                    validate_create_bundle_member(EntryCreationAction::Update(action), membership)
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}
