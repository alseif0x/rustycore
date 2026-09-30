// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Catalog adapters for canonical Player/Unit aura-effect queries.

use std::collections::HashMap;
use wow_data::SpellStore;
use wow_entities::{AppliedAuraRef, AuraApplicationLikeCpp};
pub(crate) use wow_data_model::aura_effects::{
    AppliedAuraEffectLikeCpp,
    RepresentedAbsorbShieldLikeCpp, RepresentedHealAbsorbShieldLikeCpp,
    RepresentedManaShieldLikeCpp,
};

/// Compatibility name for the shared resolved aura-effect schema.
pub(crate) use wow_data_model::aura_effects::AppliedAuraEffectLikeCpp as PlayerAuraEffectLikeCpp;

/// C++ `Unit::GetAuraEffectsByType(auraType)` for one player's applied auras.
///
/// Auras are visited in ascending slot order so the projection is deterministic
/// (C++ walks its aura list in application order); the amount prefers the
/// represented `AuraEffect` value and falls back to the spell effect's
/// no-caster calculation, exactly like the session adapter used to do.
pub(crate) fn player_aura_effects_full_by_spell_aura_type_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<PlayerAuraEffectLikeCpp> {
    wow_entities::AuraSubsystem::player_effects(
        auras,
        Some(aura_type),
        |spell_id| spell_store.get(spell_id).map(|spell| spell.effects()),
        |effect| (
            effect.effect_index,
            effect.effect,
            effect.effect_aura,
            effect.effect_misc_value_1,
            effect.effect_misc_value_2,
        ),
        wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
        |effect| effect,
    )
}

/// Every active effect of a player's applied auras, the input C++
/// `MeleeDamageBonusTaken`'s `GetTotalAuraModifier*` chain reads across several
/// aura types at once.
pub(crate) fn player_aura_effects_all_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
) -> Vec<PlayerAuraEffectLikeCpp> {
    wow_entities::AuraSubsystem::player_effects(
        auras,
        None,
        |spell_id| spell_store.get(spell_id).map(|spell| spell.effects()),
        |effect| (
            effect.effect_index,
            effect.effect,
            effect.effect_aura,
            effect.effect_misc_value_1,
            effect.effect_misc_value_2,
        ),
        wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
        |effect| effect,
    )
}

/// The `(MiscValue, amount)` projection every `GetTotalAuraModifier*` caller
/// reads.
pub(crate) fn player_aura_effects_by_spell_aura_type_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    aura_type: i32,
) -> Vec<(i32, i32)> {
    wow_entities::AuraSubsystem::player_effects(
        auras,
        Some(aura_type),
        |spell_id| spell_store.get(spell_id).map(|spell| spell.effects()),
        |effect| (
            effect.effect_index,
            effect.effect,
            effect.effect_aura,
            effect.effect_misc_value_1,
            effect.effect_misc_value_2,
        ),
        wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
        |effect| (effect.misc_value, effect.amount),
    )
}

/// C++ `Unit::CalcAbsorbResist`'s `SPELL_AURA_SCHOOL_ABSORB` selection
/// (`Unit.cpp:1812-1825`): every active absorb effect whose `MiscValue` covers
/// the incoming school mask.
///
/// Auras are visited in ascending slot order so the input to the priority sort
/// is deterministic; the amount prefers the represented `AuraEffect` value and
/// falls back to the spell effect's no-caster calculation, exactly like the
/// generic aura projection.
pub(crate) fn player_absorb_shields_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    school_mask: u32,
) -> Vec<RepresentedAbsorbShieldLikeCpp> {
    wow_entities::AuraSubsystem::player_absorb_shields(
        auras,
        school_mask,
        |spell_id| spell_store.get(spell_id).map(|spell| spell.effects()),
        |effect| (
            effect.effect_index,
            effect.effect,
            effect.effect_aura,
            effect.effect_misc_value_1,
            effect.effect_misc_value_2,
        ),
        wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
        |spell_id| {
            spell_store
                .hit_metadata_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
                .map_or(0, |metadata| metadata.category_id)
        },
        |spell_id| {
            spell_store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                difficulty_store,
                6,
                wow_data::spell::attributes::SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE,
            )
        },
    )
}

/// C++ `Unit::CalcAbsorbResist`'s school-absorb selection for a creature
/// victim. Creature aura applications use the canonical `AppliedAuraRef`
/// tables rather than the player's runtime-application map, but the mutable
/// amount still belongs to the same canonical aura subsystem. Reading that
/// amount here keeps a later hit from recreating an already-spent creature
/// shield from the spell's base points.
pub(crate) fn creature_absorb_shields_like_cpp(
    auras: &wow_entities::AuraSubsystem,
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    school_mask: u32,
) -> Vec<RepresentedAbsorbShieldLikeCpp> {
    auras.creature_absorb_shields(
        school_mask,
        |spell_id| spell_store.get(spell_id).map(|spell| spell.effects()),
        |effect| (
            effect.effect_index,
            effect.effect,
            effect.effect_aura,
            effect.effect_misc_value_1,
            effect.effect_misc_value_2,
        ),
        wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
        |spell_id| {
            spell_store
                .hit_metadata_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
                .map_or(0, |metadata| metadata.category_id)
        },
        |spell_id| {
            spell_store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                difficulty_store,
                6,
                wow_data::spell::attributes::SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE,
            )
        },
    )
}

/// C++ `Unit::CalcAbsorbResist`'s `SPELL_AURA_MANA_SHIELD` selection
/// (`Unit.cpp:1886-1897`): every active mana-shield effect whose `MiscValue`
/// covers the incoming school mask.
///
/// C++ iterates `GetAuraEffectsByType` in application order; the represented
/// projection visits auras in ascending slot order so the loop is
/// deterministic.
pub(crate) fn player_mana_shields_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
    school_mask: u32,
) -> Vec<RepresentedManaShieldLikeCpp> {
    wow_entities::AuraSubsystem::player_mana_shields(
        auras,
        school_mask,
        |spell_id| spell_store.get(spell_id).map(|spell| spell.effects()),
        |effect| (
            effect.effect_index,
            effect.effect,
            effect.effect_aura,
            effect.effect_misc_value_1,
            effect.effect_misc_value_2,
        ),
        wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
        wow_data::SpellEffectInfo::calc_value_multiplier_like_cpp,
        |spell_id| {
            spell_store.has_attribute_for_difficulty_like_cpp(
                spell_id,
                difficulty_id,
                difficulty_store,
                6,
                wow_data::spell::attributes::SPELL_ATTR6_ABSORB_CANNOT_BE_IGNORE,
            )
        },
    )
}

/// C++ `Unit::CalcHealAbsorb`'s `SPELL_AURA_SCHOOL_HEAL_ABSORB` selection
/// (`Unit.cpp:2025-2034`): every active heal-absorb effect whose `MiscValue`
/// covers the heal's school mask, in the aura order C++
/// `GetAuraEffectsByType` returns.
pub(crate) fn player_heal_absorb_shields_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    school_mask: u32,
) -> Vec<RepresentedHealAbsorbShieldLikeCpp> {
    wow_entities::AuraSubsystem::player_heal_absorb_shields(
        auras,
        school_mask,
        |spell_id| spell_store.get(spell_id).map(|spell| spell.effects()),
        |effect| (
            effect.effect_index,
            effect.effect,
            effect.effect_aura,
            effect.effect_misc_value_1,
            effect.effect_misc_value_2,
        ),
        wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
    )
}

/// C++ `Unit::HasAuraWithMechanic` (`Unit.cpp:4714-4729`) over a canonical
/// Player's `AuraApplication` map: the union of each application's
/// `SpellInfo::Mechanic` and its applied `IsEffect()` slots' mechanics, read at
/// the application's own difficulty.
pub(crate) fn aura_application_mechanic_mask_like_cpp(
    auras: &HashMap<u8, AuraApplicationLikeCpp>,
    spell_store: &SpellStore,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> u64 {
    wow_entities::AuraSubsystem::application_mechanic_mask(
        auras,
        |spell_id, difficulty_id| {
            spell_store
                .hit_metadata_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
                .map(|metadata| (i32::from(metadata.spell_mechanic), metadata.effect_mechanics))
        },
        |spell_id, difficulty_id| {
            spell_store.effects_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
        },
        |effect| (
            effect.effect_index,
            effect.effect,
            effect.effect_aura,
            effect.effect_misc_value_1,
            effect.effect_misc_value_2,
        ),
    )
}

/// C++ `Unit::HasAuraWithMechanic` over a creature's `AppliedAuraRef` list.
/// Those refs do not retain a difficulty, so the caller supplies the one it
/// resolves the rest of the victim's state with.
pub(crate) fn applied_aura_mechanic_mask_like_cpp(
    applied_auras: &[AppliedAuraRef],
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> u64 {
    wow_entities::AuraSubsystem::applied_mechanic_mask(
        applied_auras,
        difficulty_id,
        |spell_id, difficulty_id| {
            spell_store
                .hit_metadata_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
                .map(|metadata| (i32::from(metadata.spell_mechanic), metadata.effect_mechanics))
        },
        |spell_id, difficulty_id| {
            spell_store.effects_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
        },
        |effect| (
            effect.effect_index,
            effect.effect,
            effect.effect_aura,
            effect.effect_misc_value_1,
            effect.effect_misc_value_2,
        ),
    )
}

/// C++ `Unit::GetAuraEffectsByType`/`GetTotalAuraModifier*` over a creature's
/// `AuraApplicationMap`: every active effect of the unit's applied auras,
/// resolved at the caller's difficulty.
///
/// The player-side counterpart is
/// [`player_aura_effects_by_spell_aura_type_like_cpp`]; a creature's
/// `AppliedAuraRef` carries no represented effect amounts, so the effect's
/// no-caster calculation is the value, exactly like the mechanic-mask
/// projection.
pub(crate) fn creature_aura_effects_like_cpp(
    applied_auras: &[AppliedAuraRef],
    spell_store: &SpellStore,
    difficulty_id: u8,
    difficulty_store: Option<&wow_data::DifficultyStore>,
) -> Vec<AppliedAuraEffectLikeCpp> {
    wow_entities::AuraSubsystem::creature_effects(
        applied_auras,
        difficulty_id,
        |spell_id, difficulty_id| {
            spell_store.effects_for_difficulty_like_cpp(spell_id, difficulty_id, difficulty_store)
        },
        |effect| (
            effect.effect_index,
            effect.effect,
            effect.effect_aura,
            effect.effect_misc_value_1,
            effect.effect_misc_value_2,
        ),
        wow_data::SpellEffectInfo::calc_value_no_caster_like_cpp,
    )
}
