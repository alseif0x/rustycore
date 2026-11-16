use super::fixtures::{CallbackMutation, Effects, Sources, item_facts, skills_with_value};
use crate::player::enchantments::{
    SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP, SkillEnchantmentEffectOperation as Op,
    SkillEnchantmentError,
};
use crate::{PlayerSkills, update_skill_enchantments_native_like_cpp};
use wow_constants::ItemEnchantmentType;
use wow_core::ObjectGuid;

fn ids_with(slot: usize, id: u32) -> [u32; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP] {
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[slot] = id;
    ids
}

fn skill_enchantment(id: u32, skill: u32, rank: u32, spell: u32) -> crate::SkillEnchantmentRecord {
    crate::SkillEnchantmentRecord::new(
        id,
        [ItemEnchantmentType::EquipSpell as u32, 0, 0],
        [0; 3],
        [spell, 0, 0],
        0,
        [0.0; 3],
        0,
        0,
        0,
        skill,
        rank,
        0,
        0,
    )
}

fn item(
    guid_number: u32,
    slot: u8,
    enchantments: [u32; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
    socket_colors: [u32; 3],
    equipped: bool,
) -> crate::SkillEnchantmentItemFacts {
    item_facts(
        guid_number,
        200 + guid_number,
        slot,
        equipped,
        false,
        enchantments,
        [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP],
        socket_colors,
        [None; 3],
    )
}

#[test]
fn scan_includes_non_equipped_reagent_slot_34_and_excludes_slot_35() {
    let mut source = Sources::default();
    source.failed_enchantment = Some(3400);
    let mut effects = Effects::default();
    effects.add_item(item(34, 34, ids_with(0, 3400), [1; 3], false));
    let mut skills = PlayerSkills::initialize([], || {});
    let error =
        update_skill_enchantments_native_like_cpp(&mut skills, 55, 0, 1, &source, &mut effects);
    assert!(matches!(
        error,
        Err(SkillEnchantmentError::Source("enchantment source failure"))
    ));

    let mut source = Sources::default();
    source.failed_enchantment = Some(3500);
    let mut effects = Effects::default();
    effects.add_item(item(35, 35, ids_with(0, 3500), [1; 3], false));
    assert!(
        update_skill_enchantments_native_like_cpp(&mut skills, 55, 0, 1, &source, &mut effects,)
            .is_ok()
    );
    assert!(effects.operations.is_empty());
}

#[test]
fn scan_visits_all_thirteen_enchant_slots_and_stops_on_missing_direct_row() {
    const SKILL: u32 = 55;
    let (mut skills, mut sources, mut effects) = skills_with_value(SKILL, 8);
    let guid = ObjectGuid::create_item(1, 170);
    sources.add_enchantment(skill_enchantment(1712, SKILL, 8, 1712));
    effects.add_item(item(
        170,
        crate::EQUIPMENT_SLOT_RANGED,
        ids_with(12, 1712),
        [1; 3],
        true,
    ));
    update_skill_enchantments_native_like_cpp(
        &mut skills,
        SKILL as u16,
        0,
        8,
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.operations,
        [Op::CastItemEquipSpell {
            spell_id: 1712,
            item_guid: guid
        }]
    );

    let mut sources = Sources::default();
    sources.missing_enchantment = Some(1800);
    sources.add_enchantment(skill_enchantment(1801, SKILL, 1, 1801));
    let mut effects = Effects::default();
    effects.add_item(item(
        180,
        crate::EQUIPMENT_SLOT_RANGED,
        ids_with(0, 1800),
        [1; 3],
        true,
    ));
    effects.add_item(item(
        181,
        crate::EQUIPMENT_SLOT_RANGED + 1,
        ids_with(0, 1801),
        [1; 3],
        true,
    ));
    assert!(
        update_skill_enchantments_native_like_cpp(
            &mut skills,
            SKILL as u16,
            0,
            8,
            &sources,
            &mut effects,
        )
        .is_ok()
    );
    assert!(effects.operations.is_empty());
}

#[test]
fn missing_prismatic_row_skips_only_that_socket_branch_and_both_crossings_apply_same_socket() {
    const SKILL: u32 = 56;
    let (mut skills, mut sources, mut effects) = skills_with_value(SKILL, 20);
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[2] = 2002;
    ids[3] = 2003;
    ids[6] = 2006;
    sources.missing_enchantment = Some(2006);
    sources.add_enchantment(skill_enchantment(2002, SKILL, 99, 2002));
    sources.add_enchantment(skill_enchantment(2003, SKILL, 10, 2003));
    effects.add_item(item(
        200,
        crate::EQUIPMENT_SLOT_RANGED,
        ids,
        [0, 1, 1],
        true,
    ));
    update_skill_enchantments_native_like_cpp(
        &mut skills,
        SKILL as u16,
        0,
        20,
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.operations,
        [Op::CastItemEquipSpell {
            spell_id: 2003,
            item_guid: ObjectGuid::create_item(1, 200),
        }]
    );

    let (mut skills, mut sources, mut effects) = skills_with_value(SKILL, 20);
    let mut ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    ids[2] = 2012;
    ids[6] = 2016;
    sources.add_enchantment(skill_enchantment(2012, SKILL, 2, 2012));
    sources.add_enchantment(skill_enchantment(2016, SKILL, 4, 2016));
    effects.add_item(item(
        201,
        crate::EQUIPMENT_SLOT_RANGED,
        ids,
        [0, 1, 1],
        true,
    ));
    update_skill_enchantments_native_like_cpp(
        &mut skills,
        SKILL as u16,
        0,
        20,
        &sources,
        &mut effects,
    )
    .unwrap();
    let guid = ObjectGuid::create_item(1, 201);
    assert_eq!(
        effects.operations,
        [
            Op::CastItemEquipSpell {
                spell_id: 2012,
                item_guid: guid
            },
            Op::CastItemEquipSpell {
                spell_id: 2012,
                item_guid: guid
            },
            Op::CastItemEquipSpell {
                spell_id: 2016,
                item_guid: guid
            },
        ]
    );
}

#[test]
fn scan_re_reads_live_item_enchantments_after_callback_and_reports_invalidation() {
    const SKILL: u32 = 57;
    let (mut skills, mut sources, mut effects) = skills_with_value(SKILL, 5);
    sources.add_enchantment(skill_enchantment(2101, SKILL, 5, 2101));
    sources.add_enchantment(skill_enchantment(2102, SKILL, 5, 2102));
    let first = item(
        210,
        crate::EQUIPMENT_SLOT_RANGED,
        ids_with(1, 2101),
        [1; 3],
        true,
    );
    let mut next_ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    next_ids[2] = 2102;
    let second = item(210, crate::EQUIPMENT_SLOT_RANGED, next_ids, [1; 3], true);
    effects.add_item(first);
    effects
        .callback_mutations
        .insert(0, CallbackMutation::ReplaceItem(second));
    update_skill_enchantments_native_like_cpp(
        &mut skills,
        SKILL as u16,
        0,
        5,
        &sources,
        &mut effects,
    )
    .unwrap();
    let guid = ObjectGuid::create_item(1, 210);
    assert_eq!(
        effects.operations,
        [
            Op::CastItemEquipSpell {
                spell_id: 2101,
                item_guid: guid
            },
            Op::CastItemEquipSpell {
                spell_id: 2102,
                item_guid: guid
            },
        ]
    );

    let (mut skills, mut sources, mut effects) = skills_with_value(SKILL, 5);
    sources.add_enchantment(skill_enchantment(2201, SKILL, 5, 2201));
    let item = item(
        220,
        crate::EQUIPMENT_SLOT_RANGED,
        ids_with(1, 2201),
        [1; 3],
        true,
    );
    let guid = ObjectGuid::create_item(1, 220);
    effects.add_item(item);
    effects
        .callback_mutations
        .insert(0, CallbackMutation::RemoveItem(guid));
    let result = update_skill_enchantments_native_like_cpp(
        &mut skills,
        SKILL as u16,
        0,
        5,
        &sources,
        &mut effects,
    );
    assert!(matches!(result, Err(SkillEnchantmentError::ItemInvalidated(found)) if found == guid));
    assert_eq!(
        effects.operations,
        [Op::CastItemEquipSpell {
            spell_id: 2201,
            item_guid: guid
        }]
    );
}

#[test]
fn scan_observes_a_new_canonical_item_in_a_later_slot_added_by_an_earlier_callback() {
    const SKILL: u32 = 59;
    let (mut skills, mut sources, mut effects) = skills_with_value(SKILL, 5);
    sources.add_enchantment(skill_enchantment(2401, SKILL, 5, 2401));
    sources.add_enchantment(skill_enchantment(2402, SKILL, 5, 2402));
    let first = item(
        241,
        crate::EQUIPMENT_SLOT_RANGED,
        ids_with(1, 2401),
        [1; 3],
        true,
    );
    let second = item(
        240,
        crate::EQUIPMENT_SLOT_TABARD,
        ids_with(0, 2402),
        [1; 3],
        true,
    );
    effects.add_item(first);
    effects
        .callback_mutations
        .insert(0, CallbackMutation::ReplaceItem(second));
    update_skill_enchantments_native_like_cpp(
        &mut skills,
        SKILL as u16,
        0,
        5,
        &sources,
        &mut effects,
    )
    .unwrap();
    let first_guid = ObjectGuid::create_item(1, 241);
    let second_guid = ObjectGuid::create_item(1, 240);
    assert_eq!(
        effects.operations,
        [
            Op::CastItemEquipSpell {
                spell_id: 2401,
                item_guid: first_guid
            },
            Op::CastItemEquipSpell {
                spell_id: 2402,
                item_guid: second_guid
            },
            Op::UpdateVisibleItemVisual {
                item_guid: second_guid,
                inventory_slot: crate::EQUIPMENT_SLOT_TABARD
            },
        ]
    );
}

#[test]
fn direct_missing_row_returns_normally_with_prior_effect_prefix() {
    const SKILL: u32 = 58;
    let (mut skills, mut sources, mut effects) = skills_with_value(SKILL, 5);
    let mut first_ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    first_ids[1] = 2301;
    let mut later_ids = [0; SKILL_ENCHANTMENT_SLOT_COUNT_LIKE_CPP];
    later_ids[0] = 2302;
    sources.add_enchantment(skill_enchantment(2301, SKILL, 5, 2301));
    sources.missing_enchantment = Some(2302);
    let first = item(231, crate::EQUIPMENT_SLOT_RANGED, first_ids, [1; 3], true);
    let later = item(
        230,
        crate::EQUIPMENT_SLOT_RANGED + 1,
        later_ids,
        [1; 3],
        true,
    );
    effects.add_item(later);
    effects.add_item(first);
    update_skill_enchantments_native_like_cpp(
        &mut skills,
        SKILL as u16,
        0,
        5,
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.operations,
        [Op::CastItemEquipSpell {
            spell_id: 2301,
            item_guid: ObjectGuid::create_item(1, 231),
        }]
    );
}

#[test]
fn downward_crossing_removes_with_default_duration_and_equal_boundaries_do_not_cross() {
    const SKILL: u32 = 60;
    let (mut skills, mut sources, mut effects) = skills_with_value(SKILL, 5);
    let guid = ObjectGuid::create_item(1, 260);
    sources.add_enchantment(skill_enchantment(2601, SKILL, 5, 2601));
    effects.add_item(item(
        260,
        crate::EQUIPMENT_SLOT_RANGED,
        ids_with(1, 2601),
        [1; 3],
        true,
    ));
    update_skill_enchantments_native_like_cpp(
        &mut skills,
        SKILL as u16,
        5,
        4,
        &sources,
        &mut effects,
    )
    .unwrap();
    assert_eq!(
        effects.operations,
        [
            Op::RemoveAurasDueToItemSpell {
                spell_id: 2601,
                item_guid: guid,
            },
            Op::AddEnchantmentDuration {
                item_guid: guid,
                slot: 1,
                duration_ms: 0,
            },
        ]
    );

    for (old, new) in [(5, 5), (4, 4), (5, 6)] {
        let mut effects = Effects::default();
        effects.add_item(item(
            260,
            crate::EQUIPMENT_SLOT_RANGED,
            ids_with(1, 2601),
            [1; 3],
            true,
        ));
        update_skill_enchantments_native_like_cpp(
            &mut skills,
            SKILL as u16,
            old,
            new,
            &sources,
            &mut effects,
        )
        .unwrap();
        assert!(
            effects.operations.is_empty(),
            "{old} -> {new} is not a crossing"
        );
    }
}
