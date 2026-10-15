//! Names only the session scenarios read through the session root (#1363 slices 9e-13c): the
//! player cast, cast-check, spell-click, battle-pet, aura-interrupt, area-flag, capture-point,
//! spell-area autocast, dynamic-object VALUES, teleport, rest-flag, create-map, transfer-abort and
//! login names. `#![cfg(test)]`; mounted and glob-imported in cfg(test) (#1395/#1396 follow-up 1).

#![cfg(test)]

pub(crate) use super::gameobject_interaction::RepresentedCapturePointStateLikeCpp;
pub(crate) use super::player_spell_records::RepresentedPlayerSkillLikeCpp;
pub(crate) use crate::entity_update_bridge::dynamic_object_values_update_to_update_object;
pub(crate) use std::collections::HashSet;
pub(crate) use wow_data::spell_click::{
    SPELL_CLICK_USER_PARTY_LIKE_CPP, SPELL_CLICK_USER_RAID_LIKE_CPP,
};
pub(crate) use wow_data::{SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP, SpellAreaLikeCpp};
pub(crate) use wow_entities::MAX_POWERS_PER_CLASS;
pub(crate) use wow_packet::packets::spell::SpellTargetData;
pub(crate) use wow_world_application::PLAYER_FLAGS_GROUP_LEADER_LIKE_CPP;
pub(crate) use wow_world_core::session::RepresentedGetReactionInputLikeCpp;
pub(crate) use wow_world_core::session::battle_pet_adapter::{
    RepresentedBattlePetCageItemLikeCpp, RepresentedBattlePetCalculatedStatsLikeCpp,
    RepresentedBattlePetLevelCriteriaLikeCpp, RepresentedBattlePetQueryCompanionLikeCpp,
    RepresentedBattlePetSaveInfoLikeCpp,
};
pub(crate) use wow_world_core::session::movement_protocol::{
    MoveTeleportAckActionLikeCpp, TELE_TO_SEAMLESS_LIKE_CPP,
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
pub(crate) use wow_world_spell::{
    AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP, RepresentedSpellFocusObjectLikeCpp,
    SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP,
};
