// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure projection of the existing Player-owned item accumulator and a
//! borrowed canonical Player stat view.

mod equipment;
mod projection;
mod spell_bonus;

use wow_world_core::session::state::hub_support::RepresentedPlayerGearStatsLikeCpp;
use wow_world_core::session::{
    PlayerStatCatalogs, PlayerStatComputation, PlayerStatComputationRequest, PlayerStatFacts,
};

/// Calculate the gear, `UpdateAllStats` projection, and derived combat snapshot
/// from one borrowed Player view and selected immutable catalogs.
pub fn plan_player_stats(
    facts: &PlayerStatFacts<'_>,
    catalogs: &PlayerStatCatalogs<'_>,
    request: &PlayerStatComputationRequest<'_>,
) -> Option<PlayerStatComputation> {
    let (race, class, level) = match request.projection_identity() {
        Some(identity) => identity,
        None => {
            let identity = (facts.race(), facts.class(), facts.level());
            if identity.0 == 0 || identity.1 == 0 || identity.2 == 0 {
                return None;
            }
            identity
        }
    };

    let gear = request
        .gear_override()
        .cloned()
        .unwrap_or_else(|| gear_from_bonuses(facts.item_bonuses()));
    let projection = projection::project(facts, catalogs, &gear, race, class, level)?;
    let effective = equipment::effective_stats(facts, catalogs, &gear, projection, level);
    Some(PlayerStatComputation {
        gear,
        projection,
        effective,
    })
}

pub(super) fn gear_from_bonuses(
    bonuses: &wow_entities::PlayerItemBonusStateLikeCpp,
) -> RepresentedPlayerGearStatsLikeCpp {
    let mut gear = RepresentedPlayerGearStatsLikeCpp::default();
    for (target, amount) in gear.stats.iter_mut().zip(bonuses.stats_base) {
        *target = target.saturating_add(amount);
    }
    gear.attack_power = gear.attack_power.saturating_add(bonuses.attack_power_total);
    gear.ranged_attack_power = gear
        .ranged_attack_power
        .saturating_add(bonuses.ranged_attack_power_total);
    gear.health = gear.health.saturating_add(bonuses.health_base);
    gear.mana = gear.mana.saturating_add(bonuses.mana_base);
    for (target, amount) in gear.combat_ratings.iter_mut().zip(bonuses.combat_ratings) {
        *target = target.saturating_add(amount);
    }
    gear.spell_power = gear.spell_power.saturating_add(bonuses.spell_power_bonus);
    gear.armor = gear
        .armor
        .saturating_add(bonuses.armor_base)
        .saturating_add(bonuses.armor_total)
        .saturating_add(bonuses.resistances_base[0]);
    for (target, amount) in gear.resistances.iter_mut().zip(bonuses.resistances_base) {
        *target = target.saturating_add(amount);
    }
    gear.mana_regen_bonus = bonuses.mana_regen_bonus;
    gear.health_regen_bonus = bonuses.health_regen_bonus;
    gear.spell_penetration_bonus = bonuses.spell_penetration_bonus;
    gear.shield_block_base_mod = bonuses.shield_block_base_mod;
    gear.shield_block_value = bonuses.shield_block_value;
    gear.weapon_damage = bonuses.weapon_damage;
    gear.base_attack_time = bonuses.base_attack_time;
    gear
}
