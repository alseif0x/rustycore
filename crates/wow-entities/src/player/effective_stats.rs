// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Canonical, derived combat statistics owned by a Player.
//!
//! The values are rebuilt by the world stat system after level, aura or
//! equipment changes. They deliberately live on `Player`, rather than on a
//! session packet adapter, so combat consumers and update-field publication
//! read the same snapshot. Current health and power remain Unit-owned mutable
//! vitals and are therefore not duplicated here.

use super::Player;

/// C++ `Player::UpdateAllStats` result that is safe for non-network consumers.
///
/// This is a derived runtime snapshot, not a persistence record. The world
/// crate supplies the data/catalog inputs and replaces it through the named
/// Player operation below. Fields not yet backed by the 3.4.3 aura/runtime
/// catalogs retain their explicit zero/default value until that producer is
/// ported.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerEffectiveCombatStatsLikeCpp {
    pub stats: [i32; 5],
    pub stat_pos_buff: [i32; 5],
    pub stat_neg_buff: [i32; 5],
    pub base_health: i32,
    pub max_health: i64,
    pub base_mana: i32,
    pub max_mana: i64,
    pub armor: i32,
    pub resistances: [i32; 7],
    pub attack_power: i32,
    pub attack_power_mod_pos: i32,
    pub attack_power_mod_neg: i32,
    /// C++ `Unit::GetTotalAttackPowerValue(BASE_ATTACK)` multiplier.
    ///
    /// The current stat projection publishes the represented value (zero when
    /// no aura-backed multiplier is available); keeping it on the Player
    /// snapshot prevents combat consumers from reconstructing AP from an item
    /// mirror when the aura/runtime producer is added.
    pub attack_power_multiplier: f32,
    pub ranged_attack_power: i32,
    pub ranged_attack_power_mod_pos: i32,
    pub ranged_attack_power_mod_neg: i32,
    /// C++ `Unit::GetTotalAttackPowerValue(RANGED_ATTACK)` multiplier.
    pub ranged_attack_power_multiplier: f32,
    pub min_damage: f32,
    pub max_damage: f32,
    /// Final C++ `UnitData` damage ranges, including the off-hand range that
    /// is not present in the current partial packet adapter.
    pub weapon_damage: [[f32; 2]; 3],
    pub min_ranged_damage: f32,
    pub max_ranged_damage: f32,
    pub combat_ratings: [i32; 32],
    pub spell_power: i32,
    pub mana_regen: f32,
    pub mana_regen_combat: f32,
    pub mana_regen_mp5: f32,
    pub health_regen: i32,
    pub spell_penetration: i32,
    pub mainhand_expertise: f32,
    pub offhand_expertise: f32,
    pub ranged_expertise: f32,
    pub combat_rating_expertise: f32,
    pub block_pct: f32,
    pub dodge_pct: f32,
    pub dodge_from_attr: f32,
    pub parry_pct: f32,
    pub parry_from_attr: f32,
    pub crit_pct: f32,
    pub ranged_crit_pct: f32,
    pub offhand_crit_pct: f32,
    pub spell_crit_pct: [f32; 7],
    pub shield_block: i32,
    pub shield_block_crit_pct: f32,
}

impl Default for PlayerEffectiveCombatStatsLikeCpp {
    fn default() -> Self {
        Self {
            stats: [0; 5],
            stat_pos_buff: [0; 5],
            stat_neg_buff: [0; 5],
            base_health: 0,
            max_health: 0,
            base_mana: 0,
            max_mana: 0,
            armor: 0,
            resistances: [0; 7],
            attack_power: 0,
            attack_power_mod_pos: 0,
            attack_power_mod_neg: 0,
            attack_power_multiplier: 0.0,
            ranged_attack_power: 0,
            ranged_attack_power_mod_pos: 0,
            ranged_attack_power_mod_neg: 0,
            ranged_attack_power_multiplier: 0.0,
            min_damage: 0.0,
            max_damage: 0.0,
            weapon_damage: [[0.0; 2]; 3],
            min_ranged_damage: 0.0,
            max_ranged_damage: 0.0,
            combat_ratings: [0; 32],
            spell_power: 0,
            mana_regen: 0.0,
            mana_regen_combat: 0.0,
            mana_regen_mp5: 0.0,
            health_regen: 0,
            spell_penetration: 0,
            mainhand_expertise: 0.0,
            offhand_expertise: 0.0,
            ranged_expertise: 0.0,
            combat_rating_expertise: 0.0,
            block_pct: 0.0,
            dodge_pct: 0.0,
            dodge_from_attr: 0.0,
            parry_pct: 0.0,
            parry_from_attr: 0.0,
            crit_pct: 0.0,
            ranged_crit_pct: 0.0,
            offhand_crit_pct: 0.0,
            spell_crit_pct: [0.0; 7],
            shield_block: 0,
            shield_block_crit_pct: 0.0,
        }
    }
}

impl Player {
    /// Read the last complete stat snapshot published for this Player.
    #[must_use]
    pub const fn effective_combat_stats_like_cpp(&self) -> &PlayerEffectiveCombatStatsLikeCpp {
        &self.effective_combat_stats
    }

    /// Return the same non-negative total used by C++
    /// `Unit::GetTotalAttackPowerValue(BASE_ATTACK)`.
    #[must_use]
    pub fn total_attack_power_like_cpp(&self) -> f32 {
        let stats = self.effective_combat_stats_like_cpp();
        let attack_power = stats.attack_power as f32
            + stats.attack_power_mod_pos as f32
            + stats.attack_power_mod_neg as f32;
        if attack_power < 0.0 {
            0.0
        } else {
            attack_power * (1.0 + stats.attack_power_multiplier)
        }
    }

    /// Return the same non-negative total used by C++
    /// `Unit::GetTotalAttackPowerValue(RANGED_ATTACK)`.
    #[must_use]
    pub fn total_ranged_attack_power_like_cpp(&self) -> f32 {
        let stats = self.effective_combat_stats_like_cpp();
        let attack_power = stats.ranged_attack_power as f32
            + stats.ranged_attack_power_mod_pos as f32
            + stats.ranged_attack_power_mod_neg as f32;
        if attack_power < 0.0 {
            0.0
        } else {
            attack_power * (1.0 + stats.ranged_attack_power_multiplier)
        }
    }

    /// Read a final weapon range from the Player-owned snapshot. Before the
    /// first complete stat publication the snapshot is zeroed, so retain the
    /// Unit's C++ base/default range for that transitional state.
    #[must_use]
    pub fn weapon_damage_like_cpp(&self, attack: wow_constants::WeaponAttackType) -> [f32; 2] {
        let range = self.effective_combat_stats_like_cpp().weapon_damage[attack as usize];
        if range[0] > 0.0 && range[1] > 0.0 {
            range
        } else {
            self.unit().weapon_damage(attack)
        }
    }

    /// Replace the complete derived snapshot after one coherent recalculation.
    pub fn replace_effective_combat_stats_like_cpp(
        &mut self,
        stats: PlayerEffectiveCombatStatsLikeCpp,
    ) {
        self.effective_combat_stats = stats;
    }

    /// Discard derived stats when the Player lifecycle starts over.
    pub fn clear_effective_combat_stats_like_cpp(&mut self) {
        self.effective_combat_stats = PlayerEffectiveCombatStatsLikeCpp::default();
    }
}
