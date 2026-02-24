//! Agent Joined Coordinator Zome
//!
//! A minimal zome for signaling when an agent has joined a network.
//! When an agent calls `signal_joined`, it commits an AgentJoined entry to
//! their source chain and creates a link from a well-known anchor.

use hdk::prelude::*;
use agent_joined_integrity::*;
use shared_types::Signal;

/// Well-known anchor path for all joined agents
const AGENTS_ANCHOR: &str = "all_agents";

#[hdk_extern]
pub fn init(_: ()) -> ExternResult<InitCallbackResult> {
    let mut fns = HashSet::new();
    fns.insert((zome_info()?.name, "recv_remote_signal".into()));
    create_cap_grant(ZomeCallCapGrant {
        tag: "remote signals".into(),
        access: CapAccess::Unrestricted,
        functions: GrantedFunctions::Listed(fns),
    })?;
    Ok(InitCallbackResult::Pass)
}

// Post commit callback
#[hdk_extern(infallible)]
pub fn post_commit(committed_actions: Vec<SignedActionHashed>) {
    for action in committed_actions {
        // Only handle cases where an entry is being created.
        if let Action::Create(create_action) = action.action() {
            let entry_hash = create_action.entry_hash.clone();

            if let Ok(Some(record)) = get(entry_hash, GetOptions::local()) {
                if let Some(agent_joined) = record.entry().to_app_option::<AgentJoined>().ok().flatten() {
                    let signal = Signal::AgentJoined { agent: agent_joined.agent.clone() };
                    emit_signal(&signal).ok();

                    // Only signal to the progenitor pub key that is contained in the dna properties
                    if let Ok(dna_info) = dna_info() {
                        if let Ok(properties) = shared_types::Properties::try_from(dna_info.modifiers.properties) {
                            send_remote_signal(&signal, vec![properties.progenitor_pub_key]).ok();
                        }
                    }
                }
            }
        }
    }
}

// Handle remote signals
#[hdk_extern]
pub fn recv_remote_signal(signal: Signal) -> ExternResult<()> {
    emit_signal(signal)?;
    Ok(())
}

/// Signal that the current agent has joined the network.
///
/// This creates an AgentJoined entry on the source chain and links it
/// from the all-agents anchor for discoverability.
#[hdk_extern]
pub fn signal_joined(_: ()) -> ExternResult<()> {
    let agent_info = agent_info()?;

    // If already joined, silently ignore the request
    if has_agent_joined(agent_info.agent_initial_pubkey.clone())? {
        return Ok(());
    }

    let now = sys_time()?;

    let agent_joined = AgentJoined {
        agent: agent_info.agent_initial_pubkey.clone(),
        joined_at: now,
    };

    let action_hash = create_entry(&EntryTypes::AgentJoined(agent_joined.clone()))?;

    // Create link from anchor to this entry for discoverability
    let anchor = anchor_hash()?;
    create_link(
        anchor,
        action_hash,
        LinkTypes::AllAgents,
        (),
    )?;

    Ok(())
}

/// Check if the current agent has signaled they joined.
#[hdk_extern]
pub fn has_joined(_: ()) -> ExternResult<bool> {
    let agent_info = agent_info()?;
    has_agent_joined(agent_info.agent_initial_pubkey)
}

/// Check if a specific agent has signaled they joined.
#[hdk_extern]
pub fn has_agent_joined(agent: AgentPubKey) -> ExternResult<bool> {
    let joined_agents = get_joined_agents(())?;

    for (_, entry) in joined_agents {
        if entry.agent == agent {
            return Ok(true);
        }
    }

    Ok(false)
}

/// Get all agents that have signaled they joined.
#[hdk_extern]
pub fn get_joined_agents(_: ()) -> ExternResult<Vec<(ActionHash, AgentJoined)>> {
    let anchor = anchor_hash()?;

    let links = get_links(
        LinkQuery {
            base: anchor.into(),
            link_type: LinkTypes::AllAgents.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    let mut agents = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(entry) = record.entry().to_app_option::<AgentJoined>().ok().flatten() {
                    agents.push((action_hash, entry));
                }
            }
        }
    }

    Ok(agents)
}

/// Get all agents that have signaled they joined, with network strategy.
#[hdk_extern]
pub fn get_joined_agents_network(_: ()) -> ExternResult<Vec<(ActionHash, AgentJoined)>> {
    let anchor = anchor_hash()?;

    let links = get_links(
        LinkQuery {
            base: anchor.into(),
            link_type: LinkTypes::AllAgents.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Network,
    )?;

    let mut agents = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(entry) = record.entry().to_app_option::<AgentJoined>().ok().flatten() {
                    agents.push((action_hash, entry));
                }
            }
        }
    }

    Ok(agents)
}

/// Get the anchor hash for the all-agents anchor.
fn anchor_hash() -> ExternResult<EntryHash> {
    let path = Path::from(AGENTS_ANCHOR);
    path.path_entry_hash()
}
