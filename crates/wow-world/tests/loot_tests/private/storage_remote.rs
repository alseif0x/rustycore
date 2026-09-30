//! Original application scenarios over the real persistence lane.
use super::storage_support::*;

#[tokio::test]
async fn remote_master_loot_command_transports_and_commits_claim_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority =
        wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
            .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };
    let granted_item = entry.item_id;
    let inventory_before = applied_loot_item_quantity_for_test(&second, granted_item);
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    install_storage_port(
        &mut second,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        None,
    );
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);

    let request = request_master_loot_store_for_test(
        &first,
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        entry,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        process_pending_for_loot_test(&mut second).await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(
        (applied_loot_item_quantity_for_test(&second, granted_item)) - inventory_before,
        1
    );
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}

#[tokio::test]
async fn remote_roll_winner_command_transports_and_commits_claim_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority =
        wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
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
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };
    let granted_item = entry.item_id;
    let inventory_before = applied_loot_item_quantity_for_test(&second, granted_item);
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    install_storage_port(
        &mut second,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        None,
    );
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);

    let request = request_roll_loot_store_for_test(
        &first,
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        vec![entry],
        false,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        process_pending_for_loot_test(&mut second).await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(
        (applied_loot_item_quantity_for_test(&second, granted_item)) - inventory_before,
        1
    );
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}

#[tokio::test]
async fn detached_remote_claim_waits_for_every_authority_viewer_before_corpse_lifecycle_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    mutate_loot_creature_for_test(&mut first, owner, |creature| {
        creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
    })
    .unwrap();

    let authority =
        wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
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
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };

    // The remote winner closes, while the original looter deliberately
    // keeps the same authoritative Loot window open.
    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let player_registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    player_registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&player_registry));
    second.set_player_registry(player_registry);
    let granted_item = entry.item_id;
    let inventory_before = applied_loot_item_quantity_for_test(&second, granted_item);
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    install_storage_port(
        &mut second,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        None,
    );

    let request = request_roll_loot_store_for_test(
        &first,
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        vec![entry],
        false,
        Some(claim),
    );
    let target = async {
        tokio::task::yield_now().await;
        process_pending_for_loot_test(&mut second).await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(
        (applied_loot_item_quantity_for_test(&second, granted_item)) - inventory_before,
        1
    );
    assert!(
        mutate_loot_creature_for_test(&mut first, owner, |creature| {
            creature.has_lootable_dynamic_flag_like_cpp()
        })
        .unwrap(),
        "the detached winner cannot finish lifecycle while the original viewer remains open"
    );
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .players_looting,
        vec![first_guid]
    );

    first.handle_loot_release(loot_release_packet(owner)).await;
    assert!(
        !mutate_loot_creature_for_test(&mut first, owner, |creature| {
            creature.has_lootable_dynamic_flag_like_cpp()
        })
        .unwrap(),
        "the final real viewer release performs the ordinary C++ corpse transition"
    );
}

#[tokio::test]
async fn remote_disenchant_batch_uses_one_command_and_commits_all_materials_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, _first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority =
        wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
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
    let granted_item = 700;
    install_limited_test_item_template(&mut second, granted_item, 0);
    let inventory_before = applied_loot_item_quantity_for_test(&second, granted_item);
    install_limited_test_item_template(&mut second, 700, 0);
    install_storage_port(
        &mut second,
        PersistenceOutcomeLikeCpp::Applied { rows: 1 },
        None,
    );
    let player_registry = Arc::new(PlayerRegistry::default());
    let (registry_send_tx, _registry_send_rx) = flume::bounded(8);
    let mut second_info = broadcast_info(second_guid, registry_send_tx);
    second_info.command_tx = second.session_command_tx();
    player_registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(player_registry);

    let request = request_roll_loot_store_for_test(
        &first,
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
        process_pending_for_loot_test(&mut second).await;
    };
    let (result, ()) = tokio::join!(request, target);

    assert_eq!(result, MasterLootGiveResult::Stored);
    assert_eq!(
        (applied_loot_item_quantity_for_test(&second, granted_item)) - inventory_before,
        2
    );
    assert!(
        authority
            .reserve_item_for_award_like_cpp(second_guid, 0)
            .await
            .is_err()
    );
}
