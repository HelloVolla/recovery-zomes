use crate::dna_path::*;
use hdk::prelude::*;
use holochain::conductor::api::CellInfo;
use holochain::prelude::{AppBundleSource, RoleSettings, InstallAppPayload, DnaModifiersOpt, YamlProperties};
use holochain::sweettest::{SweetAgents, SweetConductorBatch, SweetConductorConfig, SweetDnaFile, app_bundle_from_dnas};
use shared_types::{MembraneProofData, Properties, RemoteCallInput};

/// Test that two agents can connect through a DNA with membrane proof validation.
/// The progenitor (first agent) creates the bundle and generates a membrane proof
/// for the second agent, who then joins with that proof.
#[tokio::test(flavor = "multi_thread")]
async fn test_multi_agent_membrane_proof_connection() {
    // Set up two separate conductors for two agents
    let conductors = SweetConductorBatch::from_config(2, SweetConductorConfig::standard()).await;

    // Generate agents for each conductor
    let agents = [
        SweetAgents::one(conductors[0].keystore()).await,
        SweetAgents::one(conductors[1].keystore()).await
    ];

    // Create the network seed (bundle_id) for the shared DNA
    let network_seed = "test-bundle-12345".to_string();

    // Create properties with agent[0] as the progenitor
    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
    };
    let yaml_properties = YamlProperties::new(serde_yaml::to_value(&properties).unwrap());

    // Load the DNA from the bundle path and configure it with network seed
    let dna = SweetDnaFile::from_bundle(&dna_path())
        .await
        .unwrap()
        .with_network_seed(network_seed.clone())
        .await;

    // Create bundle for agent1 (progenitor - no membrane proof needed)
    let bundle = app_bundle_from_dnas(&[dna.clone()], false, None).await;
    let bundle_bytes = bundle.pack().expect("failed to encode app bundle");
    let role_name = dna.dna_hash().to_string();

    // Create install payload for agent1 with properties in modifiers
    let mut roles_settings1 = std::collections::HashMap::new();
    roles_settings1.insert(
        role_name.clone().into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties.clone()),
                ..Default::default()
            }),
            membrane_proof: None,
        },
    );

    conductors[0]
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agents[0].clone()),
            source: AppBundleSource::Bytes(bundle_bytes.clone()),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings1),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[0].enable_app("test_app".to_string()).await.unwrap();

    // Get cell1 from app info
    let app1_info = conductors[0]
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell1_id = match &app1_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    // Verify agent1 can make zome calls
    let hello_response: String = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("storage"), "hello_world", "Agent1".to_string())
        .await
        .unwrap();
    assert!(hello_response.contains("Agent1"));

    // Agent1 generates a membrane proof for agent2
    let membrane_proof_data = MembraneProofData {
        bundle_id: network_seed.clone(),
        for_agent: agents[1].clone(),
    };
    let membrane_proof_bytes: SerializedBytes = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("storage"), "generate_membrane_proof", membrane_proof_data)
        .await
        .unwrap();
    let membrane_proof = MembraneProof::from(membrane_proof_bytes);

    // Install for agent2 with membrane proof
    let mut roles_settings2 = std::collections::HashMap::new();
    roles_settings2.insert(
        role_name.clone().into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties.clone()),
                ..Default::default()
            }),
            membrane_proof: Some(membrane_proof),
        },
    );

    conductors[1]
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agents[1].clone()),
            source: AppBundleSource::Bytes(bundle_bytes.clone()),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings2),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[1].enable_app("test_app".to_string()).await.unwrap();

    // Get cell2 and verify agent2 can make zome calls
    let app2_info = conductors[1]
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell2_id = match &app2_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    let hello_response2: String = conductors[1]
        .raw_handle()
        .easy_call_zome(&agents[1], None, cell2_id.clone(), ZomeName::from("storage"), "hello_world", "Agent2".to_string())
        .await
        .unwrap();
    assert!(hello_response2.contains("Agent2"));

    // Verify both agents are on the same DNA
    assert_eq!(cell1_id.dna_hash(), cell2_id.dna_hash(), "Both agents should be on the same DNA");
    assert_ne!(cell1_id.agent_pubkey(), cell2_id.agent_pubkey(), "Agents should have different public keys");
}

/// Test that an agent without a valid membrane proof cannot join the network
#[tokio::test(flavor = "multi_thread")]
async fn test_membrane_proof_required_for_non_progenitor() {
    // Set up two separate conductors for two agents
    let conductors = SweetConductorBatch::from_config(2, SweetConductorConfig::standard()).await;

    // Generate agents for each conductor
    let agents = [
        SweetAgents::one(conductors[0].keystore()).await,
        SweetAgents::one(conductors[1].keystore()).await
    ];

    // Create the network seed and properties with agent[0] as the progenitor
    let network_seed = "test-bundle-rejection-test".to_string();
    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
    };
    let yaml_properties = YamlProperties::new(serde_yaml::to_value(&properties).unwrap());

    // Load the DNA and configure it with network seed
    let dna = SweetDnaFile::from_bundle(&dna_path())
        .await
        .unwrap()
        .with_network_seed(network_seed.clone())
        .await;

    // Create bundle
    let bundle = app_bundle_from_dnas(&[dna.clone()], false, None).await;
    let bundle_bytes = bundle.pack().expect("failed to encode app bundle");
    let role_name = dna.dna_hash().to_string();

    // Install for agent1 (progenitor)
    let mut roles_settings1 = std::collections::HashMap::new();
    roles_settings1.insert(
        role_name.clone().into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties.clone()),
                ..Default::default()
            }),
            membrane_proof: None,
        },
    );

    conductors[0]
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agents[0].clone()),
            source: AppBundleSource::Bytes(bundle_bytes.clone()),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings1),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[0].enable_app("test_app".to_string()).await.unwrap();

    // Agent2 tries to install WITHOUT a membrane proof (should fail)
    let mut roles_settings2 = std::collections::HashMap::new();
    roles_settings2.insert(
        role_name.clone().into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties.clone()),
                ..Default::default()
            }),
            membrane_proof: None, // No membrane proof - this should cause failure
        },
    );

    let result = conductors[1]
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agents[1].clone()),
            source: AppBundleSource::Bytes(bundle_bytes),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings2),
            ignore_genesis_failure: false,
        })
        .await;

    assert!(result.is_err(), "Non-progenitor agent without membrane proof should fail to join");
}

/// Test that calling a function without a cap grant fails for remote agents
#[tokio::test(flavor = "multi_thread")]
async fn test_remote_call_without_cap_grant_fails() {
    // Set up two separate conductors for two agents
    let conductors = SweetConductorBatch::from_config(2, SweetConductorConfig::standard()).await;

    // Generate agents for each conductor
    let agents = [
        SweetAgents::one(conductors[0].keystore()).await,
        SweetAgents::one(conductors[1].keystore()).await
    ];

    // Create the network seed (bundle_id) for the shared DNA
    let network_seed = "test-no-cap-grant".to_string();

    // Create properties with agent[0] as the progenitor
    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
    };
    let yaml_properties = YamlProperties::new(serde_yaml::to_value(&properties).unwrap());

    // Load the DNA from the bundle path and configure it with network seed
    let dna = SweetDnaFile::from_bundle(&dna_path())
        .await
        .unwrap()
        .with_network_seed(network_seed.clone())
        .await;

    // Create bundle for agent1 (progenitor - no membrane proof needed)
    let bundle = app_bundle_from_dnas(&[dna.clone()], false, None).await;
    let bundle_bytes = bundle.pack().expect("failed to encode app bundle");
    let role_name = dna.dna_hash().to_string();

    // Create install payload for agent1 with properties in modifiers
    let mut roles_settings1 = std::collections::HashMap::new();
    roles_settings1.insert(
        role_name.clone().into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties.clone()),
                ..Default::default()
            }),
            membrane_proof: None,
        },
    );

    conductors[0]
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agents[0].clone()),
            source: AppBundleSource::Bytes(bundle_bytes.clone()),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings1),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[0].enable_app("test_app".to_string()).await.unwrap();

    // Get cell1 from app info
    let app1_info = conductors[0]
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell1_id = match &app1_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    // Agent1 generates a membrane proof for agent2
    let membrane_proof_data = MembraneProofData {
        bundle_id: network_seed.clone(),
        for_agent: agents[1].clone(),
    };
    let membrane_proof_bytes: SerializedBytes = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("storage"), "generate_membrane_proof", membrane_proof_data)
        .await
        .unwrap();
    let membrane_proof = MembraneProof::from(membrane_proof_bytes);

    // Install for agent2 with membrane proof
    let mut roles_settings2 = std::collections::HashMap::new();
    roles_settings2.insert(
        role_name.clone().into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties.clone()),
                ..Default::default()
            }),
            membrane_proof: Some(membrane_proof),
        },
    );

    conductors[1]
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agents[1].clone()),
            source: AppBundleSource::Bytes(bundle_bytes.clone()),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings2),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[1].enable_app("test_app".to_string()).await.unwrap();

    // Get cell2
    let app2_info = conductors[1]
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell2_id = match &app2_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    // Exchange peer info so the agents can find each other
    conductors.exchange_peer_info().await;

    // Agent2 tries to call hello_world on agent1 (should fail - no cap grant)
    let remote_call_input = RemoteCallInput {
        target_agent: agents[0].clone(),
        zome_name: "storage".to_string(),
        fn_name: "hello_world".to_string(),
        payload: ExternIO::encode("Remote").unwrap(),
        cap_secret: None,
    };

    let result: Result<ExternIO, _> = conductors[1]
        .raw_handle()
        .easy_call_zome(&agents[1], None, cell2_id.clone(), ZomeName::from("storage"), "call_remote_zome", remote_call_input)
        .await;

    // The call should fail because hello_world has no cap grant for remote callers
    assert!(result.is_err(), "Remote call to hello_world should fail due to missing cap grant");
}

/// Test that calling a function with an unrestricted cap grant succeeds for remote agents
#[tokio::test(flavor = "multi_thread")]
async fn test_remote_call_with_unrestricted_cap_grant_succeeds() {
    // Set up two separate conductors for two agents
    let conductors = SweetConductorBatch::from_config(2, SweetConductorConfig::standard()).await;

    // Generate agents for each conductor
    let agents = [
        SweetAgents::one(conductors[0].keystore()).await,
        SweetAgents::one(conductors[1].keystore()).await
    ];

    // Create the network seed (bundle_id) for the shared DNA
    let network_seed = "test-unrestricted-cap-grant".to_string();

    // Create properties with agent[0] as the progenitor
    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
    };
    let yaml_properties = YamlProperties::new(serde_yaml::to_value(&properties).unwrap());

    // Load the DNA from the bundle path and configure it with network seed
    let dna = SweetDnaFile::from_bundle(&dna_path())
        .await
        .unwrap()
        .with_network_seed(network_seed.clone())
        .await;

    // Create bundle for agent1 (progenitor - no membrane proof needed)
    let bundle = app_bundle_from_dnas(&[dna.clone()], false, None).await;
    let bundle_bytes = bundle.pack().expect("failed to encode app bundle");
    let role_name = dna.dna_hash().to_string();

    // Create install payload for agent1 with properties in modifiers
    let mut roles_settings1 = std::collections::HashMap::new();
    roles_settings1.insert(
        role_name.clone().into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties.clone()),
                ..Default::default()
            }),
            membrane_proof: None,
        },
    );

    conductors[0]
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agents[0].clone()),
            source: AppBundleSource::Bytes(bundle_bytes.clone()),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings1),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[0].enable_app("test_app".to_string()).await.unwrap();

    // Get cell1 from app info
    let app1_info = conductors[0]
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell1_id = match &app1_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    // Agent1 generates a membrane proof for agent2
    let membrane_proof_data = MembraneProofData {
        bundle_id: network_seed.clone(),
        for_agent: agents[1].clone(),
    };
    let membrane_proof_bytes: SerializedBytes = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("storage"), "generate_membrane_proof", membrane_proof_data)
        .await
        .unwrap();
    let membrane_proof = MembraneProof::from(membrane_proof_bytes);

    // Install for agent2 with membrane proof
    let mut roles_settings2 = std::collections::HashMap::new();
    roles_settings2.insert(
        role_name.clone().into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties.clone()),
                ..Default::default()
            }),
            membrane_proof: Some(membrane_proof),
        },
    );

    conductors[1]
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agents[1].clone()),
            source: AppBundleSource::Bytes(bundle_bytes.clone()),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings2),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[1].enable_app("test_app".to_string()).await.unwrap();

    // Get cell2
    let app2_info = conductors[1]
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell2_id = match &app2_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    // Exchange peer info so the agents can find each other
    conductors.exchange_peer_info().await;

    // Agent2 tries to call ping on agent1 (should succeed - unrestricted cap grant)
    let remote_call_input = RemoteCallInput {
        target_agent: agents[0].clone(),
        zome_name: "storage".to_string(),
        fn_name: "ping".to_string(),
        payload: ExternIO::encode(()).unwrap(),
        cap_secret: None,
    };

    let result: ExternIO = conductors[1]
        .raw_handle()
        .easy_call_zome(&agents[1], None, cell2_id.clone(), ZomeName::from("storage"), "call_remote_zome", remote_call_input)
        .await
        .expect("Remote call to ping should succeed with unrestricted cap grant");

    // The call should succeed and return "pong"
    let pong: String = result.decode().expect("Failed to decode ping response");
    assert_eq!(pong, "pong", "Ping should return 'pong'");
}

/// Test that agent join can be verified through profile creation.
///
/// This test validates the agent join verification mechanism:
/// 1. Agent1 (progenitor) creates a storage bundle
/// 2. Agent1 generates an invite code for agent2
/// 3. Agent2 joins with the invite code
/// 4. Agent2 creates a profile with a nickname
/// 5. Agent1 queries for agents with profiles
/// 6. Agent1 verifies agent2's profile exists
#[tokio::test(flavor = "multi_thread")]
async fn test_agent_join_verification_via_profile() {
    use std::collections::BTreeMap;

    // Profile struct matching profiles zome
    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
    struct Profile {
        nickname: String,
        fields: BTreeMap<String, String>,
    }

    // ZomeFnInput wrapper for profiles zome
    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
    struct ZomeFnInput<T> {
        input: T,
        #[serde(skip_serializing_if = "Option::is_none")]
        local: Option<bool>,
    }

    // Set up two separate conductors for two agents
    let conductors = SweetConductorBatch::from_config(2, SweetConductorConfig::standard()).await;

    // Generate agents for each conductor
    let agents = [
        SweetAgents::one(conductors[0].keystore()).await,
        SweetAgents::one(conductors[1].keystore()).await
    ];

    // Create the network seed (bundle_id) for the shared DNA
    let network_seed = "test-profile-verification".to_string();

    // Create properties with agent[0] as the progenitor
    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
    };
    let yaml_properties = YamlProperties::new(serde_yaml::to_value(&properties).unwrap());

    // Load the DNA from the bundle path and configure it with network seed
    let dna = SweetDnaFile::from_bundle(&dna_path())
        .await
        .unwrap()
        .with_network_seed(network_seed.clone())
        .await;

    // Create bundle for agent1 (progenitor - no membrane proof needed)
    let bundle = app_bundle_from_dnas(&[dna.clone()], false, None).await;
    let bundle_bytes = bundle.pack().expect("failed to encode app bundle");
    let role_name = dna.dna_hash().to_string();

    // Create install payload for agent1 with properties in modifiers
    let mut roles_settings1 = std::collections::HashMap::new();
    roles_settings1.insert(
        role_name.clone().into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties.clone()),
                ..Default::default()
            }),
            membrane_proof: None,
        },
    );

    conductors[0]
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agents[0].clone()),
            source: AppBundleSource::Bytes(bundle_bytes.clone()),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings1),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[0].enable_app("test_app".to_string()).await.unwrap();

    // Get cell1 from app info
    let app1_info = conductors[0]
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell1_id = match &app1_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    // Agent1 generates a membrane proof for agent2
    let membrane_proof_data = MembraneProofData {
        bundle_id: network_seed.clone(),
        for_agent: agents[1].clone(),
    };
    let membrane_proof_bytes: SerializedBytes = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("storage"), "generate_membrane_proof", membrane_proof_data)
        .await
        .unwrap();
    let membrane_proof = MembraneProof::from(membrane_proof_bytes);

    // Install for agent2 with membrane proof
    let mut roles_settings2 = std::collections::HashMap::new();
    roles_settings2.insert(
        role_name.clone().into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties.clone()),
                ..Default::default()
            }),
            membrane_proof: Some(membrane_proof),
        },
    );

    conductors[1]
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agents[1].clone()),
            source: AppBundleSource::Bytes(bundle_bytes.clone()),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings2),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[1].enable_app("test_app".to_string()).await.unwrap();

    // Get cell2
    let app2_info = conductors[1]
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell2_id = match &app2_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    // Exchange peer info so the agents can find each other
    conductors.exchange_peer_info().await;

    // Agent2 creates a profile to indicate they have joined
    let profile = Profile {
        nickname: "Agent2JoinedSuccessfully".to_string(),
        fields: BTreeMap::new(),
    };

    let _profile_record: Record = conductors[1]
        .raw_handle()
        .easy_call_zome(&agents[1], None, cell2_id.clone(), ZomeName::from("profiles"), "create_profile", profile)
        .await
        .expect("Agent2 should be able to create a profile");

    // Give some time for gossip to propagate
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Agent1 queries for all agents with profiles
    let input = ZomeFnInput::<()> { input: (), local: None };
    let agents_with_profiles: Vec<AgentPubKey> = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("profiles"), "get_agents_with_profile", input)
        .await
        .expect("Should be able to get agents with profile");

    // Verify agent2 is in the list
    assert!(
        agents_with_profiles.contains(&agents[1]),
        "Agent2 should appear in the list of agents with profiles"
    );

    // Agent1 directly queries for agent2's profile
    let input = ZomeFnInput { input: agents[1].clone(), local: None };
    let agent2_profile: Option<Record> = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("profiles"), "get_agent_profile", input)
        .await
        .expect("Should be able to get agent profile");

    // Verify the profile exists
    assert!(agent2_profile.is_some(), "Agent2's profile should be found - this confirms they successfully joined");
}
