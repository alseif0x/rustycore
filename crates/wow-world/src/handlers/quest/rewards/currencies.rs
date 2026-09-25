// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Currency grants within the quest-reward plan.

use super::*;

impl WorldSession {
    async fn grant_quest_reward_currency_like_cpp(
        &mut self,
        currency_id: u32,
        amount: u32,
        gain_source: CurrencyGainSourceLikeCpp,
    ) -> bool {
        let Some(currency_snapshot) = self.player_currencies_like_cpp() else {
            return false;
        };
        let delta = match self.add_currency_quest_reward_like_cpp(currency_id, amount, gain_source)
        {
            Ok(delta) => delta,
            Err(()) => {
                self.set_player_currencies_like_cpp(currency_snapshot);
                return false;
            }
        };

        // C++ `AddCurrency` mutates memory only. `_SaveCurrency` writes the
        // player's complete currency state once inside the closing save
        // (Player.cpp:19654), so the operation records it at its end rather
        // than committing each grant on its own.
        let _ = currency_snapshot;

        if let Some(delta) = delta {
            let (Some(quantity), Some(amount)) = (
                i32::try_from(delta.quantity).ok(),
                i32::try_from(delta.amount).ok(),
            ) else {
                return true;
            };
            let mut packet = SetCurrency {
                type_id: delta.currency_id as i32,
                quantity,
                flags: 0,
                weekly_quantity: delta
                    .weekly_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                tracked_quantity: None,
                max_quantity: delta
                    .max_quantity
                    .and_then(|value| i32::try_from(value).ok()),
                total_earned: delta
                    .total_earned
                    .and_then(|value| i32::try_from(value).ok()),
                suppress_chat_log: delta.suppress_chat_log,
                quantity_change: Some(amount),
                quantity_gain_source: Some(gain_source as i32),
                quantity_lost_source: None,
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            };
            packet.suppress_chat_log = delta.suppress_chat_log;
            self.send_packet(&packet);
        }

        true
    }

    pub(super) async fn grant_quest_reward_currencies_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let gain_source = quest.currency_gain_source_like_cpp();

        if choice.loot_item_type == QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP
            && choice.item_id != 0
            && self
                .currency_types_store()
                .is_some_and(|store| store.has_record(choice.item_id))
        {
            for ((currency_id, count), item_type) in quest
                .reward_choice_items
                .iter()
                .zip(quest.reward_choice_item_types.iter())
            {
                if *currency_id == 0
                    || *item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP
                    || *currency_id != choice.item_id
                {
                    continue;
                }

                if !self
                    .grant_quest_reward_currency_like_cpp(*currency_id, *count, gain_source)
                    .await
                {
                    return false;
                }
            }
        }

        for (currency_id, count) in quest
            .reward_currencies
            .iter()
            .zip(quest.reward_currency_amounts.iter())
        {
            if *currency_id == 0 || *count == 0 {
                continue;
            }

            if !self
                .grant_quest_reward_currency_like_cpp(*currency_id, *count, gain_source)
                .await
            {
                return false;
            }
        }

        true
    }
}
