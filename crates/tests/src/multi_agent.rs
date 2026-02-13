use crate::dna_path::*;
use hdk::prelude::*;
use holochain::conductor::api::CellInfo;
use holochain::prelude::{AppBundleSource, RoleSettings, InstallAppPayload, DnaModifiersOpt, YamlProperties};
use holochain::sweettest::{SweetAgents, SweetConductorBatch, SweetConductorConfig, SweetDnaFile, app_bundle_from_dnas};
use shared_types::{MembraneProofData, Properties, RemoteCallInput};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;

/// Helper function to generate membrane proof using the zome function.
/// This calls the progenitor's generate_membrane_proof zome function.
async fn generate_membrane_proof_via_zome(
    conductor: &holochain::sweettest::SweetConductor,
    agent: &AgentPubKey,
    cell_id: holochain::prelude::CellId,
    for_agent: AgentPubKey,
    network_pub_key: Vec<u8>,
    network_seed: String,
) -> MembraneProof {
    let membrane_proof_data = MembraneProofData {
        network_seed,
        for_agent,
        network_pub_key,
    };

    let proof_bytes: SerializedBytes = conductor
        .raw_handle()
        .easy_call_zome(agent, None, cell_id, ZomeName::from("membrane"), "generate_membrane_proof", membrane_proof_data)
        .await
        .expect("generate_membrane_proof should succeed");

    MembraneProof::from(proof_bytes)
}


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

    // Generate an ed25519 signing keypair for membrane proof signing
    let signing_key = SigningKey::generate(&mut OsRng);
    // Note: signing_pub_key no longer in Properties - it goes in the membrane proof envelope

    // Create properties with agent[0] as the progenitor only
    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
        network_pub_key: signing_key.verifying_key().to_bytes().to_vec(),
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

    // Commit private signing key to progenitor's source chain (for membrane proof generation)
    let signing_key_seed = signing_key.to_bytes().to_vec();
    let _private_action_hash: ActionHash = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("membrane"), "commit_private_signing_key", signing_key_seed)
        .await
        .unwrap();

    // Verify agent1 can make zome calls
    let ping_response: String = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("storage"), "ping", serde_bytes::ByteBuf::from(vec![]))
        .await
        .unwrap();
    assert_eq!(ping_response, "pong");

    // Create membrane proof for agent2 using the zome function
    let membrane_proof = generate_membrane_proof_via_zome(
        &conductors[0],
        &agents[0],
        cell1_id.clone(),
        agents[1].clone(),
        signing_key.verifying_key().to_bytes().to_vec(),
        network_seed.clone(),
    ).await;

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

    let ping_response2: String = conductors[1]
        .raw_handle()
        .easy_call_zome(&agents[1], None, cell2_id.clone(), ZomeName::from("storage"), "ping", serde_bytes::ByteBuf::from(vec![]))
        .await
        .unwrap();
    assert_eq!(ping_response2, "pong");

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

    // Generate an ed25519 signing keypair
    let signing_key = SigningKey::generate(&mut OsRng);
    // signing_pub_key now goes in membrane proof, not properties

    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
        network_pub_key: signing_key.verifying_key().to_bytes().to_vec(),
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

    // Generate an ed25519 signing keypair
    let signing_key = SigningKey::generate(&mut OsRng);
    // signing_pub_key now goes in membrane proof, not properties

    // Create properties with agent[0] as the progenitor only
    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
        network_pub_key: signing_key.verifying_key().to_bytes().to_vec(),
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

    // Commit private signing key to progenitor's source chain (for membrane proof generation)
    let signing_key_seed = signing_key.to_bytes().to_vec();
    let _private_action_hash: ActionHash = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("membrane"), "commit_private_signing_key", signing_key_seed)
        .await
        .unwrap();

    // Create membrane proof for agent2 using the zome function
    let membrane_proof = generate_membrane_proof_via_zome(
        &conductors[0],
        &agents[0],
        cell1_id.clone(),
        agents[1].clone(),
        signing_key.verifying_key().to_bytes().to_vec(),
        network_seed.clone(),
    ).await;

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
        payload: ExternIO::encode("world".to_string()).unwrap(),
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

    // Generate an ed25519 signing keypair
    let signing_key = SigningKey::generate(&mut OsRng);
    // signing_pub_key now goes in membrane proof, not properties

    // Create properties with agent[0] as the progenitor only
    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
        network_pub_key: signing_key.verifying_key().to_bytes().to_vec(),
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

    // Commit private signing key to progenitor's source chain (for membrane proof generation)
    let signing_key_seed = signing_key.to_bytes().to_vec();
    let _private_action_hash: ActionHash = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("membrane"), "commit_private_signing_key", signing_key_seed)
        .await
        .unwrap();

    // Create membrane proof for agent2 using the zome function
    let membrane_proof = generate_membrane_proof_via_zome(
        &conductors[0],
        &agents[0],
        cell1_id.clone(),
        agents[1].clone(),
        signing_key.verifying_key().to_bytes().to_vec(),
        network_seed.clone(),
    ).await;

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
        payload: ExternIO::encode(serde_bytes::ByteBuf::from(vec![])).unwrap(),
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
    use agent_joined_integrity::AgentJoined;

    // Set up two separate conductors for two agents
    let conductors = SweetConductorBatch::from_config(2, SweetConductorConfig::standard()).await;

    // Generate agents for each conductor
    let agents = [
        SweetAgents::one(conductors[0].keystore()).await,
        SweetAgents::one(conductors[1].keystore()).await
    ];

    // Create the network seed (bundle_id) for the shared DNA
    let network_seed = "test-profile-verification".to_string();

    // Generate an ed25519 signing keypair
    let signing_key = SigningKey::generate(&mut OsRng);
    // signing_pub_key now goes in membrane proof, not properties

    // Create properties with agent[0] as the progenitor only
    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
        network_pub_key: signing_key.verifying_key().to_bytes().to_vec(),
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

    // Commit private signing key to progenitor's source chain (for membrane proof generation)
    let signing_key_seed = signing_key.to_bytes().to_vec();
    let _private_action_hash: ActionHash = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("membrane"), "commit_private_signing_key", signing_key_seed)
        .await
        .unwrap();

    // Create membrane proof for agent2 using the zome function
    let membrane_proof = generate_membrane_proof_via_zome(
        &conductors[0],
        &agents[0],
        cell1_id.clone(),
        agents[1].clone(),
        signing_key.verifying_key().to_bytes().to_vec(),
        network_seed.clone(),
    ).await;

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

    // Agent2 signals that they have joined
    let _action_hash: ActionHash = conductors[1]
        .raw_handle()
        .easy_call_zome(&agents[1], None, cell2_id.clone(), ZomeName::from("agent_joined"), "signal_joined", ())
        .await
        .expect("Agent2 should be able to signal joined");

    // Give some time for gossip to propagate
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // Agent1 queries for all joined agents
    let joined_agents: Vec<(ActionHash, AgentJoined)> = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("agent_joined"), "get_joined_agents", ())
        .await
        .expect("Should be able to get joined agents");

    // Verify agent2 is in the list
    assert!(
        joined_agents.iter().any(|(_, entry)| entry.agent == agents[1]),
        "Agent2 should appear in the list of joined agents"
    );

    // Agent1 directly checks if agent2 joined
    let has_joined: bool = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell1_id.clone(), ZomeName::from("agent_joined"), "has_agent_joined", agents[1].clone())
        .await
        .expect("Should be able to check if agent joined");

    // Verify the check returns true
    assert!(has_joined, "Agent2 should be found - this confirms they successfully joined");
}
