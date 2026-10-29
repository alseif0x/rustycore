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
use wow_constants::PowerType;

const UNIT_DAMAGE_PARENT_BIT: usize = 32;
const UNIT_DAMAGE_FIRST_BIT: usize = 52;
const UNIT_MODS_PARENT_BIT: usize = 64;
const UNIT_BASE_HEALTH_BIT: usize = 76;
const UNIT_ATTACK_POWER_FIRST_BIT: usize = 81;
const UNIT_MIN_RANGED_DAMAGE_BIT: usize = 91;
const UNIT_POWER_PARENT_BIT: usize = 116;
const UNIT_POWER_REGEN_FLAT_FIRST_BIT: usize = 117;
const UNIT_POWER_REGEN_INTERRUPTED_FIRST_BIT: usize = 127;
const UNIT_STATS_PARENT_BIT: usize = 174;
const UNIT_STATS_FIRST_BIT: usize = 175;
const UNIT_STAT_POS_BUFF_FIRST_BIT: usize = 180;
const UNIT_STAT_NEG_BUFF_FIRST_BIT: usize = 185;
const UNIT_RESISTANCES_PARENT_BIT: usize = 190;
const UNIT_RESISTANCES_FIRST_BIT: usize = 191;

const ACTIVE_STATS_PARENT_BIT: usize = 38;
const ACTIVE_MAINHAND_EXPERTISE_BIT: usize = 36;
const ACTIVE_OFFHAND_EXPERTISE_BIT: usize = 37;
const ACTIVE_RANGED_EXPERTISE_BIT: usize = 39;
const ACTIVE_COMBAT_RATING_EXPERTISE_BIT: usize = 40;
const ACTIVE_BLOCK_BIT: usize = 41;
const ACTIVE_DODGE_BIT: usize = 42;
const ACTIVE_DODGE_FROM_ATTRIBUTE_BIT: usize = 43;
const ACTIVE_PARRY_BIT: usize = 44;
const ACTIVE_PARRY_FROM_ATTRIBUTE_BIT: usize = 45;
const ACTIVE_CRIT_BIT: usize = 46;
const ACTIVE_RANGED_CRIT_BIT: usize = 47;
const ACTIVE_OFFHAND_CRIT_BIT: usize = 48;
const ACTIVE_SHIELD_BLOCK_BIT: usize = 49;
const ACTIVE_SHIELD_BLOCK_CRIT_BIT: usize = 50;
const ACTIVE_VERSATILITY_BONUS_BIT: usize = 56;
const ACTIVE_HEALING_DONE_POS_BIT: usize = 59;
const ACTIVE_HEALING_DONE_PERCENT_BIT: usize = 61;
const ACTIVE_OVERRIDE_SPELL_POWER_BY_AP_BIT: usize = 65;
const ACTIVE_OVERRIDE_AP_BY_SPELL_POWER_BIT: usize = 66;
const ACTIVE_TARGET_RESISTANCE_BIT: usize = 67;
const ACTIVE_TARGET_PHYSICAL_RESISTANCE_BIT: usize = 68;
const ACTIVE_SPELL_MODIFIERS_PARENT_BIT: usize = 269;
const ACTIVE_SPELL_CRIT_FIRST_BIT: usize = 270;
const ACTIVE_MOD_DAMAGE_DONE_POS_FIRST_BIT: usize = 277;
const ACTIVE_MOD_DAMAGE_DONE_NEG_FIRST_BIT: usize = 284;
const ACTIVE_MOD_DAMAGE_DONE_PERCENT_FIRST_BIT: usize = 291;
const ACTIVE_COMBAT_RATINGS_PARENT_BIT: usize = 574;
const ACTIVE_COMBAT_RATINGS_FIRST_BIT: usize = 575;

/// Immutable derived-stat values captured with a Player update mask.
///
/// The update-field masks remain the publication authority. This value only
/// supplies fields that the canonical Unit/ActivePlayer value structs do not
/// store, together with the mana slot selected at capture time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerDerivedStatValuesUpdate {
    pub effective_combat_stats: PlayerEffectiveCombatStatsLikeCpp,
    pub mana_power_index: Option<usize>,
}

/// C++ `Player::UpdateAllStats` result that is safe for non-network consumers.
///
/// This is a derived runtime snapshot, not a persistence record. The world
/// crate supplies the data/catalog inputs and replaces it through the named
/// Player operation below. Fields not yet backed by the 3.4.3 aura/runtime
/// catalogs retain their explicit zero/default value until that producer is
/// ported. `Player::UpdateExpertise`'s rating and item-fit aura producers are
/// both folded by the world stat projection, so `mainhand_expertise` and
/// `offhand_expertise` carry the per-attack result while
/// `ranged_expertise`/`combat_rating_expertise` stay zero like C++.
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
    /// C++ `ActivePlayerData::ModDamageDonePos[7]` published by
    /// `Player::UpdateSpellDamageAndHealingBonus` (`StatSystem.cpp:171-197`);
    /// index 0 stays unwritten.
    pub mod_damage_done_pos: [i32; 7],
    /// C++ `ActivePlayerData::ModDamageDoneNeg[7]`.
    pub mod_damage_done_neg: [i32; 7],
    /// C++ `ActivePlayerData::ModHealingDonePos`.
    pub mod_healing_done_pos: i32,
    /// C++ `ActivePlayerData::ModDamageDonePercent[7]`
    /// (`SpellAuraEffects.cpp:4525-4548`).
    pub mod_damage_done_percent: [f32; 7],
    /// C++ `ActivePlayerData::ModHealingDonePercent`
    /// (`StatSystem.cpp:588-599`).
    pub mod_healing_done_percent: f32,
    /// C++ `Player::GetRatingBonusValue(CR_ARMOR_PENETRATION)`: the live
    /// percentage `Unit::CalcArmorReducedDamage` reads for a player attacker.
    pub armor_penetration_pct: f32,
    /// C++ `Player::m_modMeleeHitChance`: `7.5 + GetRatingBonusValue(
    /// CR_HIT_MELEE)`, the skill-based part of `MeleeSpellMissChance`.
    pub melee_hit_chance_pct: f32,
    /// C++ `ActivePlayerData::ModTargetResistance`.
    pub mod_target_resistance: i32,
    /// C++ `ActivePlayerData::ModTargetPhysicalResistance`.
    pub mod_target_physical_resistance: i32,
    /// C++ `UNIT_MOD_DAMAGE_*` `TOTAL_PCT` per attack
    /// (`Unit::UpdateDamagePctDoneMods`).
    pub weapon_damage_pct: [f32; 3],
    /// C++ `UNIT_MOD_DAMAGE_*` `TOTAL_VALUE` per attack
    /// (`Player::UpdateDamageDoneMods`).
    pub weapon_damage_flat: [f32; 3],
    /// C++ `ActivePlayerData::VersatilityBonus`.
    pub versatility_bonus: f32,
    /// C++ `ActivePlayerData::OverrideSpellPowerByAPPercent`.
    pub override_spell_power_by_ap_percent: f32,
    /// C++ `ActivePlayerData::OverrideAPBySpellPowerPercent`.
    pub override_ap_by_spell_power_percent: f32,
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
            mod_damage_done_pos: [0; 7],
            mod_damage_done_neg: [0; 7],
            mod_healing_done_pos: 0,
            mod_damage_done_percent: [1.0; 7],
            mod_healing_done_percent: 1.0,
            armor_penetration_pct: 0.0,
            melee_hit_chance_pct: 0.0,
            mod_target_resistance: 0,
            mod_target_physical_resistance: 0,
            weapon_damage_pct: [1.0, 0.5, 1.0],
            weapon_damage_flat: [0.0; 3],
            versatility_bonus: 0.0,
            override_spell_power_by_ap_percent: 0.0,
            override_ap_by_spell_power_percent: 0.0,
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

    /// Replace derived stats and mark only their existing update-field leaves.
    ///
    /// Vitals and BaseMana remain stored and published by Unit. This opt-in
    /// operation provides update-field deltas for the derived values backed by
    /// the current snapshot without changing the legacy snapshot-only setter.
    pub fn replace_effective_combat_stats_for_values(
        &mut self,
        stats: PlayerEffectiveCombatStatsLikeCpp,
    ) {
        let previous = self.effective_combat_stats;
        let mut active_changed = false;

        macro_rules! active_array {
            ($old:expr, $new:expr, $parent:expr, $first_bit:expr) => {
                for (index, (old, new)) in $old.iter().zip($new.iter()).enumerate() {
                    if old != new {
                        self.mark_active_player_data_array($parent, $first_bit, index);
                        active_changed = true;
                    }
                }
            };
        }

        if previous.min_damage != stats.min_damage {
            self.unit
                .mark_unit_data_nested(UNIT_DAMAGE_PARENT_BIT, UNIT_DAMAGE_FIRST_BIT);
        }
        if previous.max_damage != stats.max_damage {
            self.unit
                .mark_unit_data_nested(UNIT_DAMAGE_PARENT_BIT, UNIT_DAMAGE_FIRST_BIT + 1);
        }
        if previous.base_health != stats.base_health {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_BASE_HEALTH_BIT);
        }
        if previous.attack_power != stats.attack_power {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_ATTACK_POWER_FIRST_BIT);
        }
        if previous.attack_power_mod_pos != stats.attack_power_mod_pos {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_ATTACK_POWER_FIRST_BIT + 1);
        }
        if previous.attack_power_mod_neg != stats.attack_power_mod_neg {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_ATTACK_POWER_FIRST_BIT + 2);
        }
        if previous.attack_power_multiplier != stats.attack_power_multiplier {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_ATTACK_POWER_FIRST_BIT + 3);
        }
        if previous.ranged_attack_power != stats.ranged_attack_power {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_ATTACK_POWER_FIRST_BIT + 4);
        }
        if previous.ranged_attack_power_mod_pos != stats.ranged_attack_power_mod_pos {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_ATTACK_POWER_FIRST_BIT + 5);
        }
        if previous.ranged_attack_power_mod_neg != stats.ranged_attack_power_mod_neg {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_ATTACK_POWER_FIRST_BIT + 6);
        }
        if previous.ranged_attack_power_multiplier != stats.ranged_attack_power_multiplier {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_ATTACK_POWER_FIRST_BIT + 7);
        }
        if previous.min_ranged_damage != stats.min_ranged_damage {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_MIN_RANGED_DAMAGE_BIT);
        }
        if previous.max_ranged_damage != stats.max_ranged_damage {
            self.unit
                .mark_unit_data_nested(UNIT_MODS_PARENT_BIT, UNIT_MIN_RANGED_DAMAGE_BIT + 1);
        }

        for (index, (old, new)) in previous.weapon_damage[1]
            .iter()
            .zip(stats.weapon_damage[1].iter())
            .enumerate()
        {
            if old != new {
                self.unit.mark_unit_data_nested(
                    UNIT_DAMAGE_PARENT_BIT,
                    UNIT_DAMAGE_FIRST_BIT + 2 + index,
                );
            }
        }

        for (old, new, first_bit) in [
            (&previous.stats[..], &stats.stats[..], UNIT_STATS_FIRST_BIT),
            (
                &previous.stat_pos_buff[..],
                &stats.stat_pos_buff[..],
                UNIT_STAT_POS_BUFF_FIRST_BIT,
            ),
            (
                &previous.stat_neg_buff[..],
                &stats.stat_neg_buff[..],
                UNIT_STAT_NEG_BUFF_FIRST_BIT,
            ),
        ] {
            for (index, (old, new)) in old.iter().zip(new.iter()).enumerate() {
                if old != new {
                    self.unit
                        .mark_unit_data_array(UNIT_STATS_PARENT_BIT, first_bit, index);
                }
            }
        }

        for (index, (old, new)) in previous
            .resistances
            .iter()
            .zip(stats.resistances.iter())
            .enumerate()
        {
            if old != new {
                self.unit.mark_unit_data_array(
                    UNIT_RESISTANCES_PARENT_BIT,
                    UNIT_RESISTANCES_FIRST_BIT,
                    index,
                );
            }
        }

        if let Some(index) = self
            .unit
            .get_power_index(PowerType::Mana)
            .filter(|index| *index < super::MAX_POWERS_PER_CLASS)
        {
            for (old, new, first_bit) in [
                (
                    previous.mana_regen,
                    stats.mana_regen,
                    UNIT_POWER_REGEN_FLAT_FIRST_BIT,
                ),
                (
                    previous.mana_regen_combat,
                    stats.mana_regen_combat,
                    UNIT_POWER_REGEN_INTERRUPTED_FIRST_BIT,
                ),
            ] {
                if old != new {
                    self.unit
                        .mark_unit_data_array(UNIT_POWER_PARENT_BIT, first_bit, index);
                }
            }
        }

        if previous.mainhand_expertise != stats.mainhand_expertise {
            self.mark_active_player_data(ACTIVE_MAINHAND_EXPERTISE_BIT);
            active_changed = true;
        }
        if previous.offhand_expertise != stats.offhand_expertise {
            self.mark_active_player_data(ACTIVE_OFFHAND_EXPERTISE_BIT);
            active_changed = true;
        }
        if previous.ranged_expertise != stats.ranged_expertise {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_RANGED_EXPERTISE_BIT,
            );
            active_changed = true;
        }
        if previous.combat_rating_expertise != stats.combat_rating_expertise {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_COMBAT_RATING_EXPERTISE_BIT,
            );
            active_changed = true;
        }
        if previous.block_pct != stats.block_pct {
            self.mark_active_player_data_section(ACTIVE_STATS_PARENT_BIT, ACTIVE_BLOCK_BIT);
            active_changed = true;
        }
        if previous.dodge_pct != stats.dodge_pct {
            self.mark_active_player_data_section(ACTIVE_STATS_PARENT_BIT, ACTIVE_DODGE_BIT);
            active_changed = true;
        }
        if previous.dodge_from_attr != stats.dodge_from_attr {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_DODGE_FROM_ATTRIBUTE_BIT,
            );
            active_changed = true;
        }
        if previous.parry_pct != stats.parry_pct {
            self.mark_active_player_data_section(ACTIVE_STATS_PARENT_BIT, ACTIVE_PARRY_BIT);
            active_changed = true;
        }
        if previous.parry_from_attr != stats.parry_from_attr {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_PARRY_FROM_ATTRIBUTE_BIT,
            );
            active_changed = true;
        }
        if previous.crit_pct != stats.crit_pct {
            self.mark_active_player_data_section(ACTIVE_STATS_PARENT_BIT, ACTIVE_CRIT_BIT);
            active_changed = true;
        }
        if previous.ranged_crit_pct != stats.ranged_crit_pct {
            self.mark_active_player_data_section(ACTIVE_STATS_PARENT_BIT, ACTIVE_RANGED_CRIT_BIT);
            active_changed = true;
        }
        if previous.offhand_crit_pct != stats.offhand_crit_pct {
            self.mark_active_player_data_section(ACTIVE_STATS_PARENT_BIT, ACTIVE_OFFHAND_CRIT_BIT);
            active_changed = true;
        }
        if previous.shield_block != stats.shield_block {
            self.mark_active_player_data_section(ACTIVE_STATS_PARENT_BIT, ACTIVE_SHIELD_BLOCK_BIT);
            active_changed = true;
        }
        if previous.shield_block_crit_pct != stats.shield_block_crit_pct {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_SHIELD_BLOCK_CRIT_BIT,
            );
            active_changed = true;
        }
        if previous.versatility_bonus != stats.versatility_bonus {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_VERSATILITY_BONUS_BIT,
            );
            active_changed = true;
        }
        if previous.mod_healing_done_pos != stats.mod_healing_done_pos {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_HEALING_DONE_POS_BIT,
            );
            active_changed = true;
        }
        if previous.mod_healing_done_percent != stats.mod_healing_done_percent {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_HEALING_DONE_PERCENT_BIT,
            );
            active_changed = true;
        }
        if previous.override_spell_power_by_ap_percent != stats.override_spell_power_by_ap_percent {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_OVERRIDE_SPELL_POWER_BY_AP_BIT,
            );
            active_changed = true;
        }
        if previous.override_ap_by_spell_power_percent != stats.override_ap_by_spell_power_percent {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_OVERRIDE_AP_BY_SPELL_POWER_BIT,
            );
            active_changed = true;
        }
        if previous.mod_target_resistance != stats.mod_target_resistance {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_TARGET_RESISTANCE_BIT,
            );
            active_changed = true;
        }
        if previous.mod_target_physical_resistance != stats.mod_target_physical_resistance {
            self.mark_active_player_data_section(
                ACTIVE_STATS_PARENT_BIT,
                ACTIVE_TARGET_PHYSICAL_RESISTANCE_BIT,
            );
            active_changed = true;
        }

        active_array!(
            previous.spell_crit_pct,
            stats.spell_crit_pct,
            ACTIVE_SPELL_MODIFIERS_PARENT_BIT,
            ACTIVE_SPELL_CRIT_FIRST_BIT
        );
        active_array!(
            previous.mod_damage_done_pos,
            stats.mod_damage_done_pos,
            ACTIVE_SPELL_MODIFIERS_PARENT_BIT,
            ACTIVE_MOD_DAMAGE_DONE_POS_FIRST_BIT
        );
        active_array!(
            previous.mod_damage_done_neg,
            stats.mod_damage_done_neg,
            ACTIVE_SPELL_MODIFIERS_PARENT_BIT,
            ACTIVE_MOD_DAMAGE_DONE_NEG_FIRST_BIT
        );
        active_array!(
            previous.mod_damage_done_percent,
            stats.mod_damage_done_percent,
            ACTIVE_SPELL_MODIFIERS_PARENT_BIT,
            ACTIVE_MOD_DAMAGE_DONE_PERCENT_FIRST_BIT
        );
        active_array!(
            previous.combat_ratings,
            stats.combat_ratings,
            ACTIVE_COMBAT_RATINGS_PARENT_BIT,
            ACTIVE_COMBAT_RATINGS_FIRST_BIT
        );

        self.effective_combat_stats = stats;
        if active_changed {
            self.unit
                .world_mut()
                .object_mut()
                .add_to_object_update_if_needed();
        }
    }

    pub(super) fn derived_stats_values_update(
        &self,
        include_active_player: bool,
    ) -> Option<PlayerDerivedStatValuesUpdate> {
        let mana_power_index = self
            .unit
            .get_power_index(PowerType::Mana)
            .filter(|index| *index < super::MAX_POWERS_PER_CLASS);
        let unit_mask = self.unit.unit_data_changes_mask();
        let unit_derived_changed = [
            52, 53, 54, 55, 76, 81, 82, 83, 84, 85, 86, 87, 88, 91, 92, 175, 176, 177, 178, 179,
            180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 191, 192, 193, 194, 195, 196, 197,
        ]
        .into_iter()
        .any(|bit| unit_mask.is_set(bit))
            || mana_power_index.is_some_and(|index| {
                [
                    UNIT_POWER_REGEN_FLAT_FIRST_BIT,
                    UNIT_POWER_REGEN_INTERRUPTED_FIRST_BIT,
                ]
                .into_iter()
                .any(|first_bit| unit_mask.is_set(first_bit + index))
            });
        let active_mask = &self.active_player_data_changes;
        let active_derived_changed = include_active_player
            && ([
                36, 37, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 56, 59, 61, 65, 66, 67, 68,
            ]
            .into_iter()
            .any(|bit| active_mask.is_set(bit))
                || (270..298).any(|bit| active_mask.is_set(bit))
                || (575..607).any(|bit| active_mask.is_set(bit)));

        (unit_derived_changed || active_derived_changed).then_some(PlayerDerivedStatValuesUpdate {
            effective_combat_stats: self.effective_combat_stats,
            mana_power_index,
        })
    }

    /// Discard derived stats when the Player lifecycle starts over.
    pub fn clear_effective_combat_stats_like_cpp(&mut self) {
        self.effective_combat_stats = PlayerEffectiveCombatStatsLikeCpp::default();
    }
}
