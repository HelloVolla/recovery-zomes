pub mod registry_member;
pub use registry_member::*;
pub mod ping;
pub use ping::*;
pub mod storage_bundle;
use shared_types::{SignalWithOrigin, Signal};
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
    functions.insert((zome_info()?.name, "ping".into()));

    create_cap_grant(ZomeCallCapGrant {
        tag: "registry_remote".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(functions),
    })?;

    Ok(InitCallbackResult::Pass)
}

/// Handle remote signals by re-emitting the signal to the UI.
#[hdk_extern]
pub fn recv_remote_signal(signal: Signal) -> ExternResult<()> {
    emit_signal(SignalWithOrigin::Remote(signal))?;
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

fn emit_local_and_remote_signal(signal: &Signal) -> ExternResult<()> {
    emit_signal(SignalWithOrigin::Local(signal.clone()))?;

    let mut agents = get_members_pub_keys(())?;

    let my_agent = agent_info()?.agent_initial_pubkey;
    agents.retain(|agent| agent != &my_agent);

    // Signal only to the progenitor if the list is empty
    if agents.is_empty() {
        if let Ok(dna_info) = dna_info() {
            if let Ok(properties) = shared_types::Properties::try_from(dna_info.modifiers.properties) {
                agents.push(properties.progenitor_pub_key);
            }
        }
    }

    send_remote_signal(signal.clone(), agents)?;

    Ok(())
}

fn signal_action(action: SignedActionHashed) -> ExternResult<()> {
    let ActionData::Create(create_action) = &action.action().data else {
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
            emit_local_and_remote_signal(&Signal::RegistryMemberCreated {
                agent: member.agent,
                display_name: member.display_name,
                timestamp: member.timestamp,
            })?;
        }
        Some(EntryTypes::MemberStatusEntry(status_entry)) => {
            emit_local_and_remote_signal(&Signal::RegistryMemberStatusSet {
                agent: status_entry.agent,
                status: status_entry.status,
                timestamp: status_entry.timestamp,
            })?;
        }
        Some(EntryTypes::BundleMember(member)) => {
            emit_local_and_remote_signal(&Signal::BundleMemberCreated {
                agent: member.agent,
                network_seed: member.network_seed,
                status: member.status,
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
