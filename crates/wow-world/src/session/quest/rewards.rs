//! Represented quest completion, turn-in and reward choice.
//!
//! Moved out of the Session root under #605. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub fn set_game_event_quest_complete_sender_like_cpp(
        &mut self,
        sender: flume::Sender<GameEventQuestCompleteCommandLikeCpp>,
    ) {
        self.game_event_quest_complete_tx = Some(sender);
    }
    pub async fn notify_game_event_quest_complete_like_cpp(
        &self,
        quest_id: u32,
    ) -> GameEventQuestCompleteClientOutcomeLikeCpp {
        let Some(sender) = self.game_event_quest_complete_tx.as_ref() else {
            return GameEventQuestCompleteClientOutcomeLikeCpp::SenderMissing { quest_id };
        };

        let (response_tx, response_rx) = flume::bounded(1);
        let command = GameEventQuestCompleteCommandLikeCpp {
            quest_id,
            response_tx,
        };
        if sender.try_send(command).is_err() {
            return GameEventQuestCompleteClientOutcomeLikeCpp::SendFailed { quest_id };
        }

        match tokio::time::timeout(Duration::from_millis(250), response_rx.recv_async()).await {
            Ok(Ok(response)) => GameEventQuestCompleteClientOutcomeLikeCpp::Ok(response),
            Ok(Err(_)) => {
                GameEventQuestCompleteClientOutcomeLikeCpp::ResponseChannelClosed { quest_id }
            }
            Err(_) => GameEventQuestCompleteClientOutcomeLikeCpp::ResponseTimeout { quest_id },
        }
    }
    /// C++ `Player::AddCurrency(..., CurrencyGainSource::*QuestReward*)`.
    ///
    /// This represented seam intentionally fails closed for award conditions and reputation
    /// conversion currencies until those runtime systems are available to quest rewards.
    pub(crate) fn add_currency_quest_reward_like_cpp(
        &mut self,
        currency_id: u32,
        amount: u32,
        gain_source: CurrencyGainSourceLikeCpp,
    ) -> Result<Option<PlayerCurrencyDelta>, ()> {
        if amount == 0 {
            return Ok(None);
        }

        let Some(entry) = self
            .currency_types_store
            .as_ref()
            .and_then(|store| store.get(currency_id))
            .copied()
        else {
            return Err(());
        };

        let player_team = player_team_for_race_cpp(self.player_race_like_cpp());
        if (entry.is_alliance() && player_team != Team::Alliance)
            || (entry.is_horde() && player_team != Team::Horde)
        {
            return Ok(None);
        }

        if entry.award_condition_id != 0 {
            return Err(());
        }
        if entry.faction_id != 0 || currency_id == CurrencyTypes::Azerite as u32 {
            return Ok(None);
        }

        let ignore_caps = matches!(
            gain_source,
            CurrencyGainSourceLikeCpp::QuestRewardIgnoreCaps
                | CurrencyGainSourceLikeCpp::WorldQuestRewardIgnoreCaps
        );
        let mut currencies = self.player_currencies_like_cpp().ok_or(())?;
        let currency = currencies.entry(currency_id).or_insert(PlayerCurrency {
            state: PlayerCurrencyState::New,
            quantity: 0,
            weekly_quantity: 0,
            tracked_quantity: 0,
            increased_cap_quantity: 0,
            earned_quantity: 0,
            flags: 0,
        });

        let weekly_cap = entry.max_earnable_per_week;
        let mut applied = amount;
        if !ignore_caps {
            if weekly_cap != 0 && currency.weekly_quantity.saturating_add(applied) > weekly_cap {
                applied = weekly_cap.saturating_sub(currency.weekly_quantity);
            }

            let max_quantity = currency_max_quantity_cpp(&entry, currency);
            if max_quantity != 0 && currency.quantity.saturating_add(applied) > max_quantity {
                applied = max_quantity.saturating_sub(currency.quantity);
            }
        }

        if applied == 0 {
            return Ok(None);
        }

        if currency.state != PlayerCurrencyState::New {
            currency.state = PlayerCurrencyState::Changed;
        }
        currency.quantity = currency.quantity.saturating_add(applied);
        if !ignore_caps {
            if weekly_cap != 0 {
                currency.weekly_quantity = currency.weekly_quantity.saturating_add(applied);
            }
            if entry.is_tracking_quantity() {
                currency.tracked_quantity = currency.tracked_quantity.saturating_add(applied);
            }
            if entry.has_total_earned() {
                currency.earned_quantity = currency.earned_quantity.saturating_add(applied);
            }
        }

        let scaler = entry.scaler().max(1) as u32;
        let max_quantity = currency_max_quantity_cpp(&entry, currency);
        let delta = PlayerCurrencyDelta {
            currency_id,
            quantity: currency.quantity,
            amount: applied,
            weekly_quantity: ((currency.weekly_quantity / scaler) > 0)
                .then_some(currency.weekly_quantity),
            max_quantity: (max_quantity != 0).then_some(max_quantity),
            total_earned: entry.has_total_earned().then_some(currency.earned_quantity),
            suppress_chat_log: entry.is_suppressing_chat_log(false),
        };
        if !self.set_player_currencies_like_cpp(currencies) {
            return Err(());
        }
        Ok(Some(delta))
    }
    pub(crate) fn record_represented_rewarded_quest_row_like_cpp(&mut self, quest_id: u32) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.rewarded_quest_rows.insert(quest_id);
        });
    }
    pub(crate) fn complete_player_quest_status_authority_load_like_cpp(&mut self) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            state.status_authority_complete = true;
        });
    }
    pub(crate) fn represented_player_has_rewarded_quest_like_cpp(
        &self,
        quest_id: u32,
    ) -> Option<bool> {
        Some(
            self.player_quest_gameplay_snapshot_like_cpp()?
                .rewarded_quest_ids
                .contains(&quest_id),
        )
    }
    /// Set the QuestFactionReward store used by C++ quest reputation reward lookup.
    pub fn set_quest_faction_reward_store(&mut self, store: Arc<QuestFactionRewardStore>) {
        self.quests.faction_reward_store = Some(store);
    }
    /// Represented C++ `Player::LearnQuestRewardedSpells`.
    ///
    /// C++ casts each rewarded quest's `RewardSpell` only when that spell exists,
    /// has a missing `SPELL_EFFECT_LEARN_SPELL` trigger, and the first learned
    /// spell is tied to `SKILL_LINE_ABILITY_REWARDED_FROM_QUEST` when it is not
    /// already known. This represented slice applies only the resulting direct
    /// learned-spell side effect; full `CastSpell` runtime semantics remain in
    /// the spell-system roadmap.
    pub(crate) fn apply_represented_quest_rewarded_spells_like_cpp(&mut self) -> usize {
        let Some(quest_store) = self.quests.store.clone() else {
            return 0;
        };

        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return 0;
        };
        let mut quest_ids = quests
            .rewarded_quest_ids
            .iter()
            .copied()
            .collect::<Vec<_>>();
        quest_ids.sort_unstable();

        let mut learned = 0usize;
        for quest_id in quest_ids {
            let Some(quest) = quest_store.get(quest_id) else {
                continue;
            };
            if quest.reward_spell == u32::MAX && quest.source_spell_id != 0 {
                let Ok(source_spell_id) = i32::try_from(quest.source_spell_id) else {
                    continue;
                };
                learned += self.remove_represented_auras_due_to_spell_like_cpp(source_spell_id);
                continue;
            }
            for spell_id in self.represented_quest_rewarded_spell_triggers_like_cpp(quest) {
                self.learn_known_spell_like_cpp(spell_id);
                learned += 1;
            }
        }

        learned
    }
    fn represented_quest_rewarded_spell_triggers_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Vec<i32> {
        if quest.reward_spell == 0 {
            return Vec::new();
        }

        let Ok(reward_spell_id) = i32::try_from(quest.reward_spell) else {
            return Vec::new();
        };

        let Some(spell_info) = self
            .spell_store()
            .and_then(|store| store.get(reward_spell_id))
        else {
            return Vec::new();
        };

        let missing_learn_triggers = spell_info
            .effects()
            .iter()
            .filter(|effect| {
                effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL
                    && effect.effect_trigger_spell > 0
                    && !self
                        .known_spells_like_cpp()
                        .contains(&effect.effect_trigger_spell)
            })
            .map(|effect| effect.effect_trigger_spell)
            .collect::<Vec<_>>();

        if missing_learn_triggers.is_empty() || spell_info.effects().is_empty() {
            return Vec::new();
        }

        let learned_0 = spell_info.effects()[0].effect_trigger_spell;
        if learned_0 > 0
            && !self.known_spells_like_cpp().contains(&learned_0)
            && !self.skill_store().is_some_and(|store| {
                store
                    .get_skill_line_ability_map_bounds_like_cpp(learned_0)
                    .iter()
                    .any(|ability| {
                        ability.acquire_method
                            == wow_data::skill::SKILL_LINE_ABILITY_REWARDED_FROM_QUEST_LIKE_CPP
                    })
            })
        {
            return Vec::new();
        }

        missing_learn_triggers
    }
    #[cfg(test)]
    pub(in crate::session) fn represented_quest_rewarded_talent_points_like_cpp(
        &self,
    ) -> Option<u32> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.gameplay_state().quest_rewarded_talent_points
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(
                self.represented_quest_reward_talent_points_like_cpp
                    .iter()
                    .map(|reward| reward.points)
                    .sum(),
            );
        }
        canonical
    }
    pub(crate) fn add_represented_quest_reward_talent_points_like_cpp(
        &mut self,
        quest_id: u32,
        points: u32,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.add_quest_rewarded_talent_points_like_cpp(points);
            })
            .is_some();
        if canonical {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_quest_reward_talent_points_like_cpp.push(
                RepresentedQuestRewardTalentPointsLikeCpp {
                    quest_id,
                    points,
                    init_talent_for_level_unrepresented: true,
                },
            );
            return true;
        }
        let _ = quest_id;
        false
    }
    /// Set the QuestMoneyReward store (loaded from QuestMoneyReward.db2).
    pub fn set_quest_money_reward_store(&mut self, store: Arc<QuestMoneyRewardStore>) {
        self.quests.money_reward_store = Some(store);
    }
    pub(in crate::session) fn set_loaded_quest_completed_bit_like_cpp(
        &mut self,
        quest_bit: u32,
    ) -> bool {
        if quest_bit == 0 {
            return false;
        }

        let field_offset = (quest_bit - 1) / QUESTS_COMPLETED_BITS_PER_BLOCK;
        if field_offset as usize >= QUESTS_COMPLETED_BITS_SIZE {
            return false;
        }

        let canonical_changed = self
            .mutate_canonical_player_like_cpp(|player| {
                player.set_quest_completed_bit_like_cpp(quest_bit, true)
            })
            .unwrap_or(false);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
                .represented_quest_completed_bits_like_cpp
                .insert(quest_bit);
        }
        canonical_changed
    }
    pub(in crate::session) fn clear_loaded_quest_completed_bit_like_cpp(
        &mut self,
        quest_bit: u32,
    ) -> bool {
        if quest_bit == 0 {
            return false;
        }

        let field_offset = (quest_bit - 1) / QUESTS_COMPLETED_BITS_PER_BLOCK;
        if field_offset as usize >= QUESTS_COMPLETED_BITS_SIZE {
            return false;
        }

        let canonical_changed = self
            .mutate_canonical_player_like_cpp(|player| {
                player.set_quest_completed_bit_like_cpp(quest_bit, false)
            })
            .unwrap_or(false);
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self
                .represented_quest_completed_bits_like_cpp
                .remove(&quest_bit);
        }
        canonical_changed
    }
    /// C++ `Player::GetQuestXPReward`.
    pub(crate) fn quest_xp_reward_like_cpp(&self, quest: &wow_data::quest::QuestTemplate) -> u32 {
        let Some(already_rewarded) = self.represented_player_has_rewarded_quest_like_cpp(quest.id)
        else {
            return 0;
        };
        if already_rewarded && !quest.is_df_quest_like_cpp() {
            return 0;
        }

        self.calculate_quest_xp(
            quest.reward_xp_difficulty,
            self.player_quest_level_like_cpp(quest),
            quest.reward_xp_multiplier,
        )
    }
    /// C++ `Player::GetQuestMoneyReward` -> `Quest::MoneyValue`.
    pub(crate) fn quest_money_reward_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> u32 {
        let Some(store) = &self.quests.money_reward_store else {
            return 0;
        };
        let quest_level = self.player_quest_level_like_cpp(quest).max(0) as u32;
        let Some(row) = store.get(quest_level) else {
            return 0;
        };
        let difficulty = quest.reward_money_difficulty as usize;
        let Some(base) = row.difficulty.get(difficulty).copied() else {
            return 0;
        };
        ((base as f32) * quest.reward_money_multiplier).round() as u32
    }
    pub(in crate::session) fn represented_player_quest_status_is_complete_or_incomplete_like_cpp(
        &self,
        quest_id: u32,
    ) -> bool {
        self.represented_player_quest_status_like_cpp(quest_id)
            .is_some_and(|status| {
                matches!(
                    status,
                    Some(
                        crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
                            | crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
                    )
                )
            })
    }
    pub(crate) fn can_complete_repeatable_quest_represented_bounded_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        // C++ Player::CanCompleteRepeatableQuest is CanTakeQuest(false) && CanRewardQuest(false).
        // The full CanRewardQuest blocker set includes disable checks, quest status, day/week/month/
        // seasonal gates, level/skill/reputation, reward status, item/currency/money checks, etc.
        // This bounded #611 seam must not overclaim completion while those blockers remain open.
        if !self.can_take_quest(quest) {
            return false;
        }

        let Some(inventory_item_counts) = self.represented_inventory_item_counts_like_cpp() else {
            return false;
        };
        let mut saw_non_bound_item_objective = false;
        for objective in &quest.objectives {
            if objective.obj_type != QUEST_OBJECTIVE_ITEM_LIKE_CPP {
                return false;
            }

            if (objective.flags2 & QUEST_OBJECTIVE_FLAG_2_QUEST_BOUND_ITEM_LIKE_CPP) != 0 {
                continue;
            }

            saw_non_bound_item_objective = true;
            let Ok(item_id) = u32::try_from(objective.object_id) else {
                return false;
            };
            let Ok(required_count) = u32::try_from(objective.amount) else {
                return false;
            };
            if inventory_item_counts.get(&item_id).copied().unwrap_or(0) < required_count {
                return false;
            }
        }

        // Item counts are represented only as partial evidence. Do not return true until the
        // remaining C++ CanRewardQuest blockers are represented in this runtime path.
        let _ = saw_non_bound_item_objective;
        false
    }
    pub(crate) fn can_reward_quest_represented_bounded_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> bool {
        // C++ `Player::CanRewardQuest(quest, false)` requires the quest to be complete
        // unless it is a DF/turn-in-only quest, then rejects already rewarded
        // non-repeatable quests. This represented seam covers the status/reward gates
        // needed by `SendQuestGiverRequestItems`; inventory-capacity and long-tail
        // reward blockers remain in the reward handler path.
        if self.is_quest_disabled_like_cpp(quest.id) {
            return false;
        }
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };

        if !quest.is_df_quest_like_cpp()
            && !quest.is_turn_in_like_cpp()
            && !quests.statuses.get(&quest.id).is_some_and(|status| {
                status.status == crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
            })
        {
            return false;
        }

        if quests.rewarded_quest_ids.contains(&quest.id) && !quest.is_repeatable() {
            return false;
        }

        true
    }
    pub(crate) fn send_represented_quest_giver_offer_reward_like_cpp(
        &mut self,
        source_guid: ObjectGuid,
        quest: &wow_data::quest::QuestTemplate,
        auto_launched: bool,
    ) {
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: source_guid,
            giver_creature_id: quest_giver_creature_id_from_source_like_cpp(source_guid),
            quest_id: quest.id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            rewards: quest_rewards_block_like_cpp(quest),
            title: quest.log_title.clone(),
            reward_text: quest.quest_completion_log.clone(),
            auto_launched,
        });
    }
    #[cfg(test)]
    pub(crate) fn represented_confirm_barbers_choice_requests_like_cpp(
        &self,
    ) -> &[RepresentedConfirmBarbersChoiceLikeCpp] {
        &self.represented_confirm_barbers_choice_requests_like_cpp
    }
    pub(crate) fn set_represented_daily_quest_completed_like_cpp_for_test(
        &mut self,
        quest_id: u32,
        completed: bool,
    ) {
        let _ = self.mutate_player_quest_gameplay_like_cpp(|state| {
            if completed {
                state.daily_quest_ids.insert(quest_id);
            } else {
                state.daily_quest_ids.remove(&quest_id);
            }
        });
        self.sync_player_registry_state_like_cpp();
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_skill_updates_like_cpp(&self) -> &[(u32, u32)] {
        &self.represented_quest_reward_skill_updates_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_spell_casts_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardSpellCastLikeCpp] {
        &self.represented_quest_reward_spell_casts_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_titles_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardTitleLikeCpp] {
        &self.represented_quest_reward_titles_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_talent_points_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardTalentPointsLikeCpp] {
        &self.represented_quest_reward_talent_points_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_mails_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardMailLikeCpp] {
        &self.represented_quest_reward_mails_like_cpp
    }
    #[cfg(test)]
    pub(crate) fn represented_quest_reward_reputations_like_cpp(
        &self,
    ) -> &[RepresentedQuestRewardReputationLikeCpp] {
        &self.represented_quest_reward_reputations_like_cpp
    }
    pub(crate) fn represented_quest_complete_status_updates_like_cpp(
        &self,
    ) -> &[RepresentedQuestCompleteStatusUpdateLikeCpp] {
        &self.represented_quest_complete_status_updates_like_cpp
    }
    pub(crate) fn record_represented_quest_complete_status_update_like_cpp(
        &mut self,
        evidence: RepresentedQuestCompleteStatusUpdateLikeCpp,
    ) {
        self.represented_quest_complete_status_updates_like_cpp
            .push(evidence);
    }
}
