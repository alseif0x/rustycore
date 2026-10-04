// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Spell click values: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{HighGuid, MAX_GAMEOBJECT_SLOT_LIKE_CPP, ObjectGuid, PhaseShift};
use super::{Position, SPELL_CAST_SOURCE_NORMAL_LIKE_CPP};

pub(in crate::session) fn spell_effect_is_represented_summon_object_slot_like_cpp(
    effect: u32,
) -> bool {
    let slot_base = wow_data::spell::spell_effect_types::SPELL_EFFECT_SUMMON_OBJECT_SLOT1;
    let slot_end = slot_base + u32::try_from(MAX_GAMEOBJECT_SLOT_LIKE_CPP).unwrap_or(0);
    (slot_base..slot_end).contains(&effect)
}

pub(in crate::session) fn spell_effect_has_non_or_db_nearby_entry_destination_like_cpp(
    effect: &wow_data::SpellEffectInfo,
) -> bool {
    matches!(
        effect.implicit_target_1,
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY
            | wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
    ) || matches!(
        effect.implicit_target_2,
        wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY
            | wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
    )
}

// C++ `ObjectGuid::Create<HighGuid::Cast>` passes realm id 0 to
// `CreateWorldObject`, then `ObjectGuidFactory.cpp:GetRealmIdForObjectGuid(0)`
// substitutes `realm.Id.Realm`. Rust passes that already-resolved local realm
// explicitly; using the caster realm preserves the same visible GUID bits.
pub(in crate::session) fn represented_spell_cast_guid_for_map_like_cpp(
    realm_id: u16,
    map_id: u16,
    spell_id: i32,
    counter: i64,
) -> ObjectGuid {
    ObjectGuid::create_world_object(
        HighGuid::Cast,
        SPELL_CAST_SOURCE_NORMAL_LIKE_CPP,
        realm_id,
        map_id,
        0,
        u32::try_from(spell_id).unwrap_or_default(),
        counter,
    )
}

pub(crate) use wow_world_core::session::RepresentedGameObjectAccessLikeCpp;

pub(crate) use wow_world_core::session::RepresentedCreatureAccessLikeCpp;

pub(crate) use wow_world_application::RepresentedCanSeeSpellClickOutcomeLikeCpp;

pub(in crate::session) const NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP: u8 = 0x01;
pub(in crate::session) const NPC_CLICK_CAST_TARGET_CLICKER_LIKE_CPP: u8 = 0x02;
pub(in crate::session) const NPC_CLICK_CAST_ORIG_CASTER_OWNER_LIKE_CPP: u8 = 0x04;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedSpellClickUnitRefLikeCpp {
    Clicker,
    Clickee,
    Owner,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedSpellClickCastLikeCpp {
    pub spell_id: u32,
    pub caster: RepresentedSpellClickUnitRefLikeCpp,
    pub target: RepresentedSpellClickUnitRefLikeCpp,
    pub original_caster: RepresentedSpellClickUnitRefLikeCpp,
    pub cast_flags: u8,
    pub vehicle_seat_id: Option<i8>,
    pub vehicle_control_effect_index: Option<u32>,
    pub vehicle_spellmod_basepoint_value: Option<i32>,
    pub vehicle_aura_fallback_basepoint_value: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RepresentedSpellClickPlanLikeCpp {
    pub casts: Vec<RepresentedSpellClickCastLikeCpp>,
    pub exact_context_unrepresented: bool,
    pub ai_on_spell_click_unrepresented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct RepresentedSpellClickExecutionOutcomeLikeCpp {
    pub planned_casts: usize,
    pub executed_casts: usize,
    pub ai_on_spell_click_represented: bool,
    pub ai_on_spell_click_unrepresented: bool,
    pub skipped_unrepresented_caster: usize,
    pub skipped_unrepresented_target: usize,
    pub skipped_unrepresented_original_caster: usize,
    pub failed_casts: usize,
}

#[cfg(test)]
pub(crate) use wow_world_core::session::{
    RepresentedVehicleEnterRequestLikeCpp, RepresentedVehicleSeatChangeRequestLikeCpp,
    RepresentedVehicleSeatSpellClickRequestLikeCpp,
};

#[cfg(test)]
pub(crate) use wow_world_core::session::{
    RepresentedVehicleBaseMovementLikeCpp, RepresentedVehicleDismissMovementLikeCpp,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::session) enum RepresentedSpellClickClickeeCasterOutcomeLikeCpp {
    Executed,
    UnsupportedCaster,
    UnsupportedTarget,
    UnsupportedOriginalCaster,
    Failed,
}

pub(in crate::session) use wow_world_entities::RepresentedSpellClickCreatureSnapshotLikeCpp;

pub(in crate::session) fn represented_spell_click_school_damage_amount_like_cpp(
    spell_info: &wow_data::SpellInfo,
) -> Option<u32> {
    let direct_spell_effects_like_cpp: Vec<(u32, i32)> = if spell_info.effects().is_empty() {
        vec![(spell_info.effect_type, spell_info.effect_base_points)]
    } else {
        spell_info
            .effects()
            .iter()
            .filter(|effect| effect.effect != 0)
            .map(|effect| (effect.effect, effect.effect_base_points))
            .collect()
    };

    let mut damage_amount = 0_u32;
    for (effect_type, effect_base_points) in direct_spell_effects_like_cpp {
        if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(effect_type) {
            continue;
        }
        if effect_type != wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE {
            return None;
        }
        damage_amount = damage_amount.checked_add(u32::try_from(effect_base_points).ok()?)?;
    }

    Some(damage_amount)
}
