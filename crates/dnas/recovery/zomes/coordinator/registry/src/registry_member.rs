use hdk::prelude::*;
use registry_integrity::*;
use shared_types::{InviteMemberInput, MemberStatus, RegistryMemberWithStatus, SetMemberStatusInput};

/// Get a fixed anchor hash for linking all registry members
fn get_registry_anchor() -> ExternResult<AnyLinkableHash> {
    let path = Path::from("all_registry_members");
    let entry_hash = path.path_entry_hash()?;

    Ok(entry_hash.into())
}

/// Invite an agent to join the registry (creates RegistryMember entry with no status)
#[hdk_extern]
pub fn invite_member(input: InviteMemberInput) -> ExternResult<ActionHash> {
    let now = sys_time()?.as_micros();

    let member = RegistryMember {
        agent: input.agent.clone(),
        display_name: input.display_name,
        timestamp: now,
    };

    let action_hash = create_entry(&EntryTypes::RegistryMember(member))?;

    // Link from fixed anchor to member entry
    create_link(
        get_registry_anchor()?,
        action_hash.clone(),
        LinkTypes::AllRegistryMembers,
        (),
    )?;

    // Link agent pub key -> RegistryMember entry
    create_link(
        input.agent.clone(),
        action_hash.clone(),
        LinkTypes::AgentToRegistryMember,
        (),
    )?;

    // Create the initial "Invited" status entry and link it
    set_member_status(SetMemberStatusInput {
        agent: input.agent,
        new_status: MemberStatus::Invited(input.invite_code_base64),
    })?;

    Ok(action_hash)
}

/// Create a new MemberStatusEntry for an agent and link it from the agent pub key.
///
/// Multiple status entries may accumulate; use `get_member_status` to retrieve
/// the most recent one.
#[hdk_extern]
pub fn set_member_status(input: SetMemberStatusInput) -> ExternResult<ActionHash> {
    let now = sys_time()?.as_micros();

    let status_entry = MemberStatusEntry {
        agent: input.agent.clone(),
        status: input.new_status,
        timestamp: now,
    };

    let action_hash = create_entry(&EntryTypes::MemberStatusEntry(status_entry))?;

    // Link agent pub key -> MemberStatusEntry
    create_link(
        input.agent,
        action_hash.clone(),
        LinkTypes::AgentToMemberStatus,
        (),
    )?;

    Ok(action_hash)
}

/// Retrieve all registry members
#[hdk_extern]
pub fn get_registry_members(_: ()) -> ExternResult<Vec<(ActionHash, RegistryMember)>> {
    let links = get_links(
        LinkQuery {
            base: get_registry_anchor()?,
            link_type: LinkTypes::AllRegistryMembers.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    let mut members = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(member) = record.entry().to_app_option::<RegistryMember>().map_err(|e| wasm_error!(e))? {
                    members.push((action_hash, member));
                }
            }
        }
    }

    Ok(members)
}

/// Retrieve all registry members - with network access
#[hdk_extern]
pub fn get_registry_members_network(_: ()) -> ExternResult<Vec<(ActionHash, RegistryMember)>> {
    let links = get_links(
        LinkQuery {
            base: get_registry_anchor()?,
            link_type: LinkTypes::AllRegistryMembers.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Network,
    )?;

    let mut members = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::network())? {
                if let Some(member) = record.entry().to_app_option::<RegistryMember>().map_err(|e| wasm_error!(e))? {
                    members.push((action_hash, member));
                }
            }
        }
    }

    Ok(members)
}

/// Get the RegistryMember entry for a given agent pub key
#[hdk_extern]
pub fn get_member_by_agent(agent: AgentPubKey) -> ExternResult<Option<(ActionHash, RegistryMember)>> {
    let links = get_links(
        LinkQuery {
            base: agent.into(),
            link_type: LinkTypes::AgentToRegistryMember.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    // Should only be one link per agent
    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(member) = record.entry().to_app_option::<RegistryMember>().map_err(|e| wasm_error!(e))? {
                    return Ok(Some((action_hash, member)));
                }
            }
        }
    }

    Ok(None)
}

/// Get the RegistryMember entry for a given agent pub key - with network access
#[hdk_extern]
pub fn get_member_by_agent_network(agent: AgentPubKey) -> ExternResult<Option<(ActionHash, RegistryMember)>> {
    let links = get_links(
        LinkQuery {
            base: agent.into(),
            link_type: LinkTypes::AgentToRegistryMember.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Network,
    )?;

    if let Some(link) = links.first() {
        if let Some(action_hash) = link.target.clone().into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::network())? {
                if let Some(member) = record.entry().to_app_option::<RegistryMember>().map_err(|e| wasm_error!(e))? {
                    return Ok(Some((action_hash, member)));
                }
            }
        }
    }

    Ok(None)
}

/// Retrieve all MemberStatusEntry records for an agent, sorted by timestamp ascending.
fn get_all_status_entries(
    agent: AgentPubKey,
    strategy: GetStrategy,
) -> ExternResult<Vec<(ActionHash, MemberStatusEntry)>> {
    let links = get_links(
        LinkQuery {
            base: agent.into(),
            link_type: LinkTypes::AgentToMemberStatus.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        strategy.clone(),
    )?;

    let get_options: GetOptions = strategy.into();

    let mut entries: Vec<(ActionHash, MemberStatusEntry)> = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), get_options.clone())? {
                if let Some(entry) = record
                    .entry()
                    .to_app_option::<MemberStatusEntry>()
                    .map_err(|e| wasm_error!(e))?
                {
                    entries.push((action_hash, entry));
                }
            }
        }
    }

    entries.sort_by_key(|(_, e)| e.timestamp);
    Ok(entries)
}

/// Retrieve the most recent MemberStatusEntry for an agent.
#[hdk_extern]
pub fn get_member_status(agent: AgentPubKey) -> ExternResult<Option<(ActionHash, MemberStatusEntry)>> {
    let mut entries = get_all_status_entries(agent, GetStrategy::Local)?;
    Ok(entries.pop())
}

/// Retrieve the most recent MemberStatusEntry for an agent - with network access.
#[hdk_extern]
pub fn get_member_status_network(agent: AgentPubKey) -> ExternResult<Option<(ActionHash, MemberStatusEntry)>> {
    let mut entries = get_all_status_entries(agent, GetStrategy::Network)?;
    Ok(entries.pop())
}

/// Retrieve both the RegistryMember and the most recent MemberStatusEntry for
/// an agent in a single call.
///
/// `display_name` is set by the RegistryMember entry (created by the inviting
/// peer) while the `status` is set independently by another peer via
/// `set_member_status`, so the two may be authored by different agents.
#[hdk_extern]
pub fn get_member_and_status(agent: AgentPubKey) -> ExternResult<RegistryMemberWithStatus> {
    let member = get_member_by_agent(agent.clone())?.map(|(_, m)| m);
    let status = get_member_status(agent.clone())?.map(|(_, s)| s);

    Ok(RegistryMemberWithStatus {
        agent: agent.clone(),
        display_name: member.as_ref().map(|m| m.display_name.clone()).unwrap(),
        status: status.as_ref().map(|s| s.status.clone()).unwrap(),
    })
}

/// Retrieve both the RegistryMember and the most recent MemberStatusEntry for
/// an agent - with network access.
#[hdk_extern]
pub fn get_member_and_status_network(agent: AgentPubKey) -> ExternResult<RegistryMemberWithStatus> {
    let member = get_member_by_agent_network(agent.clone())?.map(|(_, m)| m);
    let status = get_member_status_network(agent.clone())?.map(|(_, s)| s);

    Ok(RegistryMemberWithStatus {
        agent: agent.clone(),
        display_name: member.as_ref().map(|m| m.display_name.clone()).unwrap(),
        status: status.as_ref().map(|s| s.status.clone()).unwrap(),
    })
}

/// Retrieve all registry members and their most recent statuses in a single call,
/// using local access for both members and statuses.
///
/// Members for which no status entry can be found are skipped.
#[hdk_extern]
pub fn get_all_members_and_statuses(_: ()) -> ExternResult<Vec<RegistryMemberWithStatus>> {
    let links = get_links(
        LinkQuery {
            base: get_registry_anchor()?,
            link_type: LinkTypes::AllRegistryMembers.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    let mut result = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::local())? {
                if let Some(member) = record
                    .entry()
                    .to_app_option::<RegistryMember>()
                    .map_err(|e| wasm_error!(e))?
                {
                    let mut status_entries =
                        get_all_status_entries(member.agent.clone(), GetStrategy::Local)?;

                    if let Some((_, status_entry)) = status_entries.pop() {
                        result.push(RegistryMemberWithStatus {
                            agent: member.agent,
                            display_name: member.display_name,
                            status: status_entry.status,
                        });
                    } else {
                        debug!(
                            "No status found for member {:?}, skipping",
                            member.agent
                        );
                    }
                }
            }
        }
    }

    Ok(result)
}

/// Mark a member as active by agent pub key
#[hdk_extern]
pub fn accept_member_by_agent(agent: AgentPubKey) -> ExternResult<ActionHash> {
    set_member_status(SetMemberStatusInput {
        agent,
        new_status: MemberStatus::Active,
    })
}

/// Revoke a member by agent pub key
#[hdk_extern]
pub fn revoke_member_by_agent(agent: AgentPubKey) -> ExternResult<ActionHash> {
    set_member_status(SetMemberStatusInput {
        agent,
        new_status: MemberStatus::Revoked,
    })
}

/// Remove a registry member by their action hash
#[hdk_extern]
pub fn remove_member(action_hash: ActionHash) -> ExternResult<ActionHash> {
    delete_entry(action_hash)
}

/// Check if an agent is a registry member (has a RegistryMember entry)
#[hdk_extern]
pub fn is_member(agent: AgentPubKey) -> ExternResult<bool> {
    Ok(get_member_by_agent(agent)?.is_some())
}

/// Check if an agent is an active member (most recent status is Active)
#[hdk_extern]
pub fn is_active_member(agent: AgentPubKey) -> ExternResult<bool> {
    if let Some((_, status_entry)) = get_member_status(agent)? {
        return Ok(matches!(status_entry.status, MemberStatus::Active));
    }
    Ok(false)
}

/// Check if the current cell is a registry cell
///
/// Returns true if this is a registry cell, false otherwise.
#[hdk_extern]
pub fn is_registry_cell(_: ()) -> ExternResult<bool> {
    let links = get_links(
        LinkQuery {
            base: get_registry_anchor()?,
            link_type: LinkTypes::AllRegistryMembers.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    Ok(!links.is_empty())
}
