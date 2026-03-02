pub mod registry_member;
pub use registry_member::*;
pub mod storage_bundle;
use shared_types::Signal;
pub use storage_bundle::*;
pub mod bundle_member;
pub use bundle_member::*;
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
    functions.insert((zome_info()?.name, "recv_remote_signal".into()));

    create_cap_grant(ZomeCallCapGrant {
        tag: "registry_remote".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(functions),
    })?;

    Ok(InitCallbackResult::Pass)
}

// Handle remote signals
#[hdk_extern]
pub fn recv_remote_signal(signal: Signal) -> ExternResult<()> {
    // Re-emit as a local signal for the UI/client to handle
    emit_signal(signal)?;
    Ok(())
}

// Post commit callback
#[hdk_extern(infallible)]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) {
    for action in committed_actions {
        if let Err(err) = signal_action(action) {
            error!("Error signaling new action: {:?}", err);
        }
    }
}

fn signal_action(action: SignedActionHashed) -> ExternResult<()> {
    let Action::Create(create_action) = action.action() else {
        return Ok(());
    };

    // Deserialize to the typed EntryTypes enum using the entry type info
    // embedded in the Create action, so we can match exhaustively.
    let entry = match get(create_action.entry_hash.clone(), GetOptions::local())? {
        Some(record) => match record.entry().as_option() {
            Some(e) => e.clone(),
            None => return Ok(()),
        },
        None => return Ok(()),
    };
    let entry_type = match create_action.entry_type.clone() {
        EntryType::App(app) => app,
        _ => return Ok(()),
    };
    let app_entry = EntryTypes::deserialize_from_type(
        entry_type.zome_index,
        entry_type.entry_index,
        &entry,
    )?;

    match app_entry {
        Some(EntryTypes::RegistryMember(member)) => {
            emit_signal(&Signal::RegistryMemberCreated {
                agent: member.agent,
                display_name: member.display_name,
            })?;
        }
        Some(EntryTypes::MemberStatusEntry(status_entry)) => {
            emit_signal(&Signal::RegistryMemberStatusSet {
                agent: status_entry.agent,
                status: status_entry.status,
            })?;
        }
        _ => {}
    }

    Ok(())
}

/// Get the progenitor (current agent) public key
#[hdk_extern]
pub fn get_my_agent_pub_key(_: ()) -> ExternResult<AgentPubKey> {
    Ok(agent_info()?.agent_initial_pubkey)
}
