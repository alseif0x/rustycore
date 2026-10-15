// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Represented quest reward projections, the rewarded-spell grant and the
//! completed-quest bits, moved from the World `session/quest/rewards.rs`
//! (#1363 slice 6a).

use super::*;

/// Represented C++ `Player::LearnQuestRewardedSpells`.
///
/// C++ casts each rewarded quest's `RewardSpell` only when that spell exists,
/// has a missing `SPELL_EFFECT_LEARN_SPELL` trigger, and the first learned
/// spell is tied to `SKILL_LINE_ABILITY_REWARDED_FROM_QUEST` when it is not
/// already known. This represented slice applies only the resulting direct
/// learned-spell side effect; full `CastSpell` runtime semantics remain in
/// the spell-system roadmap.
pub fn apply_represented_quest_rewarded_spells_like_cpp<
    H: QuestSessionHostLikeCpp + wow_world_spell::AuraHostLikeCpp,
>(
    host: &mut H,
) -> usize {
    let Some(quest_store) = host.hub_ref().catalogs.quests.store.clone() else {
        return 0;
    };

    let Some(quests) = host.player_quest_gameplay_snapshot_like_cpp() else {
        return 0;
    };
    let mut quest_ids = quests
        .rewarded_quest_ids_like_cpp()
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
            learned += wow_world_spell::remove_represented_auras_due_to_spell_like_cpp(
                host,
                source_spell_id,
            );
            continue;
        }
        for spell_id in represented_quest_rewarded_spell_triggers_like_cpp(host, quest) {
            wow_world_spell::learn_known_spell_like_cpp(host, spell_id);
            learned += 1;
        }
    }

    learned
}

fn represented_quest_rewarded_spell_triggers_like_cpp<
    H: QuestSessionHostLikeCpp + wow_world_spell::SpellbookHostLikeCpp,
>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
) -> Vec<i32> {
    if quest.reward_spell == 0 {
        return Vec::new();
    }

    let Ok(reward_spell_id) = i32::try_from(quest.reward_spell) else {
        return Vec::new();
    };

    let Some(spell_info) = host
        .hub_ref()
        .catalogs
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
                && !wow_world_spell::known_spells_like_cpp(host)
                    .contains(&effect.effect_trigger_spell)
        })
        .map(|effect| effect.effect_trigger_spell)
        .collect::<Vec<_>>();

    if missing_learn_triggers.is_empty() || spell_info.effects().is_empty() {
        return Vec::new();
    }

    let learned_0 = spell_info.effects()[0].effect_trigger_spell;
    if learned_0 > 0
        && !wow_world_spell::known_spells_like_cpp(host).contains(&learned_0)
        && !host.hub_ref().catalogs.skill_store().is_some_and(|store| {
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

pub fn set_loaded_quest_completed_bit_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    quest_bit: u32,
) -> bool {
    if quest_bit == 0 {
        return false;
    }

    let field_offset = (quest_bit - 1) / QUESTS_COMPLETED_BITS_PER_BLOCK;
    if field_offset as usize >= QUESTS_COMPLETED_BITS_SIZE {
        return false;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = host.world_test_consumer_like_cpp();
    let (quest_state, mut hub) = host.split_quest_state_mut();
    let canonical_changed = hub
        .core
        .mutate_canonical_player_like_cpp(|player| {
            player.set_quest_completed_bit_like_cpp(quest_bit, true)
        })
        .unwrap_or(false);
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test && hub.core.player_handle_like_cpp.is_none() {
        let changed = !quest_state.fixture_has_represented_quest_completed_bit_like_cpp(quest_bit);
        quest_state.fixture_set_represented_quest_completed_bit_like_cpp(quest_bit, true);
        return changed;
    }
    canonical_changed
}

pub fn clear_loaded_quest_completed_bit_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    quest_bit: u32,
) -> bool {
    if quest_bit == 0 {
        return false;
    }

    let field_offset = (quest_bit - 1) / QUESTS_COMPLETED_BITS_PER_BLOCK;
    if field_offset as usize >= QUESTS_COMPLETED_BITS_SIZE {
        return false;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = host.world_test_consumer_like_cpp();
    let (quest_state, mut hub) = host.split_quest_state_mut();
    let canonical_changed = hub
        .core
        .mutate_canonical_player_like_cpp(|player| {
            player.set_quest_completed_bit_like_cpp(quest_bit, false)
        })
        .unwrap_or(false);
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test && hub.core.player_handle_like_cpp.is_none() {
        let changed = quest_state.fixture_has_represented_quest_completed_bit_like_cpp(quest_bit);
        quest_state.fixture_set_represented_quest_completed_bit_like_cpp(quest_bit, false);
        return changed;
    }
    canonical_changed
}

/// C++ `Player::GetQuestXPReward`.
pub fn quest_xp_reward_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
) -> u32 {
    let Some(quests) = host.player_quest_gameplay_snapshot_like_cpp() else {
        return 0;
    };
    if !crate::QuestRewardCx::quest_xp_reward_allowed_like_cpp(quest, &quests) {
        return 0;
    }

    let quest_level = player_quest_level_like_cpp(host, quest);
    let (state, hub) = host.split_quest_state_ref();
    let xp_store = hub.catalogs.quests.xp_store.as_deref();
    let player_level = if xp_store.is_some() {
        hub.player_level_like_cpp()
    } else {
        0
    };
    crate::QuestRewardCx::calculate_quest_xp_reward_like_cpp(
        state,
        xp_store,
        player_level,
        quest.reward_xp_difficulty,
        quest_level,
        quest.reward_xp_multiplier,
    )
}

pub fn quest_money_reward_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
) -> u32 {
    let hub = host.hub_ref();
    let Some(store) = &hub.catalogs.quests.money_reward_store else {
        return 0;
    };
    let quest_level = player_quest_level_like_cpp(host, quest).max(0) as u32;
    let Some(row) = store.get(quest_level) else {
        return 0;
    };
    let difficulty = quest.reward_money_difficulty as usize;
    let Some(base) = row.difficulty.get(difficulty).copied() else {
        return 0;
    };
    ((base as f32) * quest.reward_money_multiplier).round() as u32
}

pub(crate) fn represented_player_quest_status_is_complete_or_incomplete_like_cpp<
    H: QuestSessionHostLikeCpp,
>(
    host: &H,
    quest_id: u32,
) -> bool {
    represented_player_quest_status_like_cpp(host, quest_id).is_some_and(|status| {
        matches!(
            status,
            Some(
                wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
                    | wow_conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP
            )
        )
    })
}

pub fn can_complete_repeatable_quest_represented_bounded_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    // C++ Player::CanCompleteRepeatableQuest is CanTakeQuest(false) && CanRewardQuest(false).
    // The full CanRewardQuest blocker set includes disable checks, quest status, day/week/month/
    // seasonal gates, level/skill/reputation, reward status, item/currency/money checks, etc.
    // This bounded #611 seam must not overclaim completion while those blockers remain open.
    if !can_take_quest(host, quest) {
        return false;
    }

    let Some(inventory_item_counts) = host.represented_inventory_item_counts_like_cpp() else {
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

pub fn can_reward_quest_represented_bounded_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &H,
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    // C++ `Player::CanRewardQuest(quest, false)` requires the quest to be complete
    // unless it is a DF/turn-in-only quest, then rejects already rewarded
    // non-repeatable quests. This represented seam covers the status/reward gates
    // needed by `SendQuestGiverRequestItems`; inventory-capacity and long-tail
    // reward blockers remain in the reward handler path.
    if crate::quest_is_disabled_like_cpp(host.hub_ref(), quest.id) {
        return false;
    }
    let Some(quests) = host.player_quest_gameplay_snapshot_like_cpp() else {
        return false;
    };

    if !quest.is_df_quest_like_cpp()
        && !quest.is_turn_in_like_cpp()
        && !quests
            .statuses_like_cpp()
            .get(&quest.id)
            .is_some_and(|status| status.status == wow_conditions::QUEST_STATUS_COMPLETE_LIKE_CPP)
    {
        return false;
    }

    if quests.rewarded_quest_ids_like_cpp().contains(&quest.id) && !quest.is_repeatable() {
        return false;
    }

    true
}

pub fn send_represented_quest_giver_offer_reward_like_cpp<H: QuestSessionHostLikeCpp>(
    host: &mut H,
    source_guid: ObjectGuid,
    quest: &wow_data::quest::QuestTemplate,
    auto_launched: bool,
) {
    host.send_packet(&QuestGiverOfferReward {
        giver_guid: source_guid,
        giver_creature_id: quest_giver_creature_id_from_source_like_cpp(source_guid),
        quest_id: quest.id,
        quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
        suggested_party_members: quest.suggested_group_num,
        rewards: crate::represented_quest_rewards_block_like_cpp(quest),
        title: quest.log_title.clone(),
        reward_text: quest.quest_completion_log.clone(),
        auto_launched,
    });
}
