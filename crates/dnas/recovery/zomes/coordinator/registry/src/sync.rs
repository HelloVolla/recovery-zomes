use hdk::prelude::*;
use registry_integrity::*;

/// Get all bundles that the current agent is a member of.
/// This is used by host agents to discover which bundles they need to clone.
#[hdk_extern]
pub fn get_my_assigned_bundles(_: ()) -> ExternResult<Vec<(ActionHash, BundleMembership)>> {
    let my_agent = agent_info()?.agent_initial_pubkey;

    let links = get_links(
        LinkQuery {
            base: my_agent.into(),
            link_type: LinkTypes::AgentToBundles.try_into_filter()?,
            tag_prefix: None,
            before: None,
            after: None,
            author: None,
        },
        GetStrategy::Local,
    )?;

    let mut memberships = Vec::new();

    for link in links {
        if let Some(action_hash) = link.target.into_action_hash() {
            if let Some(record) = get(action_hash.clone(), GetOptions::default())? {
                if let Some(membership) = record.entry().to_app_option::<BundleMembership>().map_err(|e| wasm_error!(e))? {
                    memberships.push((action_hash, membership));
                }
            }
        }
    }

    Ok(memberships)
}

/// Get bundles that an agent should clone but hasn't yet (status = Pending)
#[hdk_extern]
pub fn get_pending_bundles(_: ()) -> ExternResult<Vec<(ActionHash, BundleMembership)>> {
    let my_bundles = get_my_assigned_bundles(())?;

    let pending: Vec<(ActionHash, BundleMembership)> = my_bundles
        .into_iter()
        .filter(|(_, membership)| matches!(membership.status, MembershipStatus::Pending))
        .collect();

    Ok(pending)
}

/// Mark a bundle as cloned/active by updating the membership status.
/// This should be called by a host agent after they've cloned the bundle DNA.
#[hdk_extern]
pub fn mark_bundle_as_cloned(membership_action_hash: ActionHash) -> ExternResult<ActionHash> {
    let record = get(membership_action_hash.clone(), GetOptions::default())?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Membership not found".to_string())))?;

    let mut membership: BundleMembership = record
        .entry()
        .to_app_option()
        .map_err(|e| wasm_error!(e))?
        .ok_or_else(|| wasm_error!(WasmErrorInner::Guest("Invalid membership entry".to_string())))?;

    membership.status = MembershipStatus::Active;

    update_entry(membership_action_hash, &EntryTypes::BundleMembership(membership))
}

/// Data structure for new bundle notification signal
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NewBundleNotification {
    pub bundle_id: String,
    pub bundle_name: String,
    pub membership_action_hash: ActionHash,
}

/// Signal target agents about a new bundle they should clone.
/// This is an optimization - agents can also discover bundles via polling.
#[hdk_extern]
pub fn signal_new_bundle_to_agents(notification: NewBundleNotification) -> ExternResult<()> {
    // Emit the signal for clients to pick up
    emit_signal(&notification)?;
    Ok(())
}

/// Remote callable function for receiving bundle notifications
#[hdk_extern]
pub fn receive_bundle_notification(notification: NewBundleNotification) -> ExternResult<()> {
    // Re-emit as a local signal for the UI/client to handle
    emit_signal(&notification)?;
    Ok(())
}

/// Input for notifying specific agents about a new bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct NotifyAgentsInput {
    pub bundle_id: String,
    pub bundle_name: String,
    pub target_agents: Vec<AgentPubKey>,
    pub membership_action_hashes: Vec<ActionHash>,
}

/// Send remote call to notify agents about a new bundle
#[hdk_extern]
pub fn notify_agents_of_new_bundle(input: NotifyAgentsInput) -> ExternResult<Vec<ExternResult<()>>> {
    let mut results = Vec::new();

    for (agent, membership_hash) in input.target_agents.iter().zip(input.membership_action_hashes.iter()) {
        let notification = NewBundleNotification {
            bundle_id: input.bundle_id.clone(),
            bundle_name: input.bundle_name.clone(),
            membership_action_hash: membership_hash.clone(),
        };

        let result = call_remote(
            agent.clone(),
            zome_info()?.name,
            "receive_bundle_notification".into(),
            None,
            notification,
        );

        match result {
            Ok(ZomeCallResponse::Ok(_)) => results.push(Ok(())),
            Ok(ZomeCallResponse::NetworkError(err)) => {
                // Agent is offline - this is expected, they'll discover via polling
                results.push(Err(wasm_error!(WasmErrorInner::Guest(format!("Network error (agent offline): {}", err)))));
            }
            Ok(other) => {
                results.push(Err(wasm_error!(WasmErrorInner::Guest(format!("Remote call failed: {:?}", other)))));
            }
            Err(e) => results.push(Err(e)),
        }
    }

    Ok(results)
}
