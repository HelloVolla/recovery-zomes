use crate::*;

use hdk::prelude::*;
use registry_integrity::*;
use shared_types::{InviteAgentToBundleInput, MemberStatus, SetBundleMemberStatusInput};

/// Add an agent as a member of a storage bundle with an initial `Invited` status.
#[hdk_extern]
pub fn invite_agent_to_bundle(input: InviteAgentToBundleInput) -> ExternResult<ActionHash> {
    let member = BundleMember {
        agent: input.agent.clone(),
        network_seed: input.network_seed.clone(),
        status: MemberStatus::Invited(input.invite_code_base64),
    };

    let action_hash = create_entry(&EntryTypes::BundleMember(member.clone()))?;

    let network_seed_hash = Path::from(input.network_seed.clone()).path_entry_hash()?;

    // Link network seed -> BundleMember (tag = agent pubkey bytes, for bundle-scoped queries)
    create_link(
        network_seed_hash,
        action_hash.clone(),
        LinkTypes::NetworkSeedToBundleMembers,
        LinkTag::new(member.agent.clone().get_raw_39().to_vec()),
    )?;

    // Link from agent to their bundle member instance
    create_link(
        input.agent.clone(),
        action_hash.clone(),
        LinkTypes::AgentToBundleMembers,
        LinkTag::new(member.network_seed.as_bytes().to_vec()),
    )?;

    Ok(action_hash)
}

/// Create a new `BundleMember` entry for an agent with the given status.
///
/// Both a network seed anchor link (for `get_bundle_members`) and an agent link
/// (for `get_agent_bundle_members`) are created. No DHT lookup is performed for
/// the bundle entry, so this is safe to call before gossip starts.
///
/// Multiple entries may accumulate over time; both query functions deduplicate
/// by keeping only the newest link per agent.
#[hdk_extern]
pub fn set_bundle_member_status(input: SetBundleMemberStatusInput) -> ExternResult<ActionHash> {
    let member = BundleMember {
        agent: input.agent.clone(),
        network_seed: input.network_seed.clone(),
        status: input.status,
    };

    let action_hash = create_entry(&EntryTypes::BundleMember(member.clone()))?;

    let network_seed_hash = Path::from(input.network_seed.clone()).path_entry_hash()?;

    // Link network seed -> BundleMember (same tag scheme as invite_agent_to_bundle)
    create_link(
        network_seed_hash,
        action_hash.clone(),
        LinkTypes::NetworkSeedToBundleMembers,
        LinkTag::new(member.agent.clone().get_raw_39().to_vec()),
    )?;

    // Link agent pub key -> MemberStatusEntry
    create_link(
        input.agent,
        action_hash.clone(),
        LinkTypes::AgentToBundleMembers,
        LinkTag::new(member.network_seed.as_bytes().to_vec()),
    )?;

    Ok(action_hash)
}

/// Get all members of a specific bundle, returning only the latest entry per agent.
///
/// Queries the network seed anchor for all associated links and deduplicates them
/// based on the link tag (which is the agent pubkey bytes), keeping
/// only the entry whose link has the newest timestamp.
#[hdk_extern]
pub fn get_bundle_members(network_seed: String) -> ExternResult<Vec<(ActionHash, BundleMember)>> {
    let network_seed_hash = Path::from(network_seed).path_entry_hash()?;

    let links = get_links(
        LinkQuery {
            base: network_seed_hash.into(),
            link_type: LinkTypes::NetworkSeedToBundleMembers.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;
    let links = dedup_links_by_tag(links);

    let mut members = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(member) = record.entry().to_app_option::<BundleMember>().map_err(|e| wasm_error!(e))? {
                    members.push((action_hash, member));
                }
            }
        }
    }

    Ok(members)
}

/// Remove an agent from a bundle
#[hdk_extern]
pub fn remove_agent_from_bundle(member_action_hash: ActionHash) -> ExternResult<ActionHash> {
    delete_entry(member_action_hash)
}

/// Get all bundle member instances for a specific agent
#[hdk_extern]
pub fn get_agent_bundle_members(agent: AgentPubKey) -> ExternResult<Vec<(ActionHash, BundleMember)>> {
    let links = get_links(
        LinkQuery {
            base: agent.into(),
            link_type: LinkTypes::AgentToBundleMembers.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;
    let links = dedup_links_by_tag(links);

    let mut members = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(member) = record.entry().to_app_option::<BundleMember>().map_err(|e| wasm_error!(e))? {
                    members.push((action_hash, member));
                }
            }
        }
    }

    Ok(members)
}

/// Get bundle members for an agent filtered by Invited status
#[hdk_extern]
pub fn get_agent_invite_codes(agent: AgentPubKey) -> ExternResult<Vec<String>> {
    let links = get_links(
        LinkQuery {
            base: agent.into(),
            link_type: LinkTypes::AgentToBundleMembers.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;
    let links = dedup_links_by_tag(links);

    let mut invite_codes = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(_) = record.entry().to_app_option::<BundleMember>().map_err(|e| wasm_error!(e))? {
                    // Extract invite code from tag
                    let tag_string = String::from_utf8_lossy(&link.tag.0);
                    if let Some(invite_code) = tag_string.strip_prefix("invited:") {
                        invite_codes.push(invite_code.to_string());
                    }
                }
            }
        }
    }

    Ok(invite_codes)
}
