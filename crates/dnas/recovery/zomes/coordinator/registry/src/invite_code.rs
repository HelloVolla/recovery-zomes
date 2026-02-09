use hdk::prelude::*;
use registry_integrity::*;
use shared_types::{CreateInviteCodeInput, GetInviteCodeByCodeInput};
use crate::storage_bundle::get_storage_bundle_by_network_seed;

/// Create a new invite code for a target agent
#[hdk_extern]
pub fn create_invite_code(input: CreateInviteCodeInput) -> ExternResult<Record> {
    // Look up the bundle to get its action hash for linking
    let bundle = get_storage_bundle_by_network_seed(input.network_seed.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(format!(
            "Bundle not found: {}", input.network_seed
        ))))?;

    let invite_code = InviteCode {
        code: input.code,
        target_agent: input.target_agent.clone(),
        network_seed: input.network_seed,
        status: InviteCodeStatus::Pending,
    };

    let invite_code_action_hash = create_entry(&EntryTypes::InviteCode(invite_code.clone()))?;

    // Create link from target agent to this invite code for easy lookup
    create_link(
        input.target_agent.clone(),
        invite_code_action_hash.clone(),
        LinkTypes::AgentToInviteCodes,
        (),
    )?;

    // Create reverse link from invite code to target agent
    create_link(
        invite_code_action_hash.clone(),
        input.target_agent,
        LinkTypes::InviteCodeToAgent,
        (),
    )?;

    // Create link from bundle to this invite code
    create_link(
        bundle.0,
        invite_code_action_hash.clone(),
        LinkTypes::BundleToInviteCodes,
        (),
    )?;

    let record = get(invite_code_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not find the newly created InviteCode".to_string()
        )))?;

    Ok(record)
}

/// Get all invite codes for the current agent
#[hdk_extern]
pub fn get_my_invite_codes(_: ()) -> ExternResult<Vec<Record>> {
    let agent_info = agent_info()?;
    get_invite_codes_for_agent(agent_info.agent_initial_pubkey)
}

/// Get invite codes for a specific agent
#[hdk_extern]
pub fn get_invite_codes_for_agent(agent: AgentPubKey) -> ExternResult<Vec<Record>> {
    let links = get_links(
        LinkQuery {
            base: agent.into(),
            link_type: LinkTypes::AgentToInviteCodes.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Get invite codes for a specific bundle
#[hdk_extern]
pub fn get_invite_codes_for_bundle(network_seed: String) -> ExternResult<Vec<Record>> {
    let bundle = get_storage_bundle_by_network_seed(network_seed)?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Bundle not found".to_string())))?;

    let links = get_links(
        LinkQuery {
            base: bundle.0.into(),
            link_type: LinkTypes::BundleToInviteCodes.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    let mut records = Vec::new();
    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash, GetOptions::default())? {
                records.push(record);
            }
        }
    }

    Ok(records)
}

/// Get only pending invite codes for the current agent
#[hdk_extern]
pub fn get_my_pending_invite_codes(_: ()) -> ExternResult<Vec<Record>> {
    let all_invites = get_my_invite_codes(())?;

    let pending: Vec<Record> = all_invites
        .into_iter()
        .filter(|record| {
            if let Some(invite) = record.entry().to_app_option::<InviteCode>().ok().flatten() {
                matches!(invite.status, InviteCodeStatus::Pending)
            } else {
                false
            }
        })
        .collect();

    Ok(pending)
}

/// Mark an invite code as fulfilled
#[hdk_extern]
pub fn mark_invite_fulfilled(original_action_hash: ActionHash) -> ExternResult<Record> {
    let record = get(original_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not find the InviteCode to update".to_string()
        )))?;

    let mut invite_code: InviteCode = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Record does not contain an InviteCode entry".to_string()
        )))?;

    invite_code.status = InviteCodeStatus::Fulfilled;

    let updated_action_hash = update_entry(original_action_hash, &EntryTypes::InviteCode(invite_code))?;

    let updated_record = get(updated_action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not find the updated InviteCode".to_string()
        )))?;

    Ok(updated_record)
}

/// Revoke an invite code
#[hdk_extern]
pub fn revoke_invite_code(original_action_hash: ActionHash) -> ExternResult<Record> {
    let record = get(original_action_hash.clone(), GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not find the InviteCode to update".to_string()
        )))?;

    let mut invite_code: InviteCode = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(WasmErrorInner::Guest(e.to_string())))?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Record does not contain an InviteCode entry".to_string()
        )))?;

    invite_code.status = InviteCodeStatus::Revoked;

    let updated_action_hash = update_entry(original_action_hash, &EntryTypes::InviteCode(invite_code))?;

    let updated_record = get(updated_action_hash, GetOptions::default())?
        .ok_or(wasm_error!(WasmErrorInner::Guest(
            "Could not find the updated InviteCode".to_string()
        )))?;

    Ok(updated_record)
}

/// Get an invite code record by its action hash
#[hdk_extern]
pub fn get_invite_code(action_hash: ActionHash) -> ExternResult<Option<Record>> {
    get(action_hash, GetOptions::default())
}

/// Get an invite code by its base64 code string.
///
/// The `code` field in `InviteCode` entries stores the base64-encoded representation
/// of the full `InviteCode` (from `shared_types::InviteCode::to_base64()`).
/// This function searches all invite codes for the specified agent to find a matching code.
#[hdk_extern]
pub fn get_invite_code_by_code(input: GetInviteCodeByCodeInput) -> ExternResult<Option<(ActionHash, InviteCode)>> {
    let records = get_invite_codes_for_agent(input.agent)?;

    for record in records {
        if let Some(invite_code) = record.entry().to_app_option::<InviteCode>().ok().flatten() {
            if invite_code.code == input.code {
                // Get the action hash from the record
                let action_hash = record.action_address().clone();
                return Ok(Some((action_hash, invite_code)));
            }
        }
    }

    Ok(None)
}

/// Mark an invite code as fulfilled by its base64 code string.
///
/// This function looks up the invite code by its base64 string and target agent,
/// then updates its status to `Fulfilled`. This is the preferred way to mark
/// an invite as fulfilled from the client, as it handles the lookup internally.
///
/// # Arguments
///
/// * `input` - Contains the code string, target agent, and bundle_id
///
/// # Returns
///
/// The updated Record on success, or an error if the invite is not found.
#[hdk_extern]
pub fn mark_invite_fulfilled_by_code(input: GetInviteCodeByCodeInput) -> ExternResult<Record> {
    let (action_hash, _invite_code) = get_invite_code_by_code(input.clone())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest(
            format!("Invite code not found for agent {:?}", input.agent)
        )))?;

    mark_invite_fulfilled(action_hash)
}
