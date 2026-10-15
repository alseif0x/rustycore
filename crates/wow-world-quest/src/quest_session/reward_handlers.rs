// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest reward packet entry points moved from the World
//! `handlers/quest/handlers/reward_flow.rs` (#1363 slice 6c):
//! `HandleQuestgiverRequestRewardOpcode` and `HandleQuestgiverChooseRewardOpcode`,
//! with the `QuestChoiceItem` reader and the loaded reward-choice match. The
//! reward commit and the objective drain stay host operations.

use super::*;

pub const QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP: u32 = 0x0001_0000;

pub async fn handle_quest_giver_request_reward_with_generator_like_cpp<
    H: QuestHandlerSessionHostLikeCpp,
>(
    host: &mut H,
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
        account = host.core().account_id,
        ?guid,
        quest_id,
        "Received QuestGiverRequestReward like C++"
    );

    let quest_store = match &host.hub_ref().catalogs.quests.store {
        Some(s) => Arc::clone(s),
        None => return,
    };
    let quest = match quest_store.get(quest_id) {
        Some(q) => q.clone(),
        None => {
            warn!(
                account = host.core().account_id,
                quest_id, "RequestReward: unknown quest"
            );
            return;
        }
    };

    if crate::is_quest_disabled_like_cpp(host, quest_id) {
        debug!(
            account = host.core().account_id,
            quest_id, "RequestReward: quest disabled"
        );
        return;
    }

    if quest.flags & QUEST_FLAGS_AUTO_COMPLETE_LIKE_CPP == 0
        && !represented_quest_giver_involved_source_allows_quest_like_cpp(
            host,
            guid,
            quest_id,
            &quest_store,
        )
    {
        warn!(
            account = host.core().account_id,
            ?guid,
            quest_id,
            "RequestReward: represented involved source rejected"
        );
        return;
    }

    // C++: if (_player->CanCompleteQuest(questID)) _player->CompleteQuest(questID)
    let can_complete_now = host
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
        let completion_evidence_start = host
            .quest_state()
            .represented_quest_complete_status_updates_like_cpp()
            .len();
        crate::complete_represented_quest_after_add_with_generator_like_cpp(
            host,
            item_guid_generator,
            &quest,
        )
        .await;
        crate::save_represented_quest_statuses_completed_after_like_cpp(
            host,
            completion_evidence_start,
        )
        .await;
    }

    let is_complete = host
        .player_quest_gameplay_snapshot_like_cpp()
        .and_then(|state| state.statuses_like_cpp().get(&quest_id).map(|qs| qs.status))
        == Some(QUEST_STATUS_COMPLETE_LIKE_CPP);

    if !is_complete {
        // Objectives not finished — silently ignore
        // (C# would send SMSG_QUEST_GIVER_REQUEST_ITEMS instead)
        warn!(
            account = host.core().account_id,
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
    host.send_packet(&QuestGiverOfferReward {
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

pub async fn handle_quest_giver_choose_reward_with_generator_like_cpp<
    H: QuestHandlerSessionHostLikeCpp,
>(
    host: &mut H,
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
    let choice = match read_quest_choice_item_like_cpp(&mut pkt) {
        Ok(choice) => choice,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                ?error,
                "ChooseReward: failed to read C++ QuestChoiceItem"
            );
            return;
        }
    };
    let choice_item_id = choice.item_id;

    info!(
        account = host.core().account_id,
        ?guid,
        quest_id,
        choice_item_id,
        choice_loot_type = choice.loot_item_type,
        "Received QuestGiverChooseReward like C++"
    );

    let quest_store = match &host.hub_ref().catalogs.quests.store {
        Some(s) => Arc::clone(s),
        None => return,
    };
    let quest = match quest_store.get(quest_id) {
        Some(q) => q.clone(),
        None => {
            warn!(
                account = host.core().account_id,
                quest_id, "ChooseReward: unknown quest"
            );
            return;
        }
    };

    if crate::is_quest_disabled_like_cpp(host, quest_id) {
        debug!(
            account = host.core().account_id,
            quest_id, "ChooseReward: quest disabled"
        );
        return;
    }

    // C++ `Player::CanRewardQuest`: player must have the quest active and COMPLETE.
    let quest_status = host
        .player_quest_gameplay_snapshot_like_cpp()
        .and_then(|state| state.statuses_like_cpp().get(&quest_id).map(|qs| qs.status));
    match quest_status {
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP) => {}
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP) => {
            warn!(
                account = host.core().account_id,
                quest_id, "ChooseReward: quest not complete yet"
            );
            return;
        }
        _ => {
            warn!(
                account = host.core().account_id,
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
                account = host.core().account_id,
                quest_id,
                loot_item_type = choice.loot_item_type,
                "ChooseReward: unsupported C++ LootItemType"
            );
            return;
        }
        if !crate::represented_reward_choice_template_exists_like_cpp(host, choice) {
            warn!(
                account = host.core().account_id,
                quest_id,
                choice_item_id,
                loot_item_type = choice.loot_item_type,
                "ChooseReward: selected reward item/currency template does not exist"
            );
            return;
        }
        let valid = represented_reward_choice_matches_loaded_type_like_cpp(&quest, choice)
            || crate::represented_quest_package_choice_matches_like_cpp(host, &quest, choice);
        if !valid {
            warn!(
                account = host.core().account_id,
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
        && !represented_quest_giver_involved_source_allows_quest_like_cpp(
            host,
            guid,
            quest_id,
            &quest_store,
        )
    {
        warn!(
            account = host.core().account_id,
            ?guid,
            quest_id,
            "ChooseReward: represented involved source rejected"
        );
        return;
    }

    if !crate::represented_can_reward_quest_inventory_like_cpp(host, &quest, choice) {
        debug!(
            account = host.core().account_id,
            quest_id,
            choice_item_id,
            "ChooseReward: represented reward inventory validation rejected like C++"
        );
        return;
    }

    let rewarded = host
        .reward_represented_quest_with_generator_like_cpp(item_guid_generator, &quest, guid, choice)
        .await;
    if rewarded {
        Box::pin(
            host.drain_represented_quest_objective_progress_with_generator_like_cpp(
                item_guid_generator,
            ),
        )
        .await;
    }
}

pub fn represented_reward_choice_matches_loaded_type_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
    choice: QuestChoiceItemLikeCpp,
) -> bool {
    quest
        .reward_choice_items
        .iter()
        .zip(quest.reward_choice_item_types.iter())
        .any(|((item_id, _quantity), item_type)| {
            *item_id != 0 && *item_id == choice.item_id && *item_type == choice.loot_item_type
        })
}

pub fn read_quest_choice_item_like_cpp(
    pkt: &mut wow_packet::WorldPacket,
) -> Result<QuestChoiceItemLikeCpp, wow_packet::PacketError> {
    // C++ `QuestChoiceItem` starts with `ResetBitPos(); ReadBits(2)`, then
    // an `Item::ItemInstance`, then signed `Quantity`.
    pkt.reset_bits();
    let loot_item_type = pkt.read_bits(2)? as u8;

    let item_id = pkt.read_int32()? as u32;
    let _random_properties_seed = pkt.read_int32()?;
    let _random_properties_id = pkt.read_int32()?;

    let has_item_bonus = pkt.read_bit()?;
    pkt.reset_bits();

    let item_mod_count = pkt.read_bits(6)?;
    pkt.reset_bits();
    for _ in 0..item_mod_count {
        let _value = pkt.read_int32()?;
        let _modifier_type = pkt.read_uint8()?;
    }

    if has_item_bonus {
        let _context = pkt.read_uint8()?;
        let bonus_count = pkt.read_uint32()?;
        for _ in 0..bonus_count {
            let _bonus_id = pkt.read_uint32()?;
        }
    }

    let quantity = pkt.read_int32()?;

    Ok(QuestChoiceItemLikeCpp {
        loot_item_type,
        item_id,
        quantity,
    })
}
