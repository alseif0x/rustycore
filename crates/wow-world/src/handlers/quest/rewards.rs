// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest reward orchestration, secondary effects, and required-item removal.

use super::*;
use crate::session::RepresentedQuestRecurrenceLikeCpp;

use crate::quest::application::QuestRewardDurablePlanLikeCpp;

mod currencies;
mod items;
mod validation;

impl WorldSession {
    fn represented_direct_inventory_count_like_cpp(&self, item_entry: u32) -> Option<u32> {
        Some(
            self.resolved_inventory_items_like_cpp()?
                .values()
                .filter(|item| item.entry_id == item_entry)
                .filter_map(|inventory_item| {
                    self.resolved_inventory_item_object_like_cpp(inventory_item.guid)
                        .filter(|item| !item.is_in_trade())
                        .map(|item| item.count())
                })
                .fold(0u32, u32::saturating_add),
        )
    }

    fn plan_quest_destroy_item_count_direct_like_cpp(
        &self,
        item_entry: u32,
        count: u32,
    ) -> Option<Vec<ExtendedCostItemTurninChange>> {
        let effective_count = if count == u32::MAX {
            self.represented_direct_inventory_count_like_cpp(item_entry)?
        } else {
            count
        };

        if effective_count == 0 {
            return Some(Vec::new());
        }

        self.plan_destroy_item_count_direct_inventory(item_entry, effective_count)
    }

    async fn remove_quest_required_items_and_currencies_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let map_id = self.player_map_id_like_cpp();
        let mut item_changes = Vec::new();
        let Some(currency_snapshot) = self.player_currencies_like_cpp() else {
            return false;
        };
        let mut currency_losses = Vec::new();

        for objective in &quest.objectives {
            match objective.obj_type {
                QUEST_OBJECTIVE_ITEM_LIKE_CPP_LOCAL => {
                    let Ok(item_entry) = u32::try_from(objective.object_id) else {
                        return false;
                    };
                    let count = if (quest.flags & QUEST_FLAGS_REMOVE_SURPLUS_ITEMS_LIKE_CPP) != 0 {
                        u32::MAX
                    } else {
                        u32::try_from(objective.amount).unwrap_or(u32::MAX)
                    };
                    let Some(mut changes) =
                        self.plan_quest_destroy_item_count_direct_like_cpp(item_entry, count)
                    else {
                        return false;
                    };
                    item_changes.append(&mut changes);
                }
                QUEST_OBJECTIVE_CURRENCY_LIKE_CPP_LOCAL => {
                    let (Ok(currency_id), Ok(amount)) = (
                        u32::try_from(objective.object_id),
                        u32::try_from(objective.amount),
                    ) else {
                        return false;
                    };
                    let Some(before) = self.player_currency_quantity(currency_id) else {
                        self.set_player_currencies_like_cpp(currency_snapshot);
                        return false;
                    };
                    if !self.remove_currency(currency_id, amount) {
                        self.set_player_currencies_like_cpp(currency_snapshot);
                        return false;
                    }
                    let Some(after) = self.player_currency_quantity(currency_id) else {
                        self.set_player_currencies_like_cpp(currency_snapshot);
                        return false;
                    };
                    let removed = before.saturating_sub(after);
                    if removed > 0 {
                        currency_losses.push((currency_id, after, removed));
                    }
                }
                _ => {}
            }
        }

        if (quest.flags_ex & QUEST_FLAGS_EX_NO_ITEM_REMOVAL_LIKE_CPP) == 0 {
            for (item_entry, count) in quest.item_drop.iter().zip(quest.item_drop_quantity.iter()) {
                if *item_entry == 0 {
                    continue;
                }
                let count = if *count == 0 { u32::MAX } else { *count };
                let Some(mut changes) =
                    self.plan_quest_destroy_item_count_direct_like_cpp(*item_entry, count)
                else {
                    self.set_player_currencies_like_cpp(currency_snapshot);
                    return false;
                };
                item_changes.append(&mut changes);
            }
        }

        {
            let items = item_changes
                .iter()
                .map(|change| match *change {
                    ExtendedCostItemTurninChange::Update {
                        db_guid, new_count, ..
                    } => wow_persistence::QuestTurnInItemPersistenceLikeCpp::Update {
                        item_guid: db_guid,
                        new_count,
                    },
                    ExtendedCostItemTurninChange::Delete { db_guid, .. } => {
                        wow_persistence::QuestTurnInItemPersistenceLikeCpp::Delete {
                            item_guid: db_guid,
                        }
                    }
                })
                .collect();
            // The removals join the operation's single character transaction.
            // Their currency half is empty here because `_SaveCurrency` writes
            // the complete state once when the operation closes.
            plan.push_inventory_mutation(
                wow_persistence::PlayerInventoryPersistenceRequestLikeCpp::QuestTurnIn(
                    wow_persistence::QuestTurnInPersistenceLikeCpp {
                        owner_guid: player_guid.counter() as u64,
                        items,
                        currency_save: wow_persistence::PlayerCurrencySaveRequestLikeCpp {
                            player_guid: player_guid.counter() as u64,
                            rows: Vec::new(),
                        },
                    },
                ),
            );
        }

        self.apply_item_turnin_changes(player_guid, map_id, &item_changes);
        for (currency_id, quantity, removed) in currency_losses {
            let (Some(quantity), Some(removed)) =
                (i32::try_from(quantity).ok(), i32::try_from(removed).ok())
            else {
                continue;
            };
            self.send_packet(&SetCurrency {
                type_id: currency_id as i32,
                quantity,
                flags: 0,
                weekly_quantity: None,
                tracked_quantity: None,
                max_quantity: None,
                total_earned: None,
                suppress_chat_log: false,
                quantity_change: Some(-removed),
                quantity_gain_source: None,
                quantity_lost_source: Some(CURRENCY_DESTROY_REASON_QUEST_TURNIN_LIKE_CPP),
                first_craft_operation_id: None,
                next_recharge_time: None,
                recharge_cycle_start_time: None,
                overflown_currency_id: None,
            });
        }

        true
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    fn apply_represented_quest_reward_skill_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(test)]
        if quest.reward_skill_line_id != 0 {
            self.quest_test_fixture_like_cpp
                .represented_quest_reward_skill_updates_like_cpp
                .push((quest.reward_skill_line_id, quest.reward_skill_points));
        }
    }

    fn record_represented_quest_reward_spell_casts_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(test)]
        {
            let caster_selection_unrepresented =
                (quest.flags & QUEST_FLAGS_PLAYER_CAST_COMPLETE_LIKE_CPP) == 0;
            if quest.reward_spell > 0 {
                self.quest_test_fixture_like_cpp
                    .represented_quest_reward_spell_casts_like_cpp
                    .push(RepresentedQuestRewardSpellCastLikeCpp {
                        quest_id: quest.id,
                        spell_id: quest.reward_spell,
                        kind: RepresentedQuestRewardSpellKindLikeCpp::RewardSpell,
                        can_delay_teleport_like_cpp: self.represented_can_delay_teleport_like_cpp(),
                        spell_info_lookup_unrepresented: true,
                        caster_selection_unrepresented,
                        cast_spell_runtime_unrepresented: true,
                    });
                return;
            }

            let display_spells = quest.reward_display_spell;
            for (index, spell_id) in display_spells.into_iter().enumerate() {
                if spell_id == 0 {
                    continue;
                }
                self.quest_test_fixture_like_cpp
                    .represented_quest_reward_spell_casts_like_cpp
                    .push(RepresentedQuestRewardSpellCastLikeCpp {
                        quest_id: quest.id,
                        spell_id,
                        kind: RepresentedQuestRewardSpellKindLikeCpp::RewardDisplaySpell {
                            index: index as u8,
                        },
                        can_delay_teleport_like_cpp: self.represented_can_delay_teleport_like_cpp(),
                        spell_info_lookup_unrepresented: true,
                        caster_selection_unrepresented,
                        cast_spell_runtime_unrepresented: true,
                    });
            }
        }
        #[cfg(not(test))]
        let _ = quest;
    }

    fn apply_represented_quest_title_and_talent_rewards_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        #[cfg(test)]
        if quest.reward_title_id != 0 {
            self.quest_test_fixture_like_cpp
                .represented_quest_reward_titles_like_cpp
                .push(RepresentedQuestRewardTitleLikeCpp {
                    quest_id: quest.id,
                    title_id: quest.reward_title_id,
                    char_title_lookup_unrepresented: true,
                    set_title_runtime_unrepresented: true,
                });
        }
        if quest.reward_skill_points != 0 {
            let _ = self.add_represented_quest_reward_talent_points_like_cpp(
                quest.id,
                quest.reward_skill_points,
            );
        }
    }

    fn record_represented_quest_reward_mail_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
        quest_giver_guid: ObjectGuid,
    ) {
        #[cfg(test)]
        {
            if quest.reward_mail_template_id == 0 {
                return;
            }

            self.quest_test_fixture_like_cpp
                .represented_quest_reward_mails_like_cpp
                .push(RepresentedQuestRewardMailLikeCpp {
                    quest_id: quest.id,
                    mail_template_id: quest.reward_mail_template_id,
                    delay_secs: quest.reward_mail_delay_secs,
                    sender_entry: (quest.reward_mail_sender_entry != 0)
                        .then_some(quest.reward_mail_sender_entry),
                    quest_giver_guid: (quest.reward_mail_sender_entry == 0)
                        .then_some(quest_giver_guid),
                    mail_template_lookup_unrepresented: true,
                    mail_draft_runtime_unrepresented: true,
                    character_db_transaction_unrepresented: true,
                });
        }
        #[cfg(not(test))]
        let _ = (quest, quest_giver_guid);
    }

    #[cfg_attr(not(test), allow(unused_variables))]
    fn record_represented_quest_reward_reputation_like_cpp(
        &mut self,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let source = if quest.is_daily_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest
        } else if quest.is_weekly_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::WeeklyQuest
        } else if quest.is_monthly_like_cpp() {
            RepresentedQuestRewardReputationSourceLikeCpp::MonthlyQuest
        } else if quest.is_repeatable() {
            RepresentedQuestRewardReputationSourceLikeCpp::RepeatableQuest
        } else {
            RepresentedQuestRewardReputationSourceLikeCpp::Quest
        };
        let gain_source = match source {
            RepresentedQuestRewardReputationSourceLikeCpp::Quest => {
                ReputationGainSourceLikeCpp::Quest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::DailyQuest => {
                ReputationGainSourceLikeCpp::DailyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::WeeklyQuest => {
                ReputationGainSourceLikeCpp::WeeklyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::MonthlyQuest => {
                ReputationGainSourceLikeCpp::MonthlyQuest
            }
            RepresentedQuestRewardReputationSourceLikeCpp::RepeatableQuest => {
                ReputationGainSourceLikeCpp::RepeatableQuest
            }
        };
        let faction_store = self.faction_store().map(Arc::clone);
        let quest_faction_reward_store = self.quests.faction_reward_store.as_ref().map(Arc::clone);
        let reputation_reward_rate_store = self.reputation_reward_rate_store().map(Arc::clone);
        let reputation_spillover_template_store =
            self.reputation_spillover_template_store().map(Arc::clone);
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store().map(Arc::clone);
        let paragon_reputation_store = self.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.currency_types_store().map(Arc::clone);

        for slot in 0..wow_data::quest::QUEST_REWARD_REPUTATIONS_COUNT {
            let faction_id = quest.reward_faction_ids[slot];
            if faction_id == 0 {
                continue;
            }
            let faction_entry = match faction_store.as_deref() {
                Some(store) => match store.get(faction_id).cloned() {
                    Some(entry) => Some(entry),
                    None => continue,
                },
                None => None,
            };
            let faction_lookup_missing = faction_entry.is_none();

            let reward_faction_override = quest.reward_faction_overrides[slot];
            let (base_reputation_before_gain, no_quest_bonus, quest_faction_reward_lookup) =
                if reward_faction_override != 0 {
                    (reward_faction_override / 100, true, false)
                } else if let Some(store) = quest_faction_reward_store.as_deref() {
                    let row = if quest.reward_faction_values[slot] < 0 {
                        2
                    } else {
                        1
                    };
                    let field = quest.reward_faction_values[slot].unsigned_abs() as usize;
                    let rep = store
                        .get(row)
                        .and_then(|entry| entry.difficulty.get(field).copied())
                        .map(i32::from)
                        .unwrap_or(0);
                    (rep, false, false)
                } else {
                    (0, false, true)
                };

            if base_reputation_before_gain == 0 && !quest_faction_reward_lookup {
                continue;
            }

            let quest_level_for_gain =
                player_quest_level_like_cpp(quest, self.player_level_like_cpp()).max(0) as u32;
            let reputation_rates = self.reputation_rates_like_cpp();
            let Some(percent_before_reward_rate) = self
                .reputation_gain_percent_before_reward_rate_like_cpp(
                    gain_source,
                    quest_level_for_gain,
                    base_reputation_before_gain,
                    faction_id,
                    no_quest_bonus,
                )
            else {
                continue;
            };
            let reputation_after_low_level_rate_like_cpp = calculate_pct_i32_f32_like_cpp(
                base_reputation_before_gain,
                percent_before_reward_rate,
            );
            if reputation_after_low_level_rate_like_cpp == 0 && !quest_faction_reward_lookup {
                continue;
            }

            let (
                reputation_after_reward_rate_like_cpp,
                percent_after_reward_rate_like_cpp,
                reputation_reward_rate_lookup,
            ) = if reputation_reward_rate_store.is_some() {
                if let Some(rate) =
                    self.reputation_reward_rate_for_source_like_cpp(gain_source, faction_id)
                {
                    if rate <= 0.0 {
                        continue;
                    }
                    let percent = percent_before_reward_rate * rate;
                    (
                        calculate_pct_i32_f32_like_cpp(base_reputation_before_gain, percent),
                        percent,
                        false,
                    )
                } else {
                    (
                        reputation_after_low_level_rate_like_cpp,
                        percent_before_reward_rate,
                        false,
                    )
                }
            } else {
                (
                    reputation_after_low_level_rate_like_cpp,
                    percent_before_reward_rate,
                    true,
                )
            };
            let reputation_after_recruit_a_friend_bonus_like_cpp = calculate_pct_i32_f32_like_cpp(
                base_reputation_before_gain,
                self.apply_recruit_a_friend_reputation_bonus_like_cpp(
                    gain_source,
                    percent_after_reward_rate_like_cpp,
                ),
            );
            if reputation_after_recruit_a_friend_bonus_like_cpp == 0 && !quest_faction_reward_lookup
            {
                continue;
            }

            let current_rank_for_cap = if quest.reward_faction_cap_in[slot] != 0
                && reputation_after_recruit_a_friend_bonus_like_cpp > 0
            {
                self.canonical_player_reputation_standing_like_cpp(faction_id)
                    .map(reputation_rank_from_standing_like_cpp)
            } else {
                None
            };
            if current_rank_for_cap.is_some_and(|current_rank| {
                i32::from(current_rank) >= quest.reward_faction_cap_in[slot]
            }) {
                continue;
            }

            let no_spillover = (quest.reward_faction_flags & (1u32 << slot)) != 0;
            let modify_reputation_runtime_unrepresented =
                if let (Some(faction_entry), Some(faction_store)) =
                    (faction_entry.as_ref(), faction_store.as_deref())
                {
                    let options = crate::reputation::mgr::SetReputationOptionsLikeCpp {
                        incremental: true,
                        spillover_only: false,
                        no_spillover,
                        reputation_gain_rate: reputation_rates.gain,
                        paragon_reward_quest_status_none_like_cpp: true,
                        renown_current_level_like_cpp: 0,
                        renown_currency_increased_cap_quantity_like_cpp: 0,
                        player_race: self.player_race_like_cpp(),
                        player_class: self.player_class_like_cpp(),
                    };
                    let db_spillover_template = reputation_spillover_template_store
                        .as_deref()
                        .and_then(|store| store.get(faction_id));
                    let mutation = self.mutate_reputation_mgr_like_cpp(|mgr| {
                        let outcome = mgr.set_reputation_like_cpp(
                            faction_entry,
                            reputation_after_recruit_a_friend_bonus_like_cpp,
                            options,
                            faction_store,
                            db_spillover_template,
                            friendship_rep_reaction_store.as_deref(),
                            paragon_reputation_store.as_deref(),
                            currency_types_store.as_deref(),
                        );
                        let packet = outcome.send_state_rep_list_id.map(|rep_list_id| {
                            mgr.set_faction_standing_packet_like_cpp(Some(rep_list_id))
                        });
                        (outcome, packet)
                    });
                    let owner_unavailable = mutation.is_none();
                    if let Some((_outcome, Some(packet))) = mutation {
                        self.send_packet(&packet);
                    }
                    owner_unavailable
                } else {
                    true
                };

            #[cfg(test)]
            {
                self.quest_test_fixture_like_cpp
                    .represented_quest_reward_reputations_like_cpp
                    .push(RepresentedQuestRewardReputationLikeCpp {
                        quest_id: quest.id,
                        slot: slot as u8,
                        faction_id,
                        reward_faction_value: quest.reward_faction_values[slot],
                        reward_faction_override,
                        reward_faction_cap_in: quest.reward_faction_cap_in[slot],
                        base_reputation_before_gain,
                        reputation_after_low_level_rate_like_cpp,
                        reputation_after_reward_rate_like_cpp,
                        no_quest_bonus,
                        no_spillover,
                        source,
                        faction_store_lookup_unrepresented: faction_lookup_missing,
                        quest_faction_reward_store_lookup_unrepresented:
                            quest_faction_reward_lookup,
                        reputation_reward_rate_lookup_unrepresented: reputation_reward_rate_lookup,
                        gray_level_script_hook_unrepresented: true,
                        reputation_rank_cap_check_unrepresented: quest.reward_faction_cap_in[slot]
                            != 0
                            && reputation_after_recruit_a_friend_bonus_like_cpp > 0
                            && current_rank_for_cap.is_none(),
                        calculate_reputation_gain_unrepresented: true,
                        modify_reputation_runtime_unrepresented,
                    });
            }
        }
    }

    fn apply_quest_reward_lockout_status_like_cpp(
        &mut self,
        plan: &mut QuestRewardDurablePlanLikeCpp,
        quest: &wow_data::quest::QuestTemplate,
    ) {
        let now = GameTime::now().as_secs() as i64;
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let mut save_daily = false;
        let mut save_weekly = false;
        let mut save_monthly = false;
        let mut save_seasonal = false;

        if quest.is_daily_like_cpp() || quest.is_df_quest_like_cpp() {
            save_daily = true;
        } else if quest.is_weekly_like_cpp() {
            save_weekly = true;
        } else if quest.is_monthly_like_cpp() {
            save_monthly = true;
        } else if quest.is_seasonal_like_cpp() {
            save_seasonal = true;
        }

        let recurrence = if save_daily {
            Some(RepresentedQuestRecurrenceLikeCpp::Daily {
                quest_id: quest.id,
                now_secs: now,
                is_df_quest: quest.is_df_quest_like_cpp(),
            })
        } else if save_weekly {
            Some(RepresentedQuestRecurrenceLikeCpp::Weekly { quest_id: quest.id })
        } else if save_monthly {
            Some(RepresentedQuestRecurrenceLikeCpp::Monthly { quest_id: quest.id })
        } else if save_seasonal {
            Some(RepresentedQuestRecurrenceLikeCpp::Seasonal {
                event_id: quest.event_id_for_quest_like_cpp(),
                quest_id: quest.id,
                completed_at: now.max(0) as u64,
            })
        } else {
            None
        };
        if let Some(recurrence) = recurrence
            && !self.record_represented_quest_recurrence_like_cpp(recurrence)
        {
            return;
        }

        let owner_guid = player_guid.counter() as u64;
        let Some(recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return;
        };
        let request = if save_daily {
            let mut quest_ids = recurrence
                .daily_quest_ids_like_cpp()
                .iter()
                .copied()
                .collect::<Vec<_>>();
            quest_ids.extend(recurrence.df_quest_ids_like_cpp().iter().copied());
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Daily {
                owner_guid,
                completed_time: recurrence.last_daily_quest_time_secs_like_cpp(),
                quest_ids,
            }
        } else if save_weekly {
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Weekly {
                owner_guid,
                quest_ids: recurrence
                    .weekly_quest_ids_like_cpp()
                    .iter()
                    .copied()
                    .collect(),
            }
        } else if save_monthly {
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Monthly {
                owner_guid,
                quest_ids: recurrence
                    .monthly_quest_ids_like_cpp()
                    .iter()
                    .copied()
                    .collect(),
            }
        } else if save_seasonal {
            let completions = recurrence
                .seasonal_quests_like_cpp()
                .iter()
                .flat_map(|(event_id, quests)| {
                    quests.iter().filter_map(|(quest_id, completed_time)| {
                        Some(
                            wow_persistence::PlayerQuestSeasonalCompletionPersistenceLikeCpp {
                                quest_id: *quest_id,
                                event_id: *event_id,
                                completed_time: i64::try_from(*completed_time).ok()?,
                            },
                        )
                    })
                })
                .collect();
            wow_persistence::PlayerQuestLockoutPersistenceRequestLikeCpp::Seasonal {
                owner_guid,
                completions,
            }
        } else {
            return;
        };

        // `_SaveDailyQuestStatus` and its siblings belong to the same closing
        // transaction as the rest of the reward (Player.cpp:19634..19638).
        plan.push_lockout(request);
    }

    #[cfg(test)]
    pub(super) async fn reward_represented_quest_like_cpp(
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

    pub(super) async fn reward_represented_quest_with_generator_like_cpp(
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
