//! Names only the session scenarios read through the session root (#1363
//! slices 9e-11b): the player cast, cast-check, spell-click and battle-pet
//! names, the aura interrupt and area flags, the capture-point state, the
//! spell-area autocast names, the dynamic-object VALUES bridge, the teleport,
//! rest-flag and create-map names. `#![cfg(test)]`; the session root mounts
//! and glob-imports it under `cfg(test)` (#1395/#1396 follow-up 1).

#![cfg(test)]

pub(crate) use super::gameobject_interaction::RepresentedCapturePointStateLikeCpp;
pub(crate) use crate::entity_update_bridge::dynamic_object_values_update_to_update_object;
pub(crate) use wow_data::spell_click::{
    SPELL_CLICK_USER_PARTY_LIKE_CPP, SPELL_CLICK_USER_RAID_LIKE_CPP,
};
pub(crate) use wow_data::{SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP, SpellAreaLikeCpp};
pub(crate) use wow_packet::packets::spell::SpellTargetData;
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
    REST_FLAG_IN_CITY_LIKE_CPP, REST_FLAG_IN_FACTION_AREA_LIKE_CPP,
    create_map_instance_lock_token_like_cpp,
};
pub(crate) use wow_world_inventory::MAX_BATTLE_PET_LEVEL_LIKE_CPP;
pub(crate) use wow_world_spell::{
    AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP, RepresentedSpellFocusObjectLikeCpp,
    SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP,
};
