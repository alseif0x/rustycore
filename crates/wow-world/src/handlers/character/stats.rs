// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character equipment aggregation and effective stat publication.
//!
//! This module owns the application-side projection from equipped item data and
//! Player-owned item modifiers into the pure `wow-data` stat calculation. The
//! resulting snapshot is published to the canonical Player; packet formatting
//! remains in the character handler adapters.

use super::*;

#[derive(Debug, Clone, Default)]
pub(crate) struct RepresentedPlayerGearStatsLikeCpp {
    pub(super) stats: [i32; 5],
    pub(super) attack_power: i32,
    pub(super) ranged_attack_power: i32,
    pub(super) health: i32,
    pub(super) mana: i32,
    pub(super) combat_ratings: [i32; 32],
    pub(super) spell_power: i32,
    pub(super) armor: i32,
    pub(super) resistances: [i32; 7],
    pub(super) mana_regen_bonus: i32,
    pub(super) health_regen_bonus: i32,
    pub(super) spell_penetration_bonus: i32,
    pub(super) shield_block_base_mod: i32,
    pub(super) shield_block_value: u32,
}

impl WorldSession {
    pub(super) fn represented_player_gear_stats_like_cpp(
        &self,
        include_represented_item_bonuses: bool,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        let mut gear = RepresentedPlayerGearStatsLikeCpp::default();
        if let Some(item_stats_store) = self.item_stats_store() {
            for (slot, inventory_item) in self.resolved_inventory_items_like_cpp()? {
                if slot >= 19 {
                    continue;
                }
                let Some(entry) = item_stats_store.get(inventory_item.entry_id) else {
                    continue;
                };
                let base_stats = entry.base_stat_bonuses();
                for (target, amount) in gear.stats.iter_mut().zip(base_stats) {
                    *target = target.saturating_add(amount);
                }
                gear.attack_power = gear.attack_power.saturating_add(entry.attack_power_bonus());
                gear.ranged_attack_power = gear
                    .ranged_attack_power
                    .saturating_add(entry.ranged_attack_power_bonus());
                gear.health = gear.health.saturating_add(entry.health_bonus());
                gear.mana = gear.mana.saturating_add(entry.mana_bonus());
                for (target, amount) in gear
                    .combat_ratings
                    .iter_mut()
                    .zip(entry.combat_rating_bonuses())
                {
                    *target = target.saturating_add(amount);
                }
                gear.spell_power = gear.spell_power.saturating_add(entry.spell_power_bonus());
                gear.armor = gear.armor.saturating_add(entry.armor);
                for (target, amount) in gear.resistances.iter_mut().zip(entry.resistances) {
                    *target = target.saturating_add(i32::from(amount));
                }
            }
        }

        if include_represented_item_bonuses {
            let bonuses = self.resolved_item_bonus_state_like_cpp()?;
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
        }

        Some(gear)
    }

    pub(super) fn player_stat_system_projection_like_cpp(
        &self,
        race: u8,
        class: u8,
        level: u8,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) -> Option<PlayerStatSystemProjectionLikeCpp> {
        let base = *self.player_stats()?.get(race, class, level)?;
        let (attack_power_per_strength, attack_power_per_agility, ranged_attack_power_per_agility) =
            self.player_class_attack_power_coefficients_like_cpp(class)?;
        let rating_bonuses = std::array::from_fn(|index| {
            gear.combat_ratings[index] as f32
                * self.combat_rating_multiplier_like_cpp(level, index as u32)
        });
        let (can_parry, can_block) = self.canonical_player_parry_block_snapshot_like_cpp();

        Some(calculate_player_stat_system_like_cpp(
            PlayerStatSystemInputLikeCpp {
                base,
                class,
                level,
                attack_power_per_strength,
                attack_power_per_agility,
                ranged_attack_power_per_agility,
                stat_total_multipliers: self
                    .resolved_represented_total_stat_multipliers_like_cpp()?,
                stat_buff_total_multipliers: self
                    .resolved_represented_total_stat_buff_multipliers_like_cpp()?,
                gear_stats: gear.stats,
                gear_health: gear.health,
                gear_mana: gear.mana,
                gear_armor: gear.armor,
                gear_attack_power: gear.attack_power,
                gear_ranged_attack_power: gear.ranged_attack_power,
                rating_bonuses,
                can_parry,
                can_block,
            },
        ))
    }

    /// Publish one complete `UpdateAllStats` projection on the canonical
    /// Player. Packet adapters may format this value, while combat systems can
    /// consume it without reaching through Session or rebuilding item input.
    /// The snapshot is derived and is intentionally not a persistence record.
    pub(crate) fn publish_player_effective_combat_stats_like_cpp(
        &self,
        projection: PlayerStatSystemProjectionLikeCpp,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) {
        let mut resistances = gear.resistances;
        // Physical resistance is the final armor value after agility and flat
        // armor. School resistances remain the item/aura flat contributions.
        resistances[0] = projection.armor;
        let stats = PlayerEffectiveCombatStatsLikeCpp {
            stats: projection.stats,
            stat_pos_buff: projection.stat_pos_buff,
            stat_neg_buff: projection.stat_neg_buff,
            base_health: projection.create_health,
            max_health: projection.max_health,
            base_mana: projection.base_mana,
            max_mana: projection.max_mana,
            armor: projection.armor,
            resistances,
            attack_power: projection.attack_power,
            attack_power_mod_pos: projection.attack_power_mod_pos,
            ranged_attack_power: projection.ranged_attack_power,
            ranged_attack_power_mod_pos: projection.ranged_attack_power_mod_pos,
            combat_ratings: gear.combat_ratings,
            spell_power: gear.spell_power,
            mana_regen: gear.mana_regen_bonus as f32 / 5.0,
            mana_regen_combat: gear.mana_regen_bonus as f32 / 5.0,
            health_regen: gear.health_regen_bonus,
            spell_penetration: gear.spell_penetration_bonus,
            shield_block: i32::try_from(gear.shield_block_value)
                .unwrap_or(i32::MAX)
                .saturating_add(gear.shield_block_base_mod),
            block_pct: projection.block_pct,
            dodge_pct: projection.dodge_pct,
            dodge_from_attr: projection.dodge_from_attr,
            parry_pct: projection.parry_pct,
            parry_from_attr: projection.parry_from_attr,
            crit_pct: projection.crit_pct,
            ranged_crit_pct: projection.ranged_crit_pct,
            offhand_crit_pct: projection.offhand_crit_pct,
            spell_crit_pct: projection.spell_crit_pct,
            ..PlayerEffectiveCombatStatsLikeCpp::default()
        };
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.replace_effective_combat_stats_like_cpp(stats);
        });
    }

    pub(super) fn publish_effective_stats_like_cpp(
        &self,
        include_represented_item_bonuses: bool,
        projection: PlayerStatSystemProjectionLikeCpp,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) {
        if include_represented_item_bonuses {
            self.publish_player_effective_combat_stats_like_cpp(projection, gear);
        } else {
            let race = self.player_race_like_cpp();
            let class = self.player_class_like_cpp();
            let level = self.player_level_like_cpp();
            if let Some(full_gear) = self.represented_player_gear_stats_like_cpp(true)
                && let Some(full_projection) =
                    self.player_stat_system_projection_like_cpp(race, class, level, &full_gear)
            {
                self.publish_player_effective_combat_stats_like_cpp(full_projection, &full_gear);
            }
        }
    }
}
