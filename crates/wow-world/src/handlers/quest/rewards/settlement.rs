//! settlement operations at the existing Quest application boundary.

use super::*;

impl WorldSession {

    #[cfg(test)]
    pub(in crate::handlers::quest) async fn reward_represented_quest_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return false;
        };
        self.reward_represented_quest_with_generator_like_cpp(
            generator.as_ref(),
            quest,
            quest_giver_guid,
            choice,
        )
        .await
    }

    pub(in crate::handlers::quest) async fn reward_represented_quest_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
        choice: QuestChoiceItemLikeCpp,
    ) -> bool {
        let quest_id = quest.id;
        let choice_item_id = choice.item_id;
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let owner_guid = player_guid.counter() as u64;
        // C++ `Player::RewardQuest` mutates memory throughout and reaches the
        // database only in its closing `SaveToDB(false)` (Player.cpp:14867).
        // Every removal and grant below records what it needs durable; nothing
        // is written until the operation has finished deciding.
        let mut plan = QuestRewardDurablePlanLikeCpp::new(owner_guid, quest_id);
        self.set_represented_can_delay_teleport_like_cpp(true);

        macro_rules! reward_abort {
            () => {{
                self.set_represented_can_delay_teleport_like_cpp(false);
                return false;
            }};
        }

        if !self
            .remove_quest_required_items_and_currencies_like_cpp(&mut plan, quest)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                "RewardQuest: represented quest objective/item-drop removal failed before reward mutation"
            );
            reward_abort!();
        }

        self.remove_represented_timed_quest_like_cpp(quest_id);

        if !self
            .store_fixed_quest_reward_items_like_cpp(&mut plan, item_guid_generator, quest)
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                "RewardQuest: represented fixed reward item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .store_chosen_quest_reward_item_like_cpp(&mut plan, item_guid_generator, quest, choice)
            .await
        {
            debug!(
                account = self.account_id,
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
                choice,
            )
            .await
        {
            debug!(
                account = self.account_id,
                quest_id,
                choice_item_id,
                "RewardQuest: represented quest package item grant failed before reward mutation"
            );
            reward_abort!();
        }

        if !self
            .grant_quest_reward_currencies_like_cpp(quest, choice)
            .await
        {
            debug!(
                account = self.account_id,
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
        // Retained Rust gap: application grants the raw difficulty field,
        // rather than Player.cpp:14766 GetQuestMoneyReward's calculated value.
        let money = quest.reward_money_difficulty;
        if money > 0 {
            let Some(old_money) = self.resolved_player_money_like_cpp() else {
                reward_abort!();
            };
            let new_money =
                crate::session::loot_money_durable_outcome_like_cpp(old_money, money as u64).0;
            if old_money != new_money {
                plan.set_money(old_money, new_money);
            }
        }

        self.apply_represented_quest_title_and_talent_rewards_like_cpp(quest);
        self.record_represented_quest_reward_mail_like_cpp(quest, quest_giver_guid);
        self.apply_quest_reward_lockout_status_like_cpp(&mut plan, quest);

        let xp = self.quest_xp_reward_like_cpp(quest);
        let rewarded_slot = self.find_quest_slot_like_cpp(quest_id);

        // `_SaveQuestStatus` for this quest joins the same transaction: a
        // rewarded row for a non-repeatable quest, a delete for a repeatable
        // one, exactly as the standalone writes did. The rewarded row is
        // projected before the quest leaves the log because that projection
        // carries only the quest id for a rewarded status, so the statements
        // are identical either way.
        if !quest.is_repeatable() {
            let Some(request) =
                self.plan_quest_status_save_like_cpp(quest_id, QUEST_STATUS_REWARDED_LIKE_CPP)
            else {
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
        let Some(committed_money) = self.commit_quest_reward_plan_like_cpp(plan, quest_id).await
        else {
            reward_abort!();
        };

        // The quest leaves the log only once the transaction is known to have
        // committed. C++ mutates before its save and leaves memory ahead of a
        // failed one until relog; this server keeps the two in step, which is
        // stricter and never weaker.
        self.invalidate_player_quest_status_authority_like_cpp();
        if self.settle_represented_rewarded_quest_like_cpp(quest_id, quest.is_repeatable()) == false
        {
            self.kick("canonical Player quest owner became unavailable after durable COMMIT");
            return false;
        }
        if let Some(committed_money) = committed_money {
            self.enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money: committed_money.money_before,
                    new_money: committed_money.money_after,
                },
            );
        }

        self.sync_player_registry_state_like_cpp();
        if let Some(slot) = rewarded_slot {
            self.send_represented_quest_log_slot_update_like_cpp(slot);
        }

        info!(
            account = self.account_id,
            quest_id,
            xp,
            gold = money,
            repeatable = quest.is_repeatable(),
            "Quest rewarded"
        );

        let game_event_outcome = self
            .notify_game_event_quest_complete_like_cpp(quest_id)
            .await;
        debug!(
            account = self.account_id,
            quest_id,
            outcome = ?game_event_outcome,
            "Represented C++ GameEventMgr::HandleQuestComplete notification after quest reward"
        );

        self.send_packet(&QuestGiverQuestComplete {
            quest_id,
            xp,
            money,
            skill_line_id: quest.reward_skill_line_id,
            skill_points: quest.reward_skill_points,
            use_quest_reward_currency: false,
        });

        self.send_packet(&QuestUpdateComplete { quest_id });

        self.record_represented_quest_reward_reputation_like_cpp(quest);
        self.record_represented_quest_reward_spell_casts_like_cpp(quest);

        if xp > 0 {
            // C++ `Player::RewardQuest` calls `GiveXP(XP, nullptr)`: quest XP
            // does not consume rested XP. RAF remains mutually exclusive with
            // rested XP and may still apply inside `GiveXP`.
            self.give_xp(xp, ObjectGuid::EMPTY, 1.0).await;
        }

        self.set_represented_can_delay_teleport_like_cpp(false);

        true
    }
}
