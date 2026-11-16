use wow_constants::{InventoryType, ItemEnchantmentType, Stats, WeaponAttackType};

use super::scaling::amount_for_column;
use super::{
    PlayerSkills, SkillEnchantmentAttackPowerIndex, SkillEnchantmentAttackPowerModifier,
    SkillEnchantmentEffectOperation, SkillEnchantmentEffects, SkillEnchantmentError,
    SkillEnchantmentItemFacts, SkillEnchantmentItemTemplate, SkillEnchantmentRating,
    SkillEnchantmentRecord, SkillEnchantmentSource, SkillEnchantmentUnitMod,
    SkillEnchantmentUnitModifier,
};
use wow_core::ObjectGuid;

pub(super) fn apply_effect_column<S, E>(
    skills: &mut PlayerSkills,
    record: &SkillEnchantmentRecord,
    item: SkillEnchantmentItemFacts,
    slot: u16,
    column: usize,
    apply: bool,
    sources: &S,
    effects: &mut E,
) -> Result<(), SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    let effect_type = record.effects[column];
    let argument = record.effect_args[column];
    let operations = match effect_type {
        value
            if value == ItemEnchantmentType::None as u32
                || value == ItemEnchantmentType::CombatSpell as u32
                || value == ItemEnchantmentType::UseSpell as u32
                || value == ItemEnchantmentType::PrismaticSocket as u32
                || value == ItemEnchantmentType::ArtifactPowerBonusRankByType as u32
                || value == ItemEnchantmentType::ArtifactPowerBonusRankByID as u32
                || value == ItemEnchantmentType::BonusListID as u32
                || value == ItemEnchantmentType::BonusListCurve as u32
                || value == ItemEnchantmentType::ArtifactPowerBonusRankPicker as u32 =>
        {
            Vec::new()
        }
        value
            if value == ItemEnchantmentType::Damage as u32
                || value == ItemEnchantmentType::Totem as u32 =>
        {
            let template = item_template::<S, E>(sources, item.item_id)?;
            let attack_type =
                get_attack_by_slot_native(item.inventory_slot, template.inventory_type);
            if attack_type == WeaponAttackType::Max {
                Vec::new()
            } else {
                vec![SkillEnchantmentEffectOperation::UpdateDamageDoneMods {
                    attack_type,
                    modifier_slot: if apply { -1 } else { slot as i16 },
                }]
            }
        }
        value if value == ItemEnchantmentType::EquipSpell as u32 => {
            if argument == 0 {
                Vec::new()
            } else if apply {
                vec![SkillEnchantmentEffectOperation::CastItemEquipSpell {
                    spell_id: argument,
                    item_guid: item.item_guid,
                }]
            } else {
                vec![SkillEnchantmentEffectOperation::RemoveAurasDueToItemSpell {
                    spell_id: argument,
                    item_guid: item.item_guid,
                }]
            }
        }
        value if value == ItemEnchantmentType::Resistance as u32 => {
            let amount = amount_for_column(record, column, skills, sources, effects)?.max(1) as f32;
            vec![SkillEnchantmentEffectOperation::HandleStatFlatModifier {
                unit_mod: SkillEnchantmentUnitMod::Resistance(argument),
                modifier: SkillEnchantmentUnitModifier::TotalValue,
                amount,
                apply,
            }]
        }
        value if value == ItemEnchantmentType::Stat as u32 => {
            let amount = amount_for_column(record, column, skills, sources, effects)?.max(1);
            stat_operations(argument, amount, apply)
        }
        unknown => vec![SkillEnchantmentEffectOperation::LogUnknownEffect {
            enchantment_id: record.id,
            effect_type: unknown,
        }],
    };

    for operation in operations {
        apply_operation(skills, effects, item.item_guid, operation)?;
    }
    Ok(())
}

fn stat_operations(
    item_mod: u32,
    amount: i32,
    apply: bool,
) -> Vec<SkillEnchantmentEffectOperation> {
    use SkillEnchantmentAttackPowerIndex::{Melee, Ranged};
    use SkillEnchantmentRating as Rating;
    use SkillEnchantmentUnitMod as UnitMod;
    use SkillEnchantmentUnitModifier as Modifier;

    let simple = |unit_mod, modifier| {
        vec![SkillEnchantmentEffectOperation::HandleStatFlatModifier {
            unit_mod,
            modifier,
            amount: amount as f32,
            apply,
        }]
    };
    let rating = |rating| {
        vec![SkillEnchantmentEffectOperation::ApplyRatingMod {
            rating,
            amount,
            apply,
        }]
    };
    let multiple_ratings = |ratings: &[Rating]| {
        ratings
            .iter()
            .map(|rating| SkillEnchantmentEffectOperation::ApplyRatingMod {
                rating: *rating,
                amount,
                apply,
            })
            .collect()
    };

    match item_mod {
        0 => simple(UnitMod::Mana, Modifier::BaseValue),
        1 => simple(UnitMod::Health, Modifier::BaseValue),
        3 => primary_stat(Stats::Agility, UnitMod::Stat(Stats::Agility), amount, apply),
        4 => primary_stat(
            Stats::Strength,
            UnitMod::Stat(Stats::Strength),
            amount,
            apply,
        ),
        5 => primary_stat(
            Stats::Intellect,
            UnitMod::Stat(Stats::Intellect),
            amount,
            apply,
        ),
        6 => primary_stat(Stats::Spirit, UnitMod::Stat(Stats::Spirit), amount, apply),
        7 => primary_stat(Stats::Stamina, UnitMod::Stat(Stats::Stamina), amount, apply),
        12 => rating(Rating::DefenseSkill),
        13 => rating(Rating::Dodge),
        14 => rating(Rating::Parry),
        15 => rating(Rating::Block),
        16 => rating(Rating::HitMelee),
        17 => rating(Rating::HitRanged),
        18 => rating(Rating::HitSpell),
        19 => rating(Rating::CritMelee),
        20 => rating(Rating::CritRanged),
        21 => rating(Rating::CritSpell),
        30 => rating(Rating::HasteSpell),
        31 => multiple_ratings(&[Rating::HitMelee, Rating::HitRanged, Rating::HitSpell]),
        32 => multiple_ratings(&[Rating::CritMelee, Rating::CritRanged, Rating::CritSpell]),
        35 => rating(Rating::ResiliencePlayerDamage),
        36 => multiple_ratings(&[Rating::HasteMelee, Rating::HasteRanged, Rating::HasteSpell]),
        37 => rating(Rating::Expertise),
        38 => vec![
            attack_power(Melee, amount, apply),
            attack_power(Ranged, amount, apply),
        ],
        39 => vec![attack_power(Ranged, amount, apply)],
        40 => multiple_ratings(&[
            Rating::VersatilityDamageDone,
            Rating::VersatilityHealingDone,
            Rating::VersatilityDamageTaken,
        ]),
        43 => vec![SkillEnchantmentEffectOperation::ApplyManaRegenBonus { amount, apply }],
        44 => rating(Rating::ArmorPenetration),
        45 => vec![SkillEnchantmentEffectOperation::ApplySpellPowerBonus { amount, apply }],
        46 => vec![SkillEnchantmentEffectOperation::ApplyHealthRegenBonus { amount, apply }],
        47 => vec![SkillEnchantmentEffectOperation::ApplySpellPenetrationBonus { amount, apply }],
        48 => vec![SkillEnchantmentEffectOperation::HandleShieldBlockBaseMod {
            amount: amount as f32,
            apply,
        }],
        49 => rating(Rating::Mastery),
        _ => Vec::new(),
    }
}

fn primary_stat(
    stat: Stats,
    unit_mod: SkillEnchantmentUnitMod,
    amount: i32,
    apply: bool,
) -> Vec<SkillEnchantmentEffectOperation> {
    vec![
        SkillEnchantmentEffectOperation::HandleStatFlatModifier {
            unit_mod,
            modifier: SkillEnchantmentUnitModifier::TotalValue,
            amount: amount as f32,
            apply,
        },
        SkillEnchantmentEffectOperation::UpdateStatBuffMod(stat),
    ]
}

fn attack_power(
    index: SkillEnchantmentAttackPowerIndex,
    amount: i32,
    apply: bool,
) -> SkillEnchantmentEffectOperation {
    SkillEnchantmentEffectOperation::HandleAttackPowerModifier {
        index,
        kind: if amount >= 0 {
            SkillEnchantmentAttackPowerModifier::FlatPositive
        } else {
            SkillEnchantmentAttackPowerModifier::FlatNegative
        },
        amount: amount as f32,
        apply,
    }
}

pub(super) fn get_attack_by_slot_native(slot: u8, inventory_type: i32) -> WeaponAttackType {
    use crate::{EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND, EQUIPMENT_SLOT_RANGED};
    match slot {
        EQUIPMENT_SLOT_MAINHAND => {
            if inventory_type == InventoryType::Ranged as i32
                || inventory_type == InventoryType::RangedRight as i32
            {
                WeaponAttackType::RangedAttack
            } else {
                WeaponAttackType::BaseAttack
            }
        }
        EQUIPMENT_SLOT_OFFHAND => WeaponAttackType::OffAttack,
        EQUIPMENT_SLOT_RANGED => WeaponAttackType::RangedAttack,
        _ => WeaponAttackType::Max,
    }
}

fn item_template<S, E>(
    sources: &S,
    item_id: u32,
) -> Result<SkillEnchantmentItemTemplate, SkillEnchantmentError<S::Error, E::Error>>
where
    S: SkillEnchantmentSource,
    E: SkillEnchantmentEffects,
{
    sources
        .item_template(item_id)
        .map_err(SkillEnchantmentError::Source)?
        .ok_or(SkillEnchantmentError::MissingLiveItemTemplate(item_id))
}

pub(super) fn apply_operation<E, SError>(
    skills: &mut PlayerSkills,
    effects: &mut E,
    item_guid: ObjectGuid,
    operation: SkillEnchantmentEffectOperation,
) -> Result<(), SkillEnchantmentError<SError, E::Error>>
where
    E: SkillEnchantmentEffects,
{
    effects
        .apply_operation(skills, operation)
        .map_err(SkillEnchantmentError::Effect)?;
    if effects
        .item_by_guid(item_guid)
        .map_err(SkillEnchantmentError::Effect)?
        .is_none()
    {
        return Err(SkillEnchantmentError::ItemInvalidated(item_guid));
    }
    Ok(())
}
