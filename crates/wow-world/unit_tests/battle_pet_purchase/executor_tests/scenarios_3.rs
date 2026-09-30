//! Battle-pet purchase executor tests, part 3 of 3.
//!
//! Moved out of the battle_pet_purchase.rs root under #656; every test is unchanged.

use super::*;

#[tokio::test]
async fn recovery_after_account_transfer_compensates_without_applying_or_publishing_like_cpp() {
    // The character moved to another Battle.net account while the
    // purchase was pending: the pet must never be applied into the new
    // account, so the saga refunds the character (money travels with
    // it) exactly once and publishes nothing.
    let mut fixture = saga_fixture_like_cpp(750, Vec::new()).await;
    let mut command =
        crate::battle_pet_purchase::tests::test_command([55; 16], PLAYER_COUNTER as u64);
    command.account_id = 999;
    command.species = SAGA_SPECIES;
    command.breed = 7;
    command.quality = 1;
    command.display_id = 123;
    command.money_before = SAGA_MONEY;
    command.money_after = 750;
    fixture.store.seed_command(command);
    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.compensated, 1);
    assert_eq!(summary.applied, 0);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 1);
    assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
    assert_eq!(fixture.persistence.pet_count(), 0);
    assert_eq!(fixture.persistence.receipt_count(), 0);
    assert_eq!(
        fixture.store.command([55; 16]).expect("command").status,
        BattlePetPurchaseStatusLikeCpp::Compensated
    );
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn completed_unpublished_after_account_transfer_closes_marker_without_publishing_like_cpp() {
    // The pet was durably created in the original account's journal
    // before the transfer; the new account's client must never receive
    // an update for a pet it does not own, so recovery just closes the
    // publication marker.
    let mut fixture = saga_fixture_like_cpp(750, Vec::new()).await;
    let mut command =
        crate::battle_pet_purchase::tests::test_command([56; 16], PLAYER_COUNTER as u64);
    command.account_id = 999;
    command.species = SAGA_SPECIES;
    command.status = BattlePetPurchaseStatusLikeCpp::Completed;
    command.published = false;
    command.money_before = SAGA_MONEY;
    command.money_after = 750;
    fixture.store.seed_command(command);
    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied, 1);
    assert_eq!(summary.compensated, 0);
    assert!(fixture.store.command([56; 16]).expect("command").published);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.persistence.pet_count(), 0);
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn account_transfer_with_durable_pet_completes_without_refunding_or_publishing_like_cpp() {
    // Crash window + account transfer: the Login DB pet/receipt
    // committed under the ORIGINAL account before the Character DB
    // command completed. Recovery must not refund (the pet is durable)
    // and must not publish into the new account.
    let pet_row = saga_durable_pet_row_like_cpp(7, SAGA_SPECIES, None);
    let mut fixture = saga_fixture_like_cpp(750, vec![pet_row.clone()]).await;
    let request_key = [57; 16];
    fixture
        .persistence
        .state
        .lock()
        .expect("fake saga persistence poisoned")
        .receipts
        .insert(
            BattlePetAddRequestKeyLikeCpp::from_bytes(request_key),
            (999, pet_row),
        );
    let mut command =
        crate::battle_pet_purchase::tests::test_command(request_key, PLAYER_COUNTER as u64);
    command.account_id = 999;
    command.species = SAGA_SPECIES;
    command.breed = 7;
    command.quality = 1;
    command.display_id = 123;
    command.money_before = SAGA_MONEY;
    command.money_after = 750;
    fixture.store.seed_command(command);
    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied, 1);
    assert_eq!(summary.compensated, 0);
    let command = fixture.store.command(request_key).expect("command");
    assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
    assert!(command.published);
    // No refund, no new pet, no publication into the new account.
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 0);
    assert_eq!(fixture.persistence.pet_count(), 1);
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn purchase_binds_the_battlenet_account_identity_like_cpp() {
    // Game account 1, Battle.net account 5: the durable command and the
    // Login DB receipt authority must both use the Battle.net identity,
    // or a later account relink would replay into the wrong journal.
    let persistence = Arc::new(FakeSagaPersistenceLikeCpp::default());
    let store = Arc::new(
        FakeBattlePetPurchaseStoreLikeCpp::new().with_money(PLAYER_COUNTER as u64, SAGA_MONEY),
    );
    let registry = saga_registry_like_cpp(Arc::clone(&persistence));
    let (mut session, _send_rx) = make_saga_session_like_cpp(PLAYER_COUNTER, SAGA_MONEY);
    session.set_battlenet_account_id(5);
    session.set_battle_pet_purchase_persistence_port_like_cpp(store_handle_like_cpp(&store));
    session.set_battle_pet_account_attachment_like_cpp(
        registry
            .attach_like_cpp(5)
            .await
            .expect("attach Battle.net account 5"),
    );
    let guard = session
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
        .expect("money exclusivity");
    let outcome = session
        .execute_battle_pet_trainer_purchase_like_cpp(
            guard,
            saga_trainer_guid_like_cpp(),
            TRAINER_ID,
            saga_offer_like_cpp(SAGA_PRICE),
        )
        .await;
    assert!(matches!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
    ));
    let command = &store.commands_snapshot()[0];
    assert_eq!(command.account_id, 5);
    let receipt_account = persistence
        .state
        .lock()
        .expect("fake saga persistence poisoned")
        .receipts
        .get(&BattlePetAddRequestKeyLikeCpp::from_bytes(
            command.request_key,
        ))
        .map(|(account, _)| *account)
        .expect("receipt");
    assert_eq!(receipt_account, 5);
}

#[tokio::test]
async fn completed_unpublished_with_deleted_pet_settles_marker_and_batch_continues_like_cpp() {
    // The pet and receipt committed, then another session deleted the
    // pet before this character's recovery: nothing can be published,
    // but the charge stands and the row must converge instead of
    // blocking every later login and every newer command.
    let mut fixture = saga_fixture_like_cpp(750, Vec::new()).await;
    let deleted_key = [60; 16];
    let deleted_row = saga_durable_pet_row_like_cpp(8, SAGA_SPECIES, None);
    fixture
        .persistence
        .state
        .lock()
        .expect("fake saga persistence poisoned")
        .receipts
        .insert(
            BattlePetAddRequestKeyLikeCpp::from_bytes(deleted_key),
            (ACCOUNT_ID, deleted_row),
        );
    let mut deleted_command =
        crate::battle_pet_purchase::tests::test_command(deleted_key, PLAYER_COUNTER as u64);
    deleted_command.account_id = ACCOUNT_ID;
    deleted_command.species = SAGA_SPECIES;
    deleted_command.breed = 7;
    deleted_command.quality = 1;
    deleted_command.display_id = 123;
    deleted_command.money_before = SAGA_MONEY;
    deleted_command.money_after = 750;
    deleted_command.status = BattlePetPurchaseStatusLikeCpp::Completed;
    deleted_command.published = false;
    fixture.store.seed_command(deleted_command);
    let mut pending_command =
        crate::battle_pet_purchase::tests::test_command([61; 16], PLAYER_COUNTER as u64);
    pending_command.account_id = ACCOUNT_ID;
    pending_command.species = SAGA_SPECIES;
    pending_command.breed = 7;
    pending_command.quality = 1;
    pending_command.display_id = 123;
    pending_command.money_before = SAGA_MONEY;
    pending_command.money_after = 750;
    fixture.store.seed_command(pending_command);

    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied, 2);
    assert_eq!(summary.deferred, 0);
    // The deleted pet's marker is closed without any packet; the newer
    // command behind it converges normally with its one publication.
    let deleted = fixture.store.command(deleted_key).expect("command");
    assert_eq!(deleted.status, BattlePetPurchaseStatusLikeCpp::Completed);
    assert!(deleted.published);
    let pending = fixture.store.command([61; 16]).expect("command");
    assert_eq!(pending.status, BattlePetPurchaseStatusLikeCpp::Completed);
    assert!(pending.published);
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 0);
    let pet_guid = ObjectGuid::create_global(
        HighGuid::BattlePet,
        0,
        fixture
            .persistence
            .receipt([61; 16])
            .expect("receipt")
            .guid_counter as i64,
    );
    assert_eq!(
        fixture.send_rx.try_recv().expect("pet update"),
        wow_packet::packets::misc::BattlePetUpdates {
            pets: vec![expected_pet_packet_like_cpp(&fixture, pet_guid)],
            pet_added: true,
        }
        .to_bytes()
    );
    assert_eq!(
        fixture.send_rx.try_recv().expect("learned spells"),
        LearnedSpells::single(pending.spell_id as i32).to_bytes()
    );
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn mismatched_refund_waits_for_original_account_fence_like_cpp() {
    // No receipt, but the original account's authority is held by a
    // still-flying driver: the snapshot absence must not refund, so the
    // command waits for the fence instead of risking pet + refund.
    let mut fixture = saga_fixture_like_cpp(750, Vec::new()).await;
    let mut command =
        crate::battle_pet_purchase::tests::test_command([62; 16], PLAYER_COUNTER as u64);
    command.account_id = 999;
    command.species = SAGA_SPECIES;
    command.money_before = SAGA_MONEY;
    command.money_after = 750;
    fixture.store.seed_command(command);
    fixture
        .persistence
        .process_lease
        .store(true, Ordering::SeqCst);
    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.compensated, 0);
    assert!(summary.deferred >= 1);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 0);
    assert_eq!(
        fixture.store.command([62; 16]).expect("command").status,
        BattlePetPurchaseStatusLikeCpp::CompensationPending
    );
    // Once the original account's fence is free and the absence is
    // proven under it, the refund converges exactly once.
    fixture
        .persistence
        .process_lease
        .store(false, Ordering::SeqCst);
    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.compensated, 1);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 1);
    assert_eq!(
        fixture.store.command([62; 16]).expect("command").status,
        BattlePetPurchaseStatusLikeCpp::Compensated
    );
    assert_eq!(fixture.persistence.pet_count(), 0);
}

#[tokio::test]
async fn zero_price_purchase_grants_the_free_pet_like_cpp() {
    // C++ `HasEnoughMoney(0)` always passes and `ModifyMoney(-0)` is a
    // no-op: a zero-cost battle-pet trainer row must charge nothing and
    // still create the pet, the command and the one publication.
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    let outcome = execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(0)).await;
    let BattlePetPurchaseExecutionLikeCpp::Purchased {
        pet_guid: _,
        published,
    } = outcome
    else {
        panic!("a zero-price purchase must succeed: {outcome:?}");
    };
    assert!(published);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 0);
    let command = &fixture.store.commands_snapshot()[0];
    assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
    assert_eq!(command.price, 0);
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
    // No money update packet for a zero charge; the pet and learned
    // spell still publish once.
    assert_eq!(
        fixture.send_rx.try_recv().expect("pet update"),
        wow_packet::packets::misc::BattlePetUpdates {
            pets: vec![expected_pet_packet_like_cpp(
                &fixture,
                ObjectGuid::create_global(
                    HighGuid::BattlePet,
                    0,
                    fixture
                        .persistence
                        .receipt(command.request_key)
                        .expect("receipt")
                        .guid_counter as i64,
                ),
            )],
            pet_added: true,
        }
        .to_bytes()
    );
    assert_eq!(
        fixture.send_rx.try_recv().expect("learned spells"),
        LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
    );
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn zero_price_compensation_flips_status_without_money_mutation_like_cpp() {
    // A zero-price purchase that fails terminally compensates by
    // flipping the command once; there is no refund statement to roll
    // back, so the compensation must still converge.
    let seeded = vec![
        saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
        saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
    ];
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
    seed_third_pet_into_persistence_like_cpp(&fixture);
    let outcome = execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(0)).await;
    assert_eq!(outcome, BattlePetPurchaseExecutionLikeCpp::Compensated);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 0);
    let command = &fixture.store.commands_snapshot()[0];
    assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Compensated);
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 3);
    assert_no_packets(&fixture);
}
