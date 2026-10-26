// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Application and refresh for the race-selected resurrection sickness Aura.

use super::AuraRemovalCxLikeCpp;
use std::collections::BTreeMap;
use wow_data::SpellEffectInfo;
use wow_world_core::session::{SelectedAuraEffectRefresh, SelectedAuraKind, SelectedAuraRefresh};

const ACTIVE_EFFECT_MASK: u32 = 0b111;
const SICKNESS_AURA_TYPES: [i32; 3] = [
    wow_data::spell::aura_types::SPELL_AURA_MOD_PERCENT_STAT,
    wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
    wow_data::spell::aura_types::SPELL_AURA_NO_PVP_CREDIT,
];

fn selected_effects<'a>(
    spell_id: i32,
    rows: impl IntoIterator<Item = &'a SpellEffectInfo>,
    effect_mask: u32,
) -> Result<Vec<&'a SpellEffectInfo>, &'static str> {
    if spell_id <= 0 || effect_mask != ACTIVE_EFFECT_MASK {
        return Err("Race-selected sickness SpellInfo is not the supported three-effect Aura");
    }
    let mut effects = BTreeMap::new();
    for effect in rows {
        let index = usize::try_from(effect.effect_index)
            .map_err(|_| "Race-selected sickness effect index is invalid")?;
        if index >= SICKNESS_AURA_TYPES.len()
            || effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
            || effect.effect_aura != SICKNESS_AURA_TYPES[index]
            || !matches!(effect.implicit_target_1, 0 | 1)
            || effect.implicit_target_2 != 0
            || effects.insert(index, effect).is_some()
        {
            return Err("Race-selected sickness effect catalog differs from the supported Aura");
        }
    }
    if effects.len() != SICKNESS_AURA_TYPES.len() {
        return Err("Race-selected sickness effect catalog is incomplete");
    }
    Ok(effects.into_values().collect())
}

pub(super) fn validate_selected_sickness_data<'a>(
    spell_id: i32,
    rows: impl IntoIterator<Item = &'a SpellEffectInfo>,
    effect_mask: u32,
) -> Result<(), &'static str> {
    if effect_mask == 0 || effect_mask & !ACTIVE_EFFECT_MASK != 0 {
        return Err("Race-selected sickness application has an unsupported active mask");
    }
    selected_effects(spell_id, rows, ACTIVE_EFFECT_MASK).map(|_| ())
}

impl AuraRemovalCxLikeCpp<'_> {
    pub fn apply_resurrection_sickness_aura(
        &mut self,
        spell_id: i32,
        caster_guid: wow_core::ObjectGuid,
        spell_info: &wow_data::SpellInfo,
        duration_total: i32,
        stack_amount: u32,
        attributes1: u32,
        attributes5: u32,
        creation: Result<(u32, wow_entities::AuraCastProvenanceLikeCpp, u8), &'static str>,
    ) -> Result<(), &'static str> {
        if self.is_current_race_sickness_spell(spell_id) != Some(true) {
            return Err("Spell is not the current race-selected sickness Aura");
        }
        if duration_total <= 0 {
            return Err("Race-selected sickness duration is invalid");
        }
        wow_world_core::session::selected_aura_refreshes_timers(
            stack_amount,
            1,
            attributes1,
            attributes5,
        )?;
        let selected = selected_effects(spell_id, spell_info.effects().iter(), ACTIVE_EFFECT_MASK)?;
        let spell_store = self
            .spell_store
            .map(|store| store.as_ref())
            .ok_or("Missing SpellStore for the race-selected sickness Aura")?;
        let spell_id_u32 = u32::try_from(spell_id)
            .map_err(|_| "Invalid race-selected sickness SpellInfo key")?;
        wow_data::spell::resolve_spell_info_key_difficulty(
            spell_store,
            spell_id_u32,
            self.map_difficulty_id,
            self.difficulty_store,
        )
        .map_err(|_| "Unable to resolve race-selected sickness SpellInfo key")?
        .ok_or("Missing SpellInfo for the race-selected sickness Aura")?;
        let stat_application = super::stat_application::select_selected_aura_stat_application(
            Some(spell_store),
            spell_id,
            self.map_difficulty_id,
            self.difficulty_store,
            ACTIVE_EFFECT_MASK,
        )?;
        if stat_application
            .as_ref()
            .and_then(|selection| selection.cause)
            .is_none()
        {
            return Err("Race-selected sickness has no supported stat effects");
        }
        if self.player.is_logged_in_like_cpp() {
            self.preflight_selected_sickness_stats()?;
        }

        let aura_snapshot = self
            .player
            .aura_subsystem_snapshot_like_cpp()
            .ok_or("Missing canonical Player aura subsystem for sickness application")?;
        let mut same_caster = aura_snapshot
            .runtime_applications_like_cpp()
            .values()
            .filter(|aura| aura.spell_id == spell_id && aura.caster_guid == caster_guid)
            .collect::<Vec<_>>();
        same_caster.sort_by_key(|aura| aura.slot);
        if same_caster.len() > 1 {
            return Err("Multiple sickness AuraApplications share the same caster");
        }

        if let Some(existing) = same_caster
            .first()
            .filter(|aura| aura.effect_mask == ACTIVE_EFFECT_MASK)
        {
            let original_rows = wow_data::spell::resolve_spell_effect_rows(
                spell_store,
                spell_id,
                existing.difficulty_id,
                self.difficulty_store,
            )
            .map_err(|_| "Unable to resolve existing sickness Aura effects")?;
            let original_effects =
                selected_effects(spell_id, original_rows.iter().copied(), ACTIVE_EFFECT_MASK)?;
            let refresh_timers = wow_world_core::session::selected_aura_refreshes_timers(
                stack_amount,
                existing.stack_count,
                attributes1,
                attributes5,
            )?;
            let charges = if refresh_timers {
                let spell_store = self
                    .spell_store
                    .map(AsRef::as_ref)
                    .ok_or("Missing SpellStore for existing sickness aura charges")?;
                let spell_id_u32 =
                    u32::try_from(spell_id).map_err(|_| "Invalid selected sickness spell")?;
                let actual_difficulty = wow_data::spell::resolve_spell_info_key_difficulty(
                    spell_store,
                    spell_id_u32,
                    existing.difficulty_id,
                    self.difficulty_store,
                )
                .map_err(|_| "Unable to resolve existing sickness SpellInfo key")?
                .ok_or("Existing sickness SpellInfo key is missing")?;
                wow_world_spell::selected_aura_max_charges(
                    spell_id_u32,
                    actual_difficulty,
                    self.spell_aura_options_store,
                    self.spell_proc_store,
                    self.difficulty_store,
                )?
            } else {
                aura_snapshot
                    .aura_persistence_metadata(existing.slot)
                    .ok_or("Canonical sickness Aura persistence metadata is missing")?
                    .charges
            };
            let metadata = aura_snapshot
                .aura_persistence_metadata(existing.slot)
                .ok_or("Canonical sickness Aura persistence metadata is missing")?;
            if metadata.recalculate_mask & !ACTIVE_EFFECT_MASK != 0 {
                return Err("Canonical sickness Aura has an invalid recalculation mask");
            }
            let snapshot = aura_snapshot
                .threat_snapshot_like_cpp(existing.slot)
                .ok_or("Canonical sickness Aura effect snapshot is missing")?;
            let mut updates = Vec::with_capacity(selected.len());
            for effect in &selected {
                let original = original_effects
                    .iter()
                    .find(|original| original.effect_index == effect.effect_index)
                    .copied()
                    .ok_or("Existing sickness Aura effect is missing during refresh")?;
                let bit = 1_u32
                    .checked_shl(effect.effect_index)
                    .ok_or("Race-selected sickness effect index is invalid")?;
                let _current = snapshot
                    .effects()
                    .iter()
                    .find(|(active_bit, _, _, _)| *active_bit == bit)
                    .ok_or("Canonical sickness Aura effect no longer matches refresh")?;
                let mut refreshed = (*original).clone();
                refreshed.effect_base_points = effect.effect_base_points;
                updates.push(SelectedAuraEffectRefresh {
                    effect_index: u8::try_from(effect.effect_index)
                        .map_err(|_| "Race-selected sickness effect index is invalid")?,
                    aura_type: original.effect_aura,
                    amount: refreshed.calc_value_no_caster_like_cpp(),
                    misc_value: original.effect_misc_value_1,
                    base_amount: effect.effect_base_points,
                });
            }
            let refresh = SelectedAuraRefresh {
                kind: SelectedAuraKind::Sickness,
                spell_id,
                caster_guid,
                slot: existing.slot,
                difficulty_id: existing.difficulty_id,
                effect_mask: ACTIVE_EFFECT_MASK,
                duration_total,
                stack_amount,
                attributes1,
                attributes5,
                refresh_timers,
                charges,
                recalculate_mask: metadata.recalculate_mask,
                effects: &updates,
            };
            self.player
                .refresh_selected_aura_application(&refresh)
                .ok_or("Canonical Player disappeared while refreshing sickness Aura")??;
            self.apply_selected_sickness_stats(
                spell_id,
                ACTIVE_EFFECT_MASK,
                existing.difficulty_id,
            )?;
            return Ok(());
        }

        let (aura_flags, provenance, charges) = creation?;

        // `_AddAura` removes every existing same-spell application before it
        // admits a new Aura, including a same-caster application with a
        // different effect mask.
        for slot in aura_snapshot.ordered_slots_for_spell_removal(spell_id)? {
            self.remove_aura_like_cpp(slot)?;
        }
        let duration_total = u32::try_from(duration_total)
            .map_err(|_| "Race-selected sickness duration exceeds the application range")?;
        self.apply_aura_with_effect_mask_provenance_and_update_like_cpp(
            spell_id,
            caster_guid,
            duration_total,
            aura_flags,
            ACTIVE_EFFECT_MASK,
            provenance,
            false,
        )?;

        let aura = self
            .player
            .aura_subsystem_snapshot_like_cpp()
            .and_then(|auras| {
                auras
                    .runtime_applications_like_cpp()
                    .values()
                    .find(|aura| aura.spell_id == spell_id && aura.caster_guid == caster_guid)
                    .cloned()
            })
            .ok_or("Canonical sickness Aura disappeared after application")?;
        let metadata = wow_entities::AuraPersistenceMetadata {
            item_guid: wow_core::ObjectGuid::EMPTY,
            cast_item_id: 0,
            cast_item_level: -1,
            // Fresh AuraEffect instances start recalculatable. These three
            // handlers do not clear that state in CalculateAmount.
            recalculate_mask: ACTIVE_EFFECT_MASK,
            charges,
            base_amounts: selected
                .iter()
                .map(|effect| {
                    Ok((
                        u8::try_from(effect.effect_index)
                            .map_err(|_| "Race-selected sickness effect index is invalid")?,
                        effect.effect_base_points,
                    ))
                })
                .collect::<Result<Vec<_>, &'static str>>()?,
        };
        let initial_effects = selected
            .iter()
            .map(|effect| {
                Ok(SelectedAuraEffectRefresh {
                    effect_index: u8::try_from(effect.effect_index)
                        .map_err(|_| "Race-selected sickness effect index is invalid")?,
                    aura_type: effect.effect_aura,
                    amount: effect.calc_value_no_caster_like_cpp(),
                    misc_value: effect.effect_misc_value_1,
                    base_amount: effect.effect_base_points,
                })
            })
            .collect::<Result<Vec<_>, &'static str>>()?;
        self.player
            .initialize_selected_aura_metadata(
                SelectedAuraKind::Sickness,
                spell_id,
                aura.slot,
                metadata,
                &initial_effects,
            )
            .ok_or("Canonical Player disappeared while storing sickness metadata")??;
        Ok(())
    }
}
