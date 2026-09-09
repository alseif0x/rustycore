//! Battle-pet purchase executor tests, part 2 of 3.
//!
//! Moved out of the battle_pet_purchase.rs root under #656; every test is unchanged.

use super::*;

#[tokio::test]
async fn lost_journal_authority_defers_then_handoff_recovers_exactly_once_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    fixture
        .persistence
        .block_next_insert
        .store(true, Ordering::SeqCst);
    let persistence = fixture.persistence.clone();
    let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
        &mut fixture,
        saga_offer_like_cpp(SAGA_PRICE),
    ));
    // The apply is mid-flight inside the owner when another process wins
    // the Login DB named lock: every further fenced insert fails.
    tokio::select! {
        outcome = &mut purchase => panic!("purchase must block at the insert gate: {outcome:?}"),
        _ = persistence.insert_started.notified() => {}
    }
    persistence.simulate_process_takeover_like_cpp();
    persistence.allow_insert.notify_one();
    let outcome = purchase.await;
    assert_eq!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::RetryableDeferred
    );
    // Charged once; no pet, no receipt, no completion, no publication.
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 1);
    assert_eq!(
        fixture.store.commands_snapshot()[0].status,
        BattlePetPurchaseStatusLikeCpp::PendingApplication
    );
    assert_eq!(fixture.persistence.pet_count(), 0);
    assert_eq!(
        fixture.send_rx.try_recv().expect("charge update"),
        expect_money_update_packet_like_cpp(&fixture, 750)
    );
    assert_no_packets(&fixture);
    while fixture.send_rx.try_recv().is_ok() {}
    let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
    drop(fixture);

    // The winning process attaches a fresh owner and recovery finishes
    // the command: one pet, one completion, one recovery publication.
    let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied, 1);
    assert_eq!(restarted.persistence.pet_count(), 1);
    assert_eq!(restarted.persistence.receipt_count(), 1);
    assert_eq!(
        restarted.store.commands_snapshot()[0].status,
        BattlePetPurchaseStatusLikeCpp::Completed
    );
    assert_eq!(restarted.store.money_mutations(), 1);
    let commands = restarted.store.commands_snapshot();
    let pet_guid = ObjectGuid::create_global(
        HighGuid::BattlePet,
        0,
        restarted
            .persistence
            .receipt(commands[0].request_key)
            .expect("receipt")
            .guid_counter as i64,
    );
    assert_eq!(
        restarted.send_rx.try_recv().expect("recovery pet update"),
        wow_packet::packets::misc::BattlePetUpdates {
            pets: vec![expected_pet_packet_like_cpp(&restarted, pet_guid)],
            pet_added: true,
        }
        .to_bytes()
    );
    assert_eq!(
        restarted
            .send_rx
            .try_recv()
            .expect("recovery learned spells"),
        LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
    );
    assert_no_packets(&restarted);
}

#[tokio::test]
async fn cancelled_during_charge_pre_commit_leaves_no_trace_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    let money_tracker = fixture
        .session
        .durable_loot_money_persistence_tracker_like_cpp();
    fixture
        .store
        .block_next_charge_pre_apply
        .store(true, Ordering::SeqCst);
    let store = fixture.store.clone();
    let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
        &mut fixture,
        saga_offer_like_cpp(SAGA_PRICE),
    ));
    tokio::select! {
        outcome = &mut purchase => panic!("purchase must block at the charge gate: {outcome:?}"),
        _ = store.gate_started.notified() => {}
    }
    drop(purchase);
    fixture.store.allow_gate.notify_one();
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 0);
    assert!(fixture.store.commands_snapshot().is_empty());
    assert_eq!(fixture.persistence.pet_count(), 0);
    assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
    assert!(!money_tracker.is_indeterminate_like_cpp());
    // No authority survived the cancellation: a fresh purchase works.
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert!(matches!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
    ));
}

#[tokio::test]
async fn cancelled_after_charge_commit_recovers_to_paid_pet_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    let money_tracker = fixture
        .session
        .durable_loot_money_persistence_tracker_like_cpp();
    fixture
        .store
        .block_next_charge_post_apply
        .store(true, Ordering::SeqCst);
    let store = fixture.store.clone();
    let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
        &mut fixture,
        saga_offer_like_cpp(SAGA_PRICE),
    ));
    tokio::select! {
        outcome = &mut purchase => panic!("purchase must block at the charge gate: {outcome:?}"),
        _ = store.gate_started.notified() => {}
    }
    drop(purchase);
    fixture.store.allow_gate.notify_one();
    // The charge committed; the command is durable; nothing else ran.
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 1);
    assert_eq!(
        fixture.store.commands_snapshot()[0].status,
        BattlePetPurchaseStatusLikeCpp::PendingApplication
    );
    assert_eq!(fixture.persistence.pet_count(), 0);
    assert!(money_tracker.is_indeterminate_like_cpp());
    while fixture.send_rx.try_recv().is_ok() {}
    let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
    drop(fixture);
    let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied, 1);
    assert_eq!(restarted.persistence.pet_count(), 1);
    assert_eq!(
        restarted.store.commands_snapshot()[0].status,
        BattlePetPurchaseStatusLikeCpp::Completed
    );
    assert_eq!(restarted.store.money_mutations(), 1);
}

#[tokio::test]
async fn cancelled_during_apply_completes_through_detached_worker_and_recovery_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    fixture
        .persistence
        .block_next_insert
        .store(true, Ordering::SeqCst);
    let persistence = fixture.persistence.clone();
    let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
        &mut fixture,
        saga_offer_like_cpp(SAGA_PRICE),
    ));
    tokio::select! {
        outcome = &mut purchase => panic!("purchase must block at the insert gate: {outcome:?}"),
        _ = persistence.insert_started.notified() => {}
    }
    drop(purchase);
    // The #160 worker is detached from the cancelled caller: releasing
    // the gate lets it finish the durable insert exactly once.
    fixture.persistence.allow_insert.notify_one();
    let deadline = std::time::Instant::now() + StdDuration::from_secs(2);
    while fixture.persistence.receipt_count() == 0 && std::time::Instant::now() < deadline {
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }
    assert_eq!(fixture.persistence.receipt_count(), 1);
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    assert_eq!(
        fixture.store.commands_snapshot()[0].status,
        BattlePetPurchaseStatusLikeCpp::PendingApplication
    );
    while fixture.send_rx.try_recv().is_ok() {}
    let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
    drop(fixture);
    let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    // The receipt replay completes the command: no second pet and no
    // new charge, and because no publication was ever recorded the one
    // idempotent success publication is emitted now.
    assert_eq!(summary.applied, 1);
    assert_eq!(restarted.persistence.pet_count(), 1);
    assert_eq!(restarted.store.money_mutations(), 1);
    let commands = restarted.store.commands_snapshot();
    assert_eq!(
        commands[0].status,
        BattlePetPurchaseStatusLikeCpp::Completed
    );
    assert!(commands[0].published);
    let pet_guid = ObjectGuid::create_global(
        HighGuid::BattlePet,
        0,
        restarted
            .persistence
            .receipt(commands[0].request_key)
            .expect("receipt")
            .guid_counter as i64,
    );
    assert_eq!(
        restarted.send_rx.try_recv().expect("recovery pet update"),
        wow_packet::packets::misc::BattlePetUpdates {
            pets: vec![expected_pet_packet_like_cpp(&restarted, pet_guid)],
            pet_added: true,
        }
        .to_bytes()
    );
    assert_eq!(
        restarted
            .send_rx
            .try_recv()
            .expect("recovery learned spells"),
        LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
    );
    assert_no_packets(&restarted);
}

#[tokio::test]
async fn cancelled_before_compensation_refund_recovers_exactly_once_like_cpp() {
    let seeded = vec![
        saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
        saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
    ];
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
    let money_tracker = fixture
        .session
        .durable_loot_money_persistence_tracker_like_cpp();
    seed_third_pet_into_persistence_like_cpp(&fixture);
    fixture
        .store
        .block_next_compensate_pre_apply
        .store(true, Ordering::SeqCst);
    let store = fixture.store.clone();
    let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
        &mut fixture,
        saga_offer_like_cpp(SAGA_PRICE),
    ));
    tokio::select! {
        outcome = &mut purchase => panic!("purchase must block at the compensation gate: {outcome:?}"),
        _ = store.gate_started.notified() => {}
    }
    drop(purchase);
    fixture.store.allow_gate.notify_one();
    // The decision is durable, the refund is not: money still charged.
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 1);
    assert_eq!(
        fixture.store.commands_snapshot()[0].status,
        BattlePetPurchaseStatusLikeCpp::CompensationPending
    );
    assert!(!money_tracker.is_indeterminate_like_cpp());
    while fixture.send_rx.try_recv().is_ok() {}
    let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
    drop(fixture);
    let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.compensated, 1);
    assert_eq!(
        restarted.store.money(PLAYER_COUNTER as u64),
        Some(SAGA_MONEY)
    );
    assert_eq!(restarted.store.money_mutations(), 2);
    assert_eq!(
        restarted.store.commands_snapshot()[0].status,
        BattlePetPurchaseStatusLikeCpp::Compensated
    );
    assert_eq!(restarted.persistence.species_count(SAGA_SPECIES), 3);
    // Login recovery staged the runtime restore without a values packet.
    assert_eq!(restarted.session.player_gold_like_cpp(), SAGA_MONEY);
    assert_no_packets(&restarted);
}

#[tokio::test]
async fn cancelled_after_compensation_refund_stays_refunded_once_like_cpp() {
    let seeded = vec![
        saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
        saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
    ];
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
    let money_tracker = fixture
        .session
        .durable_loot_money_persistence_tracker_like_cpp();
    seed_third_pet_into_persistence_like_cpp(&fixture);
    fixture
        .store
        .block_next_compensate_post_apply
        .store(true, Ordering::SeqCst);
    let store = fixture.store.clone();
    let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
        &mut fixture,
        saga_offer_like_cpp(SAGA_PRICE),
    ));
    tokio::select! {
        outcome = &mut purchase => panic!("purchase must block at the compensation gate: {outcome:?}"),
        _ = store.gate_started.notified() => {}
    }
    drop(purchase);
    fixture.store.allow_gate.notify_one();
    // The refund and the status flip committed together before the
    // cancellation: exactly one refund, terminally compensated.
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 2);
    assert_eq!(
        fixture.store.commands_snapshot()[0].status,
        BattlePetPurchaseStatusLikeCpp::Compensated
    );
    assert!(money_tracker.is_indeterminate_like_cpp());
    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied + summary.compensated + summary.deferred, 0);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 2);
}

#[tokio::test]
async fn shutdown_drain_is_bounded_and_completes_inflight_purchase_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    fixture
        .persistence
        .block_next_insert
        .store(true, Ordering::SeqCst);
    let registry = Arc::clone(&fixture.registry);
    let persistence = fixture.persistence.clone();
    let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
        &mut fixture,
        saga_offer_like_cpp(SAGA_PRICE),
    ));
    let drained = tokio::select! {
        outcome = &mut purchase => panic!("purchase must block at the insert gate: {outcome:?}"),
        drained = async {
            persistence.insert_started.notified().await;
            registry
                .drain_like_cpp(StdDuration::from_millis(50))
                .await
        } => drained,
    };
    assert!(
        !drained,
        "the bounded shutdown drain must time out while the worker is blocked"
    );
    persistence.allow_insert.notify_one();
    let outcome = purchase.await;
    assert!(matches!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
    ));
    assert!(
        fixture
            .registry
            .drain_like_cpp(StdDuration::from_secs(1))
            .await,
        "after releasing the worker the bounded drain completes"
    );
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    assert_eq!(fixture.store.money_mutations(), 1);
}

#[tokio::test]
async fn deterministic_selection_flows_into_command_and_pet_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    fixture
        .session
        .set_battle_pet_purchase_selection_override_like_cpp(Some(
            BattlePetTrainerSelectionLikeCpp {
                species: SAGA_SPECIES,
                breed: 9,
                quality: 2,
                display_id: 456,
                level: 1,
            },
        ));
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert!(matches!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
    ));
    let command = &fixture.store.commands_snapshot()[0];
    assert_eq!(
        (command.breed, command.quality, command.display_id),
        (9, 2, 456)
    );
    let receipt = fixture
        .persistence
        .receipt(command.request_key)
        .expect("receipt");
    assert_eq!(
        (receipt.breed, receipt.quality, receipt.display_id),
        (9, 2, 456)
    );
}

#[tokio::test]
async fn handler_battle_pet_buy_runs_the_full_saga_like_cpp() {
    let mut fixture = saga_handler_fixture_like_cpp(SAGA_MONEY, SAGA_PRICE, None, Vec::new()).await;
    fixture
        .session
        .handle_trainer_buy_spell(saga_buy_packet_like_cpp(SAGA_SPELL_ID as i32))
        .await;
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 1);
    let commands = fixture.store.commands_snapshot();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].status,
        BattlePetPurchaseStatusLikeCpp::Completed
    );
    assert_eq!(commands[0].price, SAGA_PRICE);
    assert_eq!(commands[0].trainer_id, TRAINER_ID);
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    assert_eq!(fixture.session.player_gold_like_cpp(), 750);
    // Wire order with trainer visuals suppressed: money update, petAdded
    // journal update, dependent learned spell.
    assert_eq!(
        fixture.send_rx.try_recv().expect("money update"),
        expect_money_update_packet_like_cpp(&fixture, 750)
    );
    let commands = fixture.store.commands_snapshot();
    let pet_guid = ObjectGuid::create_global(
        HighGuid::BattlePet,
        0,
        fixture
            .persistence
            .receipt(commands[0].request_key)
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
        LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
    );
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn handler_buy_revalidates_membership_and_current_price_like_cpp() {
    // A spell outside the trainer's current spell set is rejected with
    // the C++ generic failure before any saga state exists.
    let mut fixture = saga_handler_fixture_like_cpp(SAGA_MONEY, SAGA_PRICE, None, Vec::new()).await;
    fixture
        .session
        .handle_trainer_buy_spell(saga_buy_packet_like_cpp(999_999))
        .await;
    assert_eq!(
        fixture.send_rx.try_recv().expect("teach failure"),
        TrainerBuyFailed {
            trainer_guid: saga_trainer_guid_like_cpp(),
            spell_id: 999_999,
            reason: 0,
        }
        .to_bytes()
    );
    assert_no_packets(&fixture);
    assert!(fixture.store.commands_snapshot().is_empty());
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));

    // The same spell at a different current store price charges the
    // current price, not a previously listed one.
    let mut fixture = Box::pin(saga_handler_fixture_like_cpp(
        SAGA_MONEY,
        400,
        None,
        Vec::new(),
    ))
    .await;
    fixture
        .session
        .handle_trainer_buy_spell(saga_buy_packet_like_cpp(SAGA_SPELL_ID as i32))
        .await;
    let commands = fixture.store.commands_snapshot();
    assert_eq!(commands.len(), 1);
    assert_eq!(commands[0].price, 400);
    assert_eq!(commands[0].money_after, 600);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(600));
}

#[tokio::test]
async fn handler_capacity_failure_publishes_no_packets_like_cpp() {
    // Capture fixture (capacity): the structured admission failure keeps
    // the wire silent exactly like C++.
    let mut fixture = saga_handler_fixture_like_cpp(
        SAGA_MONEY,
        SAGA_PRICE,
        None,
        vec![
            saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
            saga_durable_pet_row_like_cpp(3, SAGA_SPECIES, None),
        ],
    )
    .await;
    fixture
        .session
        .handle_trainer_buy_spell(saga_buy_packet_like_cpp(SAGA_SPELL_ID as i32))
        .await;
    assert!(fixture.store.commands_snapshot().is_empty());
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn recovery_publishes_exactly_one_pet_update_after_crash_before_apply_like_cpp() {
    // Capture fixture (recovery publication): a crash right after the
    // Character DB commit is resumed by login recovery, which applies
    // the pet and publishes the one allowed petAdded update.
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    fixture
        .store
        .block_next_charge_post_apply
        .store(true, Ordering::SeqCst);
    let store = fixture.store.clone();
    let mut purchase = Box::pin(execute_saga_purchase_like_cpp(
        &mut fixture,
        saga_offer_like_cpp(SAGA_PRICE),
    ));
    tokio::select! {
        outcome = &mut purchase => panic!("purchase must block at the charge gate: {outcome:?}"),
        _ = store.gate_started.notified() => {}
    }
    drop(purchase);
    fixture.store.allow_gate.notify_one();
    while fixture.send_rx.try_recv().is_ok() {}
    let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
    drop(fixture);

    let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied, 1);
    let commands = restarted.store.commands_snapshot();
    let pet_guid = ObjectGuid::create_global(
        HighGuid::BattlePet,
        0,
        restarted
            .persistence
            .receipt(commands[0].request_key)
            .expect("receipt")
            .guid_counter as i64,
    );
    assert_eq!(
        restarted.send_rx.try_recv().expect("recovery pet update"),
        wow_packet::packets::misc::BattlePetUpdates {
            pets: vec![expected_pet_packet_like_cpp(&restarted, pet_guid)],
            pet_added: true,
        }
        .to_bytes()
    );
    assert_eq!(
        restarted
            .send_rx
            .try_recv()
            .expect("recovery learned spells"),
        LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
    );
    assert_no_packets(&restarted);

    // A further recovery replays nothing and publishes nothing.
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied + summary.compensated + summary.deferred, 0);
    assert_no_packets(&restarted);
}

#[tokio::test]
async fn recovery_with_receipt_after_recorded_decision_completes_without_refunding_like_cpp() {
    // The terminal-failure decision was recorded, but the Login DB
    // receipt proves the pet became durable after all: recovery must
    // complete the command, never refund a durable pet.
    let pet_row = saga_durable_pet_row_like_cpp(6, SAGA_SPECIES, None);
    let mut fixture = saga_fixture_like_cpp(750, vec![pet_row.clone()]).await;
    let request_key = [99; 16];
    fixture
        .persistence
        .state
        .lock()
        .expect("fake saga persistence poisoned")
        .receipts
        .insert(
            BattlePetAddRequestKeyLikeCpp::from_bytes(request_key),
            (ACCOUNT_ID, pet_row),
        );
    let mut command =
        crate::battle_pet_purchase::tests::test_command(request_key, PLAYER_COUNTER as u64);
    command.account_id = ACCOUNT_ID;
    command.species = SAGA_SPECIES;
    command.breed = 7;
    command.quality = 1;
    command.display_id = 123;
    command.level = 1;
    command.money_before = SAGA_MONEY;
    command.money_after = 750;
    command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
    fixture.store.seed_command(command);

    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied, 1);
    assert_eq!(summary.compensated, 0);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 0);
    assert_eq!(fixture.persistence.pet_count(), 1);
    let command = fixture.store.command(request_key).expect("command");
    assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
    assert!(command.published);
    // The receipt replay resolves the packet in-process, so the one
    // success publication goes out instead of the refund.
    assert_eq!(
        fixture.send_rx.try_recv().expect("recovery pet update"),
        wow_packet::packets::misc::BattlePetUpdates {
            pets: vec![expected_pet_packet_like_cpp(
                &fixture,
                ObjectGuid::create_global(HighGuid::BattlePet, 0, 6),
            )],
            pet_added: true,
        }
        .to_bytes()
    );
    assert_eq!(
        fixture.send_rx.try_recv().expect("recovery learned spells"),
        LearnedSpells::single(command.spell_id as i32).to_bytes()
    );
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn handler_castable_battle_pet_spell_fails_closed_before_money_like_cpp() {
    // C++ would charge and cast a hybrid (learn + battle-pet summon)
    // trainer spell with the silent cap and suppressed visuals. The #164
    // acquisition planner deliberately does not project SUMMON effects
    // (`BattlePetOrSummonPath`), so today the offer fails closed BEFORE
    // money, visuals, pet or saga state: the one observable packet is
    // the C++ generic buy failure. The offer type still carries the
    // species (pure-decision test in trainer_offer.rs) and the handler
    // keeps the C++-shared cap gate and visual suppression for the day
    // the planner models hybrid casts.
    let mut fixture = saga_handler_fixture_like_cpp(
        SAGA_MONEY,
        SAGA_WRAPPER_PRICE,
        Some(SAGA_WRAPPER_LEARNED_SPELL),
        Vec::new(),
    )
    .await;
    fixture
        .session
        .handle_trainer_buy_spell(saga_buy_packet_like_cpp(SAGA_SPELL_ID as i32))
        .await;
    assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
    assert!(
        !fixture
            .session
            .known_spells_like_cpp()
            .contains(&(SAGA_WRAPPER_LEARNED_SPELL as i32)),
        "the hybrid cast must not run while the planner rejects it"
    );
    assert_eq!(fixture.persistence.pet_count(), 0);
    assert!(fixture.store.commands_snapshot().is_empty());
    assert_eq!(
        fixture.send_rx.try_recv().expect("teach failure"),
        TrainerBuyFailed {
            trainer_guid: saga_trainer_guid_like_cpp(),
            spell_id: SAGA_SPELL_ID as i32,
            reason: 0,
        }
        .to_bytes()
    );
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn handler_castable_battle_pet_spell_never_reaches_the_saga_even_uncapped_like_cpp() {
    // Same fail-closed proof with journal capacity available: the
    // rejection is the planner boundary, not the battle-pet cap.
    let mut fixture = saga_handler_fixture_like_cpp(
        SAGA_MONEY,
        SAGA_WRAPPER_PRICE,
        Some(SAGA_WRAPPER_LEARNED_SPELL),
        vec![saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None)],
    )
    .await;
    fixture
        .session
        .handle_trainer_buy_spell(saga_buy_packet_like_cpp(SAGA_SPELL_ID as i32))
        .await;
    assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
    assert_eq!(fixture.persistence.pet_count(), 1);
    assert!(fixture.store.commands_snapshot().is_empty());
    assert_eq!(
        fixture.send_rx.try_recv().expect("teach failure"),
        TrainerBuyFailed {
            trainer_guid: saga_trainer_guid_like_cpp(),
            spell_id: SAGA_SPELL_ID as i32,
            reason: 0,
        }
        .to_bytes()
    );
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn reconciled_lost_insert_reply_runs_new_pet_criteria_once_like_cpp() {
    // The Login DB insert commits but its reply is lost; the real
    // persistence reconciles the duplicate key through the receipt and
    // reports `Inserted` for this invocation. The pet was never published
    // and its C++ new-pet criteria hooks were never run, so this execution
    // owns both exactly once.
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    fixture
        .persistence
        .reconcile_next_insert_after_commit
        .store(true, Ordering::SeqCst);
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    let BattlePetPurchaseExecutionLikeCpp::Purchased {
        pet_guid,
        published,
    } = outcome
    else {
        panic!("reconciled insert must complete the purchase: {outcome:?}");
    };
    assert!(published, "a reconciled durable add still publishes once");
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    assert_eq!(fixture.persistence.receipt_count(), 1);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 1);
    let command = &fixture.store.commands_snapshot()[0];
    assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
    assert!(command.published);
    assert_eq!(
        fixture
            .session
            .represented_battle_pet_unique_owned_criteria_like_cpp(),
        1
    );
    assert_eq!(
        fixture
            .session
            .represented_battle_pet_learned_new_pet_criteria_like_cpp(),
        &[SAGA_SPECIES]
    );
    assert_eq!(
        fixture.send_rx.try_recv().expect("money update"),
        expect_money_update_packet_like_cpp(&fixture, 750)
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
        LearnedSpells::single(SAGA_SPELL_ID as i32).to_bytes()
    );
    assert_no_packets(&fixture);
}
