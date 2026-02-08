use hdk::prelude::*;
use registry_integrity::*;
use shared_types::AddAgentToBundleInput;

/// Input for registering a new storage bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterBundleInput {
    pub bundle_id: String,
    pub name: String,
    pub description: Option<String>,
}

/// Register a new storage bundle in the registry
#[hdk_extern]
pub fn register_storage_bundle(input: RegisterBundleInput) -> ExternResult<ActionHash> {
    let my_agent = agent_info()?.agent_initial_pubkey;

    let bundle_info = StorageBundleInfo {
        bundle_id: input.bundle_id.clone(),
        name: input.name,
        created_at: sys_time()?,
        description: input.description,
    };

    let action_hash = create_entry(&EntryTypes::StorageBundleInfo(bundle_info))?;

    // Create link from progenitor to bundle
    create_link(
        my_agent,
        action_hash.clone(),
        LinkTypes::ProgenitorToBundles,
        input.bundle_id.as_bytes().to_vec(),  // Store bundle_id in link tag for filtering
    )?;

    Ok(action_hash)
}

/// Get all storage bundles registered by the current agent
#[hdk_extern]
pub fn get_storage_bundles(_: ()) -> ExternResult<Vec<(ActionHash, StorageBundleInfo)>> {
    let my_agent = agent_info()?.agent_initial_pubkey;

    let links = get_links(
        LinkQuery {
            base: my_agent.into(),
            link_type: LinkTypes::ProgenitorToBundles.try_into_filter()?,
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
                if let Some(bundle_info) = record.entry().to_app_option::<StorageBundleInfo>().map_err(|e| wasm_error!(e))? {
                    bundles.push((action_hash, bundle_info));
                }
            }
        }
    }

    Ok(bundles)
}

/// Get a storage bundle by its bundle_id
#[hdk_extern]
pub fn get_storage_bundle_by_id(bundle_id: String) -> ExternResult<Option<(ActionHash, StorageBundleInfo)>> {
    let bundles = get_storage_bundles(())?;

    for (action_hash, bundle_info) in bundles {
        if bundle_info.bundle_id == bundle_id {
            return Ok(Some((action_hash, bundle_info)));
        }
    }

    Ok(None)
}

/// Add an agent as a member of a storage bundle
#[hdk_extern]
pub fn add_agent_to_bundle(input: AddAgentToBundleInput) -> ExternResult<ActionHash> {
    // First, get the bundle to ensure it exists
    let bundle = get_storage_bundle_by_id(input.bundle_id.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(format!("Bundle not found: {}", input.bundle_id))))?;

    let membership = BundleMembership {
        bundle_id: input.bundle_id.clone(),
        member_agent: input.agent.clone(),
        added_at: sys_time()?,
        status: MembershipStatus::Pending,
    };

    let action_hash = create_entry(&EntryTypes::BundleMembership(membership))?;

    // Create link from bundle to membership
    create_link(
        bundle.0.clone(),
        action_hash.clone(),
        LinkTypes::BundleToMembers,
        (),
    )?;

    // Create reverse link from agent to bundle for efficient querying by host
    create_link(
        input.agent,
        action_hash.clone(),
        LinkTypes::AgentToBundles,
        input.bundle_id.as_bytes().to_vec(),
    )?;

    Ok(action_hash)
}

/// Get all members of a specific bundle
#[hdk_extern]
pub fn get_bundle_members(bundle_id: String) -> ExternResult<Vec<(ActionHash, BundleMembership)>> {
    let bundle = get_storage_bundle_by_id(bundle_id)?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Bundle not found".to_string())))?;

    let links = get_links(
        LinkQuery {
            base: bundle.0.into(),
            link_type: LinkTypes::BundleToMembers.try_into_filter()?,
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
                if let Some(membership) = record.entry().to_app_option::<BundleMembership>().map_err(|e| wasm_error!(e))? {
                    members.push((action_hash, membership));
                }
            }
        }
    }

    Ok(members)
}

/// Remove an agent from a bundle
#[hdk_extern]
pub fn remove_agent_from_bundle(membership_action_hash: ActionHash) -> ExternResult<ActionHash> {
    delete_entry(membership_action_hash)
}

/// Update the status of a bundle membership
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateMembershipStatusInput {
    pub membership_action_hash: ActionHash,
    pub new_status: MembershipStatus,
}

#[hdk_extern]
pub fn update_membership_status(input: UpdateMembershipStatusInput) -> ExternResult<ActionHash> {
    let record = get(input.membership_action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Membership not found".to_string())))?;

    let mut membership: BundleMembership = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Invalid membership entry".to_string())))?;

    membership.status = input.new_status;

    update_entry(input.membership_action_hash, &EntryTypes::BundleMembership(membership))
}
