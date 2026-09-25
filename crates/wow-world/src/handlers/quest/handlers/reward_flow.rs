//! Quest reward dialog and completion packet handlers.

use super::*;

impl WorldSession {
    /// CMSG_QUEST_GIVER_REQUEST_REWARD — player talks to NPC to turn in a completed quest.
    /// Legacy non-canonical note: QuestHandler.HandleQuestgiverRequestReward
    /// Sent when player right-clicks a quest-ender NPC and has the quest in Complete status.
    /// Server responds with SMSG_QUEST_GIVER_OFFER_REWARD_MESSAGE (reward selection dialog).
    #[cfg(test)]
    pub async fn handle_quest_giver_request_reward(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_quest_giver_request_reward_with_generator_like_cpp(
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }

    pub async fn handle_quest_giver_request_reward_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverRequestReward: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            "Received QuestGiverRequestReward like C++"
        );

        let quest_store = match &self.quests.store {
            Some(s) => Arc::clone(s),
            None => return,
        };
        let quest = match quest_store.get(quest_id) {
            Some(q) => q.clone(),
            None => {
                warn!(
                    account = self.account_id,
                    quest_id, "RequestReward: unknown quest"
                );
                return;
            }
        };

        if self.is_quest_disabled_like_cpp(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "RequestReward: quest disabled"
            );
            return;
        }

        if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0
            && !self.represented_quest_giver_involved_source_allows_quest_like_cpp(
                guid,
                quest_id,
                &quest_store,
            )
        {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "RequestReward: represented involved source rejected"
            );
            return;
        }

        // C++: if (_player->CanCompleteQuest(questID)) _player->CompleteQuest(questID)
        let can_complete_now = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| {
                let rewarded = state.rewarded_quest_ids_like_cpp().contains(&quest_id);
                state.statuses_like_cpp().get(&quest_id).map(|status| {
                    wow_entities::represented_can_complete_quest_after_objective_like_cpp(
                        status,
                        &quest.objective_rules_like_cpp(),
                        0,
                        rewarded,
                    )
                })
            })
            .unwrap_or(false);
        if can_complete_now {
            let completion_evidence_start = self
                .represented_quest_complete_status_updates_like_cpp
                .len();
            self.complete_represented_quest_after_add_with_generator_like_cpp(
                item_guid_generator,
                &quest,
            )
            .await;
            self.save_represented_quest_statuses_completed_after_like_cpp(
                completion_evidence_start,
            )
            .await;
        }

        let is_complete = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.statuses_like_cpp().get(&quest_id).map(|qs| qs.status))
            == Some(QUEST_STATUS_COMPLETE_LIKE_CPP);

        if !is_complete {
            // Objectives not finished — silently ignore
            // (C# would send SMSG_QUEST_GIVER_REQUEST_ITEMS instead)
            warn!(
                account = self.account_id,
                quest_id, "RequestReward: quest not complete"
            );
            return;
        }

        // Build rewards block for the offer-reward dialog
        let mut rewards = QuestRewardsBlock::default();
        rewards.money = quest.reward_money_difficulty as i32;
        for i in 0..4 {
            rewards.items[i] = (quest.reward_items[i], quest.reward_amounts[i]);
        }
        for i in 0..3 {
            rewards.display_spells[i] = quest.reward_display_spell[i];
        }
        rewards.completion_spell = quest.reward_spell as i32;
        // Populate choice items for the dialog
        for i in 0..6 {
            rewards.choice_items[i] = (
                quest.reward_choice_items[i].0,
                quest.reward_choice_items[i].1,
            );
        }
        rewards.choice_item_types = quest.reward_choice_item_types;

        // C#: SendQuestGiverOfferReward(quest, questGiverGUID, true)
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: guid,
            giver_creature_id: quest_giver_creature_id_from_source_like_cpp(guid),
            quest_id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            rewards,
            title: quest.log_title.clone(),
            reward_text: quest.quest_completion_log.clone(),
            auto_launched: false,
        });
    }

    /// CMSG_QUEST_GIVER_COMPLETE_QUEST — player talks to quest-ender NPC.
    /// If objectives are done: show reward dialog. Else: show "still need X" dialog.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverCompleteQuest
    pub async fn handle_quest_giver_complete_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("QuestGiverCompleteQuest: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let from_script: bool = pkt.read_bit().unwrap_or(false);

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            from_script,
            "Received QuestGiverCompleteQuest like C++"
        );

        let quest_store = match &self.quests.store {
            Some(s) => Arc::clone(s),
            None => return,
        };

        let quest = match quest_store.get(quest_id) {
            Some(q) => q,
            None => {
                warn!(
                    account = self.account_id,
                    quest_id, "QuestGiverCompleteQuest: unknown quest"
                );
                return;
            }
        };

        if self.is_quest_disabled_like_cpp(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "QuestGiverCompleteQuest: quest disabled"
            );
            return;
        }

        if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0 {
            if from_script
                || !self.represented_quest_giver_involved_source_allows_quest_like_cpp(
                    guid,
                    quest_id,
                    &quest_store,
                )
            {
                warn!(
                    account = self.account_id,
                    ?guid,
                    quest_id,
                    from_script,
                    "QuestGiverCompleteQuest: represented involved source rejected"
                );
                return;
            }
        } else if !from_script || self.player_guid() != Some(guid) {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                from_script,
                "QuestGiverCompleteQuest: auto-complete source is not script/player"
            );
            return;
        }

        // Check if player has the quest active
        if !self.has_quest(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "Player doesn't have quest"
            );
            return;
        }

        // Build rewards block
        let mut rewards = QuestRewardsBlock::default();
        rewards.money = quest.reward_money_difficulty as i32;
        for i in 0..4 {
            rewards.items[i] = (quest.reward_items[i], quest.reward_amounts[i]);
        }
        for i in 0..3 {
            rewards.display_spells[i] = quest.reward_display_spell[i];
        }
        rewards.completion_spell = quest.reward_spell as i32;
        for i in 0..6 {
            rewards.choice_items[i] = (
                quest.reward_choice_items[i].0,
                quest.reward_choice_items[i].1,
            );
        }
        rewards.choice_item_types = quest.reward_choice_item_types;

        // Check if all objectives are done — C++ GetQuestStatus == QUEST_STATUS_COMPLETE.
        let is_complete = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.statuses_like_cpp().get(&quest_id).map(|qs| qs.status))
            == Some(QUEST_STATUS_COMPLETE_LIKE_CPP);

        if !is_complete {
            // Not all objectives done — send "you still need X" dialog
            // Legacy non-canonical note: SendQuestGiverRequestItems(quest, guid, canComplete=false, false)
            self.send_packet(&QuestGiverRequestItems {
                giver_guid: guid,
                giver_creature_id: quest_giver_creature_id_from_source_like_cpp(guid),
                quest_id,
                comp_emote_delay: 0,
                comp_emote_type: 0,
                quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
                suggested_party_members: quest.suggested_group_num,
                money_to_get: 0,
                collect: Vec::new(),
                currency: Vec::new(),
                status_flags: 0xFD,
                title: quest.log_title.clone(),
                completion_text: quest.area_description.clone(),
                auto_launched: false,
            });
            return;
        }

        // All objectives done — show offer reward dialog
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: guid,
            giver_creature_id: quest_giver_creature_id_from_source_like_cpp(guid),
            quest_id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            rewards,
            title: quest.log_title.clone(),
            reward_text: quest.quest_completion_log.clone(),
            auto_launched: false,
        });
    }

    /// CMSG_QUEST_GIVER_CHOOSE_REWARD — player clicks "Complete Quest" in reward dialog.
    /// Gives XP, gold, items. Removes quest from active log.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverChooseReward
    #[cfg(test)]
    pub async fn handle_quest_giver_choose_reward(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_quest_giver_choose_reward_with_generator_like_cpp(
            generators.item.as_ref(),
            pkt,
        )
        .await;
    }

    pub async fn handle_quest_giver_choose_reward_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => {
                warn!("ChooseReward: failed to read GUID");
                return;
            }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let choice = match WorldSession::read_quest_choice_item_like_cpp(&mut pkt) {
            Ok(choice) => choice,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "ChooseReward: failed to read C++ QuestChoiceItem"
                );
                return;
            }
        };
        let choice_item_id = choice.item_id;

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            choice_item_id,
            choice_loot_type = choice.loot_item_type,
            "Received QuestGiverChooseReward like C++"
        );

        let quest_store = match &self.quests.store {
            Some(s) => Arc::clone(s),
            None => return,
        };
        let quest = match quest_store.get(quest_id) {
            Some(q) => q.clone(),
            None => {
                warn!(
                    account = self.account_id,
                    quest_id, "ChooseReward: unknown quest"
                );
                return;
            }
        };

        if self.is_quest_disabled_like_cpp(quest_id) {
            debug!(
                account = self.account_id,
                quest_id, "ChooseReward: quest disabled"
            );
            return;
        }

        // C++ `Player::CanRewardQuest`: player must have the quest active and COMPLETE.
        let quest_status = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| state.statuses_like_cpp().get(&quest_id).map(|qs| qs.status));
        match quest_status {
            Some(QUEST_STATUS_COMPLETE_LIKE_CPP) => {}
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP) => {
                warn!(
                    account = self.account_id,
                    quest_id, "ChooseReward: quest not complete yet"
                );
                return;
            }
            _ => {
                warn!(
                    account = self.account_id,
                    quest_id, "ChooseReward: player doesn't have quest"
                );
                return;
            }
        }

        // Validate choice item — C# HandleQuestgiverChooseReward lines 255-310
        // If client sends a non-zero choice item, it must be in reward_choice_items.
        if choice_item_id != 0 {
            if choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_ITEM_LIKE_CPP
                && choice.loot_item_type != QUEST_CHOICE_LOOT_ITEM_TYPE_CURRENCY_LIKE_CPP
            {
                warn!(
                    account = self.account_id,
                    quest_id,
                    loot_item_type = choice.loot_item_type,
                    "ChooseReward: unsupported C++ LootItemType"
                );
                return;
            }
            if !self.represented_reward_choice_template_exists_like_cpp(choice) {
                warn!(
                    account = self.account_id,
                    quest_id,
                    choice_item_id,
                    loot_item_type = choice.loot_item_type,
                    "ChooseReward: selected reward item/currency template does not exist"
                );
                return;
            }
            let valid = WorldSession::represented_reward_choice_matches_loaded_type_like_cpp(
                &quest, choice,
            ) || self.represented_quest_package_choice_matches_like_cpp(&quest, choice);
            if !valid {
                warn!(
                    account = self.account_id,
                    quest_id,
                    choice_item_id,
                    loot_item_type = choice.loot_item_type,
                    "ChooseReward: choice item not valid for this quest (possible exploit)"
                );
                return;
            }
        }

        // C++ HandleQuestgiverChooseRewardOpcode keeps `object = _player` for auto-complete,
        // but non-auto-complete quests must resolve the packet source as an involved
        // Unit/GameObject and pass CanInteractWithQuestGiver before RewardQuest mutates state.
        // This represented-partial slice intentionally keeps bounded choice/package validation
        // only; full CanRewardQuest/RewardQuest side effects remain open.
        if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0
            && !self.represented_quest_giver_involved_source_allows_quest_like_cpp(
                guid,
                quest_id,
                &quest_store,
            )
        {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "ChooseReward: represented involved source rejected"
            );
            return;
        }

        if !self.represented_can_reward_quest_inventory_like_cpp(&quest, choice) {
            debug!(
                account = self.account_id,
                quest_id,
                choice_item_id,
                "ChooseReward: represented reward inventory validation rejected like C++"
            );
            return;
        }

        let rewarded = self
            .reward_represented_quest_with_generator_like_cpp(
                item_guid_generator,
                &quest,
                guid,
                choice,
            )
            .await;
        if rewarded {
            Box::pin(
                self.drain_represented_quest_objective_progress_with_generator_like_cpp(
                    item_guid_generator,
                ),
            )
            .await;
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────
}
