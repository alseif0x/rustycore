mod fixtures;
use super::*;
use fixtures::{basic, sparse};
use wow_data::{
    Db2HotfixRemovalStoreLikeCpp,
    forever_birth::{
        item_quantities::ItemEffectRelationRecord,
        item_records::{ItemEffectRecord, ItemRecords},
        item_specs::{ItemSpecOverrideRecord, ItemSpecRecord, ItemSpecRecords},
    },
    forever_initialization::{InitializationRecords, SpecializationRecord},
};
fn removals() -> Db2HotfixRemovalStoreLikeCpp {
    Default::default()
}
fn spec(id: u32, class: u8, index: i8) -> SpecializationRecord {
    SpecializationRecord {
        id,
        class,
        order_index: index,
        pet_talent_type: 0,
        role: 0,
        flags: 0,
        primary_stat_priority: 0,
        mastery_spells: [0; 2],
    }
}
#[test]
fn durability_level_28_transition_zero_token_quality_and_source_short_circuits() {
    use durability::maximum as d;
    assert_eq!(d(4, 1, 5, 0, 1).unwrap(), 55);
    assert_eq!(d(4, 1, 5, 0, 28).unwrap(), 110);
    assert_eq!(d(4, 1, 5, 0, 29).unwrap(), 115);
    assert_eq!(d(2, 1, 17, 0, 28).unwrap(), 80);
    assert_eq!(d(2, 1, 17, 0, 29).unwrap(), 85);
    assert_eq!(d(4, 1, 5, 5, 29).unwrap(), 200);
    for quality in [6, 7, 8] {
        assert_eq!(d(2, 1, 17, quality, 29).unwrap(), 0);
    }
    assert_eq!(d(0, 255, 255, 255, 1).unwrap(), 0);
    assert_eq!(d(4, 255, 21, 255, 1).unwrap(), 0);
    assert_eq!(
        d(4, 1, 5, 9, 1),
        Err(ItemTemplateError::InvalidDurabilityQuality)
    );
    assert_eq!(
        d(2, 21, 17, 1, 1),
        Err(ItemTemplateError::InvalidDurabilityWeapon)
    );
}
#[test]
fn spec_stat_types_preserve_cloak_relic_capacity_dedup_and_non_generalized_mods() {
    use spec_stats::SpecStats as S;
    assert_eq!(S::fields(4, 1, 16, [-1; 10], None).item_type, 0);
    assert!(S::fields(4, 1, 16, [-1; 10], None).has(27));
    assert_eq!(S::fields(4, 1, 5, [-1; 10], None).item_type, 1);
    for subclass in 7..=11 {
        assert!(S::fields(4, subclass, 28, [-1; 10], None).has(23));
    }
    let stats = S::fields(3, 11, 0, [71; 10], Some(0x1FFC0));
    assert!(stats.has(29) && stats.has(38));
    assert!(!stats.has(39) && !stats.has(1));
    let stats = S::fields(0, 0, 0, [71, 72, 73, 74, 19, 20, 21, 32, 36, 31], None);
    for s in [0, 1, 2, 24, 25, 4, 40] {
        assert!(stats.has(s));
    }
    let ignored = S::fields(0, 0, 0, [16, 17, 18, 28, 29, 30, 6, 7, -1, -2], None);
    assert!(!ignored.has(4) && !ignored.has(25) && !ignored.has(3));
}
#[test]
fn source_spec_matching_uses_five_bits_min_level_ignored_and_three_max_level_ranges() {
    let mut rows = ItemSpecRecords::default();
    for (id, max_level) in [(1, 40), (2, 41), (3, 110)] {
        rows.specs.push(ItemSpecRecord {
            id,
            min_level: 255,
            max_level,
            item_type: 1,
            primary: 40,
            secondary: 40,
            specialization: id as u16,
        });
    }
    let rows = rows
        .finish(Default::default(), Default::default(), &removals())
        .unwrap();
    let specs = [spec(1, 2, 4), spec(2, 2, 0), spec(3, 2, 1)];
    let (mask, sets) = specializations::sets(&basic(7), &sparse(7), &rows, |id| {
        specs.iter().find(|r| r.id == id)
    })
    .unwrap();
    assert_eq!(mask, 2);
    assert_eq!(
        sets,
        [(1 << 9) | (1 << 5) | (1 << 6), (1 << 5) | (1 << 6), 1 << 6]
    );
    let mut restricted = sparse(7);
    restricted.allowable_class = 1;
    let (mask, sets) = specializations::sets(&basic(7), &restricted, &rows, |id| {
        specs.iter().find(|r| r.id == id)
    })
    .unwrap();
    assert_eq!(mask, 0);
    assert_eq!(sets, [(1u128 << 80) - 1; 3]);
}

#[test]
fn undefined_specialization_class_shift_or_bit_index_fails_closed() {
    let rows = ItemSpecRecords {
        overrides: vec![ItemSpecOverrideRecord {
            id: 1,
            item: 7,
            specialization: 1,
        }],
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &removals())
    .unwrap();
    for (class, index) in [(0, 0), (16, 0), (1, -1), (1, 5)] {
        let invalid = spec(1, class, index);
        assert_eq!(
            specializations::sets(&basic(7), &sparse(7), &rows, |_| Some(&invalid)),
            Err(ItemTemplateError::InvalidSpecialization)
        );
    }
    let last = spec(1, 15, 4);
    let (mask, sets) =
        specializations::sets(&basic(7), &sparse(7), &rows, |_| Some(&last)).unwrap();
    assert_eq!(mask, 1 << 14);
    assert_eq!(sets, [1 << 74; 3]);
}
#[test]
fn override_branch_ignores_allowable_class_and_unresolved_override_never_falls_back() {
    let rows = ItemSpecRecords {
        specs: vec![ItemSpecRecord {
            id: 1,
            min_level: 1,
            max_level: 255,
            item_type: 1,
            primary: 40,
            secondary: 40,
            specialization: 1,
        }],
        overrides: vec![
            ItemSpecOverrideRecord {
                id: 1,
                item: 7,
                specialization: 2,
            },
            ItemSpecOverrideRecord {
                id: 2,
                item: 8,
                specialization: 99,
            },
        ],
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &removals())
    .unwrap();
    let specs = [spec(1, 1, 0), spec(2, 2, 4)];
    let mut restricted = sparse(7);
    restricted.allowable_class = 1;
    let (mask, sets) = specializations::sets(&basic(7), &restricted, &rows, |id| {
        specs.iter().find(|r| r.id == id)
    })
    .unwrap();
    assert_eq!(mask, 2);
    assert_eq!(sets, [1 << 9; 3]);
    let (mask, sets) = specializations::sets(&basic(8), &sparse(8), &rows, |id| {
        specs.iter().find(|r| r.id == id)
    })
    .unwrap();
    assert_eq!(mask, 0);
    assert_eq!(sets, [(1u128 << 80) - 1; 3]);
}
#[test]
fn templates_join_both_sources_order_duplicate_effects_swap_addons_and_leave_no_fake_template() {
    let effect = |id, slot| ItemEffectRecord {
        id,
        legacy_slot: slot,
        trigger: 0,
        charges: 0,
        cooldown: 0,
        category_cooldown: 0,
        spell_category: 0,
        spell: 0,
        specialization: 0,
        player_condition: 0,
    };
    let data = ItemRecords {
        items: vec![basic(7), basic(8)],
        sparse: vec![sparse(7), sparse(9)],
        effects: vec![effect(1, 1), effect(2, 1)],
        relations: vec![
            ItemEffectRelationRecord {
                id: 1,
                item: 7,
                effect: 1,
            },
            ItemEffectRelationRecord {
                id: 2,
                item: 7,
                effect: 2,
            },
            ItemEffectRelationRecord {
                id: 3,
                item: 7,
                effect: 1,
            },
            ItemEffectRelationRecord {
                id: 4,
                item: 7,
                effect: 99,
            },
        ],
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &removals())
    .unwrap();
    let specs = ItemSpecRecords::default()
        .finish(Default::default(), Default::default(), &removals())
        .unwrap();
    let init = InitializationRecords::default()
        .finish(Default::default(), Default::default(), &removals())
        .unwrap();
    let addon = |id, min_money, max_money| ItemAddonRow {
        id,
        flags: 42,
        food: 3,
        min_money,
        max_money,
        spell_ppm: 1.25,
        random_bonus_template: 99,
        quest_log_item: -1,
    };
    let rows =
        NumericItemTemplates::load(data, &specs, &init, vec![addon(7, 10, 3), addon(9, 1, 2)])
            .unwrap();
    assert_eq!(rows.template_count(), 1);
    assert!(rows.template(8).is_none() && rows.template(9).is_none());
    let row = rows.template(7).unwrap();
    assert_eq!(row.metadata.max_durability, 55);
    assert_eq!(row.metadata.min_money, 3);
    assert_eq!(row.metadata.max_money, 10);
    assert_eq!(row.metadata.quest_log_item, -1);
    assert_eq!(row.metadata.specializations, [(1u128 << 80) - 1; 3]);
    assert_eq!(
        rows.effects(7).map(|r| r.id).collect::<Vec<_>>(),
        vec![1, 2, 1]
    );
    assert!(rows.quantity_projection().unwrap().contains(7));
    assert!(!rows.quantity_projection().unwrap().contains(8));
}
