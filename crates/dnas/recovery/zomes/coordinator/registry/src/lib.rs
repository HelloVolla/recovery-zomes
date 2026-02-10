pub mod registry_member;
pub use registry_member::*;
pub mod storage_bundle;
pub use storage_bundle::*;
pub mod sync;
pub use sync::*;
pub mod utils;
pub use utils::*;

use hdk::prelude::*;
use registry_integrity::*;

/// Called the first time a zome call is made to the cell containing this zome
#[hdk_extern]
pub fn init() -> ExternResult<InitCallbackResult> {
    // Grant unrestricted access to certain functions that should be remotely callable
    let mut functions = HashSet::new();
    functions.insert((zome_info()?.name, "receive_bundle_notification".into()));

    create_cap_grant(ZomeCallCapGrant {
        tag: "registry_remote".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(functions),
    })?;

    Ok(InitCallbackResult::Pass)
}

/// Signal types for registry events
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Signal {
    /// A new member was invited
    MemberInvited {
        action: SignedActionHashed,
        member: RegistryMember,
    },
    /// A member status was updated
    MemberUpdated {
        action: SignedActionHashed,
        member: RegistryMember,
    },
    /// A new storage bundle was registered
    BundleRegistered {
        action: SignedActionHashed,
        bundle_info: StorageBundle,
    },
    /// An agent was added to a bundle
    AgentAddedToBundle {
        action: SignedActionHashed,
        membership: BundleMember,
    },
    /// A new bundle notification was received
    NewBundleNotification {
        bundle_id: String,
        bundle_name: String,
        membership_action_hash: ActionHash,
    },
    /// Generic entry created signal
    EntryCreated {
        action: SignedActionHashed,
        app_entry: EntryTypes,
    },
    /// Generic entry updated signal
    EntryUpdated {
        action: SignedActionHashed,
        app_entry: EntryTypes,
        original_app_entry: EntryTypes,
    },
    /// Generic entry deleted signal
    EntryDeleted {
        action: SignedActionHashed,
        original_app_entry: EntryTypes,
    },
    /// Link created signal
    LinkCreated {
        action: SignedActionHashed,
        link_type: LinkTypes,
    },
    /// Link deleted signal
    LinkDeleted {
        action: SignedActionHashed,
        create_link_action: SignedActionHashed,
        link_type: LinkTypes,
    },
}

/// Whenever an action is committed, we emit a signal to the UI elements to reactively update them
#[hdk_extern(infallible)]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) {
    for action in committed_actions {
        if let Err(err) = signal_action(action) {
            error!("Error signaling new action: {:?}", err);
        }
    }
}

fn signal_action(action: SignedActionHashed) -> ExternResult<()> {
    match action.hashed.content.clone() {
        Action::Create(_create) => {
            if let Ok(Some(app_entry)) = get_entry_for_action(&action.hashed.hash) {
                emit_signal(Signal::EntryCreated { action, app_entry })?;
            }
            Ok(())
        }
        Action::Update(update) => {
            if let Ok(Some(app_entry)) = get_entry_for_action(&action.hashed.hash) {
                if let Ok(Some(original_app_entry)) = get_entry_for_action(&update.original_action_address) {
                    emit_signal(Signal::EntryUpdated {
                        action,
                        app_entry,
                        original_app_entry,
                    })?;
                }
            }
            Ok(())
        }
        Action::Delete(delete) => {
            if let Ok(Some(original_app_entry)) = get_entry_for_action(&delete.deletes_address) {
                emit_signal(Signal::EntryDeleted {
                    action,
                    original_app_entry,
                })?;
            }
            Ok(())
        }
        Action::CreateLink(create_link) => {
            if let Ok(Some(link_type)) = LinkTypes::from_type(create_link.zome_index, create_link.link_type) {
                emit_signal(Signal::LinkCreated { action, link_type })?;
            }
            Ok(())
        }
        Action::DeleteLink(delete_link) => {
            let record = get(delete_link.link_add_address.clone(), GetOptions::local())?.ok_or(
                wasm_error!(WasmErrorInner::Guest("Failed to fetch CreateLink action".to_string())),
            )?;
            match record.action() {
                Action::CreateLink(create_link) => {
                    if let Ok(Some(link_type)) = LinkTypes::from_type(create_link.zome_index, create_link.link_type) {
                        emit_signal(Signal::LinkDeleted {
                            action,
                            link_type,
                            create_link_action: record.signed_action.clone(),
                        })?;
                    }
                    Ok(())
                }
                _ => Err(wasm_error!(WasmErrorInner::Guest("Create Link should exist".to_string()))),
            }
        }
        _ => Ok(()),
    }
}

fn get_entry_for_action(action_hash: &ActionHash) -> ExternResult<Option<EntryTypes>> {
    let record = match get_details(action_hash.clone(), GetOptions::local())? {
        Some(Details::Record(record_details)) => record_details.record,
        _ => return Ok(None),
    };
    let entry = match record.entry().as_option() {
        Some(entry) => entry,
        None => return Ok(None),
    };
    let (zome_index, entry_index) = match record.action().entry_type() {
        Some(EntryType::App(AppEntryDef {
            zome_index,
            entry_index,
            ..
        })) => (zome_index, entry_index),
        _ => return Ok(None),
    };
    EntryTypes::deserialize_from_type(*zome_index, *entry_index, entry)
}

/// Get the progenitor (current agent) public key
#[hdk_extern]
pub fn get_my_agent_pub_key(_: ()) -> ExternResult<AgentPubKey> {
    Ok(agent_info()?.agent_initial_pubkey)
}
