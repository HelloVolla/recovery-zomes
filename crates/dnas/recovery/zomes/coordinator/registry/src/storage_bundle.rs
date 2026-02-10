use crate::*;

use hdk::prelude::*;
use registry_integrity::*;
use shared_types::{InviteAgentToBundleInput, MemberStatus};

/// Input for registering a new storage bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterBundleInput {
    pub network_seed: String,
    pub progenitor: AgentPubKey,
    pub network_pub_key: Vec<u8>,
}

/// Get a fixed anchor hash for linking all storage bundles
fn get_storage_bundle_anchor() -> ExternResult<AnyLinkableHash> {
    let path = Path::from("all_storage_bundles");
    let entry_hash = path.path_entry_hash()?;

    Ok(entry_hash.into())
}

/// Register a new storage bundle in the registry
#[hdk_extern]
pub fn register_storage_bundle(input: RegisterBundleInput) -> ExternResult<ActionHash> {
    let bundle_info = StorageBundle {
        network_seed: input.network_seed.clone(),
        progenitor: input.progenitor,
        network_pub_key: input.network_pub_key,
    };

    let action_hash = create_entry(&EntryTypes::StorageBundle(bundle_info))?;

    // Create link from fixed anchor to bundle entry
    create_link(
        get_storage_bundle_anchor()?,
        action_hash.clone(),
        LinkTypes::AllStorageBundles,
        (),
    )?;

    // Create link from network seed to bundle entry
    let network_seed_hash = Path::from(input.network_seed.clone()).path_entry_hash()?;
    create_link(
        network_seed_hash,
        action_hash.clone(),
        LinkTypes::NetworkSeedToStorageBundle,
        (),
    )?;

    Ok(action_hash)
}

/// Get all storage bundles
#[hdk_extern]
pub fn get_all_storage_bundles(_: ()) -> ExternResult<Vec<(ActionHash, StorageBundle)>> {
    let links = get_links(
        LinkQuery {
            base: get_storage_bundle_anchor()?,
            link_type: LinkTypes::AllStorageBundles.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    let mut bundles = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(bundle_info) = record.entry().to_app_option::<StorageBundle>().map_err(|e| wasm_error!(e))? {
                    bundles.push((action_hash, bundle_info));
                }
            }
        }
    }

    Ok(bundles)
}

/// Get a storage bundle by its network_seed
#[hdk_extern]
pub fn get_storage_bundle_by_network_seed(network_seed: String) -> ExternResult<Option<(ActionHash, StorageBundle)>> {
    let network_seed_hash = Path::from(network_seed.clone()).path_entry_hash()?;
    let links = get_links(
        LinkQuery {
            base: network_seed_hash.into(),
            link_type: LinkTypes::NetworkSeedToStorageBundle.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    let mut bundles = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(bundle_info) = record.entry().to_app_option::<StorageBundle>().map_err(|e| wasm_error!(e))? {
                    bundles.push((action_hash, bundle_info));
                }
            }
        }
    }

    if bundles.len() > 1 {
        return Err(wasm_error!(WasmErrorInner::Guest("Multiple storage bundles found for network seed".to_string())));
    }

    Ok(bundles.first().cloned())
}

/// Add an agent as a member of a storage bundle
#[hdk_extern]
pub fn invite_agent_to_bundle(input: InviteAgentToBundleInput) -> ExternResult<ActionHash> {
    // First, get the bundle to ensure it exists
    let bundle = get_storage_bundle_by_network_seed(input.network_seed.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(format!("Bundle not found: {}", input.network_seed))))?;

    let member = BundleMember {
        agent: input.agent.clone(),
        network_seed: input.network_seed.clone(),
        status: MemberStatus::Invited(input.invite_code_base64),
    };

    let action_hash = create_entry(&EntryTypes::BundleMember(member.clone()))?;

    // Create link from bundle to bundle member
    let bundle_tag = create_member_status_tag(&member.status);
    create_link(
        bundle.0.clone(),
        action_hash.clone(),
        LinkTypes::BundleToBundleMembers,
        bundle_tag,
    )?;

    // Create link from agent to their bundle member instance
    let agent_tag = create_member_status_tag(&member.status);
    create_link(
        input.agent.clone(),
        action_hash.clone(),
        LinkTypes::AgentToBundleMembers,
        agent_tag,
    )?;

    // Keep existing link for backward compatibility
    create_link(
        input.agent,
        action_hash.clone(),
        LinkTypes::AgentToStorageBundles,
        input.network_seed.as_bytes().to_vec(),
    )?;

    Ok(action_hash)
}

/// Get all members of a specific bundle
#[hdk_extern]
pub fn get_bundle_members(network_seed: String) -> ExternResult<Vec<(ActionHash, BundleMember)>> {
    let bundle = get_storage_bundle_by_network_seed(network_seed)?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Bundle not found".to_string())))?;

    let links = get_links(
        LinkQuery {
            base: bundle.0.into(),
            link_type: LinkTypes::BundleToBundleMembers.try_into_filter()?,
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

/// Update the status of a bundle member
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateBundleMemberStatusInput {
    pub member_action_hash: ActionHash,
    pub new_status: MemberStatus,
}

#[hdk_extern]
pub fn update_bundle_member_status(input: UpdateBundleMemberStatusInput) -> ExternResult<ActionHash> {
    let record = get(input.member_action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Member not found".to_string())))?;

    let mut member: BundleMember = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Invalid member entry".to_string())))?;

    member.status = input.new_status.clone();

    let new_action_hash = update_entry(input.member_action_hash.clone(), &EntryTypes::BundleMember(member.clone()))?;

    // Update link tags for AgentToBundleMembers
    update_member_links(
        member.agent.clone(),
        input.member_action_hash.clone(),
        new_action_hash.clone(),
        &input.new_status,
        LinkTypes::AgentToBundleMembers,
    )?;

    // Update link tags for BundleToBundleMembers
    if let Some((bundle_action_hash, _)) = get_storage_bundle_by_network_seed(member.network_seed.clone())? {
        update_member_links(
            bundle_action_hash,
            input.member_action_hash.clone(),
            new_action_hash.clone(),
            &input.new_status,
            LinkTypes::BundleToBundleMembers,
        )?;
    }

    Ok(new_action_hash)
}

/// Input for updating member status by network seed
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateMemberStatusByNetworkSeedInput {
    pub network_seed: String,
    pub status: MemberStatus,
}

#[hdk_extern]
pub fn update_member_status_by_network_seed(input: UpdateMemberStatusByNetworkSeedInput) -> ExternResult<ActionHash> {
    let my_agent = agent_info()?.agent_initial_pubkey;

    // Get links from the agent to their bundle members
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

    // Find the member for this specific bundle
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(mut member) = record.entry().to_app_option::<BundleMember>().map_err(|e| wasm_error!(e))? {
                    if member.network_seed == input.network_seed {
                        member.status = input.status;
                        return update_entry(action_hash, &EntryTypes::BundleMember(member));
                    }
                }
            }
        }
    }

    Err(wasm_error!(WasmErrorInner::Guest(format!(
        "No member found for network_seed: {}",
        input.network_seed
    ))))
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
            tag_prefix: Some(LinkTag::new("invited:".as_bytes().to_vec())),
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

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

/// Input for getting bundle members by status
#[derive(Serialize, Deserialize, Debug)]
pub struct GetBundleMembersByStatusInput {
    pub network_seed: String,
    pub status_filter: Option<String>, // "invited", "active", "revoked", or None for all
}

/// Get bundle members filtered by status
#[hdk_extern]
pub fn get_bundle_members_by_status(input: GetBundleMembersByStatusInput) -> ExternResult<Vec<(ActionHash, BundleMember)>> {
    let bundle = get_storage_bundle_by_network_seed(input.network_seed)?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Bundle not found".to_string())))?;

    let tag_prefix = input.status_filter.map(|s| LinkTag::new(format!("{}:", s).as_bytes().to_vec()));

    let links = get_links(
        LinkQuery {
            base: bundle.0.into(),
            link_type: LinkTypes::BundleToBundleMembers.try_into_filter()?,
            tag_prefix,
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
                if let Some(member) = record.entry().to_app_option::<BundleMember>().map_err(|e| wasm_error!(e))? {
                    members.push((action_hash, member));
                }
            }
        }
    }

    Ok(members)
}
