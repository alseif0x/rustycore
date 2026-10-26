// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The selected aura subset accepted by persisted Player aura loading.

use wow_core::ObjectGuid;
use wow_data::{
    DifficultyStore, SpellClassOptionsStore, SpellDurationStore, SpellEffectInfo, SpellMiscStore,
    SpellStore,
};

use super::{
    selected_catalog::{
        selected_aura_options_entry, selected_spell_info_key_difficulty,
        selected_spell_misc_entry,
    },
    selected_metadata::{SelectedSpellMetadata, selected_spell_metadata},
};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum SelectedAuraKind {
    Ghost,
    Wisp,
    Sickness,
}

impl SelectedAuraKind {
    pub(super) fn for_spell(spell_id: u32, sickness_spell_id: Option<u32>) -> Option<Self> {
        match spell_id {
            8_326 => Some(Self::Ghost),
            20_584 => Some(Self::Wisp),
            _ if sickness_spell_id == Some(spell_id) => Some(Self::Sickness),
            _ => None,
        }
    }
}

pub(super) struct SelectedAuraMetadata {
    pub(super) effects: Vec<SpellEffectInfo>,
    pub(super) spell: SelectedSpellMetadata,
    pub(super) base_duration_ms: i32,
    pub(super) channeled: bool,
}

pub(super) fn resolve_selected_aura_metadata(
    kind: SelectedAuraKind,
    spell_id: u32,
    difficulty_id: u8,
    spells: &SpellStore,
    misc_store: Option<&SpellMiscStore>,
    duration_store: Option<&SpellDurationStore>,
    class_options: Option<&SpellClassOptionsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<SelectedAuraMetadata, ()> {
    let spell_id_i32 = i32::try_from(spell_id).map_err(|_| ())?;
    let effects = wow_data::spell::resolve_spell_effect_rows(
        spells,
        spell_id_i32,
        difficulty_id,
        difficulties,
    )
    .map_err(|_| ())?
    .into_iter()
    .cloned()
    .collect::<Vec<_>>();
    if !selected_aura_effect_contract(kind, &effects) {
        return Err(());
    }

    let spell = selected_spell_metadata(
        spells,
        spell_id_i32,
        difficulty_id,
        &effects,
        class_options,
        difficulties,
    )
    .map_err(|_| ())?;

    let misc_store = misc_store.ok_or(())?;
    let misc = selected_spell_misc_entry(misc_store, spell_id, difficulty_id, difficulties)
        .map_err(|_| ())?;
    if misc.is_none()
        && selected_spell_info_key_difficulty(spells, spell_id, difficulty_id, difficulties)
            .map_err(|_| ())?
            .is_none()
    {
        return Err(());
    }

    let duration_store = duration_store.ok_or(())?;
    let duration_index = misc.map_or(0, |entry| u32::from(entry.duration_index));
    let duration_entry = duration_store.get(duration_index);
    if kind == SelectedAuraKind::Sickness
        && (duration_index == 0 || duration_entry.is_none())
    {
        return Err(());
    }
    let base_duration_ms =
        wow_data::spell_duration_ms_like_cpp(duration_index, Some(duration_store));
    let duration_is_supported = match kind {
        SelectedAuraKind::Ghost | SelectedAuraKind::Wisp => base_duration_ms == -1,
        SelectedAuraKind::Sickness => base_duration_ms > 0,
    };
    if !duration_is_supported {
        return Err(());
    }

    let channelled_attributes = wow_data::spell::attributes::SPELL_ATTR1_IS_CHANNELLED
        | wow_data::spell::attributes::SPELL_ATTR1_IS_SELF_CHANNELLED;
    let channeled = spell.attributes[1] & channelled_attributes != 0;

    Ok(SelectedAuraMetadata {
        effects,
        spell,
        base_duration_ms,
        channeled,
    })
}

pub(super) fn selected_aura_options(
    spell_id: u32,
    difficulty_id: u8,
    options_store: Option<&wow_data::SpellAuraOptionsStore>,
    difficulties: Option<&DifficultyStore>,
) -> Result<u32, ()> {
    let options_store = options_store.ok_or(())?;
    let entry = selected_aura_options_entry(options_store, spell_id, difficulty_id, difficulties)
        .map_err(|_| ())?;
    Ok(entry.map_or(0, |row| row.proc_charges as u32))
}

pub(super) fn normalize_loaded_charges(full_proc_charges: u32, saved_charges: u8) -> u8 {
    if full_proc_charges == 0 {
        0
    } else if saved_charges == 0 {
        full_proc_charges as u8
    } else {
        saved_charges
    }
}

pub(super) fn sickness_recalculated_amount(
    effect: &SpellEffectInfo,
    base_amount: i32,
    stack_count: u8,
) -> Option<i32> {
    if !matches!(
        effect.effect_aura,
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE
            | wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT
            | wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT
    ) {
        return None;
    }
    let mut effect = effect.clone();
    effect.effect_base_points = base_amount;
    effect
        .calc_value_no_caster_like_cpp()
        .checked_mul(i32::from(stack_count))
}

pub(super) fn ghost_wisp_recalculated_amount(
    spell_id: u32,
    effect_index: u32,
    effect: &SpellEffectInfo,
    base_amount: i32,
    stack_count: u8,
) -> Option<i32> {
    let expected_aura = match (spell_id, effect_index) {
        (8_326, 0) => wow_data::spell::aura_types::SPELL_AURA_GHOST,
        (8_326, 1) => wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        (8_326, 2) => wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        (20_584, 0) => wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
        (20_584, 1) => wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        (20_584, 2) => wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
        _ => return None,
    };
    if effect.effect_aura != expected_aura {
        return None;
    }
    let mut effect = effect.clone();
    effect.effect_base_points = base_amount;
    effect
        .calc_value_no_caster_like_cpp()
        .checked_mul(i32::from(stack_count))
}

pub(super) fn selected_aura_effect_contract(
    kind: SelectedAuraKind,
    effects: &[SpellEffectInfo],
) -> bool {
    let expected = match kind {
        SelectedAuraKind::Ghost => [
            wow_data::spell::aura_types::SPELL_AURA_GHOST,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        ],
        SelectedAuraKind::Wisp => [
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            wow_data::spell::aura_types::SPELL_AURA_TRANSFORM,
        ],
        SelectedAuraKind::Sickness => [
            wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT,
            wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT,
        ],
    };
    if effects.len() != expected.len() {
        return false;
    }
    let mut seen_mask = 0_u32;
    for effect in effects {
        let Some(bit) = 1_u32.checked_shl(effect.effect_index) else {
            return false;
        };
        let Ok(index) = usize::try_from(effect.effect_index) else {
            return false;
        };
        if index >= expected.len()
            || seen_mask & bit != 0
            || effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
            || effect.effect_aura != expected[index]
            || !matches!(effect.implicit_target_1, 0 | 1)
            || effect.implicit_target_2 != 0
        {
            return false;
        }
        seen_mask |= bit;
    }
    seen_mask == 0b111
}

/// Apply the selected loader's represented subset of C++ `Aura::CanBeSaved`.
/// Runtime single-target state and the custom cannot-save flag are outside this
/// loader's supported metadata contract.
pub(super) fn selected_aura_can_be_saved(
    caster_guid: ObjectGuid,
    player_guid: ObjectGuid,
    item_guid: ObjectGuid,
    metadata: &SelectedAuraMetadata,
) -> bool {
    const SPELL_ATTR0_PASSIVE: u32 = 0x0000_0040;
    const SPELL_ATTR5_LIMIT_N: u32 = 0x0000_0020;
    metadata.spell.attributes[0] & SPELL_ATTR0_PASSIVE == 0
        && !metadata.channeled
        && !(caster_guid != player_guid
            && metadata.spell.attributes[5] & SPELL_ATTR5_LIMIT_N != 0)
        && !(item_guid != ObjectGuid::EMPTY && metadata.base_duration_ms == -1)
}
