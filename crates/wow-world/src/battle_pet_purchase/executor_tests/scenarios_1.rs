//! Battle-pet purchase executor tests, part 1 of 3.
//!
//! Moved out of the battle_pet_purchase.rs root under #656; every test is unchanged.

use super::*;

#[tokio::test]
async fn purchase_success_charges_once_creates_one_pet_completes_and_publishes_once_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    let BattlePetPurchaseExecutionLikeCpp::Purchased {
        pet_guid,
        published,
    } = outcome
    else {
        panic!("purchase must succeed: {outcome:?}");
    };
    assert!(published, "the Added outcome must record publication");

    // Durable facts: one charge, one completed command, one pet, one receipt.
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 1);
    let commands = fixture.store.commands_snapshot();
    assert_eq!(commands.len(), 1);
    let command = &commands[0];
    assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
    assert_eq!(command.price, SAGA_PRICE);
    assert_eq!(command.money_before, SAGA_MONEY);
    assert_eq!(command.money_after, 750);
    assert_eq!(command.species, SAGA_SPECIES);
    assert_eq!(command.breed, 7);
    assert_eq!(command.quality, 1);
    assert_eq!(command.display_id, 123);
    assert_eq!(command.level, 1);
    assert_eq!(command.trainer_id, TRAINER_ID);
    assert_eq!(command.spell_id, SAGA_SPELL_ID);
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    assert_eq!(fixture.persistence.receipt_count(), 1);
    assert_eq!(
        fixture
            .persistence
            .receipt(command.request_key)
            .map(|pet| pet.guid_counter),
        Some(pet_guid.counter() as u64)
    );

    // Runtime money mirrors the durable charge.
    assert_eq!(fixture.session.player_gold_like_cpp(), 750);

    // Capture fixture (success): money update, then the petAdded journal
    // update, then the dependent learned spell; no trainer visual kits.
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

    // C++ `LearnSpell(dependent=true)`: runtime-known, never durable.
    assert!(
        fixture
            .session
            .represented_dependent_known_spells_like_cpp()
            .contains(&(SAGA_SPELL_ID as i32))
    );
}

#[tokio::test]
async fn purchase_survives_reload_without_replaying_charge_pet_or_publication_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert!(matches!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
    ));
    while fixture.send_rx.try_recv().is_ok() {}
    let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
    let money_before = store.money(PLAYER_COUNTER as u64);
    let pets_before = persistence.pet_count();
    drop(fixture);

    let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(
        summary,
        BattlePetPurchaseRecoveryLikeCpp {
            applied: 0,
            compensated: 0,
            deferred: 0,
            terminal_failures: 0,
        }
    );
    assert_eq!(restarted.store.money(PLAYER_COUNTER as u64), money_before);
    assert_eq!(restarted.store.money_mutations(), 1);
    assert_eq!(restarted.persistence.pet_count(), pets_before);
    assert_no_packets(&restarted);
}

#[tokio::test]
async fn insufficient_money_sends_teach_failure_without_charge_command_or_pet_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(100, Vec::new()).await;
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert_eq!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::InsufficientMoney
    );
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(100));
    assert!(fixture.store.commands_snapshot().is_empty());
    assert_eq!(fixture.persistence.pet_count(), 0);
    assert_eq!(fixture.session.player_gold_like_cpp(), 100);
    // Capture fixture (insufficient money): exactly
    // SMSG_TRAINER_BUY_FAILED with C++ FailReason::NotEnoughMoney.
    assert_eq!(
        fixture.send_rx.try_recv().expect("teach failure"),
        TrainerBuyFailed {
            trainer_guid: saga_trainer_guid_like_cpp(),
            spell_id: SAGA_SPELL_ID as i32,
            reason: 1,
        }
        .to_bytes()
    );
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn charge_failure_before_character_commit_leaves_no_charge_and_no_command_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    fixture
        .store
        .fail_next_charge_pre_commit
        .store(true, Ordering::SeqCst);
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert_eq!(outcome, BattlePetPurchaseExecutionLikeCpp::ChargeDeclined);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 0);
    assert!(fixture.store.commands_snapshot().is_empty());
    assert_eq!(fixture.persistence.pet_count(), 0);
    assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn lost_charge_reply_after_character_commit_converges_to_paid_pet_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    fixture
        .store
        .lose_next_charge_reply
        .store(true, Ordering::SeqCst);
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    // The reconcile attributes the committed charge through the durable
    // row and the purchase completes normally with exactly one charge.
    assert!(matches!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::Purchased {
            published: true,
            ..
        }
    ));
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 1);
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    while fixture.send_rx.try_recv().is_ok() {}
    let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
    drop(fixture);
    let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied + summary.compensated, 0);
    assert_eq!(restarted.persistence.pet_count(), 1);
    assert_no_packets(&restarted);
}

#[tokio::test]
async fn login_insert_failure_retries_to_exactly_one_pet_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    fixture
        .persistence
        .fail_next_insert
        .store(true, Ordering::SeqCst);
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert!(matches!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
    ));
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    assert_eq!(fixture.persistence.receipt_count(), 1);
    assert_eq!(fixture.store.money_mutations(), 1);
}

#[tokio::test]
async fn lost_login_insert_reply_replays_receipt_without_a_second_pet_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    fixture
        .persistence
        .lose_next_insert_reply
        .store(true, Ordering::SeqCst);
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    // The first apply committed but its reply was lost. The #160 owner
    // cannot replay it in-process (its in-memory journal lost the pet
    // with the failed insert), so the saga's terminal-failure path runs
    // and the receipt re-check completes the command instead of
    // refunding: exactly one charge, exactly one durable pet, no
    // refund, no second pet, no publication beyond the charge.
    assert_eq!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::CompletedElsewhere
    );
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    assert_eq!(fixture.persistence.receipt_count(), 1);
    assert_eq!(fixture.store.commands_snapshot().len(), 1);
    assert_eq!(
        fixture.store.commands_snapshot()[0].status,
        BattlePetPurchaseStatusLikeCpp::Completed
    );
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 1);
    assert!(!fixture.store.commands_snapshot()[0].published);
    assert_eq!(
        fixture.send_rx.try_recv().expect("money update"),
        expect_money_update_packet_like_cpp(&fixture, 750)
    );
    assert_no_packets(&fixture);

    // The completed-but-unpublished command is the recovery-publication
    // signal: after a restart the owner holds the pet again and login
    // recovery emits the one success publication, then marks it.
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
    assert!(commands[0].published);
    assert_eq!(restarted.persistence.pet_count(), 1);
    assert_eq!(restarted.store.money_mutations(), 1);
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
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied + summary.compensated + summary.deferred, 0);
    assert_no_packets(&restarted);
}

#[tokio::test]
async fn publication_marker_failure_recovers_without_losing_delivery_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    // Fail every publication/completion mark the live path attempts.
    fixture
        .store
        .fail_marks_remaining
        .store(6, Ordering::SeqCst);
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert_eq!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::RetryableDeferred
    );
    // Pet durable + receipt durable. Both packet enqueues succeeded before
    // the marker, while marker and completion both failed to commit.
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 1);
    assert_eq!(
        fixture.store.commands_snapshot()[0].status,
        BattlePetPurchaseStatusLikeCpp::PendingApplication
    );
    assert!(!fixture.store.commands_snapshot()[0].published);
    assert_eq!(
        fixture.send_rx.try_recv().expect("money update"),
        expect_money_update_packet_like_cpp(&fixture, 750)
    );
    assert!(fixture.send_rx.try_recv().is_ok(), "live pet update");
    assert!(fixture.send_rx.try_recv().is_ok(), "live learned spell");
    assert_no_packets(&fixture);
    let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
    drop(fixture);
    let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    // The receipt replay completes the command and re-sends because the
    // first enqueue could not be proven. Enqueue attempts may repeat and
    // actual delivery remains best-effort; the pet and charge remain
    // exactly-once.
    assert_eq!(summary.applied, 1);
    let commands = restarted.store.commands_snapshot();
    assert_eq!(
        commands[0].status,
        BattlePetPurchaseStatusLikeCpp::Completed
    );
    assert!(commands[0].published);
    assert_eq!(restarted.persistence.pet_count(), 1);
    assert_eq!(restarted.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(restarted.store.money_mutations(), 1);
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
async fn closed_send_channel_preserves_recovery_publication_signal_like_cpp() {
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    let (_replacement_tx, replacement_rx) = flume::bounded::<Vec<u8>>(1);
    let disconnected_rx = std::mem::replace(&mut fixture.send_rx, replacement_rx);
    drop(disconnected_rx);

    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    let BattlePetPurchaseExecutionLikeCpp::Purchased { published, .. } = outcome else {
        panic!("durable purchase must still complete: {outcome:?}");
    };
    assert!(!published);
    let commands = fixture.store.commands_snapshot();
    assert_eq!(
        commands[0].status,
        BattlePetPurchaseStatusLikeCpp::Completed
    );
    assert!(!commands[0].published);
    assert_eq!(fixture.persistence.pet_count(), 1);
    assert_eq!(fixture.store.money_mutations(), 1);
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

    let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
    drop(fixture);
    let mut restarted = restart_saga_session_like_cpp(store, persistence, 750).await;
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied, 1);
    assert!(restarted.store.commands_snapshot()[0].published);
    assert_eq!(
        restarted
            .session
            .represented_battle_pet_unique_owned_criteria_like_cpp(),
        1
    );
    assert_eq!(
        restarted
            .session
            .represented_battle_pet_learned_new_pet_criteria_like_cpp(),
        &[SAGA_SPECIES]
    );
    assert!(restarted.send_rx.try_recv().is_ok(), "recovery pet update");
    assert!(
        restarted.send_rx.try_recv().is_ok(),
        "recovery learned spell"
    );
    assert_no_packets(&restarted);
}

#[tokio::test]
async fn capacity_reached_between_list_and_apply_compensates_exactly_once_like_cpp() {
    // Two pets of the species already exist (admission cap 3 passes),
    // then a concurrent cage fills the last slot before our apply
    // reaches the Login DB capacity lock.
    let seeded = vec![
        saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
        saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
    ];
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
    seed_third_pet_into_persistence_like_cpp(&fixture);
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert_eq!(outcome, BattlePetPurchaseExecutionLikeCpp::Compensated);

    // Charged once, refunded once, no pet, no receipt.
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 2);
    assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
    let commands = fixture.store.commands_snapshot();
    assert_eq!(commands.len(), 1);
    assert_eq!(
        commands[0].status,
        BattlePetPurchaseStatusLikeCpp::Compensated
    );
    assert!(commands[0].failure_reason.is_some());
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 3);
    assert_eq!(fixture.persistence.receipt_count(), 0);

    // Publication on compensation: the charge and the refund money
    // updates only — never a pet packet, never a learned spell.
    assert_eq!(
        fixture.send_rx.try_recv().expect("charge update"),
        expect_money_update_packet_like_cpp(&fixture, 750)
    );
    assert_eq!(
        fixture.send_rx.try_recv().expect("refund update"),
        expect_money_update_packet_like_cpp(&fixture, SAGA_MONEY)
    );
    assert_no_packets(&fixture);

    // A second compensation attempt (replay) cannot refund again.
    assert_eq!(
        fixture
            .store
            .compensate(
                commands[0].request_key,
                wow_entities::MAX_MONEY_AMOUNT,
                test_money_commit_fence_like_cpp(),
            )
            .await
            .expect("replayed compensation"),
        BattlePetPurchaseCompensationOutcomeLikeCpp::AlreadyCompensated {
            durable_money: SAGA_MONEY
        }
    );
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 2);
}

#[tokio::test]
async fn capacity_known_at_admission_is_structured_unavailable_and_wire_silent_like_cpp() {
    let seeded = vec![
        saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
        saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
        saga_durable_pet_row_like_cpp(3, SAGA_SPECIES, None),
    ];
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert_eq!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::Capacity
        )
    );
    // C++ stays silent on the wire (`Trainer.cpp:102-106`), but the
    // structured result keeps the failure observable; nothing is
    // charged, commanded, granted or published.
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 0);
    assert!(fixture.store.commands_snapshot().is_empty());
    assert_eq!(fixture.persistence.species_count(SAGA_SPECIES), 3);
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn journal_lock_held_elsewhere_is_structured_unavailable_and_wire_silent_like_cpp() {
    let first = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    let store = Arc::clone(&first.store);
    let persistence = Arc::clone(&first.persistence);
    let registry = Arc::clone(&first.registry);
    // First session holds the journal lease.
    assert!(
        first
            .session
            .battle_pet_try_acquire_journal_lease_like_cpp()
            .await
    );
    let (mut second_session, _second_rx) =
        make_saga_session_like_cpp(OTHER_PLAYER_COUNTER, SAGA_MONEY);
    second_session.set_battle_pet_purchase_persistence_port_like_cpp(store_handle_like_cpp(&store));
    second_session.set_battle_pet_account_attachment_like_cpp(
        registry
            .attach_like_cpp(ACCOUNT_ID)
            .await
            .expect("second attachment"),
    );
    let guard = second_session
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
        .expect("money exclusivity");
    let outcome = second_session
        .execute_battle_pet_trainer_purchase_like_cpp(
            guard,
            saga_trainer_guid_like_cpp(),
            TRAINER_ID,
            saga_offer_like_cpp(SAGA_PRICE),
        )
        .await;
    assert_eq!(
        outcome,
        BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::JournalLocked
        )
    );
    assert!(store.commands_snapshot().is_empty());
    assert_eq!(persistence.pet_count(), 0);
    // The first session keeps its lease and can still use the journal.
    assert!(
        first
            .session
            .battle_pet_account_owner_lease_like_cpp()
            .is_some()
    );
}

#[tokio::test]
async fn compensation_pre_commit_failure_retries_then_refunds_exactly_once_like_cpp() {
    let seeded = vec![
        saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
        saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
    ];
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
    seed_third_pet_into_persistence_like_cpp(&fixture);
    fixture
        .store
        .fail_next_compensate_pre_commit
        .store(true, Ordering::SeqCst);
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert_eq!(outcome, BattlePetPurchaseExecutionLikeCpp::Compensated);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 2);
}

#[tokio::test]
async fn lost_compensation_reply_refunds_once_and_stays_silent_after_reload_like_cpp() {
    let seeded = vec![
        saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
        saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
    ];
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
    seed_third_pet_into_persistence_like_cpp(&fixture);
    fixture
        .store
        .lose_next_compensate_reply
        .store(true, Ordering::SeqCst);
    let outcome =
        execute_saga_purchase_like_cpp(&mut fixture, saga_offer_like_cpp(SAGA_PRICE)).await;
    assert_eq!(outcome, BattlePetPurchaseExecutionLikeCpp::Compensated);
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(SAGA_MONEY));
    assert_eq!(fixture.store.money_mutations(), 2);
    assert_eq!(fixture.session.player_gold_like_cpp(), SAGA_MONEY);
    while fixture.send_rx.try_recv().is_ok() {}
    let (store, persistence) = (fixture.store.clone(), fixture.persistence.clone());
    drop(fixture);
    let mut restarted = restart_saga_session_like_cpp(store, persistence, SAGA_MONEY).await;
    let summary = restarted
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied + summary.compensated + summary.deferred, 0);
    assert_eq!(restarted.store.money_mutations(), 2);
    assert_no_packets(&restarted);
}

#[tokio::test]
async fn terminal_failure_when_character_row_is_missing_stops_retrying_like_cpp() {
    // The character was deleted after charging: the refund can never
    // apply, so the command must become TerminalFailure instead of
    // retrying forever or losing the charge silently.
    let mut fixture = saga_fixture_like_cpp(SAGA_MONEY, Vec::new()).await;
    let mut command =
        crate::battle_pet_purchase::tests::test_command([77; 16], PLAYER_COUNTER as u64);
    command.account_id = ACCOUNT_ID;
    command.status = BattlePetPurchaseStatusLikeCpp::CompensationPending;
    command.money_before = SAGA_MONEY;
    command.money_after = 750;
    fixture.store.seed_command(command);
    // The character row vanishes: no money row at all.
    fixture
        .store
        .remove_money_row_for_test_like_cpp(PLAYER_COUNTER as u64);
    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.terminal_failures, 1);
    assert_eq!(summary.compensated, 0);
    assert_eq!(
        fixture.store.command([77; 16]).expect("command").status,
        BattlePetPurchaseStatusLikeCpp::TerminalFailure
    );
    // Exactly one compensate attempt: no hot retry loop.
    assert_eq!(fixture.store.compensate_attempts.load(Ordering::SeqCst), 1);
    assert_eq!(fixture.store.money_mutations(), 0);
    assert_eq!(fixture.persistence.pet_count(), 0);
    assert_no_packets(&fixture);
}

#[tokio::test]
async fn replayed_pending_command_from_recovery_never_duplicates_pet_or_publication_like_cpp() {
    // Crash after the Login DB commit: the receipt and pet exist, the
    // Character DB command is still pending.
    let pet_row = saga_durable_pet_row_like_cpp(5, SAGA_SPECIES, None);
    let mut fixture = saga_fixture_like_cpp(750, vec![pet_row.clone()]).await;
    let request_key = [88; 16];
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
    fixture.store.seed_command(command);

    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied, 1);
    assert_eq!(fixture.persistence.pet_count(), 1);
    assert_eq!(fixture.persistence.receipt_count(), 1);
    let command = fixture.store.command(request_key).expect("command");
    assert_eq!(command.status, BattlePetPurchaseStatusLikeCpp::Completed);
    assert!(command.published);
    // The replayed receipt had no recorded publication, so recovery
    // emits the one success publication now — never a second pet.
    assert_eq!(fixture.store.money(PLAYER_COUNTER as u64), Some(750));
    assert_eq!(fixture.store.money_mutations(), 0);
    assert_eq!(
        fixture.send_rx.try_recv().expect("recovery pet update"),
        wow_packet::packets::misc::BattlePetUpdates {
            pets: vec![expected_pet_packet_like_cpp(
                &fixture,
                ObjectGuid::create_global(HighGuid::BattlePet, 0, 5),
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

    // A second recovery finds nothing unconverged and republishes
    // nothing: the durable marker proves publication already happened.
    let summary = fixture
        .session
        .recover_battle_pet_trainer_purchases_like_cpp()
        .await
        .expect("recovery runs");
    assert_eq!(summary.applied + summary.compensated + summary.deferred, 0);
    assert_eq!(fixture.persistence.pet_count(), 1);
    assert_no_packets(&fixture);
}

#[test]
fn concurrent_sessions_charge_once_grant_once_and_compensate_once_like_cpp() {
    // `tokio::join!` polls both purchase futures inline, so both live on
    // this stack at once; the pair does not fit the default test thread.
    run_async_stack_test(concurrent_sessions_charge_once_grant_once_and_compensate_once);
}
