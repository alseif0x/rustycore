// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use wow_constants::InventoryResult;
use wow_core::ObjectGuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedQuestCompleteStatusUpdateLikeCpp {
    pub quest_id: u32,
    pub old_status: u8,
    pub new_status: u8,
    pub send_quest_update_called: bool,
    pub quest_slot_state_complete_represented: bool,
    pub quest_slot_state_live_update_unrepresented: bool,
    pub visible_gameobjects_or_spellclicks_refresh_unrepresented: bool,
    pub spell_area_runtime_unrepresented: bool,
    pub tracking_event_auto_reward_unrepresented: bool,
    pub quest_tracker_complete_time_unrepresented: bool,
    pub script_status_change_unrepresented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedQuestObjectiveProgressEventLikeCpp {
    MoneyChanged {
        old_money: u64,
        new_money: u64,
    },
    #[allow(dead_code)]
    CurrencyChanged {
        currency_id: u32,
        change: i32,
    },
    #[allow(dead_code)]
    ReputationChanged {
        faction_id: u32,
        change: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedPendingQuestSharingLikeCpp {
    pub sender_guid: ObjectGuid,
    pub quest_id: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedQuestPushResultResponseLikeCpp {
    pub receiver_guid: ObjectGuid,
    pub sender_guid: ObjectGuid,
    pub parsed_quest_id: u32,
    pub pending_quest_id: u32,
    pub result: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedPushQuestToPartyOutcomeReasonLikeCpp {
    NotAllowed,
    NotDaily,
    QuestPoolActiveCheckUnrepresented,
    NotInParty,
    GroupRuntimeUnrepresented,
    ReceiverBusy,
    ReceiverDead,
    ReceiverAlreadyDone,
    ReceiverOnQuest,
    ReceiverLogFull,
    ReceiverSatisfyQuestDayAlreadyDone,
    ReceiverSatisfyQuestMinLevelLowLevel,
    ReceiverSatisfyQuestMaxLevelHighLevel,
    ReceiverSatisfyQuestClassWrongClass,
    ReceiverSatisfyQuestRaceWrongRace,
    ReceiverSatisfyQuestReputationLowFaction,
    ReceiverSatisfyQuestReputationHighFaction,
    ReceiverSatisfyQuestPreviousQuestPrerequisite,
    ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite,
    ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite,
    ReceiverSatisfyQuestExpansionRequiredExpansion,
    ReceiverCanTakeQuestInvalid,
    #[allow(dead_code)]
    ReceiverRepeatableTurnInRequestItemsUnrepresented,
    ReceiverRepeatableTurnInRequestItemsPrompted,
    ReceiverRepeatableTurnInRequestItemsPromptCommandFailed,
    ReceiverSuccessQuestDetailsPrompted,
    ReceiverQuestDetailsPromptCommandFailed,
    ReceiverEligibilityUnrepresented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedPushQuestToPartyOutcomeLikeCpp {
    pub sender_guid: Option<ObjectGuid>,
    pub quest_id: u32,
    pub target_guid: Option<ObjectGuid>,
    pub reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp,
    pub quest_pool_active_check_unrepresented: bool,
    pub group_runtime_unrepresented: bool,
    pub receiver_fanout_unrepresented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp {
    OriginalPlayerMissing,
    NotInSameRaid,
    OriginalPlayerNotActiveQuest,
    ReceiverCanTakeQuestFailed,
    ReceiverCanAddQuestLogFull,
    ReceiverCanAddQuestSourceItemFailed,
    ReceiverGiveQuestSourceItemStartQuestNoGrant,
    ReceiverGiveQuestSourceItemMaxCountNoGrant,
    ReceiverGiveQuestSourceItemStoredNewItem,
    ReceiverGiveQuestSourceItemBoundObjectiveNoGrant,
    GiveQuestSourceItemStoreNewItemUnrepresented,
    ReceiverAddQuestLocalStateRepresented,
    #[allow(dead_code)]
    AddQuestRuntimeUnrepresented,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedQuestConfirmAcceptLikeCpp {
    pub receiver_guid: Option<ObjectGuid>,
    pub sender_guid_before_clear: ObjectGuid,
    pub quest_id: u32,
    pub raw_quest_id: i32,
    pub reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    pub object_accessor_unrepresented: bool,
    pub party_runtime_unrepresented: bool,
    pub can_add_source_item_unrepresented: bool,
    pub can_add_source_item_result: Option<InventoryResult>,
    pub add_quest_runtime_unrepresented: bool,
    pub source_spell_unrepresented: bool,
    pub represented_source_spell_id: Option<u32>,
    pub represented_source_spell_self_casts: u8,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedQuestRewardSpellKindLikeCpp {
    RewardSpell,
    RewardDisplaySpell { index: u8 },
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedQuestRewardSpellCastLikeCpp {
    pub quest_id: u32,
    pub spell_id: u32,
    pub kind: RepresentedQuestRewardSpellKindLikeCpp,
    pub can_delay_teleport_like_cpp: bool,
    pub spell_info_lookup_unrepresented: bool,
    pub caster_selection_unrepresented: bool,
    pub cast_spell_runtime_unrepresented: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedQuestRewardTitleLikeCpp {
    pub quest_id: u32,
    pub title_id: u32,
    pub char_title_lookup_unrepresented: bool,
    pub set_title_runtime_unrepresented: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedQuestRewardTalentPointsLikeCpp {
    pub quest_id: u32,
    pub points: u32,
    pub init_talent_for_level_unrepresented: bool,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedQuestRewardMailLikeCpp {
    pub quest_id: u32,
    pub mail_template_id: u32,
    pub delay_secs: u32,
    pub sender_entry: Option<u32>,
    pub quest_giver_guid: Option<ObjectGuid>,
    pub mail_template_lookup_unrepresented: bool,
    pub mail_draft_runtime_unrepresented: bool,
    pub character_db_transaction_unrepresented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedQuestRewardReputationSourceLikeCpp {
    Quest,
    DailyQuest,
    WeeklyQuest,
    MonthlyQuest,
    RepeatableQuest,
}

#[cfg(any(test, feature = "test-fixtures"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedQuestRewardReputationLikeCpp {
    pub quest_id: u32,
    pub slot: u8,
    pub faction_id: u32,
    pub reward_faction_value: i32,
    pub reward_faction_override: i32,
    pub reward_faction_cap_in: i32,
    pub base_reputation_before_gain: i32,
    pub reputation_after_low_level_rate_like_cpp: i32,
    pub reputation_after_reward_rate_like_cpp: i32,
    pub no_quest_bonus: bool,
    pub no_spillover: bool,
    pub source: RepresentedQuestRewardReputationSourceLikeCpp,
    pub faction_store_lookup_unrepresented: bool,
    pub quest_faction_reward_store_lookup_unrepresented: bool,
    pub reputation_reward_rate_lookup_unrepresented: bool,
    pub gray_level_script_hook_unrepresented: bool,
    pub reputation_rank_cap_check_unrepresented: bool,
    pub calculate_reputation_gain_unrepresented: bool,
    pub modify_reputation_runtime_unrepresented: bool,
}
