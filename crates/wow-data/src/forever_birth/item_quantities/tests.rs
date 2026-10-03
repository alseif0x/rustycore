use super::*;

fn catalog(
    class: i32,
    subclass: u8,
    vendor_stack: u32,
    stackable: i32,
    effects: Vec<ItemEffectQuantityRecord>,
    relations: Vec<ItemEffectRelationRecord>,
) -> ItemQuantitySources {
    ItemQuantitySources::from_effective_records(
        vec![ItemClassQuantityRecord {
            id: 1,
            class,
            subclass,
        }],
        vec![ItemSparseQuantityRecord {
            id: 1,
            vendor_stack,
            stackable,
        }],
        effects,
        relations,
    )
    .unwrap()
}
fn effect(id: u32, legacy_slot: u8, spell_category: u16) -> ItemEffectQuantityRecord {
    ItemEffectQuantityRecord {
        id,
        legacy_slot,
        spell_category,
    }
}
fn relation(id: u32, effect: i32, item: u32) -> ItemEffectRelationRecord {
    ItemEffectRelationRecord { id, effect, item }
}

#[test]
fn quantities_follow_food_drink_dk_and_vendor_defaults() {
    let food = catalog(
        0,
        5,
        20,
        20,
        vec![effect(8, 0, 11)],
        vec![relation(1, 8, 1)],
    );
    assert_eq!(food.loadout_quantity(1, 1), Some(4));
    assert_eq!(food.loadout_quantity(1, 6), Some(10));
    let drink = catalog(
        0,
        5,
        20,
        20,
        vec![effect(8, 0, 59)],
        vec![relation(1, 8, 1)],
    );
    assert_eq!(drink.loadout_quantity(1, 6), Some(2));
    assert_eq!(
        catalog(0, 5, 0, 20, vec![], vec![]).loadout_quantity(1, 1),
        Some(1)
    );
    assert_eq!(
        catalog(0, 5, 7, 20, vec![effect(8, 0, 99)], vec![relation(1, 8, 1)])
            .loadout_quantity(1, 1),
        Some(7)
    );
}

#[test]
fn stack_clamp_is_only_for_food_drink_and_unlimited_preserves_source_sentinel() {
    for stack in [0, -1, i32::MIN, i32::MAX] {
        assert_eq!(
            catalog(0, 5, u32::MAX, stack, vec![], vec![]).loadout_quantity(1, 1),
            Some(0x7FFF_FFFE)
        );
    }
    assert_eq!(
        catalog(0, 5, 8, 3, vec![], vec![]).loadout_quantity(1, 1),
        Some(3)
    );
    for (class, subclass) in [(2, 5), (0, 4), (-1, 5)] {
        assert_eq!(
            catalog(class, subclass, u32::MAX, 1, vec![], vec![]).loadout_quantity(1, 1),
            Some(u32::MAX)
        );
    }
}

#[test]
fn effect_slot_order_and_equal_slot_reverse_relation_order_choose_first_category() {
    let sources = catalog(
        0,
        5,
        1,
        20,
        vec![effect(9, 2, 11), effect(2, 0, 11), effect(8, 0, 59)],
        // Deliberately nonascending input: source DB2 iteration is by ID.
        vec![
            relation(3, 8, 1),
            relation(1, 9, 1),
            relation(2, 2, 1),
            relation(4, 9, 1),
        ],
    );
    assert_eq!(sources.effects_by_item[&1], vec![8, 2, 9, 9]);
    assert_eq!(sources.loadout_quantity(1, 6), Some(2));
}

#[test]
fn dangling_effects_and_incomplete_item_join_do_not_manufacture_quantities() {
    let sources = ItemQuantitySources::from_effective_records(
        vec![ItemClassQuantityRecord {
            id: 1,
            class: 0,
            subclass: 5,
        }],
        vec![ItemSparseQuantityRecord {
            id: 2,
            vendor_stack: 1,
            stackable: 20,
        }],
        vec![effect(9, 0, 11)],
        vec![relation(1, 9, 1), relation(2, 9, 2)],
    )
    .unwrap();
    assert!(!sources.contains(1));
    assert!(!sources.contains(2));
    assert_eq!(sources.loadout_quantity(1, 1), None);
    assert_eq!(sources.loadout_quantity(2, 1), None);
    assert!(sources.effects_by_item.is_empty());
    let dangling = catalog(0, 5, 7, 20, vec![], vec![relation(1, 9, 1)]);
    assert_eq!(dangling.loadout_quantity(1, 1), Some(7));
}

#[test]
fn signed_effect_foreign_key_retains_uint32_lookup_bits() {
    let sources = catalog(
        0,
        5,
        1,
        20,
        vec![effect(u32::MAX, 0, 11)],
        vec![relation(1, -1, 1)],
    );
    assert_eq!(sources.loadout_quantity(1, 6), Some(10));
}

#[test]
fn every_final_source_rejects_duplicate_ids_but_equal_effect_relations_survive() {
    let basic = ItemClassQuantityRecord {
        id: 1,
        class: 0,
        subclass: 5,
    };
    let sparse = ItemSparseQuantityRecord {
        id: 1,
        vendor_stack: 1,
        stackable: 20,
    };
    assert!(
        ItemQuantitySources::from_effective_records(vec![basic, basic], vec![], vec![], vec![])
            .is_err()
    );
    assert!(
        ItemQuantitySources::from_effective_records(vec![], vec![sparse, sparse], vec![], vec![])
            .is_err()
    );
    assert!(
        ItemQuantitySources::from_effective_records(
            vec![],
            vec![],
            vec![effect(1, 0, 0); 2],
            vec![]
        )
        .is_err()
    );
    assert!(
        ItemQuantitySources::from_effective_records(
            vec![],
            vec![],
            vec![],
            vec![relation(1, 1, 1); 2]
        )
        .is_err()
    );
    let sources = catalog(
        0,
        5,
        1,
        20,
        vec![effect(1, 0, 11)],
        vec![relation(1, 1, 1), relation(2, 1, 1)],
    );
    assert_eq!(sources.effects_by_item[&1], vec![1, 1]);
    assert_eq!(sources.counts(), [1, 1, 1, 2]);
}
