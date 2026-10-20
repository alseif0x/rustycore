// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest acceptance packet entry points moved from the World
//! `handlers/quest/handlers/acceptance.rs` (#1363 slice 6c):
//! `HandleQuestgiverAcceptQuestOpcode` and `HandleQuestConfirmAccept`, with the
//! accept-quest packet reader and the pending-sharing install
//! (`Player::SetQuestSharingInfo`).

use super::*;

pub async fn handle_quest_giver_accept_quest_with_generator_like_cpp<
    H: QuestHandlerSessionHostLikeCpp,
>(
    host: &mut H,
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
        account = host.core().account_id,
        ?guid,
        quest_id,
        start_cheat,
        "Received QuestGiverAcceptQuest like C++"
    );

    // Validate represented C++ source/relation before any quest-log mutation or DB save.
    // C++ HandleQuestgiverAcceptQuestOpcode closes gossip and clears sharing info on
    // failure; this represented slice intentionally models that as no packet/no mutation.
    let quest_store = match &host.hub_ref().catalogs.quests.store {
        Some(s) => Arc::clone(s),
        None => return,
    };
    if !represented_quest_giver_accept_source_allows_quest_like_cpp(
        host,
        guid,
        quest_id,
        &quest_store,
    ) {
        warn!(
            account = host.core().account_id,
            ?guid,
            quest_id,
            "AcceptQuest: represented source/relation guard rejected quest"
        );
        return;
    }
    let Some(quest) = quest_store.get(quest_id) else {
        warn!(
            account = host.core().account_id,
            quest_id, "AcceptQuest: unknown quest"
        );
        return;
    };

    // Full eligibility check: SatisfyQuestStatus + PrevQuestId + race/class/level
    // Legacy non-canonical note: Player.CanTakeQuest(quest, true)
    if !crate::can_take_quest(host, quest) {
        warn!(
            account = host.core().account_id,
            quest_id,
            race = host.hub_ref().player_race_like_cpp(),
            class = host.hub_ref().player_class_like_cpp(),
            level = host.hub_ref().player_level_like_cpp(),
            "AcceptQuest: player does not meet requirements (CanTakeQuest failed)"
        );
        return;
    }

    // C++ Player::AddQuest uses FindQuestSlot(0) over explicit QuestLog slots.
    let Some(slot) = crate::first_free_quest_slot_like_cpp(host) else {
        warn!(account = host.core().account_id, "Quest log full");
        return;
    };

    // Build objective counts (one slot per objective)
    let obj_count = quest.objectives.len();

    let (accept_time_secs, end_time_secs) =
        quest.accepted_and_end_time_like_cpp(wow_core::GameTime::now().as_secs() as i64);

    // Add to local state
    host.invalidate_player_quest_status_authority_like_cpp();
    let status = PlayerQuestStatus {
        quest_id,
        status: QUEST_STATUS_INCOMPLETE_LIKE_CPP,
        explored: false,
        accept_time_secs,
        end_time_secs,
        objective_counts: vec![0; obj_count],
        slot,
    };
    if crate::insert_represented_quest_status_like_cpp(host, quest_id, status) == false {
        return;
    }

    crate::complete_represented_quest_after_add_with_generator_like_cpp(
        host,
        item_guid_generator,
        quest,
    )
    .await;

    // Save to DB after AddQuestAndCheckCompletion-style completion, unless
    // RewardQuest already removed/rewarded the quest.
    if let Some(status) = host
        .player_quest_gameplay_snapshot_like_cpp()
        .and_then(|state| {
            state
                .statuses_like_cpp()
                .get(&quest_id)
                .map(|status| status.status)
        })
    {
        crate::save_quest_to_db(host, quest_id, status).await;
    }
    host.sync_player_registry_state_like_cpp();
    host.send_represented_quest_log_slot_update_like_cpp(slot);
    // AzerothCore `AddQuestAndCheckCompletion` -> `OnPlayerQuestAccept`
    // (`PlayerQuest.cpp:426`); here after the accepted status was saved (or,
    // for a quest the completion check already rewarded, after that reward's
    // commit), so a module observes a durable accept.
    raise_quest_accepted_hook_like_cpp(host, quest_id, slot);

    info!(account = host.core().account_id, quest_id, "Quest accepted");

    // Notify client — quest added popup
    host.send_packet(&QuestGiverQuestComplete {
        quest_id,
        xp: 0,
        money: 0,
        skill_line_id: 0,
        skill_points: 0,
        use_quest_reward_currency: false,
    });
}

pub async fn handle_quest_confirm_accept_with_generator_like_cpp<
    H: QuestHandlerSessionHostLikeCpp,
>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    mut pkt: wow_packet::WorldPacket,
) {
    let world_test = host.world_test_consumer_like_cpp();
    let packet = match QuestConfirmAccept::read(&mut pkt) {
        Ok(packet) => packet,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                ?error,
                "QuestConfirmAccept: failed to read signed QuestID"
            );
            return;
        }
    };

    let parsed_quest_id = packet.quest_id as u32;
    let (state, hub) = host.split_quest_state_ref();
    let Some(pending) = crate::represented_pending_quest_sharing_like_cpp(hub, state) else {
        debug!(
            account = host.core().account_id,
            raw_quest_id = packet.quest_id,
            parsed_quest_id,
            "QuestConfirmAccept: no represented pending shared quest"
        );
        return;
    };

    if pending.quest_id != parsed_quest_id {
        debug!(
            account = host.core().account_id,
            pending_quest_id = pending.quest_id,
            raw_quest_id = packet.quest_id,
            parsed_quest_id,
            "QuestConfirmAccept: represented pending quest id mismatch; pending state preserved"
        );
        return;
    }

    let (state, hub) = host.split_quest_state_mut();
    crate::clear_represented_pending_quest_sharing_like_cpp(hub, state);
    host.sync_player_registry_state_like_cpp();

    let Some(quest_store) = &host.hub_ref().catalogs.quests.store else {
        debug!(
            account = host.core().account_id,
            parsed_quest_id,
            "QuestConfirmAccept: pending cleared before missing quest store like C++ order"
        );
        return;
    };

    let Some(quest) = quest_store.get(parsed_quest_id).cloned() else {
        debug!(
            account = host.core().account_id,
            parsed_quest_id,
            "QuestConfirmAccept: pending cleared before missing quest template like C++ order"
        );
        return;
    };

    let receiver_guid = host.player_guid();
    let record = |session: &mut H,
                  reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
                  can_add_source_item_unrepresented: bool,
                  can_add_source_item_result: Option<InventoryResult>,
                  add_quest_runtime_unrepresented: bool,
                  source_spell_unrepresented: bool,
                  represented_source_spell_id: Option<u32>,
                  represented_source_spell_self_casts: u8| {
        session
            .quest_state_mut()
            .record_represented_quest_confirm_accept_like_cpp(
                world_test,
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

    let Some(player_registry) = host.core().player_registry().map(Arc::clone) else {
        record(
            host,
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
            host,
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
            host,
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

    let Some(group_registry) = host.core().group_registry().map(Arc::clone) else {
        record(
            host,
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
            host,
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
            host,
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

    if !crate::can_take_quest(host, &quest) {
        record(
            host,
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

    if crate::first_free_quest_slot_like_cpp(host).is_none() {
        record(
            host,
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
        let Some(source_item_template) = host
            .hub_ref()
            .catalogs
            .item_storage_template(quest.source_item_id)
        else {
            let source_item_result = InventoryResult::ItemNotFound;
            host.core()
                .send_equip_error(source_item_result, None, None, 0, 0);
            record(
                host,
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
        let (source_item_result, source_item_dest, _) = host
            .plan_store_new_direct_inventory_item(quest.source_item_id, source_item_count)
            .unwrap_or((InventoryResult::ItemNotFound, Vec::new(), None));

        if !matches!(
            source_item_result,
            InventoryResult::Ok | InventoryResult::ItemMaxCount
        ) {
            host.core().send_equip_error(
                source_item_result,
                None,
                None,
                0,
                u32::from(source_item_template.item_limit_category),
            );
            record(
                host,
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

        let source_item_no_grant_reason = if host
            .hub_ref()
            .catalogs
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
            if !crate::add_quest_confirm_accept_local_state_like_cpp(
                host,
                item_guid_generator,
                &quest,
            )
            .await
            {
                record(
                    host,
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
                host,
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

        if !crate::add_quest_confirm_accept_local_state_like_cpp(host, item_guid_generator, &quest)
            .await
        {
            record(
                host,
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

        let Some(source_item_store_outcome) =
            crate::store_quest_source_item_with_generator_like_cpp(
                host,
                item_guid_generator,
                quest.source_item_id,
                source_item_count,
                &source_item_dest,
            )
            .await
        else {
            record(
                host,
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
            host,
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

    if !crate::add_quest_confirm_accept_local_state_like_cpp(host, item_guid_generator, &quest)
        .await
    {
        record(
            host,
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

    let represented_source_spell_id = (quest.source_spell_id > 0).then_some(quest.source_spell_id);
    let represented_source_spell_self_casts = u8::from(quest.source_spell_id > 0) * 2;
    record(
        host,
        RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp::ReceiverAddQuestLocalStateRepresented,
        false,
        None,
        false,
        false,
        represented_source_spell_id,
        represented_source_spell_self_casts,
    );
}

pub fn read_quest_giver_accept_quest_like_cpp(
    pkt: &mut wow_packet::WorldPacket,
) -> Result<(ObjectGuid, u32, bool), wow_packet::PacketError> {
    let guid = pkt.read_packed_guid()?;
    let quest_id = pkt.read_uint32().unwrap_or(0);
    let start_cheat = pkt.read_bit().unwrap_or(false);
    Ok((guid, quest_id, start_cheat))
}

pub fn set_represented_pending_quest_sharing_like_cpp<H: QuestHandlerSessionHostLikeCpp>(
    host: &mut H,
    sender_guid: ObjectGuid,
    quest_id: u32,
) {
    #[cfg(any(test, feature = "test-fixtures"))]
    if host.world_test_consumer_like_cpp() && host.core().player_handle_like_cpp.is_none() {
        host.quest_state_mut()
            .fixture_set_represented_pending_quest_sharing_like_cpp(Some(
                RepresentedPendingQuestSharingLikeCpp {
                    sender_guid,
                    quest_id,
                },
            ));
        host.sync_player_registry_state_like_cpp();
        return;
    }
    let _ = host.mutate_player_quest_gameplay_like_cpp(|state| {
        state.set_pending_share_like_cpp(Some((sender_guid, quest_id)));
    });
    host.sync_player_registry_state_like_cpp();
}

/// Raise the `player.quest_accepted` hook port for the session's player.
pub(crate) fn raise_quest_accepted_hook_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    quest_id: u32,
    slot: u8,
) {
    if !host.quest_accepted_hook_wanted_like_cpp() {
        return;
    }
    if let Some(guid) = host.player_guid() {
        host.quest_accepted_hook_like_cpp(&crate::QuestAcceptedHookEventLikeCpp {
            guid,
            quest_id,
            slot,
        });
    }
}
