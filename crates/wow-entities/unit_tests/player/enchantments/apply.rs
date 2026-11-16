use super::fixtures::{
    CallbackMutation, Effects, Sources, enchantment, item_facts, skills_with_value,
};
use crate::player::enchantments::{
    SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP, SkillEnchantmentEffectOperation as Op,
    SkillEnchantmentError, SkillEnchantmentRecord,
};
use crate::{
    PlayerSkills, SkillEnchantmentApplyArgs, apply_all_enchantments_native_like_cpp,
    apply_enchantment_native_like_cpp,
};
use wow_constants::ItemEnchantmentType;
use wow_core::ObjectGuid;

fn record_with(
    id: u32,
    effects: [u32; 3],
    args: [u32; 3],
    condition: u32,
    required_skill: u32,
    required_rank: u32,
    min_level: u32,
) -> SkillEnchantmentRecord {
    SkillEnchantmentRecord::new(
        id,
        effects,
        [5, 6, 7],
        args,
        0,
        [0.0; 3],
        0,
        0,
        condition,
        required_skill,
        required_rank,
        min_level,
        0,
    )
}

fn fact(
    guid_number: u32,
    slot: u8,
    enchantment_slot: usize,
    id: u32,
) -> (ObjectGuid, crate::SkillEnchantmentItemFacts) {
    let mut enchantments = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    enchantments[enchantment_slot] = id;
    let guid = ObjectGuid::create_item(1, i64::from(guid_number));
    (
        guid,
        item_facts(
            guid_number,
            100,
            slot,
            true,
            false,
            enchantments,
            [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
            [1; 3],
            [None; 3],
        ),
    )
}

#[test]
fn required_skill_uses_bonus_inclusive_full_width_identity() {
    let skill = 0x1_2345;
    let (mut skills, sources, mut effects) = skills_with_value(skill, 5);
    skills
        .modify_bonus(&sources, skill, 2, false, || {})
        .unwrap();
    assert_eq!(skills.value(skill), 7);

    let (guid, item) = fact(1, crate::EQUIPMENT_SLOT_MAINHAND, 1, 10);
    effects.add_item(item);
    let mut sources = sources;
    sources.add_enchantment(record_with(
        10,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [901, 0, 0],
        0,
        skill,
        7,
        0,
    ));
    apply_enchantment_native_like_cpp(
        &mut skills,
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
        [Op::CastItemEquipSpell {
            spell_id: 901,
            item_guid: guid
        }]
    );

    let too_high = record_with(
        10,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [902, 0, 0],
        0,
        skill,
        8,
        0,
    );
    // The constructor preserves the full uint32 required-skill identity.
    assert_eq!(too_high.required_skill_id(), skill);
    let mut sources = sources;
    sources.enchantments.insert(10, too_high);
    effects.operations.clear();
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        1,
        true,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert!(effects.operations.is_empty());
}

#[test]
fn guard_order_keeps_condition_and_minimum_level_before_effects() {
    let (guid, item) = fact(2, crate::EQUIPMENT_SLOT_MAINHAND, 1, 20);
    let mut sources = Sources::default();
    sources.add_enchantment(record_with(
        20,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [920, 0, 0],
        99,
        0,
        0,
        80,
    ));
    let mut effects = Effects {
        level: 79,
        condition_fits: false,
        ..Effects::default()
    };
    effects.add_item(item);
    let mut skills = PlayerSkills::initialize([], || {});
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        1,
        true,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert!(effects.operations.is_empty());

    effects.condition_fits = true;
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        1,
        true,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert!(effects.operations.is_empty());

    let args = SkillEnchantmentApplyArgs::with_options(false, true);
    effects.level = 80;
    effects.condition_fits = false;
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        1,
        true,
        args,
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.operations,
        [Op::CastItemEquipSpell {
            spell_id: 920,
            item_guid: guid
        }]
    );
}

#[test]
fn socket_prismatic_failure_stops_apply_while_missing_gem_template_is_skipped() {
    let guid = ObjectGuid::create_item(1, 3);
    let mut enchantments = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    enchantments[2] = 30;
    enchantments[6] = 31;
    let mut gem_ids = [None; 3];
    gem_ids[0] = Some(303);
    let item = item_facts(
        3,
        103,
        crate::EQUIPMENT_SLOT_MAINHAND,
        true,
        false,
        enchantments,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [0, 1, 1],
        gem_ids,
    );
    let mut sources = Sources::default();
    sources.add_enchantment(record_with(
        30,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [930, 0, 0],
        0,
        0,
        0,
        0,
    ));
    sources.add_enchantment(record_with(31, [0; 3], [0; 3], 0, 0x1_0001, 20, 0));
    let mut effects = Effects::default();
    effects.add_item(item);
    let mut skills = PlayerSkills::initialize([], || {});
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        2,
        true,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert!(effects.operations.is_empty());

    // A present gem with no admitted item-template row does not invent a failure.
    let mut sources = Sources::default();
    sources.add_enchantment(record_with(
        30,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [930, 0, 0],
        0,
        0,
        0,
        0,
    ));
    let mut effects = Effects::default();
    effects.add_item(item_facts(
        3,
        103,
        crate::EQUIPMENT_SLOT_MAINHAND,
        true,
        false,
        {
            let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
            ids[2] = 30;
            ids
        },
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        gem_ids,
    ));
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        2,
        true,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.operations,
        [Op::CastItemEquipSpell {
            spell_id: 930,
            item_guid: guid
        }]
    );
}

#[test]
fn gem_template_skill_guard_uses_bonus_inclusive_full_width_skill() {
    const SKILL: u32 = 0x1_2345;
    let (mut skills, mut sources, mut effects) = skills_with_value(SKILL, 5);
    skills
        .modify_bonus(&sources, SKILL, 2, false, || {})
        .unwrap();
    let guid = ObjectGuid::create_item(1, 31);
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[2] = 310;
    let mut gems = [None; 3];
    gems[0] = Some(311);
    effects.add_item(item_facts(
        31,
        131,
        crate::EQUIPMENT_SLOT_MAINHAND,
        true,
        false,
        ids,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        gems,
    ));
    sources.add_enchantment(enchantment(
        310,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [0; 3],
        [9310, 0, 0],
    ));
    sources
        .item_templates
        .insert(311, crate::SkillEnchantmentItemTemplate::new(0, SKILL, 8));
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        2,
        true,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert!(effects.operations.is_empty());

    sources
        .item_templates
        .insert(311, crate::SkillEnchantmentItemTemplate::new(0, SKILL, 7));
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        2,
        true,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.operations,
        [Op::CastItemEquipSpell {
            spell_id: 9310,
            item_guid: guid
        }]
    );
}

#[test]
fn broken_item_skips_effect_columns_but_keeps_visual_and_duration_tail_order() {
    let guid = ObjectGuid::create_item(1, 4);
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[0] = 40;
    ids[12] = 41;
    let mut durations = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    durations[0] = 5000;
    durations[12] = 7000;
    let item = item_facts(4, 104, 4, true, true, ids, durations, [1; 3], [None; 3]);
    let mut sources = Sources::default();
    sources.add_enchantment(enchantment(
        40,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [0; 3],
        [940, 0, 0],
    ));
    sources.add_enchantment(enchantment(
        41,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [0; 3],
        [941, 0, 0],
    ));
    let mut effects = Effects::default();
    effects.add_item(item);
    let mut skills = PlayerSkills::initialize([], || {});
    apply_all_enchantments_native_like_cpp(&mut skills, Some(guid), true, &sources, &mut effects)
        .unwrap();
    assert_eq!(
        effects.operations,
        [
            Op::UpdateVisibleItemVisual {
                item_guid: guid,
                inventory_slot: 4
            },
            Op::AddEnchantmentDuration {
                item_guid: guid,
                slot: 0,
                duration_ms: 5000
            },
            Op::AddEnchantmentDuration {
                item_guid: guid,
                slot: 12,
                duration_ms: 7000
            },
        ]
    );

    effects.operations.clear();
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        0,
        false,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.operations,
        [
            Op::UpdateVisibleItemVisual {
                item_guid: guid,
                inventory_slot: 4
            },
            Op::AddEnchantmentDuration {
                item_guid: guid,
                slot: 0,
                duration_ms: 0
            },
        ]
    );
}

#[test]
fn all_slot_overload_applies_thirteen_slots_with_unoverrideable_native_defaults() {
    let guid = ObjectGuid::create_item(1, 42);
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[0] = 420;
    ids[12] = 421;
    let mut durations = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    durations[0] = 42_000;
    durations[12] = 42_100;
    let item = item_facts(42, 142, 4, true, false, ids, durations, [1; 3], [None; 3]);
    let mut sources = Sources::default();
    sources.add_enchantment(enchantment(
        420,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [0; 3],
        [9420, 0, 0],
    ));
    sources.add_enchantment(enchantment(
        421,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [0; 3],
        [9421, 0, 0],
    ));
    let mut effects = Effects::default();
    effects.add_item(item);
    apply_all_enchantments_native_like_cpp(
        &mut PlayerSkills::initialize([], || {}),
        Some(guid),
        true,
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.operations,
        [
            Op::CastItemEquipSpell {
                spell_id: 9420,
                item_guid: guid
            },
            Op::UpdateVisibleItemVisual {
                item_guid: guid,
                inventory_slot: 4
            },
            Op::AddEnchantmentDuration {
                item_guid: guid,
                slot: 0,
                duration_ms: 42_000
            },
            Op::CastItemEquipSpell {
                spell_id: 9421,
                item_guid: guid
            },
            Op::AddEnchantmentDuration {
                item_guid: guid,
                slot: 12,
                duration_ms: 42_100
            },
        ]
    );
}

#[test]
fn tail_reads_duration_after_callback_and_preserves_completed_prefix_on_failure() {
    let guid = ObjectGuid::create_item(1, 5);
    let (expected_guid, item) = fact(5, crate::EQUIPMENT_SLOT_MAINHAND, 1, 50);
    assert_eq!(guid, expected_guid);
    let mut sources = Sources::default();
    sources.add_enchantment(enchantment(
        50,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [0; 3],
        [950, 0, 0],
    ));
    let replacement = item_facts(
        5,
        105,
        crate::EQUIPMENT_SLOT_MAINHAND,
        true,
        false,
        *item.enchantment_ids(),
        {
            let mut values = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
            values[1] = 4321;
            values
        },
        [1; 3],
        [None; 3],
    );
    let mut effects = Effects::default();
    effects.add_item(item);
    effects
        .callback_mutations
        .insert(0, CallbackMutation::ReplaceItem(replacement));
    let mut skills = PlayerSkills::initialize([], || {});
    apply_enchantment_native_like_cpp(
        &mut skills,
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
            Op::CastItemEquipSpell {
                spell_id: 950,
                item_guid: guid
            },
            Op::AddEnchantmentDuration {
                item_guid: guid,
                slot: 1,
                duration_ms: 4321
            },
        ]
    );

    effects.operations.clear();
    effects.fail_operation_at = Some(1);
    let error = apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        1,
        false,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    );
    assert!(matches!(
        error,
        Err(SkillEnchantmentError::Effect("effect callback failure"))
    ));
    assert_eq!(
        effects.operations,
        [Op::RemoveAurasDueToItemSpell {
            spell_id: 950,
            item_guid: guid
        }]
    );
}

#[test]
fn callback_item_invalidation_is_an_explicit_error_with_prefix_retained() {
    let (guid, item) = fact(6, crate::EQUIPMENT_SLOT_MAINHAND, 1, 60);
    let mut sources = Sources::default();
    sources.add_enchantment(enchantment(
        60,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [0; 3],
        [960, 0, 0],
    ));
    let mut effects = Effects::default();
    effects.add_item(item);
    effects
        .callback_mutations
        .insert(0, CallbackMutation::RemoveItem(guid));
    let mut skills = PlayerSkills::initialize([], || {});
    let result = apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        1,
        true,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    );
    assert!(matches!(result, Err(SkillEnchantmentError::ItemInvalidated(found)) if found == guid));
    assert_eq!(
        effects.operations,
        [Op::CastItemEquipSpell {
            spell_id: 960,
            item_guid: guid
        }]
    );
}

#[test]
fn null_not_equipped_empty_and_out_of_range_items_return_without_source_effects() {
    let mut skills = PlayerSkills::initialize([], || {});
    let sources = Sources::default();
    let mut effects = Effects::default();
    assert!(
        apply_enchantment_native_like_cpp(
            &mut skills,
            None,
            u16::MAX,
            true,
            SkillEnchantmentApplyArgs::new(),
            &sources,
            &mut effects,
        )
        .is_ok()
    );
    let guid = ObjectGuid::create_item(1, 7);
    effects.add_item(item_facts(
        7,
        107,
        16,
        false,
        false,
        [u32::MAX; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        [None; 3],
    ));
    assert!(
        apply_enchantment_native_like_cpp(
            &mut skills,
            Some(guid),
            u16::MAX,
            true,
            SkillEnchantmentApplyArgs::new(),
            &sources,
            &mut effects,
        )
        .is_ok()
    );
    assert!(effects.operations.is_empty());

    let guid = ObjectGuid::create_item(1, 8);
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    let item = item_facts(
        8,
        108,
        crate::EQUIPMENT_SLOT_MAINHAND,
        true,
        false,
        ids,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        [None; 3],
    );
    let mut effects = Effects::default();
    effects.add_item(item);
    assert!(
        apply_enchantment_native_like_cpp(
            &mut skills,
            Some(guid),
            1,
            true,
            SkillEnchantmentApplyArgs::new(),
            &sources,
            &mut effects,
        )
        .is_ok()
    );
    assert!(effects.operations.is_empty());

    ids[1] = 101;
    let item = item_facts(
        8,
        108,
        crate::EQUIPMENT_SLOT_MAINHAND,
        true,
        false,
        ids,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        [None; 3],
    );
    let mut effects = Effects::default();
    effects.add_item(item);
    let mut sources = Sources::default();
    sources.missing_enchantment = Some(101);
    assert!(
        apply_enchantment_native_like_cpp(
            &mut skills,
            Some(guid),
            1,
            true,
            SkillEnchantmentApplyArgs::new(),
            &sources,
            &mut effects,
        )
        .is_ok()
    );
    assert!(effects.operations.is_empty());

    sources.missing_enchantment = None;
    sources.failed_enchantment = Some(101);
    assert!(
        apply_enchantment_native_like_cpp(
            &mut skills,
            Some(guid),
            13,
            true,
            SkillEnchantmentApplyArgs::new(),
            &sources,
            &mut effects,
        )
        .is_ok()
    );
    assert!(matches!(
        apply_enchantment_native_like_cpp(
            &mut skills,
            Some(guid),
            1,
            true,
            SkillEnchantmentApplyArgs::new(),
            &sources,
            &mut effects,
        ),
        Err(SkillEnchantmentError::Source("enchantment source failure"))
    ));
}

#[test]
fn all_slot_default_condition_gate_and_source_error_keep_exact_prefix() {
    let guid = ObjectGuid::create_item(1, 8);
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[0] = 80;
    let item = item_facts(
        8,
        108,
        4,
        true,
        false,
        ids,
        [100; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        [None; 3],
    );
    let mut sources = Sources::default();
    sources.add_enchantment(record_with(
        80,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [980, 0, 0],
        1,
        0,
        0,
        0,
    ));
    let mut effects = Effects {
        condition_fits: false,
        ..Effects::default()
    };
    effects.add_item(item);
    let mut skills = PlayerSkills::initialize([], || {});
    apply_all_enchantments_native_like_cpp(&mut skills, Some(guid), true, &sources, &mut effects)
        .unwrap();
    assert!(effects.operations.is_empty());

    let mut sources = Sources::default();
    sources.add_enchantment(enchantment(
        80,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [0; 3],
        [980, 0, 0],
    ));
    sources.add_enchantment(SkillEnchantmentRecord::new(
        81,
        [ItemEnchantmentType::Resistance as u32, 0, 0],
        [4, 0, 0],
        [2, 0, 0],
        0,
        [1.0; 3],
        1,
        0,
        0,
        0,
        0,
        0,
        1,
    ));
    ids[1] = 81;
    let item = item_facts(
        8,
        108,
        16,
        true,
        false,
        ids,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        [None; 3],
    );
    let mut effects = Effects::default();
    effects.add_item(item);
    sources.fail_scaling = true;
    let result = apply_all_enchantments_native_like_cpp(
        &mut skills,
        Some(guid),
        true,
        &sources,
        &mut effects,
    );
    assert!(matches!(
        result,
        Err(SkillEnchantmentError::Source("scaling source failure"))
    ));
    assert_eq!(
        effects.operations,
        [
            Op::CastItemEquipSpell {
                spell_id: 980,
                item_guid: guid
            },
            Op::UpdateVisibleItemVisual {
                item_guid: guid,
                inventory_slot: 16
            },
        ]
    );
}

#[test]
fn effect_columns_execute_in_source_order() {
    let (guid, item) = fact(81, crate::EQUIPMENT_SLOT_MAINHAND, 1, 810);
    let mut sources = Sources::default();
    sources.add_enchantment(enchantment(
        810,
        [ItemEnchantmentType::EquipSpell as u32; 3],
        [0; 3],
        [9810, 9811, 9812],
    ));
    let mut effects = Effects::default();
    effects.add_item(item);
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
            Op::CastItemEquipSpell {
                spell_id: 9810,
                item_guid: guid
            },
            Op::CastItemEquipSpell {
                spell_id: 9811,
                item_guid: guid
            },
            Op::CastItemEquipSpell {
                spell_id: 9812,
                item_guid: guid
            },
        ]
    );
}

#[test]
fn reentrant_operation_changes_the_same_skill_owner_seen_by_later_queries() {
    const SKILL: u32 = 70_007;
    let (mut skills, sources, mut effects) = skills_with_value(SKILL, 5);
    effects.level = 0;
    effects.level_from_skill = Some(SKILL);
    effects.callback_mutations.insert(
        0,
        CallbackMutation::AddTemporarySkillBonus {
            skill: SKILL,
            amount: 2,
        },
    );
    let guid = ObjectGuid::create_item(1, 9);
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[1] = 90;
    let item = item_facts(
        9,
        109,
        16,
        true,
        false,
        ids,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        [1; 3],
        [None; 3],
    );
    let mut sources = sources;
    sources.add_enchantment(SkillEnchantmentRecord::new(
        90,
        [
            ItemEnchantmentType::Stat as u32,
            ItemEnchantmentType::Stat as u32,
            0,
        ],
        [1, 1, 0],
        [3, 3, 0],
        0x20,
        [1.0, 1.0, 0.0],
        3,
        0,
        0,
        0,
        0,
        0,
        0,
    ));
    sources.scaling_row_count = 100;
    sources.scaling_columns.insert((5, 3), 2.0);
    sources.scaling_columns.insert((7, 3), 4.0);
    effects.add_item(item);
    apply_enchantment_native_like_cpp(
        &mut skills,
        Some(guid),
        1,
        true,
        SkillEnchantmentApplyArgs::new(),
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(skills.value(SKILL), 7);
    assert!(matches!(
        effects.operations.as_slice(),
        [
            Op::HandleStatFlatModifier { amount: 2.0, .. },
            Op::UpdateStatBuffMod(_),
            Op::HandleStatFlatModifier { amount: 4.0, .. },
            Op::UpdateStatBuffMod(_),
        ]
    ));
}
