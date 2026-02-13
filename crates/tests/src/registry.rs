use crate::dna_path::*;
use hdk::prelude::*;
use holochain::conductor::api::CellInfo;
use holochain::prelude::{AppBundleSource, RoleSettings, InstallAppPayload, DnaModifiersOpt, YamlProperties};
use holochain::sweettest::{SweetAgents, SweetConductorBatch, SweetConductorConfig, SweetDnaFile, app_bundle_from_dnas};
use shared_types::{Properties, MemberStatus, InviteAgentToBundleInput, GetInviteCodeByCodeInput};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use registry_integrity::BundleMember;

/// Test marking an invite as fulfilled by code
#[tokio::test(flavor = "multi_thread")]
async fn test_mark_invite_fulfilled_by_code() {
    let conductors = SweetConductorBatch::from_config(2, SweetConductorConfig::standard()).await;

    let agents = [
        SweetAgents::one(conductors[0].keystore()).await,
        SweetAgents::one(conductors[1].keystore()).await,
    ];

    let network_seed = "test-invite-fulfillment".to_string();
    let signing_key = SigningKey::generate(&mut OsRng);

    let properties = Properties {
        progenitor_pub_key: agents[0].clone(),
        network_pub_key: signing_key.verifying_key().to_bytes().to_vec(),
    };
    let yaml_properties = YamlProperties::new(serde_yaml::to_value(&properties).unwrap());

    let dna = SweetDnaFile::from_bundle(&dna_path())
        .await
        .unwrap()
        .with_network_seed(network_seed.clone())
        .await;

    let bundle = app_bundle_from_dnas(&[dna.clone()], false, None).await;
    let bundle_bytes = bundle.pack().expect("failed to encode app bundle");
    let role_name = dna.dna_hash().to_string();

    let mut roles_settings0 = std::collections::HashMap::new();
    roles_settings0.insert(
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
            roles_settings: Some(roles_settings0),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[0].enable_app("test_app".to_string()).await.unwrap();

    let app0_info = conductors[0]
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell0_id = match &app0_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    // Commit private signing key to progenitor's source chain (for membrane proof generation)
    let signing_key_seed = signing_key.to_bytes().to_vec();
    let _private_action_hash: ActionHash = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell0_id.clone(), ZomeName::from("membrane"), "commit_private_signing_key", signing_key_seed)
        .await
        .unwrap();

    let _: String = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell0_id.clone(), ZomeName::from("storage"), "ping", serde_bytes::ByteBuf::from(vec![]))
        .await
        .unwrap();

    let membrane_proof_data = shared_types::MembraneProofData {
        network_seed: network_seed.clone(),
        for_agent: agents[1].clone(),
        network_pub_key: signing_key.verifying_key().to_bytes().to_vec(),
    };

    let proof_bytes: SerializedBytes = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell0_id.clone(), ZomeName::from("membrane"), "generate_membrane_proof", membrane_proof_data)
        .await
        .expect("generate_membrane_proof should succeed");

    let membrane_proof = MembraneProof::from(proof_bytes);

    // Install for invitee (agent 1) - effectively joining the registry network
    let mut roles_settings1 = std::collections::HashMap::new();
    roles_settings1.insert(
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
            roles_settings: Some(roles_settings1),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductors[1].enable_app("test_app".to_string()).await.unwrap();

    let app1_info = conductors[1]
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell1_id = match &app1_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    // Exchange peer info
    conductors.exchange_peer_info().await;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct RegisterBundleInput {
        pub network_seed: String,
        pub progenitor: AgentPubKey,
        pub network_pub_key: Vec<u8>,
    }

    let register_start_input = RegisterBundleInput {
        network_seed: network_seed.clone(),
        progenitor: agents[0].clone(),
        network_pub_key: signing_key.verifying_key().to_bytes().to_vec(),
    };

    let _bundle_hash: ActionHash = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell0_id.clone(), ZomeName::from("registry"), "register_storage_bundle", register_start_input)
        .await
        .unwrap();

    let invite_code_str = "base64invitecodestring".to_string();
    let invite_input = InviteAgentToBundleInput {
        network_seed: network_seed.clone(),
        agent: agents[1].clone(),
        invite_code_base64: invite_code_str.clone(),
    };

    let _member_hash: ActionHash = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell0_id.clone(), ZomeName::from("registry"), "invite_agent_to_bundle", invite_input)
        .await
        .unwrap();

    // Verify status is Invited
    // Progenitor checks
    let members: Vec<(ActionHash, BundleMember)> = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell0_id.clone(), ZomeName::from("registry"), "get_agent_bundle_members", agents[1].clone())
        .await
        .unwrap();

    assert_eq!(members.len(), 1);
    match &members[0].1.status {
        MemberStatus::Invited(code) => assert_eq!(code, &invite_code_str),
        _ => panic!("Expected Invited status"),
    }

    let fulfill_input = GetInviteCodeByCodeInput {
        code: invite_code_str.clone(),
        agent: agents[1].clone(),
    };

    let _update_hash: ActionHash = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell0_id.clone(), ZomeName::from("registry"), "mark_invite_fulfilled_by_code", fulfill_input)
        .await
        .unwrap();

    let members_updated: Vec<(ActionHash, BundleMember)> = conductors[0]
        .raw_handle()
        .easy_call_zome(&agents[0], None, cell0_id.clone(), ZomeName::from("registry"), "get_agent_bundle_members", agents[1].clone())
        .await
        .unwrap();

    match &members_updated[0].1.status {
        MemberStatus::Active => (),
        _ => panic!("Expected Active status after fulfillment"),
    }
}
