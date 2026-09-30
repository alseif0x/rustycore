// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3

//! C++ `ObjectMgr::LoadPlayerInfo` player stat data.
//!
//! Primary stats come from `player_classlevelstats` plus the signed
//! `player_racestats` modifiers. Base mana comes from the client
//! `gt/BaseMp.txt` GameTable through `ObjectMgr::GetPlayerClassLevelInfo`.
//! C++ does not consume the legacy C# `player_levelstats.basehp/basemana`
//! projection.

/// C++ `PlayerLevelInfo` plus the class/level `GtBaseMP` value.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PlayerLevelStats {
    pub strength: u16,
    pub agility: u16,
    pub stamina: u16,
    pub intellect: u16,
    pub spirit: u16,
    pub base_mana: u32,
}

impl PlayerLevelStats {
    pub const fn primary_stats_like_cpp(&self) -> [u16; 5] {
        [
            self.strength,
            self.agility,
            self.stamina,
            self.intellect,
            self.spirit,
        ]
    }
}

/// C++ `Player::UpdateSpellDamageAndHealingBonus` inputs
/// (`StatSystem.cpp:171-197`) built from `Unit::SpellBaseDamageBonusDone` and
/// `Unit::SpellBaseHealingBonusDone` (`Unit.cpp:6860-6890`, `7282-7315`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerSpellBonusInputLikeCpp {
    /// C++ `Player::GetBaseSpellPowerBonus()`: the item/enchant spell power
    /// `Player::ApplySpellPowerBonus` (`StatSystem.cpp:153-168`) accumulates.
    pub base_spell_power: i32,
    /// `GetTotalAuraModifierByMiscMask(SPELL_AURA_MOD_DAMAGE_DONE, 1 << school)`
    /// per school; index 0 is never published by C++.
    pub damage_done_flat: [i32; 7],
    /// Negative `SPELL_AURA_MOD_DAMAGE_DONE` sums per school
    /// (`ActivePlayerData::ModDamageDoneNeg`).
    pub damage_done_neg: [i32; 7],
    /// `SPELL_AURA_MOD_SPELL_DAMAGE_OF_STAT_PERCENT` sums per school and stat
    /// (the effect `MiscValue` is the school mask, `MiscValueB` the stat).
    pub damage_of_stat_percent: [[i32; 5]; 7],
    /// `GetTotalAuraModifier(SPELL_AURA_MOD_HEALING_DONE, SPELL_SCHOOL_MASK_ALL)`.
    pub healing_done_flat: i32,
    /// `SPELL_AURA_MOD_SPELL_HEALING_OF_STAT_PERCENT` sums per stat index (the
    /// effect `MiscValue` is the stat).
    pub healing_of_stat_percent: [i32; 5],
    /// `ActivePlayerData::OverrideSpellPowerByAPPercent`; `> 0` replaces both
    /// bonuses with `CalculatePct(GetTotalAttackPowerValue(BASE_ATTACK), pct)`.
    pub override_spell_power_by_ap_pct: f32,
    /// `ActivePlayerData::ModDamageDonePercent[7]` from
    /// `AuraEffect::HandleModDamagePercentDone`
    /// (`SpellAuraEffects.cpp:4525-4548`): the product of
    /// `1 + amount/100` over the active `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE`
    /// (79) effects intersecting each school, `1.0` when none does.
    pub damage_done_percent: [f32; 7],
    /// `ActivePlayerData::ModHealingDonePercent` from
    /// `Player::UpdateHealingDonePercentMod` (`StatSystem.cpp:588-599`): the
    /// product of `1 + amount/100` over the active
    /// `SPELL_AURA_MOD_HEALING_DONE_PERCENT` (136) effects, `1.0` when none.
    pub healing_done_percent: f32,
    /// `SPELL_AURA_MOD_VERSATILITY` (471) sum, published as
    /// `ActivePlayerData::VersatilityBonus`.
    pub versatility_bonus_aura: i32,
    /// `Unit::UpdateDamagePctDoneMods` (`Unit.cpp:9033-9072`) per attack:
    /// base factor (mainhand/ranged 1.0, offhand 0.5) times the
    /// `SPELL_AURA_MOD_DAMAGE_PERCENT_DONE` (79) physical multiplier.
    pub weapon_damage_pct: [f32; 3],
    /// `Player::UpdateDamageDoneMods` (`Player.cpp:4965-5015`) per attack: the
    /// `SPELL_AURA_MOD_DAMAGE_DONE` (13) physical sum plus the weapon
    /// enchantment damage, published as the `UNIT_MOD_DAMAGE_*` `TOTAL_VALUE`.
    pub weapon_damage_flat: [f32; 3],
    /// `SPELL_AURA_MOD_TARGET_RESISTANCE` (123) sums covering the full
    /// `SPELL_SCHOOL_MASK_SPELL`, published as `ModTargetResistance`.
    pub target_resistance_aura: i32,
    /// C++ `Player::m_spellPenetrationItemMod` from item/enchant
    /// `ITEM_MOD_SPELL_PENETRATION`; `ApplySpellPenetrationBonus`
    /// (`StatSystem.cpp:231-235`) writes `-amount` into `ModTargetResistance`.
    pub item_spell_penetration: i32,
    /// `SPELL_AURA_MOD_TARGET_RESISTANCE` (123) sums covering
    /// `SPELL_SCHOOL_MASK_NORMAL`, published as `ModTargetPhysicalResistance`.
    pub target_physical_resistance_aura: i32,
}

impl Default for PlayerSpellBonusInputLikeCpp {
    fn default() -> Self {
        Self {
            base_spell_power: 0,
            damage_done_flat: [0; 7],
            damage_done_neg: [0; 7],
            damage_of_stat_percent: [[0; 5]; 7],
            healing_done_flat: 0,
            healing_of_stat_percent: [0; 5],
            override_spell_power_by_ap_pct: 0.0,
            damage_done_percent: [1.0; 7],
            healing_done_percent: 1.0,
            weapon_damage_pct: [1.0, 0.5, 1.0],
            weapon_damage_flat: [0.0; 3],
            versatility_bonus_aura: 0,
            target_resistance_aura: 0,
            item_spell_penetration: 0,
            target_physical_resistance_aura: 0,
        }
    }
}

/// Inputs currently represented by Rust for C++ `Player::UpdateAllStats`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerStatSystemInputLikeCpp {
    pub base: PlayerLevelStats,
    pub class: u8,
    pub level: u8,
    pub attack_power_per_strength: u8,
    pub attack_power_per_agility: u8,
    pub ranged_attack_power_per_agility: u8,
    /// C++ UnitMods `TOTAL_PCT` after represented
    /// `SPELL_AURA_MOD_TOTAL_STAT_PERCENTAGE` effects, one factor per stat.
    pub stat_total_multipliers: [f32; 5],
    /// C++ `Unit::UpdateStatBuffMod` factor for the client-visible positive
    /// and negative flat-stat buff fields.
    pub stat_buff_total_multipliers: [f32; 5],
    pub gear_stats: [i32; 5],
    pub gear_health: i32,
    pub gear_mana: i32,
    pub gear_armor: i32,
    /// C++ `GetPctModifierValue(UNIT_MOD_ARMOR, BASE_PCT)` from
    /// `SPELL_AURA_MOD_BASE_RESISTANCE_PCT` effects carrying the normal school
    /// mask.
    pub armor_base_pct: f32,
    /// C++ `GetFlatModifierValue(UNIT_MOD_ARMOR, TOTAL_VALUE)` from
    /// `SPELL_AURA_MOD_RESISTANCE`/`SPELL_AURA_MOD_BASE_RESISTANCE` effects
    /// carrying the normal school mask.
    pub armor_flat_aura: i32,
    /// `SPELL_AURA_MOD_RESISTANCE_OF_STAT_PERCENT` amounts carrying the normal
    /// school mask, aggregated per `MiscValueB` stat index (`CalculatePct`).
    pub armor_of_stat_percent: [i32; 5],
    /// C++ `GetPctModifierValue(UNIT_MOD_ARMOR, TOTAL_PCT)` from
    /// `SPELL_AURA_MOD_RESISTANCE_PCT` effects carrying the normal school mask.
    pub armor_total_pct: f32,
    /// C++ `GetTotalAuraMultiplier(SPELL_AURA_MOD_BONUS_ARMOR_PCT)`.
    pub armor_bonus_pct: f32,
    /// C++ `GetTotalAuraModifier(SPELL_AURA_MOD_DODGE_PERCENT)`.
    pub spell_dodge_pct: f32,
    /// C++ `GetTotalAuraModifier(SPELL_AURA_MOD_PARRY_PERCENT)`.
    pub spell_parry_pct: f32,
    /// C++ `GetTotalAuraModifier(SPELL_AURA_MOD_BLOCK_PERCENT)`.
    pub spell_block_pct: f32,
    /// C++ `GetBaseModValue(CRIT_PERCENTAGE, FLAT_MOD)` from
    /// `UpdateWeaponDependentCritAuras(BASE_ATTACK)`.
    pub crit_mainhand_aura_pct: f32,
    /// C++ `GetBaseModValue(OFFHAND_CRIT_PERCENTAGE, FLAT_MOD)`.
    pub crit_offhand_aura_pct: f32,
    /// C++ `GetBaseModValue(RANGED_CRIT_PERCENTAGE, FLAT_MOD)`.
    pub crit_ranged_aura_pct: f32,
    /// C++ `GetTotalAuraModifier(SPELL_AURA_MOD_SPELL_CRIT_CHANCE)` plus
    /// `GetTotalAuraModifier(SPELL_AURA_MOD_CRIT_PCT)`.
    pub spell_crit_aura_pct: f32,
    pub gear_attack_power: i32,
    pub gear_ranged_attack_power: i32,
    /// C++ `GetFlatModifierValue(UNIT_MOD_ATTACK_POWER, TOTAL_VALUE)` from
    /// `SPELL_AURA_MOD_ATTACK_POWER` (99).
    pub attack_power_flat_aura: i32,
    /// C++ `GetPctModifierValue(UNIT_MOD_ATTACK_POWER, TOTAL_PCT)` from
    /// `SPELL_AURA_MOD_ATTACK_POWER_PCT` (166).
    pub attack_power_total_pct: f32,
    /// C++ `GetFlatModifierValue(UNIT_MOD_ATTACK_POWER_RANGED, TOTAL_VALUE)`
    /// from `SPELL_AURA_MOD_RANGED_ATTACK_POWER` (124).
    pub ranged_attack_power_flat_aura: i32,
    /// C++ `GetPctModifierValue(UNIT_MOD_ATTACK_POWER_RANGED, TOTAL_PCT)` from
    /// `SPELL_AURA_MOD_RANGED_ATTACK_POWER_PCT` (167).
    pub ranged_attack_power_total_pct: f32,
    /// C++ `Player::UpdateAttackPowerAndDamage` (`StatSystem.cpp:341-379`):
    /// `Some(ActivePlayerData::OverrideAPBySpellPowerPercent)` while
    /// `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` is active. Both attack mods
    /// then read `CalculatePct(min(ModHealingDonePos, ModDamageDonePos[HOLY..MAX]),
    /// percent)`; `None` keeps the strength/agility/level base.
    pub attack_power_override_by_spell_power_pct: Option<f32>,
    /// C++ `Player::UpdateSpellDamageAndHealingBonus` producers.
    pub spell_bonus: PlayerSpellBonusInputLikeCpp,
    pub rating_bonuses: [f32; 32],
    pub can_parry: bool,
    pub can_block: bool,
}

/// C++-shaped result of the represented `Player::UpdateAllStats` inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerStatSystemProjectionLikeCpp {
    pub stats: [i32; 5],
    pub stat_pos_buff: [i32; 5],
    pub stat_neg_buff: [i32; 5],
    pub create_health: i32,
    pub base_mana: i32,
    pub max_health: i64,
    pub max_mana: i64,
    pub armor: i32,
    pub attack_power: i32,
    pub attack_power_mod_pos: i32,
    /// C++ `UnitData::AttackPowerMultiplier` (`TOTAL_PCT - 1.0`).
    pub attack_power_multiplier: f32,
    pub ranged_attack_power: i32,
    pub ranged_attack_power_mod_pos: i32,
    /// C++ `UnitData::RangedAttackPowerMultiplier` (`TOTAL_PCT - 1.0`).
    pub ranged_attack_power_multiplier: f32,
    pub total_attack_power: i32,
    pub total_ranged_attack_power: i32,
    pub block_pct: f32,
    pub dodge_pct: f32,
    pub dodge_from_attr: f32,
    pub parry_pct: f32,
    pub parry_from_attr: f32,
    pub crit_pct: f32,
    pub ranged_crit_pct: f32,
    pub offhand_crit_pct: f32,
    pub spell_crit_pct: [f32; 7],
    /// C++ `ActivePlayerData::ModDamageDonePos[7]`; index 0 stays unwritten and
    /// each magic school is `max(SpellBaseDamageBonusDone(1 << school) - Neg, 0)`.
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
    /// C++ `ActivePlayerData::ModTargetResistance` (`StatSystem.cpp:231-235`,
    /// `SpellAuraEffects.cpp:3507-3530`).
    pub mod_target_resistance: i32,
    /// C++ `ActivePlayerData::ModTargetPhysicalResistance`.
    pub mod_target_physical_resistance: i32,
    /// C++ `ActivePlayerData::VersatilityBonus` (bit 56), clamped at zero by
    /// `SetUpdateFieldStatValue`.
    pub versatility_bonus: f32,
    /// C++ `ActivePlayerData::OverrideSpellPowerByAPPercent` (bit 65): the
    /// accumulated `SPELL_AURA_OVERRIDE_SPELL_POWER_BY_AP_PCT` amount, `0.0`
    /// when no such effect is active.
    pub override_spell_power_by_ap_percent: f32,
    /// C++ `ActivePlayerData::OverrideAPBySpellPowerPercent` (bit 66): the
    /// accumulated `SPELL_AURA_OVERRIDE_ATTACK_POWER_BY_SP_PCT` amount, `0.0`
    /// when no such effect is active.
    pub override_ap_by_spell_power_percent: f32,
    /// C++ `UNIT_MOD_DAMAGE_*` `TOTAL_PCT` per attack.
    pub weapon_damage_pct: [f32; 3],
    /// C++ `UNIT_MOD_DAMAGE_*` `TOTAL_VALUE` per attack.
    pub weapon_damage_flat: [f32; 3],
}
