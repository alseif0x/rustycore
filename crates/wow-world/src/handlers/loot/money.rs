// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot money application and distribution.

// Explicit database imports: this module reaches its parent through
// `use super::*`, and the persistence inventory cannot resolve a glob, so
// without these every database access in the file is invisible to the
// ratchet (see #277).
use super::*;

impl WorldSession {
    pub(super) async fn apply_durable_represented_loot_money_payout_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        notified_amount: u64,
        durable_applied_amount: u64,
        sole_looter: bool,
        apply_money: bool,
        publish: bool,
    ) -> ApplyLootMoneyResultLikeCpp {
        if self.player_guid().is_none() {
            return ApplyLootMoneyResultLikeCpp::TargetMismatch;
        }
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return ApplyLootMoneyResultLikeCpp::TargetMismatch;
        };
        let new_money = if apply_money {
            old_money
                .checked_add(durable_applied_amount)
                .filter(|money| *money <= MAX_MONEY_AMOUNT)
                .unwrap_or(old_money)
        } else {
            old_money
        };

        if apply_money && !self.set_player_gold_like_cpp(new_money) {
            return ApplyLootMoneyResultLikeCpp::TargetMismatch;
        }
        if apply_money && durable_applied_amount != 0 {
            self.quest_state
                .enqueue_represented_quest_objective_progress_like_cpp(
                    RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                        old_money,
                        new_money,
                    },
                );
        }
        if publish {
            self.send_packet(&LootMoneyNotify {
                money: notified_amount,
                money_mod: 0,
                sole_looter,
            });
        }
        if apply_money || publish {
            self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            )
            .await;
        }

        ApplyLootMoneyResultLikeCpp::Applied
    }
}
