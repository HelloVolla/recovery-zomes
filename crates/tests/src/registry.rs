use crate::dna_path::*;
use hdk::prelude::*;
use holochain::conductor::api::CellInfo;
use holochain::prelude::{AppBundleSource, RoleSettings, InstallAppPayload, DnaModifiersOpt, YamlProperties};
use holochain::sweettest::{SweetAgents, SweetConductor, SweetDnaFile, app_bundle_from_dnas};
use shared_types::{Properties, MemberStatus, InviteAgentToBundleInput, InviteMemberInput, SetMemberStatusInput, SetBundleMemberStatusInput, RegistryMemberWithStatus};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use registry_integrity::{BundleMember, MemberStatusEntry};

/// Test that `set_bundle_member_status` correctly creates new `BundleMember` entries
/// for an agent and that `get_bundle_members` deduplicates by agent, returning only
/// the latest entry per agent (no duplicates across multiple status updates).
#[tokio::test(flavor = "multi_thread")]
async fn test_set_bundle_member_status_deduplicates() {
    let network_seed = "test-bundle-member-dedup";
    let (conductor, progenitor, cell_id) =
        setup_registry_single_agent(network_seed).await;

    // Use a fresh fake agent key as the bundle member
    let member_agent = SweetAgents::one(conductor.keystore()).await;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct RegisterBundleInput {
        pub network_seed: String,
        pub progenitor: AgentPubKey,
        pub network_pub_key: Vec<u8>,
        pub dna_hash: Vec<u8>,
    }

    // Register a bundle so invite_agent_to_bundle has something to link from
    let register_input = RegisterBundleInput {
        network_seed: network_seed.to_string(),
        progenitor: progenitor.clone(),
        network_pub_key: vec![0u8; 32],
        dna_hash: cell_id.dna_hash().get_raw_39().to_vec(),
    };
    let _: ActionHash = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "register_storage_bundle",
            register_input,
        )
        .await
        .unwrap();

    // Invite the agent to the bundle (status: Invited)
    let _: ActionHash = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "invite_agent_to_bundle",
            InviteAgentToBundleInput {
                network_seed: network_seed.to_string(),
                agent: member_agent.clone(),
                invite_code_base64: "initial-invite-code".to_string(),
            },
        )
        .await
        .unwrap();

    // Sanity check: one member entry, status Invited
    let members_after_invite: Vec<(ActionHash, BundleMember)> = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "get_bundle_members",
            network_seed.to_string(),
        )
        .await
        .unwrap();
    assert_eq!(members_after_invite.len(), 1, "Expected exactly 1 member after invite");
    assert!(
        matches!(members_after_invite[0].1.status, MemberStatus::Invited(_)),
        "Initial status should be Invited, got {:?}",
        members_after_invite[0].1.status
    );

    // Update status to Active via set_bundle_member_status (adds a second BundleMember entry)
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let _: ActionHash = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "set_bundle_member_status",
            SetBundleMemberStatusInput {
                agent: member_agent.clone(),
                network_seed: network_seed.to_string(),
                status: MemberStatus::Active,
            },
        )
        .await
        .unwrap();

    // Update status to Revoked (adds a third BundleMember entry)
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;
    let _: ActionHash = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "set_bundle_member_status",
            SetBundleMemberStatusInput {
                agent: member_agent.clone(),
                network_seed: network_seed.to_string(),
                status: MemberStatus::Revoked,
            },
        )
        .await
        .unwrap();

    // get_bundle_members must return exactly one entry per agent (deduplicated),
    // reflecting only the latest status update (Revoked).
    let members_final: Vec<(ActionHash, BundleMember)> = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "get_bundle_members",
            network_seed.to_string(),
        )
        .await
        .expect("get_bundle_members should succeed");

    assert_eq!(
        members_final.len(),
        1,
        "get_bundle_members should return exactly 1 entry after 3 status updates (no duplicates), got {}",
        members_final.len()
    );
    assert_eq!(
        members_final[0].1.agent,
        member_agent,
        "Returned member should be the correct agent"
    );
    assert!(
        matches!(members_final[0].1.status, MemberStatus::Revoked),
        "Latest status should be Revoked, got {:?}",
        members_final[0].1.status
    );
}

/// Helper: spin up a single-conductor registry cell as the progenitor.
///
/// Returns `(conductor, agent_pub_key, cell_id)`.
async fn setup_registry_single_agent(
    network_seed: &str,
) -> (SweetConductor, AgentPubKey, holochain::prelude::CellId) {
    let mut conductor = SweetConductor::from_standard_config().await;
    let agent = SweetAgents::one(conductor.keystore()).await;

    let signing_key = SigningKey::generate(&mut OsRng);
    let properties = Properties {
        progenitor_pub_key: agent.clone(),
        network_pub_key: signing_key.verifying_key().to_bytes().to_vec(),
    };
    let yaml_properties = YamlProperties::new(serde_yaml::to_value(&properties).unwrap());

    let dna = SweetDnaFile::from_bundle(&dna_path())
        .await
        .unwrap()
        .with_network_seed(network_seed.to_string())
        .await;

    let bundle = app_bundle_from_dnas(&[dna.clone()], false, None).await;
    let bundle_bytes = bundle.pack().expect("failed to encode app bundle");
    let role_name = dna.dna_hash().to_string();

    let mut roles_settings = std::collections::HashMap::new();
    roles_settings.insert(
        role_name.into(),
        RoleSettings::Provisioned {
            modifiers: Some(DnaModifiersOpt {
                properties: Some(yaml_properties),
                ..Default::default()
            }),
            membrane_proof: None,
        },
    );

    conductor
        .raw_handle()
        .install_app_bundle(InstallAppPayload {
            agent_key: Some(agent.clone()),
            source: AppBundleSource::Bytes(bundle_bytes),
            installed_app_id: Some("test_app".to_string()),
            network_seed: None,
            roles_settings: Some(roles_settings),
            ignore_genesis_failure: false,
        })
        .await
        .unwrap();
    conductor.enable_app("test_app".to_string()).await.unwrap();

    let app_info = conductor
        .raw_handle()
        .get_app_info(&"test_app".to_string())
        .await
        .unwrap()
        .unwrap();
    let cell_id = match &app_info.cell_info.values().next().unwrap()[0] {
        CellInfo::Provisioned(c) => c.cell_id.clone(),
        _ => panic!("Expected provisioned cell"),
    };

    (conductor, agent, cell_id)
}

/// Test that `get_member_and_status` returns the correct combined view after
/// calling `invite_member` (which creates a RegistryMember + initial Invited
/// status) followed by a subsequent `set_member_status` call that marks the
/// member as Active.
#[tokio::test(flavor = "multi_thread")]
async fn test_get_member_and_status_combined_view() {
    let (conductor, progenitor, cell_id) =
        setup_registry_single_agent("test-member-and-status").await;

    // Use a fresh fake agent key as the invitee
    let invitee = SweetAgents::one(conductor.keystore()).await;

    let invite_input = InviteMemberInput {
        agent: invitee.clone(),
        display_name: Some("Alice".to_string()),
        invite_code_base64: "fake-invite-code-base64".to_string(),
    };

    // Invite the member (creates RegistryMember + Invited status)
    let _: ActionHash = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "invite_member",
            invite_input,
        )
        .await
        .unwrap();

    // Now mark the member as Active
    let status_input = SetMemberStatusInput {
        agent: invitee.clone(),
        new_status: MemberStatus::Active,
    };
    let _: ActionHash = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "set_member_status",
            status_input,
        )
        .await
        .unwrap();

    // Retrieve the combined view
    let result: RegistryMemberWithStatus = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "get_member_and_status",
            invitee.clone(),
        )
        .await
        .expect("get_member_and_status should succeed");

    assert_eq!(result.agent, invitee, "Agent key should match the invitee");
    assert_eq!(
        result.display_name,
        Some("Alice".to_string()),
        "Display name should be 'Alice'"
    );
    assert!(
        matches!(result.status, MemberStatus::Active),
        "Status should be Active after the update, got {:?}",
        result.status
    );
}

/// Test that `get_member_status` returns only the most recent status entry when
/// multiple `MemberStatusEntry` records exist for the same agent.
///
/// The sequence committed is: Invited → Active → Revoked.
/// The result should be Revoked.
#[tokio::test(flavor = "multi_thread")]
async fn test_get_member_status_returns_latest() {
    let (conductor, progenitor, cell_id) =
        setup_registry_single_agent("test-member-status-latest").await;

    let invitee = SweetAgents::one(conductor.keystore()).await;

    // Status 1: Invited
    let _: ActionHash = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "set_member_status",
            SetMemberStatusInput {
                agent: invitee.clone(),
                new_status: MemberStatus::Invited("code-abc".to_string()),
            },
        )
        .await
        .unwrap();

    // Small sleep so sys_time() advances between commits
    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Status 2: Active
    let _: ActionHash = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "set_member_status",
            SetMemberStatusInput {
                agent: invitee.clone(),
                new_status: MemberStatus::Active,
            },
        )
        .await
        .unwrap();

    tokio::time::sleep(std::time::Duration::from_millis(5)).await;

    // Status 3: Revoked (should be the one returned)
    let _: ActionHash = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "set_member_status",
            SetMemberStatusInput {
                agent: invitee.clone(),
                new_status: MemberStatus::Revoked,
            },
        )
        .await
        .unwrap();

    // get_member_status returns Option<(ActionHash, MemberStatusEntry)>
    let result: Option<(ActionHash, MemberStatusEntry)> = conductor
        .raw_handle()
        .easy_call_zome(
            &progenitor,
            None,
            cell_id.clone(),
            ZomeName::from("registry"),
            "get_member_status",
            invitee.clone(),
        )
        .await
        .expect("get_member_status should succeed");

    let (_, latest_entry) = result.expect("Should have at least one status entry");

    assert!(
        matches!(latest_entry.status, MemberStatus::Revoked),
        "Most recent status should be Revoked, got {:?}",
        latest_entry.status
    );
    assert_eq!(
        latest_entry.agent, invitee,
        "Status entry should reference the correct agent"
    );
}
