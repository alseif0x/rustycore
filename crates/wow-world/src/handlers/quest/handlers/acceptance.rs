//! Quest acceptance and shared-quest confirmation handler implementations.

use super::*;

impl WorldSession {
    /// CMSG_QUEST_GIVER_ACCEPT_QUEST — player clicks "Accept" in the quest details dialog.
    /// Saves quest to characters DB and confirms to the client.
    /// Legacy non-canonical note: QuestHandler.HandleQuestGiverAcceptQuest
    #[cfg(test)]
    pub async fn handle_quest_giver_accept_quest(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_quest_giver_accept_quest_with_generator_like_cpp(generators.item.as_ref(), pkt)
            .await;
    }

    pub async fn handle_quest_giver_accept_quest_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let (guid, quest_id, start_cheat) = match read_quest_giver_accept_quest_like_cpp(&mut pkt) {
            Ok(packet) => packet,
            Err(_) => {
                warn!("QuestGiverAcceptQuest: failed to read packet");
                return;
            }
        };

        info!(
            account = self.account_id,
            ?guid,
            quest_id,
            start_cheat,
            "Received QuestGiverAcceptQuest like C++"
        );

        // Validate represented C++ source/relation before any quest-log mutation or DB save.
        // C++ HandleQuestgiverAcceptQuestOpcode closes gossip and clears sharing info on
        // failure; this represented slice intentionally models that as no packet/no mutation.
        let quest_store = match &self.quests.store {
            Some(s) => Arc::clone(s),
            None => return,
        };
        if !self.represented_quest_giver_accept_source_allows_quest_like_cpp(
            guid,
            quest_id,
            &quest_store,
        ) {
            warn!(
                account = self.account_id,
                ?guid,
                quest_id,
                "AcceptQuest: represented source/relation guard rejected quest"
            );
            return;
        }
        let Some(quest) = quest_store.get(quest_id) else {
            warn!(
                account = self.account_id,
                quest_id, "AcceptQuest: unknown quest"
            );
            return;
        };

        // Full eligibility check: SatisfyQuestStatus + PrevQuestId + race/class/level
        // Legacy non-canonical note: Player.CanTakeQuest(quest, true)
        if !self.can_take_quest(quest) {
            warn!(
                account = self.account_id,
                quest_id,
                race = self.player_race_like_cpp(),
                class = self.player_class_like_cpp(),
                level = self.player_level_like_cpp(),
                "AcceptQuest: player does not meet requirements (CanTakeQuest failed)"
            );
            return;
        }

        // C++ Player::AddQuest uses FindQuestSlot(0) over explicit QuestLog slots.
        let Some(slot) = self.first_free_quest_slot_like_cpp() else {
            warn!(account = self.account_id, "Quest log full");
            return;
        };

        // Build objective counts (one slot per objective)
        let obj_count = quest.objectives.len();

        let (accept_time_secs, end_time_secs) =
            quest.accepted_and_end_time_like_cpp(wow_core::GameTime::now().as_secs() as i64);

        // Add to local state
        self.invalidate_player_quest_status_authority_like_cpp();
        let status = PlayerQuestStatus {
            quest_id,
            status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs,
            end_time_secs,
            objective_counts: vec![0; obj_count],
            slot,
        };
        if self.insert_represented_quest_status_like_cpp(quest_id, status) == false {
            return;
        }

        self.complete_represented_quest_after_add_with_generator_like_cpp(
            item_guid_generator,
            quest,
        )
        .await;

        // Save to DB after AddQuestAndCheckCompletion-style completion, unless
        // RewardQuest already removed/rewarded the quest.
        if let Some(status) = self
            .player_quest_gameplay_snapshot_like_cpp()
            .and_then(|state| {
                state
                    .statuses_like_cpp()
                    .get(&quest_id)
                    .map(|status| status.status)
            })
        {
            self.save_quest_to_db(quest_id, status).await;
        }
        self.sync_player_registry_state_like_cpp();
        self.send_represented_quest_log_slot_update_like_cpp(slot);

        info!(account = self.account_id, quest_id, "Quest accepted");

        // Notify client — quest added popup
        self.send_packet(&QuestGiverQuestComplete {
            quest_id,
            xp: 0,
            money: 0,
            skill_line_id: 0,
            skill_points: 0,
            use_quest_reward_currency: false,
        });
    }

    /// CMSG_QUEST_CONFIRM_ACCEPT — confirm accepting a shared quest.
    ///
    /// C++ anchor: `WorldSession::HandleQuestConfirmAccept`, `QuestHandler.cpp:499-531`.
    /// Represented-partial: validates against session-local pending sharing state, clears before
    /// quest-template lookup like C++, then records safe represented post-template gates.
    /// No-source-item quests and source-item no-grant branches consume only local quest-log insertion
    /// + Character DB status save + PlayerRegistry snapshot sync from `Player::AddQuest`. Real
    /// `StoreNewItem`/`SendNewItem`, criteria/completion, timed/PvP, scripts, and `SendQuestUpdate`
    /// packet fanout remain explicit no-mutation boundaries.
    #[cfg(test)]
    pub async fn handle_quest_confirm_accept(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        self.handle_quest_confirm_accept_with_generator_like_cpp(generators.item.as_ref(), pkt)
            .await;
    }

    pub async fn handle_quest_confirm_accept_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let packet = match QuestConfirmAccept::read(&mut pkt) {
            Ok(packet) => packet,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    ?error,
                    "QuestConfirmAccept: failed to read signed QuestID"
                );
                return;
            }
        };

        let parsed_quest_id = packet.quest_id as u32;
        let Some(pending) = self.represented_pending_quest_sharing_like_cpp() else {
            debug!(
                account = self.account_id,
                raw_quest_id = packet.quest_id,
                parsed_quest_id,
                "QuestConfirmAccept: no represented pending shared quest"
            );
            return;
        };

        if pending.quest_id != parsed_quest_id {
            debug!(
                account = self.account_id,
                pending_quest_id = pending.quest_id,
                raw_quest_id = packet.quest_id,
                parsed_quest_id,
                "QuestConfirmAccept: represented pending quest id mismatch; pending state preserved"
            );
            return;
        }

        self.clear_represented_pending_quest_sharing_like_cpp();

        let Some(quest_store) = &self.quests.store else {
            debug!(
                account = self.account_id,
                parsed_quest_id,
                "QuestConfirmAccept: pending cleared before missing quest store like C++ order"
            );
            return;
        };

        let Some(quest) = quest_store.get(parsed_quest_id).cloned() else {
            debug!(
                account = self.account_id,
                parsed_quest_id,
                "QuestConfirmAccept: pending cleared before missing quest template like C++ order"
            );
            return;
        };

        let receiver_guid = self.player_guid();
        let record = |session: &mut WorldSession,
                      reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
                      can_add_source_item_unrepresented: bool,
                      can_add_source_item_result: Option<InventoryResult>,
                      add_quest_runtime_unrepresented: bool,
                      source_spell_unrepresented: bool,
                      represented_source_spell_id: Option<u32>,
                      represented_source_spell_self_casts: u8| {
            session.record_represented_quest_confirm_accept_like_cpp(
                RepresentedQuestConfirmAcceptLikeCpp {
                    receiver_guid,
                    sender_guid_before_clear: pending.sender_guid,
                    quest_id: parsed_quest_id,
                    raw_quest_id: packet.quest_id,
                    reason,
                    object_accessor_unrepresented: true,
                    party_runtime_unrepresented: true,
                    can_add_source_item_unrepresented,
                    can_add_source_item_result,
                    add_quest_runtime_unrepresented,
                    source_spell_unrepresented,
                    represented_source_spell_id,
                    represented_source_spell_self_casts,
                },
            );
        };

        let Some(player_registry) = self.player_registry().map(Arc::clone) else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let Some(sender_active_status) =
            player_registry.quest_active_status(pending.sender_guid, parsed_quest_id)
        else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerMissing,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let Some(receiver_guid) = receiver_guid else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let Some(group_registry) = self.group_registry().map(Arc::clone) else {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        };

        let same_represented_group = group_registry.snapshots().into_iter().any(|group| {
            let members = &group.members;
            members.contains(&receiver_guid) && members.contains(&pending.sender_guid)
        });
        if !same_represented_group {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::NotInSameRaid,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if !matches!(
            sender_active_status,
            Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP | QUEST_STATUS_COMPLETE_LIKE_CPP)
        ) {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::OriginalPlayerNotActiveQuest,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if !self.can_take_quest(&quest) {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanTakeQuestFailed,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if self.first_free_quest_slot_like_cpp().is_none() {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        if quest.source_item_id > 0 {
            let Some(source_item_template) = self.item_storage_template(quest.source_item_id)
            else {
                let source_item_result = InventoryResult::ItemNotFound;
                self.send_equip_error(source_item_result, None, None, 0, 0);
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
                    false,
                    Some(source_item_result),
                    false,
                    false,
                    None,
                    0,
                );
                return;
            };

            let source_item_count = quest.source_item_count.max(1);
            let (source_item_result, source_item_dest, _) = self
                .plan_store_new_direct_inventory_item(quest.source_item_id, source_item_count)
                .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));

            if !matches!(
                source_item_result,
                InventoryResult::Ok | InventoryResult::ItemMaxCount
            ) {
                self.send_equip_error(
                    source_item_result,
                    None,
                    None,
                    0,
                    u32::from(source_item_template.item_limit_category),
                );
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestSourceItemFailed,
                    false,
                    Some(source_item_result),
                    false,
                    false,
                    None,
                    0,
                );
                return;
            }

            let source_item_no_grant_reason = if self
                .item_template_start_quest_id(quest.source_item_id)
                .is_some_and(|start_quest_id| start_quest_id == quest.id as i32)
            {
                Some(RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStartQuestNoGrant)
            } else if source_item_result == InventoryResult::ItemMaxCount {
                Some(RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemMaxCountNoGrant)
            } else {
                None
            };

            if let Some(source_item_no_grant_reason) = source_item_no_grant_reason {
                if !self
                    .add_quest_confirm_accept_local_state_like_cpp(item_guid_generator, &quest)
                    .await
                {
                    record(
                        self,
                        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                        false,
                        None,
                        false,
                        false,
                        None,
                        0,
                    );
                    return;
                }

                let represented_source_spell_id =
                    (quest.source_spell_id > 0).then_some(quest.source_spell_id);
                let represented_source_spell_self_casts = u8::from(quest.source_spell_id > 0) * 2;
                record(
                    self,
                    source_item_no_grant_reason,
                    false,
                    Some(source_item_result),
                    false,
                    false,
                    represented_source_spell_id,
                    represented_source_spell_self_casts,
                );
                return;
            }

            if !self
                .add_quest_confirm_accept_local_state_like_cpp(item_guid_generator, &quest)
                .await
            {
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                    false,
                    None,
                    false,
                    false,
                    None,
                    0,
                );
                return;
            }

            let Some(source_item_store_outcome) = self
                .store_quest_source_item_with_generator_like_cpp(
                    item_guid_generator,
                    quest.source_item_id,
                    source_item_count,
                    &source_item_dest,
                )
                .await
            else {
                record(
                    self,
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::GiveQuestSourceItemStoreNewItemUnrepresented,
                    false,
                    Some(source_item_result),
                    true,
                    quest.source_spell_id > 0,
                    None,
                    0,
                );
                return;
            };

            let represented_source_spell_id =
                (quest.source_spell_id > 0).then_some(quest.source_spell_id);
            let represented_source_spell_self_casts = u8::from(quest.source_spell_id > 0) * 2;
            let source_item_store_reason = match source_item_store_outcome {
                QuestSourceItemStoreOutcomeLikeCpp::StoredNewItem => {
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemStoredNewItem
                }
                QuestSourceItemStoreOutcomeLikeCpp::BoundObjectiveNoGrant => {
                    RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverGiveQuestSourceItemBoundObjectiveNoGrant
                }
            };
            record(
                self,
                source_item_store_reason,
                false,
                Some(source_item_result),
                false,
                false,
                represented_source_spell_id,
                represented_source_spell_self_casts,
            );
            return;
        }

        if !self
            .add_quest_confirm_accept_local_state_like_cpp(item_guid_generator, &quest)
            .await
        {
            record(
                self,
                RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverCanAddQuestLogFull,
                false,
                None,
                false,
                false,
                None,
                0,
            );
            return;
        }

        let represented_source_spell_id =
            (quest.source_spell_id > 0).then_some(quest.source_spell_id);
        let represented_source_spell_self_casts = u8::from(quest.source_spell_id > 0) * 2;
        record(
            self,
            RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
            false,
            None,
            false,
            false,
            represented_source_spell_id,
            represented_source_spell_self_casts,
        );
    }
}
