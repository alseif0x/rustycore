// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Resolve selected execution effects through the canonical difficulty rows.

use super::SpellExecutionHostLikeCpp;
use crate::aura_session::{selected_catalog, selected_spell_metadata};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectedCastThreatPlan {
    Legacy,
    ExcludedByAttributes,
    Selected {
        whole_spell_positive: bool,
        attributes: [u32; 15],
    },
}

pub(super) fn resolve<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
    spell_info: &mut wow_data::SpellInfo,
) -> Result<SelectedCastThreatPlan, &'static str> {
    let actual_difficulty = selected_spell_info_key_difficulty(host, spell_id)?;
    resolve_at_difficulty(host, spell_id, spell_info, actual_difficulty)
}

pub(super) fn resolve_transient_full_aura<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> Result<(wow_data::SpellInfo, SelectedCastThreatPlan), &'static str> {
    let actual_difficulty = selected_spell_info_key_difficulty(host, spell_id)?;
    let catalogs = host.hub_ref().catalogs;
    let spell_store = catalogs.spell_store().ok_or("Spell not found")?;
    let difficulty_store = catalogs.difficulty_store().map(AsRef::as_ref);
    let attributes = selected_catalog::selected_spell_attributes(
        spell_store,
        spell_id,
        actual_difficulty,
        difficulty_store,
    )
    .map_err(|_| "Unable to resolve selected spell attributes")?
    .ok_or("Unable to resolve selected spell attributes")?;
    if attributes[1]
        & (wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED
            | wow_data::spell::attributes::SPELL_ATTR1_IS_SELF_CHANNELLED)
        != 0
    {
        return Err("Selected transient SpellInfo cannot represent a channelled cast");
    }

    let mut spell_info = transient_spell_info_carrier(spell_id);
    let threat_plan =
        resolve_at_difficulty(host, spell_id, &mut spell_info, actual_difficulty)?;
    if !has_supported_full_aura_shape(spell_id, &spell_info) {
        return Err("Selected spell does not have the supported three-effect Aura shape");
    }
    Ok((spell_info, threat_plan))
}

fn selected_spell_info_key_difficulty<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
) -> Result<u8, &'static str> {
    let catalogs = host.hub_ref().catalogs;
    let spell_store = catalogs.spell_store().ok_or("Spell not found")?;
    let requested_difficulty = host.core().current_map_difficulty_id_like_cpp();
    let difficulty_store = catalogs.difficulty_store().map(AsRef::as_ref);
    let spell_id_u32 = u32::try_from(spell_id).map_err(|_| "Unable to resolve selected spell key")?;
    selected_catalog::selected_spell_info_key_difficulty(
        spell_store,
        spell_id_u32,
        requested_difficulty,
        difficulty_store,
    )
    .map_err(|_| "Unable to resolve selected spell key")?
    .ok_or("Spell not found")
}

fn resolve_at_difficulty<H: SpellExecutionHostLikeCpp>(
    host: &H,
    spell_id: i32,
    spell_info: &mut wow_data::SpellInfo,
    actual_difficulty: u8,
) -> Result<SelectedCastThreatPlan, &'static str> {
    let catalogs = host.hub_ref().catalogs;
    let spell_store = catalogs.spell_store().ok_or("Spell not found")?;
    let difficulty_store = catalogs.difficulty_store().map(AsRef::as_ref);
    let effects = wow_data::spell::resolve_spell_effect_rows(
        spell_store,
        spell_id,
        actual_difficulty,
        difficulty_store,
    )
    .map_err(|_| "Unable to resolve selected spell effects")?
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();

    if effects.is_empty() {
        return Err("Unable to resolve selected spell effects");
    }

    let attributes = selected_catalog::selected_spell_attributes(
        spell_store,
        spell_id,
        actual_difficulty,
        difficulty_store,
    )
    .map_err(|_| "Unable to resolve selected spell attributes")?;
    let attributes = match attributes {
        Some(attributes) => attributes,
        None => [0; 15],
    };

    if attributes[1] & wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT != 0
        || attributes[2] & wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT != 0
        || attributes[4] & wow_data::spell::attributes::SPELL_ATTR4_NO_HARMFUL_THREAT != 0
    {
        set_selected_effects(spell_info, effects);
        return Ok(SelectedCastThreatPlan::ExcludedByAttributes);
    }

    let metadata = selected_spell_metadata(
        spell_store,
        spell_id,
        actual_difficulty,
        &effects,
        catalogs
            .spell_catalogs
            .spell_class_options_store()
            .map(|store| store.as_ref()),
        difficulty_store,
    )
    .map_err(|_| "Unable to resolve selected cast polarity metadata")?;
    let whole_spell_positive = metadata.polarity.whole_spell_positive;
    let attributes = metadata.attributes;
    set_selected_effects(spell_info, effects);

    Ok(SelectedCastThreatPlan::Selected {
        whole_spell_positive,
        attributes,
    })
}

// This projection is limited to the full-mask selected server route. The
// complete key set's lack of difficulty zero proves the zero-only
// SpellCastingRequirements row absent (SpellMgr.cpp:2537-2538), including focus 0.
// FULL (SpellDefines.h:244-261,446; Spell.cpp:3540,3816,5977) skips the
// power/reagent paths, while completion's explicit selected-FULL gate skips
// cooldown bookkeeping and its event. The exact Aura shapes exclude
// AddFarsight, the only post-SpellGo cast-time reader; known non-channelled
// attributes also keep channel cast-time handling out of this projection.
// Ordinary client and other nonzero casts retain their existing scalar readers.
fn transient_spell_info_carrier(spell_id: i32) -> wow_data::SpellInfo {
    // The carrier stays transient; selected effects fill it only after the
    // canonical nonzero SpellInfo key has been proven.
    wow_data::SpellInfo {
        spell_id,
        cast_time_ms: 0,
        cooldown_ms: 0,
        recovery_time_ms: 0,
        effect_type: 0,
        effect_base_points: 0,
        effect_bonus_coefficient: 0.0,
        aura_type: None,
        display_flags: 0,
        requires_spell_focus: 0,
        power_costs: Vec::new(),
        effects: Vec::new(),
    }
}

fn has_supported_full_aura_shape(spell_id: i32, spell_info: &wow_data::SpellInfo) -> bool {
    let effects = spell_info.effects();
    let expected_auras = match spell_id {
        8326 => [
            wow_data::spell::aura_types::SPELL_AURA_GHOST,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        ],
        20584 => [
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
        ],
        _ => [
            wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT,
        ],
    };
    effects.len() == 3
        && [0_u32, 1, 2]
            .into_iter()
            .zip(expected_auras)
            .all(|(effect_index, aura_type)| {
                effects.iter().any(|effect| {
                    effect.effect_index == effect_index
                        && effect.effect
                            == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                        && effect.effect_aura == aura_type
                })
            })
}

fn set_selected_effects(
    spell_info: &mut wow_data::SpellInfo,
    effects: Vec<wow_data::SpellEffectInfo>,
) {
    let primary = effects.iter().find(|effect| effect.effect_index == 0);
    spell_info.effect_type = primary.map_or(0, |effect| effect.effect);
    spell_info.effect_base_points = primary.map_or(0, |effect| effect.effect_base_points);
    spell_info.aura_type = primary.and_then(|effect| {
        (effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA)
            .then_some(effect.effect_aura)
    });
    spell_info.effects = effects;
}
