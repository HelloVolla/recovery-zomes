
use hdk::prelude::*;
use registry_integrity::*;

/// Input for registering a new storage bundle
#[derive(Serialize, Deserialize, Debug)]
pub struct RegisterBundleInput {
    pub network_seed: String,
    pub progenitor: AgentPubKey,
    pub network_pub_key: Vec<u8>,
    pub dna_hash: Vec<u8>,
}

/// Get a fixed anchor hash for linking all storage bundles
fn get_storage_bundle_anchor() -> ExternResult<AnyLinkableHash> {
    let path = Path::from("all_storage_bundles");
    let entry_hash = path.path_entry_hash()?;

    Ok(entry_hash.into())
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

/// Register a new storage bundle in the registry
#[hdk_extern]
pub fn register_storage_bundle(input: RegisterBundleInput) -> ExternResult<ActionHash> {
    let bundle_info = StorageBundle {
        network_seed: input.network_seed.clone(),
        progenitor: input.progenitor,
        network_pub_key: input.network_pub_key,
        dna_hash: input.dna_hash,
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
