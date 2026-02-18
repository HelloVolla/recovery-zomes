use hdk::prelude::*;
use registry_integrity::*;
use shared_types::{ InviteMemberInput, MemberStatus, UpdateMemberByAgentInput };

/// Get a fixed anchor hash for linking all registry members
fn get_registry_anchor() -> ExternResult<AnyLinkableHash> {
    let path = Path::from("all_registry_members");
    let entry_hash = path.path_entry_hash()?;

    Ok(entry_hash.into())
}

/// Invite an agent to join the registry (creates entry with Invited status)
#[hdk_extern]
pub fn invite_member(input: InviteMemberInput) -> ExternResult<ActionHash> {
    let member = RegistryMember {
        agent: input.agent.clone(),
        display_name: input.display_name,
        status: MemberStatus::Invited(input.invite_code_base64),
    };

    let action_hash = create_entry(&EntryTypes::RegistryMember(member))?;

    // Link from fixed anchor to member entry
    create_link(
        get_registry_anchor()?,
        action_hash.clone(),
        LinkTypes::AllRegistryMembers,
        (),
    )?;

    // Link a member's agent pub key to their entry
    create_link(
        input.agent,
        action_hash.clone(),
        LinkTypes::AgentToRegistryMember,
        (),
    )?;

    Ok(action_hash)
}

/// Get all registry members for the current agent
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

/// Get all registry members for the current agent - with network access.
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

/// Get a registry member by their agent pub key
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

/// Get a registry member by their agent pub key - with network access.
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

/// Get only active registry members
#[hdk_extern]
pub fn get_active_members(_: ()) -> ExternResult<Vec<(ActionHash, RegistryMember)>> {
    let all_members = get_registry_members(())?;

    let active: Vec<(ActionHash, RegistryMember)> = all_members
        .into_iter()
        .filter(|(_, member)| matches!(member.status, MemberStatus::Active))
        .collect();

    Ok(active)
}

/// Get invited registry members
#[hdk_extern]
pub fn get_invited_members(_: ()) -> ExternResult<Vec<(ActionHash, RegistryMember)>> {
    let all_members = get_registry_members(())?;

    let invited: Vec<(ActionHash, RegistryMember)> = all_members
        .into_iter()
        .filter(|(_, member)| matches!(member.status, MemberStatus::Invited(_)))
        .collect();

    Ok(invited)
}

/// Update member status input
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateMemberStatusInput {
    pub member_action_hash: ActionHash,
    pub new_status: MemberStatus,
}

/// Update the status of a registry member by action hash
#[hdk_extern]
pub fn update_member_status(input: UpdateMemberStatusInput) -> ExternResult<ActionHash> {
    let record = get(input.member_action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Member not found".to_string())))?;

    let mut member: RegistryMember = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Invalid member entry".to_string())))?;

    member.status = input.new_status;

    update_entry(input.member_action_hash, &EntryTypes::RegistryMember(member))
}

#[hdk_extern]
pub fn update_member_status_by_agent(input: UpdateMemberByAgentInput) -> ExternResult<ActionHash> {
    let (action_hash, _) = get_member_by_agent(input.agent.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(format!(
            "Member not found for agent: {:?}",
            input.agent
        ))))?;

    update_member_status(UpdateMemberStatusInput {
        member_action_hash: action_hash,
        new_status: input.new_status,
    })
}

/// Update the status of a registry member by agent pub key - with network access.
#[hdk_extern]
pub fn update_member_status_by_agent_network(input: UpdateMemberByAgentInput) -> ExternResult<ActionHash> {
    let (action_hash, _) = get_member_by_agent_network(input.agent.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(format!(
            "Member not found for agent: {:?}",
            input.agent
        ))))?;

    let record = get(action_hash.clone(), GetOptions::network())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Member not found".to_string())))?;

    let mut member: RegistryMember = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Invalid member entry".to_string())))?;

    member.status = input.new_status;

    update_entry(action_hash, &EntryTypes::RegistryMember(member))
}

/// Mark a member as active by action hash
#[hdk_extern]
pub fn accept_member(member_action_hash: ActionHash) -> ExternResult<ActionHash> {
    update_member_status(UpdateMemberStatusInput {
        member_action_hash,
        new_status: MemberStatus::Active,
    })
}

/// Mark a member as active by agent pub key
#[hdk_extern]
pub fn accept_member_by_agent(agent: AgentPubKey) -> ExternResult<ActionHash> {
    update_member_status_by_agent(UpdateMemberByAgentInput {
        agent,
        new_status: MemberStatus::Active,
    })
}

/// Revoke a member by action hash
#[hdk_extern]
pub fn revoke_member(member_action_hash: ActionHash) -> ExternResult<ActionHash> {
    update_member_status(UpdateMemberStatusInput {
        member_action_hash,
        new_status: MemberStatus::Revoked,
    })
}

/// Revoke a member by agent pub key
#[hdk_extern]
pub fn revoke_member_by_agent(agent: AgentPubKey) -> ExternResult<ActionHash> {
    update_member_status_by_agent(UpdateMemberByAgentInput {
        agent,
        new_status: MemberStatus::Revoked,
    })
}

/// Remove a registry member by their action hash
#[hdk_extern]
pub fn remove_member(action_hash: ActionHash) -> ExternResult<ActionHash> {
    delete_entry(action_hash)
}

/// Check if an agent is a registry member
#[hdk_extern]
pub fn is_member(agent: AgentPubKey) -> ExternResult<bool> {
    Ok(get_member_by_agent(agent)?.is_some())
}

/// Check if an agent is an active member
#[hdk_extern]
pub fn is_active_member(agent: AgentPubKey) -> ExternResult<bool> {
    if let Some((_, member)) = get_member_by_agent(agent)? {
        return Ok(matches!(member.status, MemberStatus::Active));
    }
    Ok(false)
}

/// Check if the current cell is a registry cell
///
/// Returns true if this is a registry cell, false otherwise.
#[hdk_extern]
pub fn is_registry_cell(_: ()) -> ExternResult<bool> {
    // Check if there are any registry members linked from the fixed anchor
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
