// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical refresh for the selected resurrection aura applications.

use super::PlayerAuraRemovalAccessLikeCpp;
use std::time::Instant;
use wow_entities::{
    AppliedAuraRef, AuraPersistenceMetadata, AuraThreatSnapshotLikeCpp,
    RepresentedAuraEffectAmountLikeCpp, VisibleAuraEffectAmountLikeCpp,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedAuraKind {
    Ghost,
    Wisp,
    Sickness,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SelectedAuraEffectRefresh {
    pub effect_index: u8,
    pub aura_type: i32,
    pub amount: i32,
    pub misc_value: i32,
    pub base_amount: i32,
}

pub struct SelectedAuraRefresh<'a> {
    pub kind: SelectedAuraKind,
    pub spell_id: i32,
    pub caster_guid: wow_core::ObjectGuid,
    pub slot: u8,
    pub effect_mask: u32,
    pub duration_total: i32,
    pub stack_amount: u32,
    pub attributes1: u32,
    pub attributes5: u32,
    pub charges: u8,
    pub refresh_timers: bool,
    pub recalculate_mask: u32,
    pub effects: &'a [SelectedAuraEffectRefresh],
}

const ATTR1_AURA_UNIQUE: u32 = 0x0000_0800;
const ATTR5_AURA_UNIQUE_PER_CASTER: u32 = 0x2000_0000;

/// Calculate whether `Aura::ModStackAmount` refreshes timers and charges.
/// C++ computes this before changing the stack count (`SpellAuras.cpp:1078-1094`);
/// `SharedDefines.h` defines Attr1 AuraUnique as `0x800` and Attr5
/// AuraUniquePerCaster as `0x20000000`. Selected AuraOptions rows with actual
/// stacking remain outside this narrow implementation.
pub fn selected_aura_refreshes_timers(
    stack_amount: u32,
    current_stack: u8,
    attributes1: u32,
    attributes5: u32,
) -> Result<bool, &'static str> {
    if stack_amount != 0 {
        return Err("Selected cumulative Aura stacking is unsupported");
    }
    if current_stack == 0 {
        return Err("Selected Aura has an invalid zero stack count");
    }
    Ok(1 >= current_stack
        && attributes1 & ATTR1_AURA_UNIQUE == 0
        && attributes5 & ATTR5_AURA_UNIQUE_PER_CASTER == 0)
}

fn expected_aura_types(kind: SelectedAuraKind, spell_id: i32) -> Option<[i32; 3]> {
    use wow_data::spell::aura_types as aura;

    match kind {
        SelectedAuraKind::Ghost if spell_id == 8_326 => Some([
            aura::SPELL_AURA_GHOST,
            aura::SPELL_AURA_MOD_INCREASE_SPEED,
            aura::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        ]),
        SelectedAuraKind::Wisp if spell_id == 20_584 => Some([
            aura::SPELL_AURA_MOD_INCREASE_SPEED,
            aura::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            aura::SPELL_AURA_TRANSFORM,
        ]),
        SelectedAuraKind::Sickness if spell_id > 0 => Some([
            aura::SPELL_AURA_MOD_PERCENT_STAT,
            aura::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            aura::SPELL_AURA_NO_PVP_CREDIT,
        ]),
        _ => None,
    }
}

impl PlayerAuraRemovalAccessLikeCpp<'_> {
    /// Install the source-known metadata for a newly created selected aura.
    /// Callers supply the live effect recalculation state; this does not infer
    /// missing metadata during a refresh.
    pub fn initialize_selected_aura_metadata(
        &self,
        kind: SelectedAuraKind,
        spell_id: i32,
        slot: u8,
        metadata: AuraPersistenceMetadata,
        effects: &[SelectedAuraEffectRefresh],
    ) -> Option<Result<(), &'static str>> {
        let expected = expected_aura_types(kind, spell_id)?;
        self.core.with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let Some(application) = auras.runtime_application_like_cpp(slot) else {
                return Err("Selected AuraApplication disappeared while initializing metadata");
            };
            if application.spell_id != spell_id
                || application.slot != slot
                || application.effect_mask != 0b111
                || auras.aura_persistence_metadata(slot).is_some()
                || metadata.recalculate_mask & !application.effect_mask != 0
                || (kind == SelectedAuraKind::Wisp && metadata.recalculate_mask & 0b100 != 0)
                || metadata.base_amounts.len() != expected.len()
                || effects.len() != expected.len()
            {
                return Err("Selected AuraApplication metadata does not match its active effects");
            }
            let active_mask = application.effect_mask;
            let caster_guid = application.caster_guid;
            let mut seen = 0_u32;
            for &(index, _) in &metadata.base_amounts {
                let bit = 1_u32
                    .checked_shl(u32::from(index))
                    .ok_or("Selected AuraApplication has an invalid base effect index")?;
                if index >= 3 || seen & bit != 0 || active_mask & bit == 0 {
                    return Err("Selected AuraApplication has duplicate base effect metadata");
                }
                seen |= bit;
            }
            if seen != active_mask {
                return Err("Selected AuraApplication base effect metadata is incomplete");
            }
            let mut updates = [None; 3];
            let mut update_mask = 0_u32;
            for effect in effects {
                let index = usize::from(effect.effect_index);
                let bit = 1_u32
                    .checked_shl(u32::from(effect.effect_index))
                    .ok_or("Selected AuraApplication effect index is invalid")?;
                if index >= expected.len()
                    || expected[index] != effect.aura_type
                    || active_mask & bit == 0
                    || update_mask & bit != 0
                {
                    return Err("Selected AuraApplication effect metadata is invalid");
                }
                update_mask |= bit;
                updates[index] = Some(*effect);
                if !metadata.base_amounts.iter().any(|(base_index, amount)| {
                    usize::from(*base_index) == index && *amount == effect.base_amount
                }) {
                    return Err("Selected AuraApplication base amount differs from its metadata");
                }
            }
            if update_mask != active_mask || updates.iter().any(Option::is_none) {
                return Err("Selected AuraApplication effect metadata is incomplete");
            }
            let snapshot = auras
                .threat_snapshot_like_cpp(slot)
                .cloned()
                .ok_or("Selected AuraApplication effect snapshot is missing")?;
            let mut snapshot_mask = 0_u32;
            for &(bit, aura_type, amount, misc_value) in snapshot.effects() {
                let index = bit.trailing_zeros() as usize;
                let Some(update) = updates.get(index).and_then(|entry| *entry) else {
                    return Err("Selected AuraApplication effect snapshot differs from its kind");
                };
                if bit.count_ones() != 1
                    || index >= expected.len()
                    || snapshot_mask & bit != 0
                    || expected[index] != aura_type
                    || active_mask & bit == 0
                    || update.amount != amount
                    || update.misc_value != misc_value
                {
                    return Err("Selected AuraApplication effect snapshot is inconsistent");
                }
                snapshot_mask |= bit;
            }
            if snapshot_mask != 0 && snapshot_mask != active_mask {
                return Err("Selected AuraApplication effect snapshot is incomplete");
            }
            if snapshot_mask == 0 {
                if kind != SelectedAuraKind::Sickness {
                    return Err("Selected resurrection aura effect snapshot is missing");
                }
                for index in 0..expected.len() {
                    let applied =
                        AppliedAuraRef::new(spell_id as u32, caster_guid, slot, 1_u32 << index);
                    if auras.applied_auras.contains(&applied)
                        || auras.applied_aura_amounts.contains_key(&applied)
                        || auras.applied_aura_misc_values.contains_key(&applied)
                        || auras
                            .applied_aura_types
                            .values()
                            .any(|effects| effects.contains(&applied))
                    {
                        return Err("Selected sickness Aura has effect indexes without a snapshot");
                    }
                }
                let effects = updates
                    .iter()
                    .enumerate()
                    .map(|(index, update)| {
                        let effect = update.expect("complete selected effect updates validated");
                        (
                            1_u32 << index,
                            effect.aura_type,
                            effect.amount,
                            effect.misc_value,
                        )
                    })
                    .collect::<Vec<_>>();
                let seeded = AuraThreatSnapshotLikeCpp::new(snapshot.interrupt_flags(), effects);
                let interrupt_flags = seeded.interrupt_flags();
                auras.insert_threat_snapshot_like_cpp(slot, seeded);
                for (index, update) in updates.iter().enumerate() {
                    let effect = update.expect("complete selected effect updates validated");
                    let applied =
                        AppliedAuraRef::new(spell_id as u32, caster_guid, slot, 1_u32 << index);
                    auras.register_applied_aura(
                        applied,
                        None,
                        interrupt_flags[0],
                        interrupt_flags[1],
                    );
                    auras.register_applied_aura_effect_like_cpp(
                        applied,
                        effect.aura_type,
                        effect.amount,
                        effect.misc_value,
                    );
                }
            }
            if !auras.set_aura_persistence_metadata(slot, metadata) {
                return Err("Selected AuraApplication disappeared while storing metadata");
            }
            Ok(())
        })
    }

    /// Refresh one selected Aura in its existing slot. This mirrors
    /// `Unit::_TryStackingOrRefreshingExistingAura` followed by
    /// `Aura::SetStackAmount` while retaining the Aura's cast identity.
    pub fn refresh_selected_aura_application(
        &self,
        refresh: &SelectedAuraRefresh<'_>,
    ) -> Option<Result<(bool, bool), &'static str>> {
        let expected = expected_aura_types(refresh.kind, refresh.spell_id)?;
        if refresh.effect_mask != 0b111
            || refresh.effects.len() != expected.len()
            || refresh.recalculate_mask & !refresh.effect_mask != 0
            || match refresh.kind {
                SelectedAuraKind::Ghost | SelectedAuraKind::Wisp => refresh.duration_total != -1,
                SelectedAuraKind::Sickness => refresh.duration_total <= 0,
            }
        {
            return Some(Err("Selected aura refresh data is incomplete or invalid"));
        }
        if refresh.kind == SelectedAuraKind::Wisp && refresh.recalculate_mask & 0b100 != 0 {
            return Some(Err(
                "Wisp transform recalculation must clear its recalculation bit",
            ));
        }

        self.core.with_owned_player_mut_like_cpp(|player| {
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let Some(application) = auras.runtime_application_like_cpp(refresh.slot) else {
                return Err("Canonical selected AuraApplication disappeared");
            };
            if application.spell_id != refresh.spell_id
                || application.caster_guid != refresh.caster_guid
                || application.slot != refresh.slot
                || application.effect_mask != refresh.effect_mask
            {
                return Err("Canonical selected AuraApplication no longer matches refresh");
            }
            let current_refresh_timers = selected_aura_refreshes_timers(
                refresh.stack_amount,
                application.stack_count,
                refresh.attributes1,
                refresh.attributes5,
            )?;
            if refresh.refresh_timers != current_refresh_timers {
                return Err("Selected Aura refresh policy differs from its canonical stack");
            }

            let mut updates = [None; 3];
            let mut update_mask = 0_u32;
            for effect in refresh.effects {
                let index = usize::from(effect.effect_index);
                let bit = 1_u32
                    .checked_shl(u32::from(effect.effect_index))
                    .ok_or("Selected aura refresh has an invalid effect index")?;
                if index >= expected.len()
                    || expected[index] != effect.aura_type
                    || update_mask & bit != 0
                    || refresh.effect_mask & bit == 0
                {
                    return Err("Selected aura refresh has an invalid effect mapping");
                }
                update_mask |= bit;
                updates[index] = Some(*effect);
            }
            if update_mask != refresh.effect_mask || updates.iter().any(Option::is_none) {
                return Err("Selected aura refresh does not cover all active effects");
            }

            let Some(snapshot) = auras.threat_snapshot_like_cpp(refresh.slot).cloned() else {
                return Err("Canonical selected aura effect snapshot is missing");
            };
            if snapshot.effects().len() != expected.len() {
                return Err("Canonical selected aura effect snapshot is incomplete");
            }
            let Some(mut metadata) = auras.aura_persistence_metadata(refresh.slot).cloned() else {
                return Err("Canonical selected aura persistence metadata is missing");
            };
            if metadata.recalculate_mask & !refresh.effect_mask != 0
                || metadata.base_amounts.len() != expected.len()
            {
                return Err("Canonical selected aura persistence metadata is incomplete");
            }
            let expected_recalculate_mask = if refresh.kind == SelectedAuraKind::Wisp {
                metadata.recalculate_mask & !0b100
            } else {
                metadata.recalculate_mask
            };
            if refresh.recalculate_mask != expected_recalculate_mask {
                return Err("Selected aura refresh changed unrelated recalculation bits");
            }
            let mut base_mask = 0_u32;
            for &(index, _) in &metadata.base_amounts {
                let bit = 1_u32
                    .checked_shl(u32::from(index))
                    .ok_or("Canonical selected aura has an invalid base effect index")?;
                if index >= 3 || base_mask & bit != 0 || refresh.effect_mask & bit == 0 {
                    return Err("Canonical selected aura has duplicate base effect metadata");
                }
                base_mask |= bit;
            }
            if base_mask != refresh.effect_mask {
                return Err("Canonical selected aura base effect metadata is incomplete");
            }

            let mut ordered_updates = Vec::with_capacity(expected.len());
            let mut snapshot_mask = 0_u32;
            let mut run_amount_changed = false;
            let mut swim_amount_changed = false;
            for &(bit, aura_type, amount, misc_value) in snapshot.effects() {
                if bit.count_ones() != 1
                    || refresh.effect_mask & bit == 0
                    || snapshot_mask & bit != 0
                {
                    return Err("Canonical selected aura snapshot has an invalid effect mask");
                }
                snapshot_mask |= bit;
                let index = bit.trailing_zeros() as usize;
                let Some(update) = updates.get(index).and_then(|entry| *entry) else {
                    return Err("Canonical selected aura effect no longer matches refresh");
                };
                if update.aura_type != aura_type || update.misc_value != misc_value {
                    return Err("Canonical selected aura effect metadata changed");
                }
                let applied = AppliedAuraRef::new(
                    refresh.spell_id as u32,
                    refresh.caster_guid,
                    refresh.slot,
                    bit,
                );
                let Some(current_amount) = auras.applied_aura_amounts.get(&applied).copied() else {
                    return Err("Canonical selected aura amount index is missing");
                };
                if !auras.applied_auras.contains(&applied)
                    || !auras
                        .applied_aura_types
                        .get(&aura_type)
                        .is_some_and(|effects| effects.contains(&applied))
                    || current_amount != amount
                {
                    return Err("Canonical selected aura amount index is inconsistent");
                }
                match aura_type {
                    wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED => {
                        run_amount_changed |= amount != update.amount;
                    }
                    wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED => {
                        swim_amount_changed |= amount != update.amount;
                    }
                    _ => {}
                }
                ordered_updates.push((bit, update));
            }
            if snapshot_mask != refresh.effect_mask {
                return Err("Canonical selected aura snapshot does not cover every active effect");
            }

            let mut base_amount_mask = 0_u32;
            for effect in refresh.effects {
                let bit = 1_u32 << effect.effect_index;
                base_amount_mask |= bit;
                let Some(base) = metadata
                    .base_amounts
                    .iter_mut()
                    .find(|(index, _)| *index == effect.effect_index)
                else {
                    return Err("Canonical selected aura base effect metadata is missing");
                };
                base.1 = effect.base_amount;
            }
            if base_amount_mask != refresh.effect_mask {
                return Err("Selected aura refresh base effect metadata is incomplete");
            }
            let cast_item_changed = !metadata.item_guid.is_empty();
            metadata.item_guid = wow_core::ObjectGuid::EMPTY;
            if cast_item_changed {
                metadata.cast_item_id = 0;
                metadata.cast_item_level = -1;
            }
            metadata.recalculate_mask = refresh.recalculate_mask;
            if refresh.refresh_timers {
                metadata.charges = refresh.charges;
            }

            let represented_amounts: Vec<RepresentedAuraEffectAmountLikeCpp> = ordered_updates
                .iter()
                .map(|(bit, effect)| RepresentedAuraEffectAmountLikeCpp {
                    effect_index: bit.trailing_zeros() as u8,
                    amount: effect.amount,
                })
                .collect();
            {
                let application = auras
                    .runtime_application_mut_like_cpp(refresh.slot)
                    .expect("selected AuraApplication validated above");
                if refresh.refresh_timers {
                    application.duration_total = refresh.duration_total;
                    application.duration_remaining = refresh.duration_total;
                    application.applied_at = Instant::now();
                }
                application.stack_count = 1;
                application.represented_effect_amounts = represented_amounts.clone();
                if let Some(effect) = represented_amounts.first() {
                    application.represented_amount = effect.amount;
                }
            }

            let mut refreshed_effects = Vec::with_capacity(ordered_updates.len());
            for (bit, effect) in ordered_updates {
                let applied = AppliedAuraRef::new(
                    refresh.spell_id as u32,
                    refresh.caster_guid,
                    refresh.slot,
                    bit,
                );
                if let Some(effects) = auras.applied_aura_types.get_mut(&effect.aura_type) {
                    effects.retain(|known| *known != applied);
                    effects.push(applied);
                }
                auras.register_applied_aura_effect_like_cpp(
                    applied,
                    effect.aura_type,
                    effect.amount,
                    effect.misc_value,
                );
                refreshed_effects.push((bit, effect.aura_type, effect.amount, effect.misc_value));
            }
            auras.insert_threat_snapshot_like_cpp(
                refresh.slot,
                AuraThreatSnapshotLikeCpp::new(snapshot.interrupt_flags(), refreshed_effects),
            );
            if let Some(visible) = auras
                .visible_aura_applications_like_cpp
                .get_mut(&refresh.slot)
            {
                visible.effect_amounts = represented_amounts
                    .iter()
                    .map(|effect| VisibleAuraEffectAmountLikeCpp {
                        effect_index: effect.effect_index,
                        amount: effect.amount,
                    })
                    .collect();
            }
            if !auras.set_aura_persistence_metadata(refresh.slot, metadata) {
                return Err("Canonical selected AuraApplication disappeared during refresh");
            }
            auras.visible_auras_to_update.insert(refresh.slot);
            Ok((run_amount_changed, swim_amount_changed))
        })
    }
}
