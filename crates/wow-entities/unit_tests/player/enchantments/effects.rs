use super::fixtures::{Effects, Sources, enchantment, item_facts};
use crate::player::enchantments::effects::get_attack_by_slot_native;
use crate::player::enchantments::{
    SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP, SkillEnchantmentAttackPowerIndex as AttackPowerIndex,
    SkillEnchantmentAttackPowerModifier as AttackPowerModifier,
    SkillEnchantmentEffectOperation as Op, SkillEnchantmentRating as Rating,
    SkillEnchantmentUnitMod as UnitMod, SkillEnchantmentUnitModifier as Modifier,
};
use crate::{PlayerSkills, SkillEnchantmentApplyArgs, apply_enchantment_native_like_cpp};
use wow_constants::{InventoryType, ItemEnchantmentType, Stats, WeaponAttackType};
use wow_core::ObjectGuid;

fn apply_first_column(
    effect: u32,
    argument: u32,
    amount: i32,
    inventory_slot: u8,
    inventory_type: i32,
    apply: bool,
) -> Vec<Op> {
    let item_id = 100;
    let item_guid = ObjectGuid::create_item(1, 1);
    let mut enchantment_ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    enchantment_ids[1] = 7;
    let item = item_facts(
        1,
        item_id,
        inventory_slot,
        true,
        false,
        enchantment_ids,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        [None; 3],
    );
    let mut sources = Sources::default();
    sources.add_enchantment(enchantment(
        7,
        [effect, 0, 0],
        [amount, 0, 0],
        [argument, 0, 0],
    ));
    sources.add_template(item_id, inventory_type);
    let mut effects = Effects::default();
    effects.add_item(item);
    apply_enchantment_native_like_cpp(
        &mut PlayerSkills::initialize([], || {}),
        Some(item_guid),
        1,
        apply,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .expect("admitted operation succeeds");
    effects.operations
}

fn stat_op(unit_mod: UnitMod, modifier: Modifier, amount: f32) -> Vec<Op> {
    vec![
        Op::HandleStatFlatModifier {
            unit_mod,
            modifier,
            amount,
            apply: true,
        },
        Op::UpdateStatBuffMod(match unit_mod {
            UnitMod::Stat(stat) => stat,
            _ => unreachable!("only primary stats have stat-buff updates"),
        }),
    ]
}

#[test]
fn target_attack_routing_includes_native_ranged_slot_and_only_two_mainhand_inventory_types() {
    assert_eq!(
        get_attack_by_slot_native(crate::EQUIPMENT_SLOT_RANGED, -19),
        WeaponAttackType::RangedAttack
    );
    assert_eq!(
        get_attack_by_slot_native(crate::EQUIPMENT_SLOT_MAINHAND, InventoryType::Ranged as i32,),
        WeaponAttackType::RangedAttack
    );
    assert_eq!(
        get_attack_by_slot_native(
            crate::EQUIPMENT_SLOT_MAINHAND,
            InventoryType::RangedRight as i32,
        ),
        WeaponAttackType::RangedAttack
    );
    assert_eq!(
        get_attack_by_slot_native(crate::EQUIPMENT_SLOT_MAINHAND, -19),
        WeaponAttackType::BaseAttack
    );
    assert_eq!(
        get_attack_by_slot_native(crate::EQUIPMENT_SLOT_OFFHAND, -19),
        WeaponAttackType::OffAttack
    );
    assert_eq!(get_attack_by_slot_native(34, 999), WeaponAttackType::Max);

    assert_eq!(
        apply_first_column(
            ItemEnchantmentType::Damage as u32,
            0,
            4,
            crate::EQUIPMENT_SLOT_RANGED,
            InventoryType::Ranged as i32,
            true,
        ),
        [Op::UpdateDamageDoneMods {
            attack_type: WeaponAttackType::RangedAttack,
            modifier_slot: -1,
        }]
    );
    assert_eq!(
        apply_first_column(
            ItemEnchantmentType::Totem as u32,
            0,
            4,
            crate::EQUIPMENT_SLOT_MAINHAND,
            InventoryType::RangedRight as i32,
            false,
        ),
        [
            Op::UpdateDamageDoneMods {
                attack_type: WeaponAttackType::RangedAttack,
                modifier_slot: 1,
            },
            Op::AddEnchantmentDuration {
                item_guid: ObjectGuid::create_item(1, 1),
                slot: 1,
                duration_ms: 0,
            }
        ]
    );
}

#[test]
fn all_native_stat_columns_have_exact_operations_and_group_order() {
    let expected = [
        (
            0,
            vec![Op::HandleStatFlatModifier {
                unit_mod: UnitMod::Mana,
                modifier: Modifier::BaseValue,
                amount: 7.0,
                apply: true,
            }],
        ),
        (
            1,
            vec![Op::HandleStatFlatModifier {
                unit_mod: UnitMod::Health,
                modifier: Modifier::BaseValue,
                amount: 7.0,
                apply: true,
            }],
        ),
        (
            3,
            stat_op(UnitMod::Stat(Stats::Agility), Modifier::TotalValue, 7.0),
        ),
        (
            4,
            stat_op(UnitMod::Stat(Stats::Strength), Modifier::TotalValue, 7.0),
        ),
        (
            5,
            stat_op(UnitMod::Stat(Stats::Intellect), Modifier::TotalValue, 7.0),
        ),
        (
            6,
            stat_op(UnitMod::Stat(Stats::Spirit), Modifier::TotalValue, 7.0),
        ),
        (
            7,
            stat_op(UnitMod::Stat(Stats::Stamina), Modifier::TotalValue, 7.0),
        ),
        (
            12,
            vec![Op::ApplyRatingMod {
                rating: Rating::DefenseSkill,
                amount: 7,
                apply: true,
            }],
        ),
        (
            13,
            vec![Op::ApplyRatingMod {
                rating: Rating::Dodge,
                amount: 7,
                apply: true,
            }],
        ),
        (
            14,
            vec![Op::ApplyRatingMod {
                rating: Rating::Parry,
                amount: 7,
                apply: true,
            }],
        ),
        (
            15,
            vec![Op::ApplyRatingMod {
                rating: Rating::Block,
                amount: 7,
                apply: true,
            }],
        ),
        (
            16,
            vec![Op::ApplyRatingMod {
                rating: Rating::HitMelee,
                amount: 7,
                apply: true,
            }],
        ),
        (
            17,
            vec![Op::ApplyRatingMod {
                rating: Rating::HitRanged,
                amount: 7,
                apply: true,
            }],
        ),
        (
            18,
            vec![Op::ApplyRatingMod {
                rating: Rating::HitSpell,
                amount: 7,
                apply: true,
            }],
        ),
        (
            19,
            vec![Op::ApplyRatingMod {
                rating: Rating::CritMelee,
                amount: 7,
                apply: true,
            }],
        ),
        (
            20,
            vec![Op::ApplyRatingMod {
                rating: Rating::CritRanged,
                amount: 7,
                apply: true,
            }],
        ),
        (
            21,
            vec![Op::ApplyRatingMod {
                rating: Rating::CritSpell,
                amount: 7,
                apply: true,
            }],
        ),
        (
            30,
            vec![Op::ApplyRatingMod {
                rating: Rating::HasteSpell,
                amount: 7,
                apply: true,
            }],
        ),
        (
            31,
            [Rating::HitMelee, Rating::HitRanged, Rating::HitSpell]
                .map(|rating| Op::ApplyRatingMod {
                    rating,
                    amount: 7,
                    apply: true,
                })
                .to_vec(),
        ),
        (
            32,
            [Rating::CritMelee, Rating::CritRanged, Rating::CritSpell]
                .map(|rating| Op::ApplyRatingMod {
                    rating,
                    amount: 7,
                    apply: true,
                })
                .to_vec(),
        ),
        (
            35,
            vec![Op::ApplyRatingMod {
                rating: Rating::ResiliencePlayerDamage,
                amount: 7,
                apply: true,
            }],
        ),
        (
            36,
            [Rating::HasteMelee, Rating::HasteRanged, Rating::HasteSpell]
                .map(|rating| Op::ApplyRatingMod {
                    rating,
                    amount: 7,
                    apply: true,
                })
                .to_vec(),
        ),
        (
            37,
            vec![Op::ApplyRatingMod {
                rating: Rating::Expertise,
                amount: 7,
                apply: true,
            }],
        ),
        (
            38,
            [AttackPowerIndex::Melee, AttackPowerIndex::Ranged]
                .map(|index| Op::HandleAttackPowerModifier {
                    index,
                    kind: AttackPowerModifier::FlatPositive,
                    amount: 7.0,
                    apply: true,
                })
                .to_vec(),
        ),
        (
            39,
            vec![Op::HandleAttackPowerModifier {
                index: AttackPowerIndex::Ranged,
                kind: AttackPowerModifier::FlatPositive,
                amount: 7.0,
                apply: true,
            }],
        ),
        (
            40,
            [
                Rating::VersatilityDamageDone,
                Rating::VersatilityHealingDone,
                Rating::VersatilityDamageTaken,
            ]
            .map(|rating| Op::ApplyRatingMod {
                rating,
                amount: 7,
                apply: true,
            })
            .to_vec(),
        ),
        (
            43,
            vec![Op::ApplyManaRegenBonus {
                amount: 7,
                apply: true,
            }],
        ),
        (
            44,
            vec![Op::ApplyRatingMod {
                rating: Rating::ArmorPenetration,
                amount: 7,
                apply: true,
            }],
        ),
        (
            45,
            vec![Op::ApplySpellPowerBonus {
                amount: 7,
                apply: true,
            }],
        ),
        (
            46,
            vec![Op::ApplyHealthRegenBonus {
                amount: 7,
                apply: true,
            }],
        ),
        (
            47,
            vec![Op::ApplySpellPenetrationBonus {
                amount: 7,
                apply: true,
            }],
        ),
        (
            48,
            vec![Op::HandleShieldBlockBaseMod {
                amount: 7.0,
                apply: true,
            }],
        ),
        (
            49,
            vec![Op::ApplyRatingMod {
                rating: Rating::Mastery,
                amount: 7,
                apply: true,
            }],
        ),
    ];

    for (item_mod, expected_operations) in expected {
        assert_eq!(
            apply_first_column(ItemEnchantmentType::Stat as u32, item_mod, 7, 16, 0, true),
            expected_operations,
            "ITEM_MOD {item_mod}"
        );
    }
    for item_mod in [2, 8, 11, 22, 29, 33, 34, 41, 42, 50, u32::MAX] {
        assert!(
            apply_first_column(ItemEnchantmentType::Stat as u32, item_mod, 7, 16, 0, true)
                .is_empty(),
            "unknown stat {item_mod} has no gameplay callback"
        );
    }
}

#[test]
fn no_op_types_stay_local_while_unknown_outer_type_is_reported_and_later_column_runs() {
    let no_op_types = [
        ItemEnchantmentType::None,
        ItemEnchantmentType::CombatSpell,
        ItemEnchantmentType::UseSpell,
        ItemEnchantmentType::PrismaticSocket,
        ItemEnchantmentType::ArtifactPowerBonusRankByType,
        ItemEnchantmentType::ArtifactPowerBonusRankByID,
        ItemEnchantmentType::BonusListID,
        ItemEnchantmentType::BonusListCurve,
        ItemEnchantmentType::ArtifactPowerBonusRankPicker,
    ];
    for kind in no_op_types {
        assert!(apply_first_column(kind as u32, 1, 5, 16, 0, true).is_empty());
    }

    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[1] = 8;
    let mut sources = Sources::default();
    sources.add_enchantment(enchantment(
        8,
        [999, ItemEnchantmentType::Stat as u32, 0],
        [0, 6, 0],
        [0, 0, 0],
    ));
    let mut effects = Effects::default();
    effects.add_item(item_facts(
        2,
        200,
        16,
        true,
        false,
        ids,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        [None; 3],
    ));
    let guid = ObjectGuid::create_item(1, 2);
    apply_enchantment_native_like_cpp(
        &mut PlayerSkills::initialize([], || {}),
        Some(guid),
        1,
        true,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.operations,
        [
            Op::LogUnknownEffect {
                enchantment_id: 8,
                effect_type: 999
            },
            Op::HandleStatFlatModifier {
                unit_mod: UnitMod::Mana,
                modifier: Modifier::BaseValue,
                amount: 6.0,
                apply: true,
            },
        ]
    );
}

#[test]
fn equip_spell_keeps_item_provenance_and_remove_uses_same_spell() {
    let item_id = 300;
    let guid = ObjectGuid::create_item(1, 3);
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[1] = 9;
    let item = item_facts(
        3,
        item_id,
        16,
        true,
        false,
        ids,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        [None; 3],
    );
    let mut sources = Sources::default();
    sources.add_enchantment(enchantment(
        9,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [0; 3],
        [9001, 0, 0],
    ));
    let mut effects = Effects::default();
    effects.add_item(item);
    let mut skills = PlayerSkills::initialize([], || {});
    for apply in [true, false] {
        apply_enchantment_native_like_cpp(
            &mut skills,
            Some(guid),
            1,
            apply,
            SkillEnchantmentApplyArgs::new(),
            &sources,
            &mut effects,
        )
        .unwrap();
    }
    assert_eq!(
        effects.operations,
        [
            Op::CastItemEquipSpell {
                spell_id: 9001,
                item_guid: guid
            },
            Op::RemoveAurasDueToItemSpell {
                spell_id: 9001,
                item_guid: guid
            },
            Op::AddEnchantmentDuration {
                item_guid: guid,
                slot: 1,
                duration_ms: 0,
            },
        ]
    );
}
