//! Names only the session scenarios read through the session root (#1363 6a-13c, 12b4): cast,
//! spell-click, battle-pet, aura, area, capture-point, autocast, VALUES, teleport, rest, map,
//! login, quest, void-storage. `#![cfg(test)]`; mounted and glob-imported in cfg(test) (#1395/6).

#![cfg(test)]

pub(crate) use super::gameobject_interaction::RepresentedCapturePointStateLikeCpp;
pub(crate) use super::player_spell_records::RepresentedPlayerSkillLikeCpp;
pub(crate) use crate::entity_update_bridge::dynamic_object_values_update_to_update_object;
pub(crate) use std::collections::HashSet;
pub(crate) use wow_constants::currency::CurrencyGainSourceLikeCpp;
pub(crate) use wow_constants::quest::{
    QUEST_OBJECTIVE_CURRENCY_LIKE_CPP, QUEST_OBJECTIVE_HAVE_CURRENCY_LIKE_CPP,
    QUEST_OBJECTIVE_INCREASE_REPUTATION_LIKE_CPP, QUEST_OBJECTIVE_ITEM_LIKE_CPP,
    QUEST_OBJECTIVE_MAX_REPUTATION_LIKE_CPP, QUEST_OBJECTIVE_MIN_REPUTATION_LIKE_CPP,
    QUEST_OBJECTIVE_MONEY_LIKE_CPP, QUEST_OBJECTIVE_OBTAIN_CURRENCY_LIKE_CPP,
};
pub(crate) use wow_data::skill::SKILL_RIDING_LIKE_CPP;
pub(crate) use wow_data::spell_click::{
    SPELL_CLICK_USER_PARTY_LIKE_CPP, SPELL_CLICK_USER_RAID_LIKE_CPP,
};
pub(crate) use wow_data::{SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP, SpellAreaLikeCpp};
pub(crate) use wow_entities::MAX_POWERS_PER_CLASS;
pub(crate) use wow_entities::PlayerVoidStorageItemLikeCpp as RepresentedVoidStorageItemLikeCpp;
pub(crate) use wow_packet::packets::spell::SpellTargetData;
pub(crate) use wow_world_application::PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP;
pub(crate) use wow_world_core::session::battle_pet_adapter::{
    RepresentedBattlePetCageItemLikeCpp, RepresentedBattlePetCalculatedStatsLikeCpp,
    RepresentedBattlePetLevelCriteriaLikeCpp, RepresentedBattlePetQueryCompanionLikeCpp,
    RepresentedBattlePetSaveInfoLikeCpp,
};
pub(crate) use wow_world_core::session::{
    PLAYER_FLAGS_RESTING_LIKE_CPP, PLAYER_LOCAL_FLAG_OVERRIDE_TRANSPORT_SERVER_TIME_LIKE_CPP,
    RepresentedGetReactionInputLikeCpp, ReputationGainSourceLikeCpp, game_time_ms_like_cpp,
    movement_protocol::{MoveTeleportAckActionLikeCpp, TELE_TO_SEAMLESS_LIKE_CPP},
};
pub(crate) use wow_world_entities::SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP;
pub(crate) use wow_world_entities::spell_click_values::{
    NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP, NPC_CLICK_CAST_ORIG_CASTER_OWNER_LIKE_CPP,
    NPC_CLICK_CAST_TARGET_CLICKER_LIKE_CPP, RepresentedSpellClickExecutionOutcomeLikeCpp,
    RepresentedSpellClickPlanLikeCpp, RepresentedSpellClickUnitRefLikeCpp,
};
pub(crate) use wow_world_entities::{
    CLASS_DEATH_KNIGHT_LIKE_CPP, DEATH_KNIGHT_ESCAPE_SPELL_LIKE_CPP,
    DEATH_KNIGHT_START_MAP_LIKE_CPP, TELE_REVIVE_AT_TELEPORT_LIKE_CPP,
    TELE_TO_NOT_LEAVE_COMBAT_LIKE_CPP, TELE_TO_SPELL_LIKE_CPP,
    TRANSFER_ABORT_INSUF_EXPAN_LVL_LIKE_CPP, TRANSFER_ABORT_UNIQUE_MESSAGE_LIKE_CPP,
};
pub(crate) use wow_world_instances::{
    AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP, CreateMapSideEffectApplySummaryLikeCpp,
    DIFFICULTY_10_N_LIKE_CPP, DIFFICULTY_NORMAL_LIKE_CPP, DIFFICULTY_NORMAL_RAID_LIKE_CPP,
    REST_FLAG_IN_CITY_LIKE_CPP, REST_FLAG_IN_FACTION_AREA_LIKE_CPP,
    TRANSFER_ABORT_MAX_PLAYERS_LIKE_CPP, TRANSFER_ABORT_NEED_GROUP_LIKE_CPP,
    TRANSFER_ABORT_TOO_MANY_INSTANCES_LIKE_CPP, TRANSFER_ABORT_ZONE_IN_COMBAT_LIKE_CPP,
    create_map_instance_lock_token_like_cpp,
};
pub(crate) use wow_world_inventory::MAX_BATTLE_PET_LEVEL_LIKE_CPP;
pub(crate) use wow_world_inventory::item_push_result_from_send_new_item_plan;
pub(crate) use wow_world_quest::{
    QUEST_FLAGS_EX_RECAST_ACCEPT_SPELL_ON_LOGIN_LIKE_CPP, QUEST_FLAGS_PLAYER_CAST_ACCEPT_LIKE_CPP,
    ResetSeasonalQuestStatusReasonLikeCpp, SeasonalQuestStatusDbRowLikeCpp,
};
pub(crate) use wow_world_quest::{
    RepresentedPendingQuestSharingLikeCpp, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedQuestCompleteStatusUpdateLikeCpp, RepresentedQuestConfirmAcceptLikeCpp,
    RepresentedQuestPushResultResponseLikeCpp,
};
pub(crate) use wow_world_spell::spell_cast_adapter::represented_spell_cast_guid_for_map_like_cpp;
pub(crate) use wow_world_spell::{
    AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP, RepresentedPlayerSpellLikeCpp,
    RepresentedSpellFocusObjectLikeCpp, SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP,
};
