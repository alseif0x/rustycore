// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! C++ `WorldSession::HandleQuestgiverCompleteQuest` dialog decision and the
//! shared quest reward block, owned by the application layer.

use wow_constants::quest::QUEST_OBJECTIVE_ITEM_LIKE_CPP;
use wow_data::quest::QuestTemplate;
use wow_packet::packets::quest::QuestRewardsBlock;

/// C++ `Quest::HasQuestObjectiveType(QUEST_OBJECTIVE_ITEM)`.
pub fn represented_quest_has_item_objective_like_cpp(quest: &QuestTemplate) -> bool {
    quest
        .objectives
        .iter()
        .any(|objective| objective.obj_type == QUEST_OBJECTIVE_ITEM_LIKE_CPP)
}

/// Which dialog C++ sends back for one quest-completion request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedQuestCompleteDialogLikeCpp {
    RequestItems {
        can_complete: bool,
        auto_launched: bool,
    },
    OfferReward {
        auto_launched: bool,
    },
}

/// C++ `HandleQuestgiverCompleteQuest` branch selection.
///
/// The bounded `CanRewardQuest`/`CanCompleteRepeatableQuest` projections stay
/// with the World session that evaluates them; this owns which dialog the
/// evaluated quest produces and with which flags.
pub fn represented_quest_complete_dialog_like_cpp(
    quest: &QuestTemplate,
    quest_is_complete: bool,
    can_reward_quest: bool,
    can_complete_repeatable_quest: bool,
) -> RepresentedQuestCompleteDialogLikeCpp {
    if !quest_is_complete {
        let can_complete = if quest.is_repeatable() {
            can_complete_repeatable_quest
        } else {
            can_reward_quest
        };
        return RepresentedQuestCompleteDialogLikeCpp::RequestItems {
            can_complete,
            auto_launched: false,
        };
    }

    if represented_quest_has_item_objective_like_cpp(quest) {
        return RepresentedQuestCompleteDialogLikeCpp::RequestItems {
            can_complete: can_reward_quest,
            auto_launched: false,
        };
    }

    RepresentedQuestCompleteDialogLikeCpp::OfferReward {
        auto_launched: true,
    }
}

/// C++ `WorldPackets::Quest::QuestRewards` projection for one quest template.
pub fn represented_quest_rewards_block_like_cpp(quest: &QuestTemplate) -> QuestRewardsBlock {
    let mut rewards = QuestRewardsBlock {
        money: quest.reward_money_difficulty as i32,
        completion_spell: quest.reward_spell as i32,
        ..QuestRewardsBlock::default()
    };
    for (idx, reward_item) in quest.reward_items.iter().enumerate() {
        if let Some(reward_slot) = rewards.items.get_mut(idx) {
            let amount = quest.reward_amounts.get(idx).copied().unwrap_or(0);
            *reward_slot = (*reward_item, amount);
        }
    }
    for (idx, display_spell) in quest.reward_display_spell.iter().enumerate() {
        if let Some(slot) = rewards.display_spells.get_mut(idx) {
            *slot = *display_spell;
        }
    }
    for (idx, choice_item) in quest.reward_choice_items.iter().enumerate() {
        if let Some(slot) = rewards.choice_items.get_mut(idx) {
            *slot = *choice_item;
        }
    }
    rewards.choice_item_types = quest.reward_choice_item_types;
    rewards
}
