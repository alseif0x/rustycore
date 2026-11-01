//! Executor-test fixtures: in-memory Login DB persistence, saga sessions
//! over flume channels and the fault-injection matrix.
//!
//! Separated from the executor_tests root under #656.

use super::*;

mod persistence;
mod session;
mod stores;

pub(crate) use persistence::{FakeSagaPersistenceLikeCpp, owner_of, saga_durable_pet_row_like_cpp};

#[cfg(test)]
pub(crate) use persistence::seed_third_pet_into_persistence_like_cpp;

pub(crate) use session::{SagaFixtureLikeCpp, make_saga_session_like_cpp, saga_fixture_like_cpp};

#[cfg(test)]
pub(crate) use session::{
    assert_no_packets, execute_saga_purchase_like_cpp, expect_money_update_packet_like_cpp,
    expected_pet_packet_like_cpp, restart_saga_session_like_cpp, run_async_stack_test,
    saga_buy_packet_like_cpp, saga_handler_fixture_like_cpp,
};

pub(crate) use stores::{
    insert_saga_trainer_creature_like_cpp, saga_learn_effect_like_cpp, saga_offer_like_cpp,
    saga_registry_like_cpp, saga_selection_like_cpp, saga_species_store_like_cpp,
    saga_summon_effect_like_cpp, saga_trainer_guid_like_cpp, store_handle_like_cpp,
};

pub(crate) const PLAYER_COUNTER: i64 = 42;

pub(crate) const OTHER_PLAYER_COUNTER: i64 = 43;

pub(crate) const ACCOUNT_ID: u32 = 1;

pub(crate) const TRAINER_ID: u32 = 7;

pub(crate) const SAGA_SPELL_ID: u32 = 54_330;

pub(crate) const SAGA_SPECIES: u32 = 11;

pub(crate) const LEGACY_UNIQUE_SPECIES: u32 = 12;

pub(crate) const SAGA_PRICE: u32 = 250;

pub(crate) const SAGA_MONEY: u64 = 1_000;

pub(crate) const REALM_ID: u16 = 7;

pub(crate) const VIRTUAL_REALM: u32 = 0x0102_0007;

pub(crate) async fn concurrent_sessions_charge_once_grant_once_and_compensate_once() {
    let seeded = vec![
        saga_durable_pet_row_like_cpp(1, SAGA_SPECIES, None),
        saga_durable_pet_row_like_cpp(2, SAGA_SPECIES, None),
    ];
    let mut first = saga_fixture_like_cpp(SAGA_MONEY, seeded).await;
    let store = Arc::clone(&first.store);
    let registry = Arc::clone(&first.registry);
    let persistence = Arc::clone(&first.persistence);
    store.seed_money_like_cpp(OTHER_PLAYER_COUNTER as u64, SAGA_MONEY);
    let (mut second_session, second_rx) =
        make_saga_session_like_cpp(OTHER_PLAYER_COUNTER, SAGA_MONEY);
    second_session.set_battle_pet_purchase_persistence_port_like_cpp(store_handle_like_cpp(&store));
    second_session.set_battle_pet_account_attachment_like_cpp(
        registry
            .attach_like_cpp(ACCOUNT_ID)
            .await
            .expect("second attachment"),
    );
    let first_purchase = async {
        let guard = first
            .session
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
            .expect("money exclusivity");
        first
            .session
            .execute_battle_pet_trainer_purchase_like_cpp(
                guard,
                saga_trainer_guid_like_cpp(),
                TRAINER_ID,
                saga_offer_like_cpp(SAGA_PRICE),
            )
            .await
    };
    let second_purchase = async {
        let guard = second_session
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
            .expect("money exclusivity");
        second_session
            .execute_battle_pet_trainer_purchase_like_cpp(
                guard,
                saga_trainer_guid_like_cpp(),
                TRAINER_ID,
                saga_offer_like_cpp(SAGA_PRICE),
            )
            .await
    };
    let (first_outcome, second_outcome) = tokio::join!(first_purchase, second_purchase);
    let outcomes = [first_outcome, second_outcome];
    // The #160 journal lease serializes same-account sessions: exactly
    // one session is admitted and purchases; the other receives the
    // structured journal-lock result without a charge or a command.
    let purchased = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome,
                BattlePetPurchaseExecutionLikeCpp::Purchased {
                    published: true,
                    ..
                }
            )
        })
        .count();
    let locked = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome,
                BattlePetPurchaseExecutionLikeCpp::Unavailable(
                    BattlePetPurchaseAdmissionFailureLikeCpp::JournalLocked
                )
            )
        })
        .count();
    assert_eq!((purchased, locked), (1, 1), "{outcomes:?}");
    // Exactly one new pet (third of the species), one Completed
    // command, one charge; the locked session was never charged.
    assert_eq!(persistence.species_count(SAGA_SPECIES), 3);
    assert_eq!(persistence.receipt_count(), 1);
    let statuses: Vec<_> = store
        .commands_snapshot()
        .into_iter()
        .map(|command| command.status)
        .collect();
    assert_eq!(statuses, vec![BattlePetPurchaseStatusLikeCpp::Completed]);
    let balances: Vec<_> = [PLAYER_COUNTER as u64, OTHER_PLAYER_COUNTER as u64]
        .into_iter()
        .map(|guid| store.money(guid).expect("money row"))
        .collect();
    assert!(
        balances.contains(&750) && balances.contains(&SAGA_MONEY),
        "winner charged once, loser never charged: {balances:?}"
    );
    assert_eq!(store.money_mutations(), 1);
    drop(second_rx);
    // Releasing the winner frees the journal; the loser's retry then
    // meets the filled capacity as a structured unavailable, still
    // without a charge — two sessions can never duplicate the outcome.
    let (winning_session, mut losing_session) = if matches!(
        outcomes[0],
        BattlePetPurchaseExecutionLikeCpp::Purchased { .. }
    ) {
        (first.session, second_session)
    } else {
        (second_session, first.session)
    };
    drop(winning_session);
    let retry_guard = losing_session
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
        .expect("money exclusivity");
    let retry_outcome = losing_session
        .execute_battle_pet_trainer_purchase_like_cpp(
            retry_guard,
            saga_trainer_guid_like_cpp(),
            TRAINER_ID,
            saga_offer_like_cpp(SAGA_PRICE),
        )
        .await;
    assert_eq!(
        retry_outcome,
        BattlePetPurchaseExecutionLikeCpp::Unavailable(
            BattlePetPurchaseAdmissionFailureLikeCpp::Capacity
        )
    );
    assert_eq!(persistence.species_count(SAGA_SPECIES), 3);
    assert_eq!(persistence.receipt_count(), 1);
    assert_eq!(store.money_mutations(), 1);
    assert_eq!(store.commands_snapshot().len(), 1);
}

pub(crate) const SAGA_CREATURE_ENTRY: u32 = 123;

pub(crate) const SAGA_SUMMON_PROPERTIES_ID: u32 = 700;

pub(crate) const SAGA_SUMMON_SLOT_MINIPET_RAW: i64 = 5;

pub(crate) const SAGA_SUMMON_FROM_JOURNAL_RAW: i64 = 0x0020_0000;

pub(crate) const SAGA_WRAPPER_LEARNED_SPELL: u32 = 54_331;

pub(crate) const SAGA_WRAPPER_PRICE: u32 = 25;
