// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use super::equipment;
use super::spell_bonus;
use wow_data::player::PlayerDamagePctOperation;
use wow_data::{
    PlayerStatSystemInputLikeCpp, PlayerStatSystemProjectionLikeCpp,
    calculate_player_stat_system_like_cpp,
};
use wow_world_core::session::state::hub_support::{
    RepresentedPlayerGearStatsLikeCpp, SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
};
use wow_world_core::session::{PlayerStatCatalogs, PlayerStatFacts};

pub(super) fn project(
    facts: &PlayerStatFacts<'_>,
    catalogs: &PlayerStatCatalogs<'_>,
    gear: &RepresentedPlayerGearStatsLikeCpp,
    race: u8,
    class: u8,
    level: u8,
    damage_pct_operation: PlayerDamagePctOperation,
) -> Option<PlayerStatSystemProjectionLikeCpp> {
    let base = *catalogs.player_stats()?.get(race, class, level)?;
    let (attack_power_per_strength, attack_power_per_agility, ranged_attack_power_per_agility) =
        catalogs.player_class_attack_power_coefficients(class)?;
    let rating_bonuses = std::array::from_fn(|index| {
        gear.combat_ratings[index] as f32 * catalogs.combat_rating_multiplier(level, index as u32)
    });
    let aura = facts.aura_view();
    let spell_bonus =
        spell_bonus::represented_spell_bonus(facts, catalogs, gear, damage_pct_operation);

    let projection = calculate_player_stat_system_like_cpp(PlayerStatSystemInputLikeCpp {
        base,
        class,
        level,
        attack_power_per_strength,
        attack_power_per_agility,
        ranged_attack_power_per_agility,
        stat_base_multipliers: aura.stat_base_multipliers(),
        stat_total_multipliers: aura.total_stat_multipliers(),
        stat_buff_total_multipliers: aura.total_stat_buff_multipliers(),
        gear_stats: gear.stats,
        gear_health: gear.health,
        gear_mana: gear.mana,
        gear_armor: gear.armor,
        armor_base_pct: aura.resistance_aura_multiplier(
            wow_data::spell::aura_types::SPELL_AURA_MOD_BASE_RESISTANCE_PCT,
            SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
        ),
        armor_flat_aura: aura.resistance_aura_flat(SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP) as i32,
        armor_of_stat_percent: aura.armor_of_stat_percent(),
        armor_total_pct: aura.resistance_aura_multiplier(
            wow_data::spell::aura_types::SPELL_AURA_MOD_RESISTANCE_PCT,
            SPELL_SCHOOL_MASK_NORMAL_LIKE_CPP,
        ),
        armor_bonus_pct: aura
            .total_aura_multiplier(wow_data::spell::aura_types::SPELL_AURA_MOD_BONUS_ARMOR_PCT),
        spell_dodge_pct: aura
            .total_aura_modifier(wow_data::spell::aura_types::SPELL_AURA_MOD_DODGE_PERCENT),
        spell_parry_pct: aura
            .total_aura_modifier(wow_data::spell::aura_types::SPELL_AURA_MOD_PARRY_PERCENT),
        spell_block_pct: aura
            .total_aura_modifier(wow_data::spell::aura_types::SPELL_AURA_MOD_BLOCK_PERCENT),
        crit_mainhand_aura_pct: equipment::weapon_crit_aura_modifier(
            facts,
            catalogs,
            wow_constants::WeaponAttackType::BaseAttack,
        ),
        crit_offhand_aura_pct: equipment::weapon_crit_aura_modifier(
            facts,
            catalogs,
            wow_constants::WeaponAttackType::OffAttack,
        ),
        crit_ranged_aura_pct: equipment::weapon_crit_aura_modifier(
            facts,
            catalogs,
            wow_constants::WeaponAttackType::RangedAttack,
        ),
        spell_crit_aura_pct: aura
            .total_aura_modifier(wow_data::spell::aura_types::SPELL_AURA_MOD_SPELL_CRIT_CHANCE)
            + aura.total_aura_modifier(wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_PCT),
        gear_attack_power: gear.attack_power,
        gear_ranged_attack_power: gear.ranged_attack_power,
        attack_power_flat_aura: aura.attack_power_flat_aura(),
        attack_power_total_pct: aura
            .total_aura_multiplier(wow_data::spell::aura_types::SPELL_AURA_MOD_ATTACK_POWER_PCT),
        ranged_attack_power_flat_aura: aura.ranged_attack_power_flat_aura(class),
        ranged_attack_power_total_pct: aura.ranged_attack_power_total_pct(class),
        attack_power_override_by_spell_power_pct: aura
            .override_attack_power_by_spell_power_percent(),
        spell_bonus,
        rating_bonuses,
        can_parry: facts.can_parry(),
        can_block: facts.can_block(),
    });

    let limits = catalogs.limits();
    let mut projection = projection;
    projection.block_pct = limits.clamp_block_like_cpp(projection.block_pct);
    projection.dodge_pct = limits.clamp_dodge_like_cpp(projection.dodge_pct);
    projection.parry_pct = limits.clamp_parry_like_cpp(projection.parry_pct);
    projection.crit_pct = limits.clamp_crit_like_cpp(projection.crit_pct);
    projection.ranged_crit_pct = limits.clamp_crit_like_cpp(projection.ranged_crit_pct);
    projection.offhand_crit_pct = limits.clamp_crit_like_cpp(projection.offhand_crit_pct);
    Some(projection)
}
