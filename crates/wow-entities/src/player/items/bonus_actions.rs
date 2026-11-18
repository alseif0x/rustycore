// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Item bonus action plans and shared stat helpers.

use super::super::{
    ApplyEnchantmentCombatRating, ApplyEnchantmentEffectAction, ApplyEnchantmentUnitMod,
    ApplyEnchantmentUnitModifier, BASE_MAXDAMAGE, BASE_MINDAMAGE, EQUIPMENT_SLOT_MAINHAND,
    EQUIPMENT_SLOT_OFFHAND, InventoryType, ItemModType, SpellSchools, Stats, WeaponAttackType,
    WeaponDamageBoundLikeCpp,
};
use super::enchantment::apply_enchantment_stat_actions;

/// C++ `Player::_ApplyItemBonuses` static ItemSparse stat-loop subset.
pub fn item_stat_bonus_actions_like_cpp(
    stats: &[(i8, i16); 10],
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    let mut actions = Vec::new();
    for &(stat_type, amount) in stats {
        if stat_type == -1 || amount == 0 {
            continue;
        }
        if amount < 0 {
            actions.push(ApplyEnchantmentEffectAction::UnhandledStatModifier {
                item_mod: item_mod_type_from_u32(stat_type as u32),
                amount: amount.unsigned_abs().into(),
                apply,
            });
            continue;
        }
        let item_mod = item_mod_type_from_u32(stat_type as u32);
        actions.extend(item_bonus_stat_actions_like_cpp(
            item_mod,
            amount as u32,
            apply,
        ));
    }
    actions
}

/// C++ `Player::_ApplyItemBonuses` scaling-stat stat loop.
pub fn item_scaling_stat_bonus_actions_like_cpp(
    stat_ids: &[i32; 10],
    bonuses: &[i32; 10],
    ssd_multiplier: i32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    let mut actions = Vec::new();
    for (&stat_type, &bonus) in stat_ids.iter().zip(bonuses.iter()) {
        if stat_type == -1 {
            continue;
        }
        let val = (ssd_multiplier * bonus) / 10_000;
        if val == 0 {
            continue;
        }
        if val < 0 {
            actions.push(ApplyEnchantmentEffectAction::UnhandledStatModifier {
                item_mod: item_mod_type_from_u32(stat_type as u32),
                amount: val.unsigned_abs(),
                apply,
            });
            continue;
        }
        let item_mod = item_mod_type_from_u32(stat_type as u32);
        actions.extend(item_bonus_stat_actions_like_cpp(
            item_mod, val as u32, apply,
        ));
    }
    actions
}

/// C++ `_ApplyItemBonuses` direct `ItemTemplate::GetResistance(school)` loop.
pub fn item_resistance_bonus_actions_like_cpp(
    resistances: &[i16; 7],
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    let mut actions = Vec::new();
    for (school, resistance) in resistances.iter().copied().enumerate() {
        if resistance == 0 {
            continue;
        }
        if resistance < 0 {
            continue;
        }
        actions.push(unit_modifier(
            ApplyEnchantmentUnitMod::Resistance(school as u32),
            ApplyEnchantmentUnitModifier::BaseValue,
            resistance as u32,
            apply,
        ));
    }
    actions
}

/// C++ `_ApplyItemBonuses` direct `ActivePlayerData::ShieldBlock` assignment.
pub fn item_shield_block_bonus_action_like_cpp(
    shield_block_value: i16,
    is_armor_shield: bool,
    apply: bool,
) -> Option<ApplyEnchantmentEffectAction> {
    if !is_armor_shield || shield_block_value <= 0 {
        return None;
    }

    Some(ApplyEnchantmentEffectAction::SetShieldBlockValue {
        amount: if apply { shield_block_value as u32 } else { 0 },
    })
}

/// C++ `Player::_ApplyWeaponDamage` direct non-scaling weapon field actions.
pub fn item_weapon_damage_actions_like_cpp(
    slot: u8,
    inventory_type: InventoryType,
    min_damage: f32,
    max_damage: f32,
    item_delay: u16,
    apply: bool,
    is_in_feral_form: bool,
    can_use_attack_type: bool,
    has_shapeshift_combat_round_time: bool,
    can_modify_stats: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    const BASE_ATTACK_TIME_LIKE_CPP: u32 = 2_000;

    let attack_type = get_attack_by_slot(slot, inventory_type);
    if attack_type == WeaponAttackType::Max || (!is_in_feral_form && apply && !can_use_attack_type)
    {
        return Vec::new();
    }

    let mut actions = Vec::new();
    let mut changed_damage = false;

    if min_damage > 0.0 {
        let amount = if apply { min_damage } else { BASE_MINDAMAGE };
        actions.push(ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
            attack_type,
            bound: WeaponDamageBoundLikeCpp::Min,
            amount_bits: amount.to_bits(),
        });
        changed_damage = true;
    }

    if max_damage > 0.0 {
        let amount = if apply { max_damage } else { BASE_MAXDAMAGE };
        actions.push(ApplyEnchantmentEffectAction::SetBaseWeaponDamage {
            attack_type,
            bound: WeaponDamageBoundLikeCpp::Max,
            amount_bits: amount.to_bits(),
        });
        changed_damage = true;
    }

    if item_delay != 0 && !has_shapeshift_combat_round_time {
        actions.push(ApplyEnchantmentEffectAction::SetBaseAttackTime {
            attack_type,
            time_ms: if apply {
                u32::from(item_delay)
            } else {
                BASE_ATTACK_TIME_LIKE_CPP
            },
        });
    }

    if can_modify_stats && (changed_damage || item_delay != 0) {
        actions.push(ApplyEnchantmentEffectAction::UpdateDamagePhysical { attack_type });
    }

    actions
}

fn item_bonus_stat_actions_like_cpp(
    item_mod: ItemModType,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    match item_mod {
        ItemModType::Agility => item_bonus_primary_stat_actions(
            ApplyEnchantmentUnitMod::StatAgility,
            Stats::Agility,
            amount,
            apply,
        ),
        ItemModType::Strength => item_bonus_primary_stat_actions(
            ApplyEnchantmentUnitMod::StatStrength,
            Stats::Strength,
            amount,
            apply,
        ),
        ItemModType::Intellect => item_bonus_primary_stat_actions(
            ApplyEnchantmentUnitMod::StatIntellect,
            Stats::Intellect,
            amount,
            apply,
        ),
        ItemModType::Spirit => item_bonus_primary_stat_actions(
            ApplyEnchantmentUnitMod::StatSpirit,
            Stats::Spirit,
            amount,
            apply,
        ),
        ItemModType::Stamina => item_bonus_primary_stat_actions(
            ApplyEnchantmentUnitMod::StatStamina,
            Stats::Stamina,
            amount,
            apply,
        ),
        ItemModType::HasteMeleeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HasteMelee], amount, apply)
        }
        ItemModType::HasteRangedRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HasteRanged], amount, apply)
        }
        ItemModType::ExtraArmor => vec![unit_modifier(
            ApplyEnchantmentUnitMod::Armor,
            ApplyEnchantmentUnitModifier::TotalValue,
            amount,
            apply,
        )],
        ItemModType::FireResistance => {
            item_bonus_resistance_actions(SpellSchools::Fire, amount, apply)
        }
        ItemModType::FrostResistance => {
            item_bonus_resistance_actions(SpellSchools::Frost, amount, apply)
        }
        ItemModType::HolyResistance => {
            item_bonus_resistance_actions(SpellSchools::Holy, amount, apply)
        }
        ItemModType::ShadowResistance => {
            item_bonus_resistance_actions(SpellSchools::Shadow, amount, apply)
        }
        ItemModType::NatureResistance => {
            item_bonus_resistance_actions(SpellSchools::Nature, amount, apply)
        }
        ItemModType::ArcaneResistance => {
            item_bonus_resistance_actions(SpellSchools::Arcane, amount, apply)
        }
        ItemModType::AgiStrInt => [
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatAgility,
                Stats::Agility,
                amount,
                apply,
            ),
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatStrength,
                Stats::Strength,
                amount,
                apply,
            ),
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatIntellect,
                Stats::Intellect,
                amount,
                apply,
            ),
        ]
        .concat(),
        ItemModType::AgiStr => [
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatAgility,
                Stats::Agility,
                amount,
                apply,
            ),
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatStrength,
                Stats::Strength,
                amount,
                apply,
            ),
        ]
        .concat(),
        ItemModType::AgiInt => [
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatAgility,
                Stats::Agility,
                amount,
                apply,
            ),
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatIntellect,
                Stats::Intellect,
                amount,
                apply,
            ),
        ]
        .concat(),
        ItemModType::StrInt => [
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatStrength,
                Stats::Strength,
                amount,
                apply,
            ),
            item_bonus_primary_stat_actions(
                ApplyEnchantmentUnitMod::StatIntellect,
                Stats::Intellect,
                amount,
                apply,
            ),
        ]
        .concat(),
        _ => apply_enchantment_stat_actions(item_mod, amount, apply),
    }
}

fn item_bonus_primary_stat_actions(
    unit_mod: ApplyEnchantmentUnitMod,
    stat: Stats,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    vec![
        unit_modifier(
            unit_mod,
            ApplyEnchantmentUnitModifier::BaseValue,
            amount,
            apply,
        ),
        ApplyEnchantmentEffectAction::UpdateStatBuffMod(stat),
    ]
}

fn item_bonus_resistance_actions(
    school: SpellSchools,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    vec![unit_modifier(
        ApplyEnchantmentUnitMod::Resistance(school as u32),
        ApplyEnchantmentUnitModifier::BaseValue,
        amount,
        apply,
    )]
}

pub(super) fn primary_stat_actions(
    unit_mod: ApplyEnchantmentUnitMod,
    stat: Stats,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    vec![
        unit_modifier(
            unit_mod,
            ApplyEnchantmentUnitModifier::TotalValue,
            amount,
            apply,
        ),
        ApplyEnchantmentEffectAction::UpdateStatBuffMod(stat),
    ]
}

pub(super) fn unit_modifier(
    unit_mod: ApplyEnchantmentUnitMod,
    modifier: ApplyEnchantmentUnitModifier,
    amount: u32,
    apply: bool,
) -> ApplyEnchantmentEffectAction {
    ApplyEnchantmentEffectAction::UnitModifier {
        unit_mod,
        modifier,
        amount,
        apply,
    }
}

pub(super) fn rating_actions(
    ratings: &[ApplyEnchantmentCombatRating],
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    ratings
        .iter()
        .map(|rating| ApplyEnchantmentEffectAction::RatingModifier {
            rating: *rating,
            amount,
            apply,
        })
        .collect()
}

pub(super) const fn get_attack_by_slot(
    slot: u8,
    inventory_type: InventoryType,
) -> WeaponAttackType {
    match slot {
        EQUIPMENT_SLOT_MAINHAND => {
            if matches!(
                inventory_type,
                InventoryType::Ranged | InventoryType::RangedRight
            ) {
                WeaponAttackType::RangedAttack
            } else {
                WeaponAttackType::BaseAttack
            }
        }
        EQUIPMENT_SLOT_OFFHAND => WeaponAttackType::OffAttack,
        _ => WeaponAttackType::Max,
    }
}

pub(super) const fn item_mod_type_from_u32(value: u32) -> ItemModType {
    match value {
        0 => ItemModType::Mana,
        1 => ItemModType::Health,
        3 => ItemModType::Agility,
        4 => ItemModType::Strength,
        5 => ItemModType::Intellect,
        6 => ItemModType::Spirit,
        7 => ItemModType::Stamina,
        12 => ItemModType::DefenseSkillRating,
        13 => ItemModType::DodgeRating,
        14 => ItemModType::ParryRating,
        15 => ItemModType::BlockRating,
        16 => ItemModType::HitMeleeRating,
        17 => ItemModType::HitRangedRating,
        18 => ItemModType::HitSpellRating,
        19 => ItemModType::CritMeleeRating,
        20 => ItemModType::CritRangedRating,
        21 => ItemModType::CritSpellRating,
        28 => ItemModType::HasteMeleeRating,
        29 => ItemModType::HasteRangedRating,
        30 => ItemModType::HasteSpellRating,
        31 => ItemModType::HitRating,
        32 => ItemModType::CritRating,
        36 => ItemModType::HasteRating,
        37 => ItemModType::ExpertiseRating,
        38 => ItemModType::AttackPower,
        39 => ItemModType::RangedAttackPower,
        43 => ItemModType::ManaRegeneration,
        44 => ItemModType::ArmorPenetrationRating,
        45 => ItemModType::SpellPower,
        46 => ItemModType::HealthRegen,
        47 => ItemModType::SpellPenetration,
        48 => ItemModType::BlockValue,
        50 => ItemModType::ExtraArmor,
        51 => ItemModType::FireResistance,
        52 => ItemModType::FrostResistance,
        53 => ItemModType::HolyResistance,
        54 => ItemModType::ShadowResistance,
        55 => ItemModType::NatureResistance,
        56 => ItemModType::ArcaneResistance,
        71 => ItemModType::AgiStrInt,
        72 => ItemModType::AgiStr,
        73 => ItemModType::AgiInt,
        74 => ItemModType::StrInt,
        _ => ItemModType::None,
    }
}
