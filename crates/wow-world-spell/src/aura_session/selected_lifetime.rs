// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Explicit Map-owned lifetime step for the selected resurrection auras.
//!
//! C++ anchors: `Unit::Update`/`SpellUpdate` (`Unit.cpp:2854-2865`),
//! `Aura::UpdateOwner` (`SpellAuras.cpp:778`) and `Aura::IsExpired`
//! (`SpellAuras.h:176-177`). Production activation belongs to the admitted
//! Map Recovery coordinator, not the legacy Session expiry sweep.

use super::AuraHostLikeCpp;
use wow_entities::{AppliedAuraRef, AuraSubsystem};

const GHOST_SPELL: i32 = 8_326;
const WISP_SPELL: i32 = 20_584;
const SELECTED_EFFECT_MASK: u32 = 0b111;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SelectedAuraLifetimeResult {
    /// Canonical applications whose positive remaining time was decremented.
    pub decremented: usize,
    /// Expired applications removed successfully through the current host.
    pub expired: usize,
}

fn expected_effect_types(spell_id: i32, sickness_spell_id: Option<i32>) -> Option<[i32; 3]> {
    use wow_data::spell::aura_types as aura;

    match spell_id {
        GHOST_SPELL => Some([
            aura::SPELL_AURA_GHOST,
            aura::SPELL_AURA_MOD_INCREASE_SPEED,
            aura::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
        ]),
        WISP_SPELL => Some([
            aura::SPELL_AURA_MOD_INCREASE_SPEED,
            aura::SPELL_AURA_MOD_INCREASE_SWIM_SPEED,
            aura::SPELL_AURA_TRANSFORM,
        ]),
        _ if sickness_spell_id == Some(spell_id) => Some([
            aura::SPELL_AURA_MOD_PERCENT_STAT,
            aura::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
            aura::SPELL_AURA_NO_PVP_CREDIT,
        ]),
        _ => None,
    }
}

pub(crate) fn validate_application(
    auras: &AuraSubsystem,
    spell_id: i32,
    slot: u8,
    sickness_spell_id: Option<i32>,
) -> Result<i32, &'static str> {
    let Some(application) = auras.runtime_application_like_cpp(slot) else {
        return Err("Selected Aura application disappeared during lifetime preparation");
    };
    let expected = expected_effect_types(spell_id, sickness_spell_id)
        .ok_or("Unsupported selected Aura spell in lifetime preparation")?;
    if application.spell_id != spell_id
        || application.slot != slot
        || application.effect_mask == 0
        || application.effect_mask & !SELECTED_EFFECT_MASK != 0
    {
        return Err("Selected Aura application has an unsupported active effect mask");
    }
    let Some(snapshot) = auras.threat_snapshot_like_cpp(slot) else {
        return Err("Selected Aura effect snapshot is missing during lifetime preparation");
    };
    let mut snapshot_mask = 0_u32;
    for &(bit, aura_type, amount, _) in snapshot.effects() {
        if bit.count_ones() != 1 || application.effect_mask & bit == 0 || snapshot_mask & bit != 0 {
            return Err("Selected Aura effect snapshot has an invalid active mask");
        }
        let index = bit.trailing_zeros() as usize;
        if index >= expected.len() || expected[index] != aura_type {
            return Err("Selected Aura effect snapshot differs from the selected spell");
        }
        let applied = AppliedAuraRef::new(
            u32::try_from(spell_id).map_err(|_| "Invalid selected Aura spell id")?,
            application.caster_guid,
            slot,
            bit,
        );
        if !auras.applied_auras.contains(&applied)
            || auras.applied_aura_amounts.get(&applied) != Some(&amount)
            || !auras
                .applied_aura_types
                .get(&aura_type)
                .is_some_and(|entries| entries.contains(&applied))
        {
            return Err("Selected Aura effect indexes are inconsistent");
        }
        snapshot_mask |= bit;
    }
    if snapshot_mask != application.effect_mask {
        return Err("Selected Aura effect snapshot does not cover its active effects");
    }
    Ok(application.duration_remaining)
}

/// Advance selected Aura durations by the admitted Map diff and remove expired
/// applications through the existing Aura removal participants.
pub fn tick_selected_aura_lifetimes<H: AuraHostLikeCpp>(
    host: &mut H,
    diff_ms: u32,
) -> Result<SelectedAuraLifetimeResult, &'static str> {
    if host.state() != wow_world_core::session::SessionState::LoggedIn {
        return Err("Selected Aura lifetime step requires a logged-in Session");
    }
    let player_guid = host
        .player_guid()
        .ok_or("Selected Aura lifetime step has no current Player GUID")?;
    let (race_id, has_auras) = host
        .core()
        .with_owned_player_like_cpp(|player| {
            let world = player.unit().world();
            (player.guid() == player_guid
                && world.object().is_in_world()
                && world.has_current_map()
                && world.object().guid() == player_guid)
                .then(|| {
                    (
                        player.race_like_cpp(),
                        !player
                            .unit()
                            .subsystems()
                            .auras
                            .runtime_applications_like_cpp()
                            .is_empty(),
                    )
                })
        })
        .flatten()
        .ok_or("Selected Aura lifetime step has no current in-world canonical Player")?;
    if !has_auras {
        return Ok(SelectedAuraLifetimeResult::default());
    }
    if race_id == 0 {
        return Err("Selected Aura lifetime step has no current race identity");
    }
    let sickness_spell = {
        let hub = host.hub_ref();
        let races = hub
            .catalogs
            .chr
            .races_store
            .as_deref()
            .ok_or("Missing ChrRaces catalog for selected Aura lifetimes")?;
        let race = races
            .get(u32::from(race_id))
            .ok_or("Current race row is missing for selected Aura lifetimes")?;
        if race.res_sickness_spell_id < 0 {
            return Err("Current race has an invalid resurrection sickness spell id");
        }
        (race.res_sickness_spell_id > 0).then_some(race.res_sickness_spell_id)
    };

    let mut spell_ids = vec![GHOST_SPELL, WISP_SPELL];
    if let Some(spell_id) = sickness_spell {
        spell_ids.push(spell_id);
    }
    spell_ids.sort_unstable();
    spell_ids.dedup();

    let prepared = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            if player.guid() != player_guid
                || player.unit().world().object().guid() != player_guid
                || !player.unit().world().object().is_in_world()
                || !player.unit().world().has_current_map()
                || player.race_like_cpp() != race_id
            {
                return Err("Canonical Player is not the current in-world Session owner");
            }
            let auras = &mut player.unit_mut().subsystems_mut().auras;
            let mut applications = Vec::new();
            for spell_id in &spell_ids {
                for slot in auras.ordered_slots_for_spell_removal(*spell_id)? {
                    let remaining = validate_application(auras, *spell_id, slot, sickness_spell)?;
                    applications.push((*spell_id, slot, remaining));
                }
            }

            let mut decremented = 0;
            let mut expired = Vec::new();
            for (spell_id, slot, remaining) in applications {
                if remaining == 0 {
                    expired.push((spell_id, slot));
                    continue;
                }
                if remaining < 0 {
                    continue;
                }
                // Aura::Update performs signed-duration minus uint32 diff,
                // then clamps only if the resulting int32 is negative.
                let next = ((remaining as u32).wrapping_sub(diff_ms) as i32).max(0);
                if next != remaining {
                    let application = auras
                        .runtime_application_mut_like_cpp(slot)
                        .ok_or("Selected Aura application disappeared during countdown")?;
                    application.duration_remaining = next;
                    decremented += 1;
                }
                if next == 0 {
                    expired.push((spell_id, slot));
                }
            }
            Ok((decremented, expired))
        })
        .ok_or("Selected Aura lifetime step has no current canonical Player owner")??;

    let (decremented, expired) = prepared;
    let mut removed = 0;
    for (_, slot) in expired {
        host.remove_aura(slot)?;
        removed += 1;
    }
    Ok(SelectedAuraLifetimeResult {
        decremented,
        expired: removed,
    })
}
