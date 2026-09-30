//! Original application scenarios over the real persistence lane.
use super::storage_support::*;

#[tokio::test]
async fn remote_roll_timeout_then_release_fans_out_once_and_finalizes_corpse_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    mutate_loot_creature_for_test(&mut first, owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
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

    // The target no longer has a live loot window. The source closes its
    // window only after the remote request times out with the target's
    // detached persistence worker still owning the claim.
    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));
    second.set_player_registry(registry);

    let granted_item = entry.item_id;
    let inventory_before = applied_loot_item_quantity_for_test(&second, granted_item);
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    let commit_gate = Arc::new(tokio::sync::Notify::new());
    install_storage_port(&mut second, PersistenceOutcomeLikeCpp::Applied { rows: 1 }, Some(Arc::clone(&commit_gate)));

    let mut request = Box::pin(
        request_roll_loot_store_for_test(&first, 
            second_guid,
            owner,
            represented_loot_object_guid_like_cpp(owner),
            0,
            0,
            vec![entry],
            false,
            Some(claim),
        ),
    );
    let mut target = Box::pin(async {
        tokio::task::yield_now().await;
        process_pending_for_loot_test(&mut second).await;
    });
    let result = tokio::select! {
        result = &mut request => result,
        _ = &mut target => panic!("target must remain behind the COMMIT gate"),
    };
    drop(request);
    assert_eq!(result, MasterLootGiveResult::TargetMismatch);

    first.handle_loot_release(loot_release_packet(owner)).await;
    commit_gate.notify_one();
    target.await;

    assert_eq!((applied_loot_item_quantity_for_test(&second, granted_item)) - inventory_before, 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken,
        "the roll claim is terminal after the durable commit"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&first_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRemoved as u16)
            .count(),
        1,
        "the pre-COMMIT route publishes the roll removal exactly once"
    );
    assert!(
        !mutate_loot_creature_for_test(&mut first, owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "post-COMMIT completion must finish the corpse lifecycle without an active view"
    );
}

#[tokio::test]
async fn remote_disenchant_timeout_then_release_fans_out_once_and_finalizes_corpse_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    mutate_loot_creature_for_test(&mut first, owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
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

    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));
    second.set_player_registry(registry);

    let granted_item = 700;
    install_limited_test_item_template(&mut second, granted_item, 0);
    let inventory_before = applied_loot_item_quantity_for_test(&second, granted_item);
    install_limited_test_item_template(&mut second, 700, 0);
    let commit_gate = Arc::new(tokio::sync::Notify::new());
    install_storage_port(&mut second, PersistenceOutcomeLikeCpp::Applied { rows: 1 }, Some(Arc::clone(&commit_gate)));

    let mut request = Box::pin(
        request_roll_loot_store_for_test(&first, 
            second_guid,
            owner,
            represented_loot_object_guid_like_cpp(owner),
            0,
            0,
            materials,
            true,
            Some(claim),
        ),
    );
    let mut target = Box::pin(async {
        tokio::task::yield_now().await;
        process_pending_for_loot_test(&mut second).await;
    });
    let result = tokio::select! {
        result = &mut request => result,
        _ = &mut target => panic!("target must remain behind the COMMIT gate"),
    };
    drop(request);
    assert_eq!(result, MasterLootGiveResult::TargetMismatch);

    first.handle_loot_release(loot_release_packet(owner)).await;
    commit_gate.notify_one();
    target.await;

    assert_eq!((applied_loot_item_quantity_for_test(&second, granted_item)) - inventory_before, 2);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(second_guid, 0)
            .await
            .is_err(),
        "the original roll claim remains terminal after every material commits"
    );
    assert_eq!(
        drain_server_opcodes_like_cpp(&first_rx)
            .into_iter()
            .filter(|opcode| *opcode == wow_constants::ServerOpcodes::LootRemoved as u16)
            .count(),
        1,
        "the material batch publishes the original roll removal exactly once"
    );
    assert!(
        !mutate_loot_creature_for_test(&mut first, owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "post-COMMIT completion must finish the corpse lifecycle without an active view"
    );
}

#[tokio::test]
async fn remote_master_timeout_then_release_still_fans_out_and_finalizes_corpse_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);
    first.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
        corpse_decay_looted: 0.5,
        ..LootDropRatesLikeCpp::default()
    });
    mutate_loot_creature_for_test(&mut first, owner, |creature| {
            creature.creature.set_corpse_delay(120, false);
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(true, false);
        })
        .unwrap();

    let authority = wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
        .unwrap();
    let claim = authority
        .reserve_item_for_award_like_cpp(second_guid, 0)
        .await
        .unwrap();
    let entry = match claim.payload_like_cpp() {
        LootClaimPayload::Item(entry) => entry.clone(),
        LootClaimPayload::Money(_) => panic!("expected item claim"),
    };

    // The target already closed its own view. The source will close while
    // the detached target worker owns the claim as `Persisting`.
    second.handle_loot_release(loot_release_packet(owner)).await;
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    let registry = Arc::new(PlayerRegistry::default());
    let mut first_info = broadcast_info(first_guid, first.send_tx().clone());
    first_info.command_tx = first.session_command_tx();
    registry.register_or_replace(first_guid, first_info, Default::default());
    let mut second_info = broadcast_info(second_guid, second.send_tx().clone());
    second_info.command_tx = second.session_command_tx();
    registry.register_or_replace(second_guid, second_info, Default::default());
    first.set_player_registry(Arc::clone(&registry));
    second.set_player_registry(registry);

    let granted_item = entry.item_id;
    let inventory_before = applied_loot_item_quantity_for_test(&second, granted_item);
    install_limited_test_item_template(&mut second, entry.item_id, 0);
    let commit_gate = Arc::new(tokio::sync::Notify::new());
    install_storage_port(&mut second, PersistenceOutcomeLikeCpp::Applied { rows: 1 }, Some(Arc::clone(&commit_gate)));

    let mut request = Box::pin(request_master_loot_store_for_test(&first, 
        second_guid,
        owner,
        represented_loot_object_guid_like_cpp(owner),
        0,
        0,
        entry,
        Some(claim),
    ));
    let mut target = Box::pin(async {
        tokio::task::yield_now().await;
        process_pending_for_loot_test(&mut second).await;
    });
    let result = tokio::select! {
        result = &mut request => result,
        _ = &mut target => panic!("target must remain behind the COMMIT gate"),
    };
    drop(request);
    assert_eq!(result, MasterLootGiveResult::TargetMismatch);

    first.handle_loot_release(loot_release_packet(owner)).await;
    commit_gate.notify_one();
    target.await;

    assert_eq!((applied_loot_item_quantity_for_test(&second, granted_item)) - inventory_before, 1);
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert!(
        drain_server_opcodes_like_cpp(&first_rx)
            .contains(&(wow_constants::ServerOpcodes::LootRemoved as u16)),
        "the pre-COMMIT route survives request timeout and CMSG_LOOT_RELEASE"
    );
    assert!(
        !mutate_loot_creature_for_test(&mut first, owner, |creature| {
                creature.has_lootable_dynamic_flag_like_cpp()
            })
            .unwrap(),
        "completion must run AllLootRemovedFromCorpse without an active view"
    );
}
