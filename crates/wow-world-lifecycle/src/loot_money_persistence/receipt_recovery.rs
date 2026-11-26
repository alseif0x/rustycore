use std::{sync::Arc, time::Duration};
use wow_loot::{MoneyReceiptDecision, QuarantinedMoneyClaim};
use wow_persistence::{
    GroupLootMoneyPersistenceOutcomeLikeCpp, GroupLootMoneyPersistencePortLikeCpp,
    GroupLootMoneyReconciliationLikeCpp,
};

/// A single reader per indeterminate claim. It carries no session, guard or
/// strong reference to the source, and never resubmits the payout.
pub(super) fn retry_quarantined_money_receipt(
    fence: QuarantinedMoneyClaim,
    writer: Arc<dyn GroupLootMoneyPersistencePortLikeCpp>,
    id: Option<[u8; 16]>,
    outcomes: Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>,
) {
    tokio::spawn(async move {
        let mut delay = Duration::from_secs(1);
        while fence.is_current() {
            tokio::time::sleep(delay).await;
            if !fence.is_current() {
                break;
            }
            let decision = match writer
                .reconcile_group_loot_money_like_cpp(id, outcomes.clone())
                .await
            {
                GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop => {
                    MoneyReceiptDecision::Committed
                }
                GroupLootMoneyReconciliationLikeCpp::RolledBack => MoneyReceiptDecision::RolledBack,
                GroupLootMoneyReconciliationLikeCpp::Indeterminate { .. } => {
                    delay = (delay * 2).min(Duration::from_secs(60));
                    continue;
                }
            };
            fence.settle(decision);
            break;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        collections::HashMap,
        sync::atomic::{AtomicUsize, Ordering},
    };
    use wow_core::ObjectGuid;
    use wow_loot::{CreatureLoot, OwnedLootAuthority, OwnedLootAuthorityLifecycle};
    use wow_persistence::{
        GroupLootMoneyPersistenceAttemptLikeCpp, GroupLootMoneyPersistenceRequestLikeCpp,
        PersistenceFutureLikeCpp,
    };

    struct ReceiptReader(AtomicUsize);
    impl GroupLootMoneyPersistencePortLikeCpp for ReceiptReader {
        fn attempt_group_loot_money_like_cpp(
            &self,
            _: GroupLootMoneyPersistenceRequestLikeCpp,
        ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyPersistenceAttemptLikeCpp> {
            panic!("late receipt recovery must never submit a payout");
        }

        fn reconcile_group_loot_money_like_cpp(
            &self,
            id: Option<[u8; 16]>,
            _: Vec<GroupLootMoneyPersistenceOutcomeLikeCpp>,
        ) -> PersistenceFutureLikeCpp<'_, GroupLootMoneyReconciliationLikeCpp> {
            assert!(id.is_some());
            let result = match self.0.fetch_add(1, Ordering::SeqCst) {
                0 => GroupLootMoneyReconciliationLikeCpp::Indeterminate { reason: None },
                1 => GroupLootMoneyReconciliationLikeCpp::CommittedOrCapOnlyNoop,
                _ => panic!("settled receipt was queried again"),
            };
            Box::pin(std::future::ready(result))
        }
    }

    #[tokio::test]
    async fn late_writer_read_settles_once_without_resubmitting_or_publishing() {
        let looter = ObjectGuid::create_uniq(54);
        let authority = OwnedLootAuthority::new();
        authority.replace_like_cpp(
            Some(CreatureLoot {
                loot_guid: ObjectGuid::create_uniq(55),
                coins: 7,
                unlooted_count: 0,
                loot_type: 1,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: vec![],
                players_looting: vec![],
                allowed_looters: vec![looter],
                items: vec![],
                looted_by_player: false,
            }),
            HashMap::new(),
        );
        let claim = authority.reserve_money_like_cpp(looter).await.unwrap();
        let mut guard = claim.begin_persistence_guard_like_cpp().unwrap();
        let id = guard.money_claim_id().unwrap();
        let fence = guard.quarantine_money_for_receipt().unwrap();
        let reader = Arc::new(ReceiptReader(AtomicUsize::new(0)));
        retry_quarantined_money_receipt(
            fence,
            reader.clone(),
            id,
            vec![GroupLootMoneyPersistenceOutcomeLikeCpp {
                recipient_guid: 54,
                before: 10,
                after: 17,
                applied_delta: 7,
            }],
        );
        tokio::time::timeout(Duration::from_secs(10), async {
            while authority.lifecycle_like_cpp() == OwnedLootAuthorityLifecycle::Quarantined {
                tokio::time::sleep(Duration::from_millis(30)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(reader.0.load(Ordering::SeqCst), 2);
        assert_eq!(authority.shared_snapshot_like_cpp().unwrap().loot.coins, 0);
    }
}
