//! Player item-effect action planning helpers.

use super::{
    ApplyEnchantmentBaseMod, ApplyEnchantmentCombatRating, ApplyEnchantmentEffectAction,
    ApplyEnchantmentEffectKind, ApplyEnchantmentEffectRef, ApplyEnchantmentRandomSuffixRef,
    ApplyEnchantmentUnitMod, ApplyEnchantmentUnitModifier, ArenaEnchantmentItemRef,
    RemoveArenaEnchantmentAction, SkillEnchantmentItemRef, UpdateSkillEnchantmentAction,
    UpdateSkillEnchantmentReason, WeaponDamageBoundLikeCpp, get_attack_by_slot,
    item_mod_type_from_u32,
};
use crate::{
    BASE_MAXDAMAGE, BASE_MINDAMAGE, EQUIPMENT_SLOT_MAINHAND, EQUIPMENT_SLOT_OFFHAND, Item,
    ItemStorageTemplate,
};
use wow_constants::{
    EnchantmentSlot, InventoryType, ItemEnchantmentType, ItemModType, Stats, WeaponAttackType,
    spell::SpellSchools,
};
use wow_core::ObjectGuid;

pub(super) fn arena_enchantment_ref_by_guid(
    items: &[ArenaEnchantmentItemRef],
    guid: ObjectGuid,
) -> Option<ArenaEnchantmentItemRef> {
    items.iter().find(|item| item.guid == guid).copied()
}

pub(super) fn push_arena_inventory_enchantment_action(
    actions: &mut Vec<RemoveArenaEnchantmentAction>,
    items: &[ArenaEnchantmentItemRef],
    item_guid: ObjectGuid,
    bag: u8,
    slot: u8,
    enchantment_slot: EnchantmentSlot,
) {
    match arena_enchantment_ref_by_guid(items, item_guid) {
        Some(item) if item.arena_allowed => {}
        Some(_) => actions.push(RemoveArenaEnchantmentAction::ClearInventoryEnchantment {
            item_guid,
            bag,
            slot,
            enchantment_slot,
        }),
        None => actions.push(RemoveArenaEnchantmentAction::MissingInventoryItemRef {
            item_guid,
            bag,
            slot,
            enchantment_slot,
        }),
    }
}

pub(super) const fn is_socket_enchantment_slot(slot: EnchantmentSlot) -> bool {
    matches!(
        slot,
        EnchantmentSlot::EnhancementSocket
            | EnchantmentSlot::EnhancementSocket2
            | EnchantmentSlot::EnhancementSocket3
    )
}

pub(super) fn apply_enchantment_effect_action(
    item: &Item,
    item_template: Option<&ItemStorageTemplate>,
    enchantment_slot: EnchantmentSlot,
    enchantment_id: i32,
    random_suffix: Option<ApplyEnchantmentRandomSuffixRef>,
    apply: bool,
    effect: ApplyEnchantmentEffectRef,
) -> Vec<ApplyEnchantmentEffectAction> {
    match effect.effect_kind {
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::None) => {
            vec![ApplyEnchantmentEffectAction::Noop]
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::CombatSpell) => {
            vec![ApplyEnchantmentEffectAction::DeferredCombatSpell]
        }
        ApplyEnchantmentEffectKind::Known(
            kind @ (ItemEnchantmentType::Damage | ItemEnchantmentType::Totem),
        ) => {
            let Some(template) = item_template else {
                return vec![ApplyEnchantmentEffectAction::MissingItemTemplateForAttack {
                    effect_kind: ApplyEnchantmentEffectKind::Known(kind),
                }];
            };
            let attack_type = get_attack_by_slot(item.slot(), template.inventory_type);
            if attack_type == WeaponAttackType::Max {
                vec![ApplyEnchantmentEffectAction::Noop]
            } else {
                vec![ApplyEnchantmentEffectAction::UpdateDamageDoneMods {
                    attack_type,
                    modifier_slot: if apply { -1 } else { enchantment_slot as i16 },
                }]
            }
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::EquipSpell) => {
            if effect.arg == 0 {
                vec![ApplyEnchantmentEffectAction::Noop]
            } else if apply {
                vec![ApplyEnchantmentEffectAction::CastEquipSpell {
                    spell_id: effect.arg,
                    item_guid: item.object().guid(),
                }]
            } else {
                vec![ApplyEnchantmentEffectAction::RemoveEquipSpellAura {
                    spell_id: effect.arg,
                    item_guid: item.object().guid(),
                }]
            }
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::Resistance) => {
            let amount =
                resolve_enchantment_effect_amount(item, enchantment_id, random_suffix, effect);
            vec![ApplyEnchantmentEffectAction::UnitModifier {
                unit_mod: ApplyEnchantmentUnitMod::Resistance(effect.arg),
                modifier: ApplyEnchantmentUnitModifier::TotalValue,
                amount,
                apply,
            }]
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::Stat) => {
            let amount =
                resolve_enchantment_effect_amount(item, enchantment_id, random_suffix, effect);
            apply_enchantment_stat_actions(item_mod_type_from_u32(effect.arg), amount, apply)
        }
        ApplyEnchantmentEffectKind::Known(ItemEnchantmentType::UseSpell) => {
            vec![ApplyEnchantmentEffectAction::DeferredUseSpell]
        }
        ApplyEnchantmentEffectKind::Known(
            ItemEnchantmentType::PrismaticSocket
            | ItemEnchantmentType::ArtifactPowerBonusRankByType
            | ItemEnchantmentType::ArtifactPowerBonusRankByID
            | ItemEnchantmentType::BonusListID
            | ItemEnchantmentType::BonusListCurve
            | ItemEnchantmentType::ArtifactPowerBonusRankPicker,
        ) => vec![ApplyEnchantmentEffectAction::Noop],
        ApplyEnchantmentEffectKind::Unknown(effect_type) => {
            vec![ApplyEnchantmentEffectAction::Unknown { effect_type }]
        }
    }
}

fn resolve_enchantment_effect_amount(
    item: &Item,
    enchantment_id: i32,
    random_suffix: Option<ApplyEnchantmentRandomSuffixRef>,
    effect: ApplyEnchantmentEffectRef,
) -> u32 {
    if effect.amount != 0
        || !matches!(
            effect.effect_kind,
            ApplyEnchantmentEffectKind::Known(
                ItemEnchantmentType::Resistance | ItemEnchantmentType::Stat
            )
        )
    {
        return effect.amount;
    }

    let Some(random_suffix) = random_suffix else {
        return effect.amount;
    };
    if item.data().random_properties_id.unsigned_abs() != random_suffix.id {
        return effect.amount;
    }

    random_suffix
        .amount_for(enchantment_id, item.data().property_seed)
        .unwrap_or(effect.amount)
}

fn apply_enchantment_stat_actions(
    item_mod: ItemModType,
    amount: u32,
    apply: bool,
) -> Vec<ApplyEnchantmentEffectAction> {
    match item_mod {
        ItemModType::Mana => vec![unit_modifier(
            ApplyEnchantmentUnitMod::Mana,
            ApplyEnchantmentUnitModifier::BaseValue,
            amount,
            apply,
        )],
        ItemModType::Health => vec![unit_modifier(
            ApplyEnchantmentUnitMod::Health,
            ApplyEnchantmentUnitModifier::BaseValue,
            amount,
            apply,
        )],
        ItemModType::Agility => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatAgility,
            Stats::Agility,
            amount,
            apply,
        ),
        ItemModType::Strength => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatStrength,
            Stats::Strength,
            amount,
            apply,
        ),
        ItemModType::Intellect => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatIntellect,
            Stats::Intellect,
            amount,
            apply,
        ),
        ItemModType::Spirit => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatSpirit,
            Stats::Spirit,
            amount,
            apply,
        ),
        ItemModType::Stamina => primary_stat_actions(
            ApplyEnchantmentUnitMod::StatStamina,
            Stats::Stamina,
            amount,
            apply,
        ),
        ItemModType::DefenseSkillRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::DefenseSkill], amount, apply)
        }
        ItemModType::DodgeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Dodge], amount, apply)
        }
        ItemModType::ParryRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Parry], amount, apply)
        }
        ItemModType::BlockRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Block], amount, apply)
        }
        ItemModType::HitMeleeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HitMelee], amount, apply)
        }
        ItemModType::HitRangedRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HitRanged], amount, apply)
        }
        ItemModType::HitSpellRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HitSpell], amount, apply)
        }
        ItemModType::CritMeleeRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::CritMelee], amount, apply)
        }
        ItemModType::CritRangedRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::CritRanged], amount, apply)
        }
        ItemModType::CritSpellRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::CritSpell], amount, apply)
        }
        ItemModType::HasteSpellRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::HasteSpell], amount, apply)
        }
        ItemModType::HitRating => rating_actions(
            &[
                ApplyEnchantmentCombatRating::HitMelee,
                ApplyEnchantmentCombatRating::HitRanged,
                ApplyEnchantmentCombatRating::HitSpell,
            ],
            amount,
            apply,
        ),
        ItemModType::CritRating => rating_actions(
            &[
                ApplyEnchantmentCombatRating::CritMelee,
                ApplyEnchantmentCombatRating::CritRanged,
                ApplyEnchantmentCombatRating::CritSpell,
            ],
            amount,
            apply,
        ),
        ItemModType::HasteRating => rating_actions(
            &[
                ApplyEnchantmentCombatRating::HasteMelee,
                ApplyEnchantmentCombatRating::HasteRanged,
                ApplyEnchantmentCombatRating::HasteSpell,
            ],
            amount,
            apply,
        ),
        ItemModType::ExpertiseRating => {
            rating_actions(&[ApplyEnchantmentCombatRating::Expertise], amount, apply)
        }
        ItemModType::AttackPower => vec![
            unit_modifier(
                ApplyEnchantmentUnitMod::AttackPower,
                ApplyEnchantmentUnitModifier::TotalValue,
                amount,
                apply,
            ),
            unit_modifier(
                ApplyEnchantmentUnitMod::AttackPowerRanged,
                ApplyEnchantmentUnitModifier::TotalValue,
                amount,
                apply,
            ),
        ],
        ItemModType::RangedAttackPower => vec![unit_modifier(
            ApplyEnchantmentUnitMod::AttackPowerRanged,
            ApplyEnchantmentUnitModifier::TotalValue,
            amount,
            apply,
        )],
        ItemModType::ManaRegeneration => {
            vec![ApplyEnchantmentEffectAction::ManaRegenBonus { amount, apply }]
        }
        ItemModType::ArmorPenetrationRating => rating_actions(
            &[ApplyEnchantmentCombatRating::ArmorPenetration],
            amount,
            apply,
        ),
        ItemModType::SpellPower => {
            vec![ApplyEnchantmentEffectAction::SpellPowerBonus { amount, apply }]
        }
        ItemModType::HealthRegen => {
            vec![ApplyEnchantmentEffectAction::HealthRegenBonus { amount, apply }]
        }
        ItemModType::SpellPenetration => {
            vec![ApplyEnchantmentEffectAction::SpellPenetrationBonus { amount, apply }]
        }
        ItemModType::BlockValue => vec![ApplyEnchantmentEffectAction::BaseModFlatValue {
            base_mod: ApplyEnchantmentBaseMod::ShieldBlockValue,
            amount,
            apply,
        }],
        _ => vec![ApplyEnchantmentEffectAction::UnhandledStatModifier {
            item_mod,
            amount,
            apply,
        }],
    }
}

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

fn primary_stat_actions(
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

fn unit_modifier(
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

fn rating_actions(
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

pub(super) fn skill_enchantment_transition(
    curr_value: u16,
    new_value: u16,
    required_skill_rank: u16,
) -> Option<bool> {
    if curr_value < required_skill_rank && new_value >= required_skill_rank {
        Some(true)
    } else if new_value < required_skill_rank && curr_value >= required_skill_rank {
        Some(false)
    } else {
        None
    }
}

pub(super) fn push_update_skill_enchantment_action(
    actions: &mut Vec<UpdateSkillEnchantmentAction>,
    item: SkillEnchantmentItemRef,
    enchantment_slot: EnchantmentSlot,
    enchantment_id: i32,
    reason: UpdateSkillEnchantmentReason,
    apply: bool,
) {
    let action = if apply {
        UpdateSkillEnchantmentAction::Apply {
            item_guid: item.item_guid,
            inventory_slot: item.inventory_slot,
            enchantment_slot,
            enchantment_id,
            reason,
        }
    } else {
        UpdateSkillEnchantmentAction::Remove {
            item_guid: item.item_guid,
            inventory_slot: item.inventory_slot,
            enchantment_slot,
            enchantment_id,
            reason,
        }
    };
    actions.push(action);
}
