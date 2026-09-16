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
    /// C++ `UnitData` weapon ranges installed by `_ApplyWeaponDamage`.
    pub(super) weapon_damage: [[f32; 2]; 3],
    pub(super) base_attack_time: [u32; 3],
}

impl WorldSession {
    pub(super) fn mana_regen_from_stats_like_cpp(
        &self,
        level: u8,
        class: u8,
        stats: [i32; 5],
    ) -> f32 {
        // C++ `Player::OCTRegenMPPerSpirit` returns Spirit multiplied by the
        // level/class row from `RegenMPPerSpt.txt`; `UpdateManaRegen` then
        // multiplies that value by sqrt(Intellect). Aura percentages are a
        // separate producer and are intentionally not folded into this
        // equipment projection.
        (stats[3].max(0) as f32).sqrt()
            * stats[4] as f32
            * self.mana_regen_ratio_like_cpp(level, class)
    }

    pub(super) fn mana_regen_aura_multiplier_like_cpp(&self) -> f32 {
        let mana = PowerType::Mana as i32;
        self.resolved_total_aura_multiplier_by_spell_aura_type_and_misc_value_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN_PERCENT,
            mana,
        )
        .unwrap_or(1.0)
            * self
                .resolved_total_aura_multiplier_by_spell_aura_type_and_misc_value_like_cpp(
                    wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_PCT,
                    mana,
                )
                .unwrap_or(1.0)
    }

    pub(super) fn mana_regen_mp5_from_auras_like_cpp(&self, stats: [i32; 5]) -> f32 {
        let mana = PowerType::Mana as i32;
        let flat = self
            .resolved_total_aura_modifier_by_spell_aura_type_and_misc_value_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_POWER_REGEN,
                mana,
            )
            .unwrap_or(0) as f32
            / 5.0;
        let from_stat = self
            .resolved_aura_effects_by_spell_aura_type_like_cpp(
                wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_FROM_STAT,
            )
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(stat_index, amount)| {
                usize::try_from(stat_index)
                    .ok()
                    .and_then(|index| stats.get(index).copied())
                    .map(|stat| stat as f32 * amount as f32 / 500.0)
            })
            .sum::<f32>();
        flat + from_stat
    }

    pub(super) fn mana_regen_interrupt_modifier_like_cpp(&self) -> f32 {
        self.resolved_aura_effects_by_spell_aura_type_like_cpp(
            wow_data::spell::aura_types::SPELL_AURA_MOD_MANA_REGEN_INTERRUPT,
        )
        .unwrap_or_default()
        .into_iter()
        .map(|(_, amount)| amount)
        .sum::<i32>()
        .min(100) as f32
            / 100.0
    }

    pub(super) fn represented_player_gear_stats_like_cpp(
        &self,
        _include_represented_item_bonuses: bool,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        let mut gear = RepresentedPlayerGearStatsLikeCpp::default();
        // C++ keeps the result of `_ApplyItemBonuses` on Player and uses that
        // accumulator for every subsequent stat calculation. Reading the
        // inventory here as well would count an item once through its sparse
        // row and again through the canonical Player modifier state after an
        // equip/swap. Login seeds the same accumulator before this projection,
        // so this is the single contribution path for every lifecycle.
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
        gear.weapon_damage = bonuses.weapon_damage;
        gear.base_attack_time = bonuses.base_attack_time;

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
        level: u8,
        projection: PlayerStatSystemProjectionLikeCpp,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) {
        let mut resistances = gear.resistances;
        // Physical resistance is the final armor value after agility and flat
        // armor. School resistances remain the item/aura flat contributions.
        resistances[0] = projection.armor;
        let weapon_damage = wow_data::player::effective_weapon_damage_ranges_like_cpp(
            projection,
            gear.weapon_damage,
            gear.base_attack_time,
        );
        // C++ `Player::UpdateExpertise` (`StatSystem.cpp:759-786`) truncates the
        // combat-rating bonus to `int32`, adds the `SPELL_AURA_MOD_EXPERTISE`
        // sum whose spell is fit for that attack's weapon, clamps at zero and
        // writes `MainhandExpertise`/`OffhandExpertise` per attack.
        // `ActivePlayerData::RangedExpertise`/`CombatRatingExpertise` are never
        // written by C++ (`UpdateExpertise` returns early for `RANGED_ATTACK`)
        // and stay at their zero create value.
        let rating_expertise = (gear.combat_ratings[23] as f32
            * self.combat_rating_multiplier_like_cpp(level, 23))
        .trunc();
        let mainhand_expertise = (rating_expertise
            + self.represented_expertise_aura_modifier_like_cpp(
                wow_constants::WeaponAttackType::BaseAttack,
            ) as f32)
            .max(0.0);
        let offhand_expertise = (rating_expertise
            + self.represented_expertise_aura_modifier_like_cpp(
                wow_constants::WeaponAttackType::OffAttack,
            ) as f32)
            .max(0.0);
        let mana_regen_mp5 = gear.mana_regen_bonus as f32 / 5.0
            + self.mana_regen_mp5_from_auras_like_cpp(projection.stats);
        let mana_regen_from_spirit = self.mana_regen_from_stats_like_cpp(
            level,
            self.player_class_like_cpp(),
            projection.stats,
        ) * self.mana_regen_aura_multiplier_like_cpp();
        let mana_regen_combat =
            mana_regen_mp5 + mana_regen_from_spirit * self.mana_regen_interrupt_modifier_like_cpp();
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
            attack_power_multiplier: 0.0,
            ranged_attack_power: projection.ranged_attack_power,
            ranged_attack_power_mod_pos: projection.ranged_attack_power_mod_pos,
            ranged_attack_power_multiplier: 0.0,
            min_damage: weapon_damage[0][0],
            max_damage: weapon_damage[0][1],
            weapon_damage,
            min_ranged_damage: weapon_damage[2][0],
            max_ranged_damage: weapon_damage[2][1],
            combat_ratings: gear.combat_ratings,
            spell_power: gear.spell_power,
            mana_regen: mana_regen_from_spirit + mana_regen_mp5,
            mana_regen_combat,
            health_regen: gear.health_regen_bonus,
            spell_penetration: gear.spell_penetration_bonus,
            mainhand_expertise,
            offhand_expertise,
            ranged_expertise: 0.0,
            combat_rating_expertise: 0.0,
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
        level: u8,
        _include_represented_item_bonuses: bool,
        projection: PlayerStatSystemProjectionLikeCpp,
        gear: &RepresentedPlayerGearStatsLikeCpp,
    ) {
        // The canonical Player accumulator is always the source for this
        // projection. The boolean remains at the adapter boundary for
        // compatibility with callers that already name the C++ option, but a
        // second inventory-derived path is deliberately impossible here.
        self.publish_player_effective_combat_stats_like_cpp(level, projection, gear);
    }
}
