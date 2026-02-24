pub mod registry_member;
pub use registry_member::*;
pub mod member_status;
pub use member_status::*;
pub mod storage_bundle;
pub use storage_bundle::*;
pub mod bundle_member;
pub use bundle_member::*;

use hdi::prelude::*;

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[hdk_entry_types]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    RegistryMember(RegistryMember),
    MemberStatusEntry(MemberStatusEntry),
    StorageBundle(StorageBundle),
    BundleMember(BundleMember),
}

#[derive(Serialize, Deserialize)]
#[hdk_link_types]
pub enum LinkTypes {
    /// Link from member's agent pub key to their RegistryMember entry
    AgentToRegistryMember,
    /// Link from agent pub key to their MemberStatusEntry instances
    AgentToMemberStatus,
    /// Link from agent pub key to their BundleMember instances (with tags for filtering)
    AgentToBundleMembers,
    /// Link from bundle entry hash to its member agents
    BundleToBundleMembers,
    /// Link from agent to bundles they are a member of
    AgentToStorageBundles,
    /// Link from network seed to storage bundle
    NetworkSeedToStorageBundle,
    /// Link from fixed anchor to registry members
    AllRegistryMembers,
    /// Link from fixed anchor to storage bundles
    AllStorageBundles,
}

#[hdk_extern]
pub fn validate(op: Op) -> ExternResult<ValidateCallbackResult> {
    match op.flattened::<EntryTypes, LinkTypes>()? {
        FlatOp::StoreEntry(store_entry) => match store_entry {
            OpEntry::CreateEntry { app_entry, action } => match app_entry {
                EntryTypes::RegistryMember(_) => Ok(ValidateCallbackResult::Valid),
                EntryTypes::MemberStatusEntry(_) => Ok(ValidateCallbackResult::Valid),
                EntryTypes::StorageBundle(bundle) => {
                    validate_create_storage_bundle_info(EntryCreationAction::Create(action), bundle)
                }
                EntryTypes::BundleMember(member) => {
                    validate_create_bundle_member(EntryCreationAction::Create(action), member)
                }
            },
            OpEntry::UpdateEntry { app_entry, action, .. } => match app_entry {
                EntryTypes::RegistryMember(_) => Ok(ValidateCallbackResult::Valid),
                EntryTypes::MemberStatusEntry(_) => Ok(ValidateCallbackResult::Valid),
                EntryTypes::StorageBundle(bundle) => {
                    validate_create_storage_bundle_info(EntryCreationAction::Update(action), bundle)
                }
                EntryTypes::BundleMember(member) => {
                    validate_create_bundle_member(EntryCreationAction::Update(action), member)
                }
            },
            _ => Ok(ValidateCallbackResult::Valid),
        },
        _ => Ok(ValidateCallbackResult::Valid),
    }
}
