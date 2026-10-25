// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::BTreeMap;
use wow_constants::WeaponAttackType;
use wow_data::PlayerStatSystemProjectionLikeCpp;
use wow_entities::PlayerEffectiveCombatStatsLikeCpp;
use wow_world_core::session::state::hub_support::RepresentedPlayerGearStatsLikeCpp;
use wow_world_core::session::{PlayerStatCatalogs, PlayerStatFacts, PlayerStatWeaponFacts};

/// C++ `SPELL_SCHOOL_MASK_NORMAL` (`SharedDefines.h:329`).
const SPELL_SCHOOL_MASK_NORMAL: i32 = 1;
/// C++ `ITEM_ENCHANTMENT_TYPE_DAMAGE` (`DBCEnums.h:965`).
const ITEM_ENCHANTMENT_TYPE_DAMAGE: u8 = 2;
/// C++ `ITEM_ENCHANTMENT_TYPE_TOTEM` (`DBCEnums.h:969`).
const ITEM_ENCHANTMENT_TYPE_TOTEM: u8 = 6;

pub(super) fn weapon_damage_pct(
    facts: &PlayerStatFacts<'_>,
    catalogs: &PlayerStatCatalogs<'_>,
) -> [f32; 3] {
    // C++ `Unit::UpdateDamagePctDoneMods` (`Unit.cpp:9033-9072`).
    let aura = facts.aura_view();
    let effects = aura.effects_with_spell_and_misc_by_aura_type(
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_PERCENT_DONE,
    );
    std::array::from_fn(|index| {
        let attack = <WeaponAttackType as num_traits::FromPrimitive>::from_usize(index)
            .unwrap_or(WeaponAttackType::BaseAttack);
        let base = match attack {
            WeaponAttackType::OffAttack => 0.5_f32,
            _ => 1.0_f32,
        };
        let weapon_item_id = weapon_for_attack(facts, attack).map(PlayerStatWeaponFacts::entry_id);
        // Keep the inherited 0.5 offhand base; the existing represented path
        // does not include C++'s separate offhand-damage aura sum.
        base * effects
            .iter()
            .filter(|(spell_id, misc_value, _)| {
                misc_value & SPELL_SCHOOL_MASK_NORMAL != 0
                    && catalogs.represented_aura_spell_fits_weapon(*spell_id, weapon_item_id)
            })
            .fold(1.0_f32, |acc, (_, _, amount)| {
                acc * (1.0 + *amount as f32 / 100.0)
            })
    })
}

pub(super) fn weapon_damage_flat(
    facts: &PlayerStatFacts<'_>,
    catalogs: &PlayerStatCatalogs<'_>,
) -> [f32; 3] {
    // C++ `Player::UpdateDamageDoneMods` (`Player.cpp:4965-5015`).
    let aura = facts.aura_view();
    let effects = aura.effects_with_spell_and_misc_by_aura_type(
        wow_data::spell::aura_types::SPELL_AURA_MOD_DAMAGE_DONE,
    );
    std::array::from_fn(|index| {
        let attack = <WeaponAttackType as num_traits::FromPrimitive>::from_usize(index)
            .unwrap_or(WeaponAttackType::BaseAttack);
        let weapon_item_id = weapon_for_attack(facts, attack).map(PlayerStatWeaponFacts::entry_id);
        let aura_sum = effects
            .iter()
            .filter(|(spell_id, misc_value, _)| {
                misc_value & SPELL_SCHOOL_MASK_NORMAL != 0
                    && catalogs.represented_aura_spell_fits_weapon(*spell_id, weapon_item_id)
            })
            .map(|(_, _, amount)| *amount)
            .sum::<i32>();
        aura_sum as f32 + weapon_enchant_damage(facts, catalogs, attack)
    })
}

fn weapon_enchant_damage(
    facts: &PlayerStatFacts<'_>,
    catalogs: &PlayerStatCatalogs<'_>,
    attack: WeaponAttackType,
) -> f32 {
    let Some(item) = weapon_for_attack(facts, attack) else {
        return 0.0;
    };
    let Some(enchantment_store) = catalogs.item_enchantments() else {
        return 0.0;
    };
    let delay_seconds = catalogs.weapon_delay_seconds(item.entry_id());
    let is_shaman = facts.class() == 7;

    // C++ enchantment scaling loop: `Player.cpp:4991-5015`.
    item.enchantment_ids()
        .iter()
        .filter_map(|enchantment_id| enchantment_store.get(*enchantment_id))
        .flat_map(|entry| {
            entry
                .effect
                .iter()
                .zip(entry.effect_scaling_points.iter())
                .map(move |(effect, points)| match *effect {
                    ITEM_ENCHANTMENT_TYPE_DAMAGE => *points,
                    ITEM_ENCHANTMENT_TYPE_TOTEM if is_shaman => *points * delay_seconds,
                    _ => 0.0,
                })
        })
        .sum()
}

pub(super) fn weapon_crit_aura_modifier(
    facts: &PlayerStatFacts<'_>,
    catalogs: &PlayerStatCatalogs<'_>,
    attack: WeaponAttackType,
) -> f32 {
    // C++ `Player::UpdateWeaponDependentCritAuras` (`Player.cpp:8079-8107`).
    let weapon_item_id = weapon_for_attack(facts, attack).map(PlayerStatWeaponFacts::entry_id);
    let aura = facts.aura_view();
    let weapon_dependent = aura
        .effect_amounts_by_aura_type(
            wow_data::spell::aura_types::SPELL_AURA_MOD_WEAPON_CRIT_PERCENT,
        )
        .into_iter()
        .filter(|(spell_id, _)| {
            catalogs.represented_aura_spell_fits_weapon(*spell_id, weapon_item_id)
        })
        .map(|(_, amount)| amount)
        .sum::<i32>();
    let global = aura
        .effect_amounts_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_CRIT_PCT)
        .into_iter()
        .map(|(_, amount)| amount)
        .sum::<i32>();
    (weapon_dependent + global) as f32
}

fn expertise_aura_modifier(
    facts: &PlayerStatFacts<'_>,
    catalogs: &PlayerStatCatalogs<'_>,
    attack: WeaponAttackType,
) -> i32 {
    // C++ `Player::UpdateExpertise` (`StatSystem.cpp:759-786`).
    let weapon_item_id = weapon_for_attack(facts, attack).map(PlayerStatWeaponFacts::entry_id);
    let aura = facts.aura_view();
    let effects =
        aura.effect_amounts_by_aura_type(wow_data::spell::aura_types::SPELL_AURA_MOD_EXPERTISE);

    let mut same_effect_groups: BTreeMap<u32, i32> = BTreeMap::new();
    let mut modifier = 0;
    for (spell_id, amount) in effects {
        if !catalogs.represented_aura_spell_fits_weapon(spell_id, weapon_item_id) {
            continue;
        }
        // A spell belongs to at most one same-effect group per aura type.
        let same_effect_group = aura
            .spell_group_map_bounds(spell_id as u32)
            .iter()
            .copied()
            .find(|group_id| {
                aura.same_effect_stack_rule_aura_types(*group_id)
                    .is_some_and(|aura_types| {
                        aura_types.contains(&wow_data::spell::aura_types::SPELL_AURA_MOD_EXPERTISE)
                    })
            });
        if let Some(group_id) = same_effect_group {
            same_effect_groups
                .entry(group_id)
                .and_modify(|current| {
                    if current.unsigned_abs() < amount.unsigned_abs() {
                        *current = amount;
                    }
                })
                .or_insert(amount);
        } else {
            modifier += amount;
        }
    }
    modifier + same_effect_groups.values().sum::<i32>()
}

pub(super) fn effective_stats(
    facts: &PlayerStatFacts<'_>,
    catalogs: &PlayerStatCatalogs<'_>,
    gear: &RepresentedPlayerGearStatsLikeCpp,
    projection: PlayerStatSystemProjectionLikeCpp,
    level: u8,
) -> PlayerEffectiveCombatStatsLikeCpp {
    // This assembles the same derived fields previously published after the
    // `Player::UpdateAllStats` projection; it does not add a Player-side store.
    let mut resistances = gear.resistances;
    // Physical resistance is the final armor value after agility and flat
    // armor; the six magic schools come from `Unit::UpdateResistances`.
    resistances[0] = projection.armor;
    for (index, value) in facts
        .aura_view()
        .school_resistances(gear)
        .into_iter()
        .enumerate()
    {
        resistances[index + 1] = value;
    }
    let weapon_damage = wow_data::player::effective_weapon_damage_ranges_like_cpp(
        projection,
        gear.weapon_damage,
        gear.base_attack_time,
        catalogs.shapeshift_combat_round_time(facts.form_id()),
    );

    let rating_expertise =
        (gear.combat_ratings[23] as f32 * catalogs.combat_rating_multiplier(level, 23)).trunc();
    let mainhand_expertise = (rating_expertise
        + expertise_aura_modifier(facts, catalogs, WeaponAttackType::BaseAttack) as f32)
        .max(0.0);
    let offhand_expertise = (rating_expertise
        + expertise_aura_modifier(facts, catalogs, WeaponAttackType::OffAttack) as f32)
        .max(0.0);

    let armor_penetration_rating = crate::CR_ARMOR_PENETRATION_LIKE_CPP;
    let armor_penetration_pct = (gear.combat_ratings[usize::from(armor_penetration_rating)] as f32
        * catalogs.combat_rating_multiplier(level, u32::from(armor_penetration_rating)))
    .clamp(0.0, 100.0);
    let melee_hit_chance_pct = 7.5
        + gear.combat_ratings[usize::from(crate::CR_HIT_MELEE_LIKE_CPP)] as f32
            * catalogs.combat_rating_multiplier(level, u32::from(crate::CR_HIT_MELEE_LIKE_CPP));

    let aura = facts.aura_view();
    let mana_regen_mp5 =
        gear.mana_regen_bonus as f32 / 5.0 + aura.mana_regen_mp5_from_auras(projection.stats);
    let mana_regen_from_spirit =
        catalogs.mana_regen_from_stats(level, facts.class(), projection.stats)
            * aura.mana_regen_aura_multiplier();
    let mana_regen_combat =
        mana_regen_mp5 + mana_regen_from_spirit * aura.mana_regen_interrupt_modifier();

    PlayerEffectiveCombatStatsLikeCpp {
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
        attack_power_multiplier: projection.attack_power_multiplier,
        ranged_attack_power: projection.ranged_attack_power,
        ranged_attack_power_mod_pos: projection.ranged_attack_power_mod_pos,
        ranged_attack_power_multiplier: projection.ranged_attack_power_multiplier,
        min_damage: weapon_damage[0][0],
        max_damage: weapon_damage[0][1],
        weapon_damage,
        min_ranged_damage: weapon_damage[2][0],
        max_ranged_damage: weapon_damage[2][1],
        combat_ratings: gear.combat_ratings,
        spell_power: gear.spell_power,
        mod_damage_done_pos: projection.mod_damage_done_pos,
        mod_damage_done_neg: projection.mod_damage_done_neg,
        mod_healing_done_pos: projection.mod_healing_done_pos,
        mod_damage_done_percent: projection.mod_damage_done_percent,
        mod_healing_done_percent: projection.mod_healing_done_percent,
        armor_penetration_pct,
        melee_hit_chance_pct,
        mod_target_resistance: projection.mod_target_resistance,
        mod_target_physical_resistance: projection.mod_target_physical_resistance,
        weapon_damage_pct: projection.weapon_damage_pct,
        weapon_damage_flat: projection.weapon_damage_flat,
        versatility_bonus: projection.versatility_bonus,
        override_spell_power_by_ap_percent: projection.override_spell_power_by_ap_percent,
        override_ap_by_spell_power_percent: projection.override_ap_by_spell_power_percent,
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
    }
}

fn weapon_for_attack<'facts, 'view>(
    facts: &'view PlayerStatFacts<'facts>,
    attack: WeaponAttackType,
) -> Option<&'view PlayerStatWeaponFacts> {
    match attack {
        WeaponAttackType::BaseAttack => facts.main_hand(),
        WeaponAttackType::OffAttack => facts.off_hand(),
        WeaponAttackType::RangedAttack => facts.ranged(),
        WeaponAttackType::Max => None,
    }
}
