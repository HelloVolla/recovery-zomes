use hdk::prelude::*;
use registry_integrity::*;
use shared_types::MemberStatus;

/// Get all bundles that the current agent is a member of.
/// This is used by host agents to discover which bundles they need to clone.
#[hdk_extern]
pub fn get_my_assigned_bundles(_: ()) -> ExternResult<Vec<(ActionHash, BundleMember)>> {
    let my_agent = agent_info()?.agent_initial_pubkey;

    let links = get_links(
        LinkQuery {
            base: my_agent.into(),
            link_type: LinkTypes::AgentToStorageBundles.try_into_filter()?,
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
            if let Some(record) = get(action_hash.clone(), GetOptions::local())? {
                if let Some(member) = record.entry().to_app_option::<BundleMember>().map_err(|e| wasm_error!(e))? {
                    members.push((action_hash, member));
                }
            }
        }
    }

    Ok(members)
}

/// Get bundles that an agent is invited to but hasn't accepted yet (status = Invited)
#[hdk_extern]
pub fn get_invited_bundles(_: ()) -> ExternResult<Vec<(ActionHash, BundleMember)>> {
    let my_bundles = get_my_assigned_bundles(())?;

    let invited: Vec<(ActionHash, BundleMember)> = my_bundles
        .into_iter()
        .filter(|(_, member)| matches!(member.status, MemberStatus::Invited(_)))
        .collect();

    Ok(invited)
}

/// Mark a bundle as cloned/active by updating the member status.
/// This should be called by a host agent after they've cloned the bundle DNA.
#[hdk_extern]
pub fn mark_bundle_as_cloned(member_action_hash: ActionHash) -> ExternResult<ActionHash> {
    let record = get(member_action_hash.clone(), GetOptions::local())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Member not found".to_string())))?;

    let mut member: BundleMember = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Invalid member entry".to_string())))?;

    member.status = MemberStatus::Active;

    update_entry(member_action_hash, &EntryTypes::BundleMember(member))
}
