//! Stored money requires and consumes the original typed persistence port.
use super::money_support::*;
use wow_persistence::{
    PersistenceFutureLikeCpp, StoredItemMoneyPersistenceAttemptLikeCpp,
    StoredItemMoneyPersistenceOutcomeLikeCpp, StoredItemMoneyPersistencePortLikeCpp,
    StoredItemMoneyPersistenceRequestLikeCpp, StoredItemMoneyReconciliationLikeCpp,
};
use wow_world::test_fixtures::loot::consume_stored_money_with_port_for_test;

struct StoredItemMoneyPortFixtureLikeCpp {
    attempt: StoredItemMoneyPersistenceAttemptLikeCpp,
}

impl StoredItemMoneyPersistencePortLikeCpp for StoredItemMoneyPortFixtureLikeCpp {
    fn attempt_stored_item_money_like_cpp(
        &self,
        _request: StoredItemMoneyPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemMoneyPersistenceAttemptLikeCpp> {
        Box::pin(std::future::ready(self.attempt.clone()))
    }

    fn reconcile_stored_item_money_like_cpp(
        &self,
        _request: StoredItemMoneyPersistenceRequestLikeCpp,
        _outcome: StoredItemMoneyPersistenceOutcomeLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, StoredItemMoneyReconciliationLikeCpp> {
        Box::pin(std::future::ready(
            StoredItemMoneyReconciliationLikeCpp::Committed,
        ))
    }
}

#[tokio::test]
async fn stored_item_money_worker_requires_and_uses_the_typed_persistence_port_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 501);
    let item_guid = ObjectGuid::create_item(1, 502);
    session.set_player_guid(Some(player_guid));
    clear_money_persistence_outcome_for_test(&mut session);

    assert!(
        consume_stored_money_with_port_for_test(&session, item_guid, 7)
            .await
            .is_none(),
        "production-shaped stored money fails closed without its typed port"
    );

    session.set_stored_item_money_persistence_port_like_cpp(Arc::new(
        StoredItemMoneyPortFixtureLikeCpp {
            attempt: StoredItemMoneyPersistenceAttemptLikeCpp::Applied(
                StoredItemMoneyPersistenceOutcomeLikeCpp {
                    before: 100,
                    after: 107,
                    applied_delta: 7,
                    notified_amount: 7,
                },
            ),
        },
    ));
    let (_, _, applied_delta, notified_amount) =
        consume_stored_money_with_port_for_test(&session, item_guid, 7)
            .await
            .expect("typed stored-money port outcome");
    assert_eq!((applied_delta, notified_amount), (7, 7));
}
