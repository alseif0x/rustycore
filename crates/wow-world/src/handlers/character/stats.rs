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
pub(crate) use crate::session::hub_support::RepresentedPlayerGearStatsLikeCpp;
use crate::session::hub_support::{
    SPELL_SCHOOL_MASK_ALL_LIKE_CPP, SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
    SPELL_SCHOOL_MASK_SPELL_LIKE_CPP,
};

impl WorldSession {
    pub(super) fn represented_player_gear_stats_like_cpp(
        &self,
        _include_represented_item_bonuses: bool,
    ) -> Option<RepresentedPlayerGearStatsLikeCpp> {
        let (state, hub) = crate::session::split_inventory_ref(self);
        state.represented_player_gear_stats_like_cpp(hub, _include_represented_item_bonuses)
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
            self.catalogs
                .player_class_attack_power_coefficients_like_cpp(class)?;
        let rating_bonuses = std::array::from_fn(|index| {
            gear.combat_ratings[index] as f32
                * self
                    .catalogs
                    .combat_rating_multiplier_like_cpp(level, index as u32)
        });
        let (can_parry, can_block) = self.core.canonical_player_parry_block_snapshot_like_cpp();
        let spell_bonus = {
            let (s, h) = crate::session::split_inventory_ref(self);
            s.represented_spell_bonus_like_cpp(h, gear)
        };

        Some(
            self.apply_stats_limits_like_cpp(calculate_player_stat_system_like_cpp(
                PlayerStatSystemInputLikeCpp {
                    base,
                    class,
                    level,
                    attack_power_per_strength,
                    attack_power_per_agility,
                    ranged_attack_power_per_agility,
                    stat_total_multipliers: crate::session::hub_ref(self)
                        .resolved_represented_total_stat_multipliers_like_cpp()?,
                    stat_buff_total_multipliers: crate::session::hub_ref(self)
                        .resolved_represented_total_stat_buff_multipliers_like_cpp()?,
                    gear_stats: gear.stats,
                    gear_health: gear.health,
                    gear_mana: gear.mana,
                    gear_armor: gear.armor,
                    armor_base_pct: crate::session::hub_ref(self)
                        .represented_resistance_aura_multiplier_like_cpp(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_BASE_RESISTANCE_PCT,
                            SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
                        ),
                    armor_flat_aura: crate::session::hub_ref(self)
                        .represented_resistance_aura_flat_like_cpp(
                            SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
                        ) as i32,
                    armor_of_stat_percent: crate::session::hub_ref(self)
                        .represented_armor_of_stat_percent_like_cpp(),
                    armor_total_pct: crate::session::hub_ref(self)
                        .represented_resistance_aura_multiplier_like_cpp(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE_PCT,
                            SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
                        ),
                    armor_bonus_pct: crate::session::hub_ref(self)
                        .represented_total_aura_multiplier_like_cpp(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_BONUS_ARMOR_PCT,
                        ),
                    spell_dodge_pct: crate::session::hub_ref(self)
                        .represented_total_aura_modifier_like_cpp(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_DODGE_PERCENT,
                        ),
                    spell_parry_pct: crate::session::hub_ref(self)
                        .represented_total_aura_modifier_like_cpp(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_PARRY_PERCENT,
                        ),
                    spell_block_pct: crate::session::hub_ref(self)
                        .represented_total_aura_modifier_like_cpp(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_BLOCK_PERCENT,
                        ),
                    crit_mainhand_aura_pct: self.represented_weapon_crit_aura_modifier_like_cpp(
                        wow_constants::WeaponAttackType::BaseAttack,
                    ),
                    crit_offhand_aura_pct: self.represented_weapon_crit_aura_modifier_like_cpp(
                        wow_constants::WeaponAttackType::OffAttack,
                    ),
                    crit_ranged_aura_pct: self.represented_weapon_crit_aura_modifier_like_cpp(
                        wow_constants::WeaponAttackType::RangedAttack,
                    ),
                    spell_crit_aura_pct: crate::session::hub_ref(self)
                        .represented_total_aura_modifier_like_cpp(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_CRIT_CHANCE,
                        )
                        + crate::session::hub_ref(self).represented_total_aura_modifier_like_cpp(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_PCT,
                        ),
                    gear_attack_power: gear.attack_power,
                    gear_ranged_attack_power: gear.ranged_attack_power,
                    attack_power_flat_aura: crate::session::hub_ref(self)
                        .represented_attack_power_flat_aura_like_cpp(),
                    attack_power_total_pct: crate::session::hub_ref(self)
                        .represented_total_aura_multiplier_like_cpp(
                            wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACK_POWER_PCT,
                        ),
                    ranged_attack_power_flat_aura: crate::session::hub_ref(self)
                        .represented_ranged_attack_power_flat_aura_like_cpp(class),
                    ranged_attack_power_total_pct: crate::session::hub_ref(self)
                        .represented_ranged_attack_power_total_pct_like_cpp(class),
                    attack_power_override_by_spell_power_pct: crate::session::hub_ref(self)
                        .represented_override_attack_power_by_spell_power_pct_like_cpp(),
                    spell_bonus,
                    rating_bonuses,
                    can_parry,
                    can_block,
                },
            )),
        )
    }

    pub(crate) fn apply_represented_shapeshift_base_attack_time_like_cpp(&mut self) -> bool {
        let (state, mut hub) = crate::session::split_inventory_mut(self);
        state.apply_represented_shapeshift_base_attack_time_like_cpp(&mut hub)
    }

    /// C++ `Player::InitDataForForm` plus the `UpdateDamagePhysical` refresh at
    /// a shapeshift aura apply/removal: reinstall the base attack times and
    /// republish the weapon ranges the form rescales.
    pub(crate) fn sync_represented_shapeshift_form_like_cpp(
        &mut self,
        mutation: crate::session::RepresentedShapeshiftMutationLikeCpp,
    ) {
        if !self.apply_represented_shapeshift_base_attack_time_like_cpp() {
            return;
        }
        // C++ `AuraEffect::HandleShapeshiftBoosts`
        // (`SpellAuraEffects.cpp:1394-1464`) owns the form's bonus spells and
        // the stance-gated self-aura sweep on both directions.
        match mutation {
            crate::session::RepresentedShapeshiftMutationLikeCpp::Applied { form_id } => {
                self.apply_represented_shapeshift_boosts_like_cpp(form_id);
            }
            crate::session::RepresentedShapeshiftMutationLikeCpp::Removed {
                removed_form,
                new_form,
            } => {
                self.remove_represented_shapeshift_boosts_like_cpp(removed_form, new_form);
            }
        }
        // C++ `Player::InitDataForForm` (`Player.cpp:22090`) updates the displayed
        // power before refreshing the equipped items' form-gated spells and
        // item-set auras, and `UpdateAttackPowerAndDamage` republishes after.
        self.sync_represented_display_power_like_cpp();
        self.refresh_represented_item_effects_at_form_change_like_cpp();
        let _ = self.send_stat_update();
    }

    /// C++ `CONFIG_STATS_LIMITS_*` (`World.cpp:1664-1668`): cap the block,
    /// dodge, parry and crit percentages at the point
    /// `Player::UpdateBlockPercentage`/`UpdateDodgePercentage`/
    /// `UpdateParryPercentage`/`UpdateCritPercentage` publish them. Applying it
    /// to the single projection producer keeps the login create snapshot and the
    /// canonical effective-stats snapshot identical.
    fn apply_stats_limits_like_cpp(
        &self,
        mut projection: PlayerStatSystemProjectionLikeCpp,
    ) -> PlayerStatSystemProjectionLikeCpp {
        let limits = self.stats_limits_like_cpp();
        projection.block_pct = limits.clamp_block_like_cpp(projection.block_pct);
        projection.dodge_pct = limits.clamp_dodge_like_cpp(projection.dodge_pct);
        projection.parry_pct = limits.clamp_parry_like_cpp(projection.parry_pct);
        projection.crit_pct = limits.clamp_crit_like_cpp(projection.crit_pct);
        projection.ranged_crit_pct = limits.clamp_crit_like_cpp(projection.ranged_crit_pct);
        projection.offhand_crit_pct = limits.clamp_crit_like_cpp(projection.offhand_crit_pct);
        projection
    }
}




