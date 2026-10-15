//! Names only the session scenarios read through the session root (#1363
//! slices 9e, 10a and 10b): the player cast, cast-check and spell-click names,
//! the aura interrupt flags, the capture-point state and the spell-area
//! autocast names. `#![cfg(test)]`; the session root mounts and glob-imports
//! it under `cfg(test)` (#1395 follow-up 1, #1396 follow-up 1).

#![cfg(test)]

pub(crate) use super::gameobject_interaction::RepresentedCapturePointStateLikeCpp;
pub(crate) use wow_data::spell_click::{
    SPELL_CLICK_USER_PARTY_LIKE_CPP, SPELL_CLICK_USER_RAID_LIKE_CPP,
};
pub(crate) use wow_data::{SPELL_AREA_FLAG_AUTOCAST_LIKE_CPP, SpellAreaLikeCpp};
pub(crate) use wow_packet::packets::spell::SpellTargetData;
pub(crate) use wow_world_core::session::RepresentedGetReactionInputLikeCpp;
pub(crate) use wow_world_entities::SPELL_AURA_INTERRUPT_FLAG_TURNING_LIKE_CPP;
pub(crate) use wow_world_entities::spell_click_values::{
    NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP, NPC_CLICK_CAST_ORIG_CASTER_OWNER_LIKE_CPP,
    NPC_CLICK_CAST_TARGET_CLICKER_LIKE_CPP, RepresentedSpellClickExecutionOutcomeLikeCpp,
    RepresentedSpellClickPlanLikeCpp, RepresentedSpellClickUnitRefLikeCpp,
};
pub(crate) use wow_world_inventory::MAX_BATTLE_PET_LEVEL_LIKE_CPP;
pub(crate) use wow_world_spell::{
    RepresentedSpellFocusObjectLikeCpp, SPELL_AURA_INTERRUPT_FLAG_MOVING_LIKE_CPP,
};
