// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ordered application flow for one selected quest reward.

use super::super::QuestRewardDurablePlanLikeCpp;
use super::QuestRewardCx;
use wow_core::ObjectGuid;

#[cfg(any(test, feature = "test-fixtures"))]
use super::xp_grants::QuestXpGainFixtureRefsLikeCpp;

impl QuestRewardCx<'_> {
    pub async fn reward_quest_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice_item_id: u32,
        choice_loot_item_type: u8,
        item_loot_item_type: u8,
        currency_loot_item_type: u8,
        #[cfg(any(test, feature = "test-fixtures"))]
        xp_fixtures: &mut QuestXpGainFixtureRefsLikeCpp<'_>,
        #[cfg(any(test, feature = "test-fixtures"))]
        teleport_fixture: &mut wow_world_core::session::state::TeleportState,
        #[cfg(any(test, feature = "test-fixtures"))]
        item_planning_fixtures: &super::item_planning::QuestRewardItemPlanningFixtureRefsLikeCpp<'_>,
        loot: &wow_world_loot::LootState,
        #[cfg(any(test, feature = "test-fixtures"))]
        registry_transport: &Option<Box<wow_world_core::session::PlayerTransportLoginStateLikeCpp>>,
        #[cfg(any(test, feature = "test-fixtures"))]
        registry_vehicle_and_pet: (
            &Option<wow_entities::Vehicle>, &Option<i32>, &Option<u32>,
            &Option<wow_core::ObjectGuid>,
        ),
        #[cfg(any(test, feature = "test-fixtures"))]
        reputation_fixtures: &mut super::reputation::QuestRewardReputationFixtureRefsLikeCpp<'_>,
    ) -> bool {
        let quest_id = quest.id;
        let Some(player_guid) = self.player.player_guid_like_cpp() else {
            return false;
        };
        let owner_guid = player_guid.counter() as u64;
        // C++ `Player::RewardQuest` mutates memory throughout and reaches the
        // database only in its closing `SaveToDB(false)` (Player.cpp:14867).
        // Every removal and grant below records what it needs durable; nothing
        // is written until the operation has finished deciding.
        let mut plan = QuestRewardDurablePlanLikeCpp::new(owner_guid, quest_id);
        self.set_can_delay_teleport_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            teleport_fixture,
            true,
        );

        macro_rules! reward_abort {
            () => {{
                self.set_can_delay_teleport_like_cpp(
                    #[cfg(any(test, feature = "test-fixtures"))]
                    teleport_fixture,
                    false,
                );
                return false;
            }};
        }

        if !self
            .remove_quest_required_items_and_currencies_like_cpp(
                &mut plan,
                quest,
                #[cfg(any(test, feature = "test-fixtures"))]
                xp_fixtures.stats_fixture_refs_like_cpp(),
            )
            .await
        {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id,
                "RewardQuest: represented quest objective/item-drop removal failed before reward mutation"
            );
            reward_abort!();
        }

        Self::remove_represented_timed_quest_like_cpp(
            &mut self.quest_state,
            &mut self.player,
            quest_id,
            self.world_test_consumer,
        );

        if !self
            .store_fixed_quest_reward_items_like_cpp(
                &mut plan, item_guid_generator, quest,
                #[cfg(any(test, feature = "test-fixtures"))]
                item_planning_fixtures,
                #[cfg(any(test, feature = "test-fixtures"))]
                xp_fixtures.vitals_fixture_refs_like_cpp(),
                #[cfg(any(test, feature = "test-fixtures"))]
                reputation_fixtures.state_like_cpp(),
            )
            .await
        {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id,
                "RewardQuest: represented fixed reward item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .store_chosen_quest_reward_item_like_cpp(
                &mut plan,
                item_guid_generator,
                quest,
                choice_item_id,
                choice_loot_item_type,
                item_loot_item_type,
                #[cfg(any(test, feature = "test-fixtures"))]
                item_planning_fixtures,
                #[cfg(any(test, feature = "test-fixtures"))]
                xp_fixtures.vitals_fixture_refs_like_cpp(),
                #[cfg(any(test, feature = "test-fixtures"))]
                reputation_fixtures.state_like_cpp(),
            )
            .await
        {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id,
                choice_item_id,
                "RewardQuest: represented chosen reward item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .store_quest_package_reward_items_like_cpp(
                &mut plan,
                item_guid_generator,
                quest,
                choice_item_id,
                choice_loot_item_type,
                item_loot_item_type,
                #[cfg(any(test, feature = "test-fixtures"))]
                item_planning_fixtures,
                #[cfg(any(test, feature = "test-fixtures"))]
                xp_fixtures.vitals_fixture_refs_like_cpp(),
                #[cfg(any(test, feature = "test-fixtures"))]
                reputation_fixtures.state_like_cpp(),
            )
            .await
        {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id,
                choice_item_id,
                "RewardQuest: represented quest package item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .grant_quest_reward_currencies_like_cpp(
                quest,
                choice_item_id,
                choice_loot_item_type,
                currency_loot_item_type,
            )
            .await
        {
            tracing::debug!(
                account = self.player.account_id_like_cpp(),
                quest_id,
                choice_item_id,
                "RewardQuest: represented quest reward currency grant failed before reward mutation"
            );
            reward_abort!();
        }

        self.apply_represented_quest_reward_skill_like_cpp(quest);

        // C++ `ModifyMoney` is another in-memory mutation whose row is part of
        // the closing character save. Record the operation's money instead of
        // committing it on its own: the transaction below makes the whole
        // reward durable at once, so a money failure can no longer leave the
        // earlier grants written and the quest retryable.
        if !self.record_quest_reward_money_like_cpp(
            &mut plan,
            quest.reward_money_difficulty as i32,
        ) {
            reward_abort!();
        }

        self.apply_represented_quest_title_and_talent_rewards_like_cpp(quest);
        self.record_represented_quest_reward_mail_like_cpp(quest, quest_giver_guid);
        Self::apply_quest_reward_lockout_status_like_cpp(
            &mut self.quest_state,
            &mut self.player,
            &mut plan,
            quest,
            self.world_test_consumer,
        );

        let xp = self.quest_xp_reward_like_cpp(quest);
        let objective_access = self.player.quest_objective_access_like_cpp();
        let rewarded_slot = super::super::objective_progress::find_quest_slot_like_cpp(
            &objective_access,
            self.quest_state,
            quest_id,
            self.world_test_consumer,
        );

        // `_SaveQuestStatus` for this quest joins the same transaction.
        if !quest.is_repeatable() {
            let Some(request) = self.plan_quest_status_for_reward_like_cpp(
                quest_id,
                wow_conditions::QUEST_STATUS_REWARDED_LIKE_CPP,
            ) else {
                reward_abort!();
            };
            plan.set_quest_status(request);
        } else {
            plan.set_quest_status(
                wow_persistence::PlayerQuestStatusPersistenceRequestLikeCpp::Delete {
                    owner_guid,
                    quest_id,
                },
            );
        }

        // Everything the operation decided is now durable or nothing is.
        let Some(committed_money) = self
            .commit_quest_reward_plan_like_cpp(plan, quest_id)
            .await
        else {
            reward_abort!();
        };

        self.invalidate_player_quest_status_authority_like_cpp();
        if !self.settle_rewarded_quest_like_cpp(quest_id, quest.is_repeatable()) {
            self.player
                .quarantine_like_cpp("canonical Player quest owner became unavailable after durable COMMIT");
            return false;
        }
        if let Some(committed_money) = committed_money {
            self.enqueue_money_changed_like_cpp(
                committed_money.money_before,
                committed_money.money_after,
            );
        }

        self.sync_quest_reward_registry_like_cpp(
            loot,
            #[cfg(any(test, feature = "test-fixtures"))]
            item_planning_fixtures,
            #[cfg(any(test, feature = "test-fixtures"))]
            xp_fixtures.vitals_fixture_refs_like_cpp(),
            #[cfg(any(test, feature = "test-fixtures"))]
            registry_transport,
            #[cfg(any(test, feature = "test-fixtures"))]
            registry_vehicle_and_pet,
        );
        if let Some(slot) = rewarded_slot {
            self.send_quest_reward_log_slot_update_like_cpp(slot);
        }

        let money = quest.reward_money_difficulty;
        tracing::info!(
            account = self.player.account_id_like_cpp(),
            quest_id,
            xp,
            gold = money,
            repeatable = quest.is_repeatable(),
            "Quest rewarded"
        );

        let game_event_outcome = self
            .notify_game_event_quest_complete_like_cpp(quest_id)
            .await;
        tracing::debug!(
            account = self.player.account_id_like_cpp(),
            quest_id,
            outcome = ?game_event_outcome,
            "Represented C++ GameEventMgr::HandleQuestComplete notification after quest reward"
        );

        let _ = self.send_packet_like_cpp(&wow_packet::packets::quest::QuestGiverQuestComplete {
            quest_id,
            xp,
            money,
            skill_line_id: quest.reward_skill_line_id,
            skill_points: quest.reward_skill_points,
            use_quest_reward_currency: false,
        });

        let _ = self.send_packet_like_cpp(&wow_packet::packets::quest::QuestUpdateComplete {
            quest_id,
        });

        self.record_represented_quest_reward_reputation_like_cpp(
            quest,
            #[cfg(any(test, feature = "test-fixtures"))]
            reputation_fixtures,
        );
        self.record_represented_quest_reward_spell_casts_like_cpp(
            quest,
            #[cfg(any(test, feature = "test-fixtures"))]
            teleport_fixture,
        );

        if xp > 0 {
            // C++ `Player::RewardQuest` calls `GiveXP(XP, nullptr)`: quest XP
            // does not consume rested XP. RAF remains mutually exclusive with
            // rested XP and may still apply inside `GiveXP`.
            self.give_xp_like_cpp(
                xp,
                ObjectGuid::EMPTY,
                1.0,
                #[cfg(any(test, feature = "test-fixtures"))]
                xp_fixtures,
            )
            .await;
        }

        self.set_can_delay_teleport_like_cpp(
            #[cfg(any(test, feature = "test-fixtures"))]
            teleport_fixture,
            false,
        );

        true
    }

    fn record_quest_reward_money_like_cpp(
        &self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        money: i32,
    ) -> bool {
        if money <= 0 {
            return true;
        }
        let Some(old_money) = self
            .inventory
            .resolved_player_money_with_quest_reward_access_like_cpp(&self.player)
        else {
            return false;
        };
        let new_money = old_money
            .checked_add(money as u64)
            .filter(|money| *money <= wow_entities::MAX_MONEY_AMOUNT)
            .unwrap_or(old_money);
        if old_money != new_money {
            plan.set_money(old_money, new_money);
        }
        true
    }
}
