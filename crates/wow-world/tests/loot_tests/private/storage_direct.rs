//! Original application scenarios over the real persistence lane.
use super::storage_support::*;

#[tokio::test]
async fn failed_authoritative_item_store_rolls_back_for_retry_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let granted_item = 25;
    let inventory_before = applied_loot_item_quantity_for_test(&first, granted_item);
    install_storage_port(&mut first, PersistenceOutcomeLikeCpp::Failed { reason: "planned transaction failure".to_owned() }, None);
    let loot_obj = represented_loot_object_guid_like_cpp(owner);

    handle_loot_item_for_test(&mut first, loot_item_packet(loot_obj, 0)).await;
    let authority = wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
        .unwrap();
    assert!(
        !authority
            .snapshot_for_player_like_cpp(first_guid)
            .unwrap()
            .loot
            .items[0]
            .taken
    );

    install_storage_port(&mut first, PersistenceOutcomeLikeCpp::Applied { rows: 1 }, None);
    handle_loot_item_for_test(&mut first, loot_item_packet(loot_obj, 0)).await;
    assert_eq!((applied_loot_item_quantity_for_test(&first, granted_item)) - inventory_before, 1);
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
async fn item_waiter_waking_on_replacement_rolls_back_new_generation_claim_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
        .unwrap();
    let blocker = authority
        .reserve_item_like_cpp(first_guid, 0)
        .await
        .unwrap();
    let granted_item = 25;
    let inventory_before = applied_loot_item_quantity_for_test(&first, granted_item);
    install_storage_port(&mut first, PersistenceOutcomeLikeCpp::Applied { rows: 1 }, None);
    let loot_obj = represented_loot_object_guid_like_cpp(owner);
    let waiter = tokio::spawn(async move {
        handle_loot_item_for_test(&mut first, loot_item_packet(loot_obj, 0)).await;
        first
    });
    for _ in 0..4 {
        tokio::task::yield_now().await;
    }

    let mut replacement = authoritative_test_loot_like_cpp(0, true);
    replacement.loot_guid = loot_obj;
    replacement.allowed_looters = vec![first_guid, second_guid];
    replacement.items[0].allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    drop(blocker);
    let _first = waiter.await.unwrap();

    assert_eq!((applied_loot_item_quantity_for_test(&_first, granted_item)) - inventory_before, 0);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert!(!snapshot.loot.items[0].taken);
}

#[tokio::test]
async fn stale_active_item_view_cannot_claim_replacement_generation_like_cpp() {
    let (mut first, _first_rx, _second, _second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
        .unwrap();
    let mut replacement = authoritative_test_loot_like_cpp(0, true);
    replacement.loot_guid = represented_loot_object_guid_like_cpp(owner);
    replacement.allowed_looters = vec![first_guid, second_guid];
    replacement.items[0].allowed_looters = vec![first_guid, second_guid];
    let replacement_generation = authority.replace_like_cpp(Some(replacement), HashMap::new());
    let granted_item = 25;
    let inventory_before = applied_loot_item_quantity_for_test(&first, granted_item);
    install_storage_port(&mut first, PersistenceOutcomeLikeCpp::Applied { rows: 1 }, None);

    handle_loot_item_for_test(&mut first, loot_item_packet(
            represented_loot_object_guid_like_cpp(owner),
            0,
        ))
        .await;

    assert_eq!((applied_loot_item_quantity_for_test(&first, granted_item)) - inventory_before, 0);
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert_eq!(snapshot.generation, replacement_generation);
    assert!(!snapshot.loot.items[0].taken);
}

#[tokio::test]
async fn two_sessions_claim_one_authoritative_item_exactly_once_like_cpp() {
    let (mut first, _first_rx, mut second, _second_rx, owner, first_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let granted_item = 25;
    let inventory_before = applied_loot_item_quantity_for_test(&second, granted_item) + applied_loot_item_quantity_for_test(&first, granted_item);
    install_storage_port(&mut first, PersistenceOutcomeLikeCpp::Applied { rows: 1 }, None);
    install_storage_port(&mut second, PersistenceOutcomeLikeCpp::Applied { rows: 1 }, None);
    let barrier = Arc::new(tokio::sync::Barrier::new(3));
    let loot_obj = represented_loot_object_guid_like_cpp(owner);

    let first_barrier = Arc::clone(&barrier);
    let first_task = tokio::spawn(async move {
        first_barrier.wait().await;
        handle_loot_item_for_test(&mut first, loot_item_packet(loot_obj, 0)).await;
        first
    });
    let second_barrier = Arc::clone(&barrier);
    let second_task = tokio::spawn(async move {
        second_barrier.wait().await;
        handle_loot_item_for_test(&mut second, loot_item_packet(loot_obj, 0)).await;
        second
    });
    barrier.wait().await;

    let mut first = first_task.await.unwrap();
    let _second = second_task.await.unwrap();
    assert_eq!((applied_loot_item_quantity_for_test(&first, granted_item) + applied_loot_item_quantity_for_test(&_second, granted_item)) - inventory_before, 1);
    let authority = wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut first, owner)
        .unwrap();
    let snapshot = authority.snapshot_for_player_like_cpp(first_guid).unwrap();
    assert!(snapshot.loot.items[0].taken);
    assert_eq!(snapshot.loot.unlooted_count, 0);
}

#[tokio::test]
async fn failed_disenchant_batch_grants_zero_and_original_slot_retries_like_cpp() {
    let (mut session, _rx, _second, _second_rx, owner, player_guid, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            0, true,
        ));
    let authority = wow_world::test_fixtures::loot::loot_recovery_authority_for_test(&mut session, owner)
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
    let granted_item = 700;
    install_limited_test_item_template(&mut session, granted_item, 0);
    let inventory_before = applied_loot_item_quantity_for_test(&session, granted_item);
    // Both material grants are already planned when this seam rejects the
    // one transaction, modelling a failure while persisting its second
    // output. Runtime observes neither output.
    install_storage_port(&mut session, PersistenceOutcomeLikeCpp::Failed { reason: "planned transaction failure".to_owned() }, None);

    assert!(
        !store_loot_materials_for_test(&mut session, &materials, 0, Some(&claim), owner, represented_loot_object_guid_like_cpp(owner), 0, player_guid, false)
            .await
    );
    assert_eq!((applied_loot_item_quantity_for_test(&session, granted_item)) - inventory_before, 0);
    assert!(!claim.is_committed_like_cpp());
    drop(claim);
    assert!(
        authority
            .reserve_item_for_award_like_cpp(player_guid, 0)
            .await
            .is_ok()
    );
}

