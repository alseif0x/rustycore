//! Persistence scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn cancelled_world_owner_claim_after_commit_reconciles_cache_and_forces_reload_like_cpp() {
    let (mut session, _send_rx, _second, _second_rx, owner_guid, player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(player_guid, 0)
        .await
        .unwrap();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (commit_tx, commit_rx) = tokio::sync::oneshot::channel();
    let guard = session.begin_durable_item_loot_persistence_like_cpp();
    let worker = super::super::spawn_loot_claim_persistence_worker_like_cpp(
        async move {
            let _ = started_tx.send(());
            let _ = commit_rx.await;
            Ok::<(), ()>(())
        },
        Some(claim),
        Some((
            guard,
            DurableItemLootCompletionLikeCpp {
                owner_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: None,
                durable_item_money_notified_amount: None,
                durable_item_money_balance_applied: None,
                item_fanout: None,
                runtime_inventory_applied: Arc::new(AtomicBool::new(false)),
            },
        )),
    )
    .unwrap();
    let waiter = tokio::spawn(async move { worker.await });
    started_rx.await.unwrap();
    waiter.abort();
    let _ = waiter.await;
    commit_tx.send(()).unwrap();

    session.wait_for_active_loot_persistence_like_cpp().await;

    assert!(session.is_disconnecting());
    assert!(
        authority
            .snapshot_for_player_like_cpp(player_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert!(
        session.loot_table.get(&owner_guid).unwrap().items[0].taken,
        "master/roll/direct world-owner grants share this claimed-store recovery path"
    );
}
#[tokio::test]
async fn local_disenchant_batch_commits_all_materials_and_original_claim_like_cpp() {
    let (mut session, rx, _second, _second_rx, owner, player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&rx);
    let shared_send = session.send_tx().clone();
    session.install_realm_send_channel_for_test(shared_send);
    let authority = session
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(player_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(player_guid, generation, 0, false, Some(player_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(player_guid, 0)
        .await
        .unwrap();
    let materials = represented_disenchant_test_outputs_like_cpp(player_guid, 700);
    let grants = Arc::new(AtomicUsize::new(0));
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(
        HighGuid::Item,
        70_000,
    )));
    session.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);

    assert!(
        session
            .store_direct_disenchant_batch_like_cpp(
                &materials,
                0,
                Some(&claim),
                Some(LootItemClaimCommitContextLikeCpp {
                    owner_guid: owner,
                    loot_obj: represented_loot_object_guid_like_cpp(owner),
                    loot_list_id: 0,
                    player_guid,
                    free_for_all: false,
                }),
            )
            .await
    );
    assert_eq!(grants.load(Ordering::SeqCst), 2);
    assert_eq!(
        drain_server_opcodes_like_cpp(&rx),
        vec![
            wow_constants::ServerOpcodes::ItemPushResult as u16,
            wow_constants::ServerOpcodes::ItemPushResult as u16,
            wow_constants::ServerOpcodes::LootRemoved as u16,
        ],
        "C++ Loot::AutoStore sends every material before the original roll slot is removed"
    );
    assert!(claim.is_committed_like_cpp());
    assert!(
        authority
            .reserve_item_for_award_like_cpp(player_guid, 0)
            .await
            .is_err()
    );
}
#[tokio::test]
async fn remote_disenchant_batch_uses_one_command_and_commits_all_materials_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, _first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(second_guid)
        .unwrap()
        .generation;
    authority
        .finish_item_roll_like_cpp(second_guid, generation, 0, false, Some(second_guid))
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let materials = represented_disenchant_test_outputs_like_cpp(second_guid, 700);
    let grants = Arc::new(AtomicUsize::new(0));
    install_limited_test_item_template(&mut second, 700, 0);
    second.set_loot_item_store_test_seam_like_cpp(Arc::clone(&grants), true);
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);

    let request = first.request_represented_remote_loot_roll_winner_store_like_cpp(
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        materials,
        true,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        // One drain handles the complete two-material result. A former
        // implementation required one command/ack round-trip per item.
        second.process_represented_session_commands_like_cpp().await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(grants.load(Ordering::SeqCst), 2);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(second_guid, 0)
            .await
            .is_err()
    );
}
