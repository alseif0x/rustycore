use std::sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
};
use wow_core::ObjectGuid;
use wow_loot::{OwnedLootAuthority, OwnedLootSnapshot};
use wow_world_core::player_directory::PlayerRegistry;

/// Routing state for one durable item claim. The completion owns this
/// independently of the packet waiter, so a timeout, cancellation, or later
/// `CMSG_LOOT_RELEASE` cannot suppress the result of an already ordered claim.
/// C++ serializes these handlers and notifies synchronously; Rust's SQL wait is
/// an implementation detail, so the pre-COMMIT cohort is retained and the
/// exact COMMIT snapshot adds viewers that opened during that wait. `published`
/// is shared with the normal handler path and provides the single publication
/// CAS.
#[derive(Clone)]
pub struct DurableLootItemFanoutLikeCpp {
    pub owner_guid: ObjectGuid,
    pub loot_obj: ObjectGuid,
    pub loot_list_id: u8,
    pub player_guid: ObjectGuid,
    pub free_for_all: bool,
    pub authority: OwnedLootAuthority,
    pub authority_generation: u64,
    pub precommit_snapshot: OwnedLootSnapshot,
    /// Exact post-mutation pool captured by the claim commit while the
    /// authority mutex is still held. A viewer opening after that point has
    /// already observed the removed item and must not receive a stale
    /// `LootRemoved` fanout.
    pub committed_snapshot: Arc<OnceLock<OwnedLootSnapshot>>,
    pub source_send_tx: flume::Sender<Vec<u8>>,
    pub player_registry: Option<Arc<PlayerRegistry>>,
    pub map_id: u16,
    pub instance_id: u32,
    pub published: Arc<AtomicBool>,
}

impl std::fmt::Debug for DurableLootItemFanoutLikeCpp {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("DurableLootItemFanoutLikeCpp")
            .field("owner_guid", &self.owner_guid)
            .field("loot_obj", &self.loot_obj)
            .field("loot_list_id", &self.loot_list_id)
            .field("player_guid", &self.player_guid)
            .field("free_for_all", &self.free_for_all)
            .field("authority_generation", &self.authority_generation)
            .field("map_id", &self.map_id)
            .field("instance_id", &self.instance_id)
            .field("published", &self.published.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

/// Runtime publication retained by a detached durable-loot worker after its
/// SQL transaction commits. Every detached item grant is tracked until the
/// live Player inventory is synchronized; Item owners additionally use the
/// completion to publish stored-container removal/release state. The same
/// tracker also prevents a committed stored-container money payout from being
/// overwritten by a stale disconnect save.
#[derive(Debug, Clone)]
pub struct DurableItemLootCompletionLikeCpp {
    pub owner_guid: ObjectGuid,
    pub loot_list_id: u8,
    pub player_guid: ObjectGuid,
    pub item_owner_auto_release: bool,
    /// Delta accepted from the character row locked in the same transaction
    /// that deletes an Item owner's stored-money row. Keeping a delta preserves
    /// intervening local/group changes; `None` identifies an item grant.
    pub durable_item_money_applied_amount: Option<u64>,
    /// Original C++ loot-money notification amount. This is retained instead
    /// of reconstructing it from runtime balances, and is emitted even when it
    /// is zero.
    pub durable_item_money_notified_amount: Option<u64>,
    /// Exact-once gate shared with the per-character durable money tracker.
    /// A save fence may apply the balance delta before this completion gets a
    /// chance to publish source removal and client notification.
    pub durable_item_money_balance_applied: Option<Arc<AtomicBool>>,
    /// Retained only for object-owned item claims. Stored-container money has
    /// its own durable fanout route.
    pub item_fanout: Option<DurableLootItemFanoutLikeCpp>,
    /// Exact-once gate for item/source publication and lifecycle. This is
    /// intentionally separate from `durable_item_money_balance_applied`.
    /// Item-grant completions observed false require a relog; stored-money
    /// completions can publish source removal after a save applied the balance.
    pub runtime_inventory_applied: Arc<AtomicBool>,
}

#[derive(Debug, Default)]
struct DurableItemLootPersistenceStateLikeCpp {
    in_flight: usize,
    completions: Vec<DurableItemLootCompletionLikeCpp>,
}

/// Session-local counterpart to an authority persistence guard for durable
/// loot grants. It lets logout/disconnect wait for detached item/money
/// transactions, publish their committed runtime state, and only then run C++
/// `DoLootReleaseAll` and save the Player.
#[derive(Debug, Clone)]
pub struct DurableItemLootPersistenceTrackerLikeCpp {
    state: Arc<Mutex<DurableItemLootPersistenceStateLikeCpp>>,
    changed: tokio::sync::watch::Sender<u64>,
}

impl Default for DurableItemLootPersistenceTrackerLikeCpp {
    fn default() -> Self {
        let (changed, _) = tokio::sync::watch::channel(0);
        Self {
            state: Arc::new(Mutex::new(DurableItemLootPersistenceStateLikeCpp::default())),
            changed,
        }
    }
}

impl DurableItemLootPersistenceTrackerLikeCpp {
    pub fn begin_like_cpp(&self) -> DurableItemLootPersistenceGuardLikeCpp {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .in_flight += 1;
        DurableItemLootPersistenceGuardLikeCpp {
            tracker: self.clone(),
            completion: None,
        }
    }

    pub async fn wait_until_idle_like_cpp(&self) {
        let mut changed = self.changed.subscribe();
        loop {
            if self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .in_flight
                == 0
            {
                return;
            }
            if changed.changed().await.is_err() {
                return;
            }
        }
    }

    pub fn take_completions_like_cpp(&self) -> Vec<DurableItemLootCompletionLikeCpp> {
        std::mem::take(
            &mut self
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .completions,
        )
    }
}

#[derive(Debug)]
pub struct DurableItemLootPersistenceGuardLikeCpp {
    tracker: DurableItemLootPersistenceTrackerLikeCpp,
    completion: Option<DurableItemLootCompletionLikeCpp>,
}

impl DurableItemLootPersistenceGuardLikeCpp {
    pub fn mark_committed_like_cpp(&mut self, completion: DurableItemLootCompletionLikeCpp) {
        self.completion = Some(completion);
    }
}

impl Drop for DurableItemLootPersistenceGuardLikeCpp {
    fn drop(&mut self) {
        let mut state = self
            .tracker
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(completion) = self.completion.take() {
            state.completions.push(completion);
        }
        state.in_flight = state.in_flight.saturating_sub(1);
        drop(state);
        self.tracker
            .changed
            .send_modify(|version| *version = version.wrapping_add(1));
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::DurableItemLootPersistenceTrackerLikeCpp;

    #[tokio::test]
    async fn completion_between_idle_check_and_wait_poll_is_not_lost_like_cpp() {
        let tracker = DurableItemLootPersistenceTrackerLikeCpp::default();
        let guard = tracker.begin_like_cpp();
        let mut changed = tracker.changed.subscribe();
        assert_eq!(
            tracker
                .state
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .in_flight,
            1
        );

        // Deliberately publish after the locked busy observation but before
        // `changed()` is first polled. watch retains the version transition.
        drop(guard);
        tokio::time::timeout(Duration::from_secs(1), changed.changed())
            .await
            .expect("durable item persistence wake must not be lost")
            .unwrap();
        tracker.wait_until_idle_like_cpp().await;
    }
}
