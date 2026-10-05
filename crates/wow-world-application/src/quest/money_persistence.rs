// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::sync::Arc;

use wow_world_core::session::QuestRewardPlayerAccessLikeCpp;
use wow_world_inventory::InventoryState;
use wow_world_lifecycle::{ExclusivePlayerMoneyPersistenceLikeCpp, SessionLifecycleState};

use super::{RepresentedQuestObjectiveProgressEventLikeCpp, SessionQuestState};

/// Apply completed detached loot deltas before an absolute money save.
pub async fn reconcile_durable_loot_money_before_save_like_cpp(
    lifecycle: &mut SessionLifecycleState,
    inventory: &mut InventoryState,
    quest_state: &mut SessionQuestState,
    player: &mut QuestRewardPlayerAccessLikeCpp<'_>,
) -> bool {
    let tracker = Arc::clone(lifecycle.durable_loot_money_persistence_tracker_like_cpp());
    tracker.wait_until_idle_like_cpp().await;
    let completions = tracker.pending_completions_like_cpp();
    for completion in completions {
        if completion
            .applied
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .is_err()
        {
            continue;
        }
        let Some(old_money) =
            inventory.resolved_player_money_with_quest_reward_access_like_cpp(player)
        else {
            player.quarantine_like_cpp(
                "canonical Player money owner is unavailable during durable reconciliation",
            );
            return false;
        };
        let new_money = old_money
            .checked_add(completion.durable_applied_amount)
            .filter(|money| *money <= wow_entities::MAX_MONEY_AMOUNT)
            .unwrap_or(old_money);
        if !inventory.set_player_gold_with_quest_reward_access_like_cpp(player, new_money) {
            player.quarantine_like_cpp(
                "canonical Player money owner became unavailable during durable reconciliation",
            );
            return false;
        }
        if old_money != new_money {
            quest_state.enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                },
            );
        }
    }

    // Do not drain money criteria while the save fence is held. That path
    // can reward a quest and re-enter save_player_gold, which would wait
    // on this same fence. Queue the exact transition here; normal command
    // publication or the save caller drains it only after releasing the
    // fence. This keeps a save-first completion from losing MoneyChanged.

    if tracker.is_indeterminate_like_cpp() {
        player.quarantine_like_cpp(
            "loot-money COMMIT outcome is unknown; skipping absolute money save",
        );
        return false;
    }
    true
}

/// Close payout admission, reconcile completed workers, then acquire the
/// per-character mutation lock. The returned guard must span the caller's
/// absolute-money persistence COMMIT.
pub async fn begin_exclusive_player_money_persistence_like_cpp(
    lifecycle: &mut SessionLifecycleState,
    inventory: &mut InventoryState,
    quest_state: &mut SessionQuestState,
    player: &mut QuestRewardPlayerAccessLikeCpp<'_>,
) -> Option<ExclusivePlayerMoneyPersistenceLikeCpp> {
    let tracker = Arc::clone(lifecycle.durable_loot_money_persistence_tracker_like_cpp());
    let save_fence = tracker.close_admission_for_save_like_cpp();
    tracker.wait_until_idle_like_cpp().await;
    if !reconcile_durable_loot_money_before_save_like_cpp(lifecycle, inventory, quest_state, player)
        .await
    {
        return None;
    }
    let mutation_lock = tracker.lock_money_mutation_like_cpp().await;
    Some(ExclusivePlayerMoneyPersistenceLikeCpp::new(
        save_fence,
        mutation_lock,
    ))
}
