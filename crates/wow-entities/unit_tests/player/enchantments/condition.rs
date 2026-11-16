use super::super::{
    EnchantmentConditionFacts, EnchantmentConditionSource, GemConditionEvaluationError,
    GemConditionOperandSide, enchantment_condition_fits_native_like_cpp,
};
use crate::{
    EQUIPMENT_SLOT_END, Item, PlayerInventoryRuntime, PlayerInventoryStorage, SocketedGem,
};
use std::collections::BTreeMap;
use wow_core::ObjectGuid;

#[derive(Default)]
struct Sources {
    conditions: BTreeMap<u32, EnchantmentConditionFacts>,
    gem_colors: BTreeMap<u32, u32>,
    missing_condition: Option<u32>,
    failed_condition: Option<u32>,
    failed_gem: Option<u32>,
}

impl EnchantmentConditionSource for Sources {
    type Error = &'static str;

    fn condition(&self, id: u32) -> Result<Option<EnchantmentConditionFacts>, Self::Error> {
        if self.failed_condition == Some(id) {
            return Err("condition source unavailable");
        }
        if self.missing_condition == Some(id) {
            return Ok(None);
        }
        Ok(self.conditions.get(&id).copied())
    }

    fn gem_color_mask(&self, item_entry: u32) -> Result<Option<u32>, Self::Error> {
        if self.failed_gem == Some(item_entry) {
            return Err("gem source unavailable");
        }
        Ok(self.gem_colors.get(&item_entry).copied())
    }
}

fn condition(
    id: u32,
    left_types: [u8; 5],
    operators: [u8; 5],
    right_types: [u8; 5],
    right_values: [u8; 5],
) -> EnchantmentConditionFacts {
    EnchantmentConditionFacts::new(id, left_types, operators, right_types, right_values)
}

fn item(entry: u32, serial: i64, gems: &[i32], broken: bool) -> Item {
    let guid = ObjectGuid::create_item(1, serial);
    let mut item = Item::new(0);
    item.object_mut().create(guid);
    item.object_mut().set_entry(entry);
    item.set_max_durability(100);
    item.set_durability(if broken { 0 } else { 100 });
    item.set_gems(
        gems.iter()
            .map(|item_id| SocketedGem {
                item_id: *item_id,
                ..SocketedGem::default()
            })
            .collect(),
    );
    item
}

fn install(
    inventory: &mut PlayerInventoryStorage,
    runtime: &mut PlayerInventoryRuntime,
    slot: u8,
    item: Item,
) {
    let guid = item.object().guid();
    inventory.items[usize::from(slot)] = Some(guid);
    runtime.store_item_object_like_cpp(item);
}

fn evaluate(
    inventory: &PlayerInventoryStorage,
    runtime: &PlayerInventoryRuntime,
    id: u32,
    except_slot: i8,
    sources: &Sources,
) -> Result<bool, GemConditionEvaluationError<&'static str>> {
    enchantment_condition_fits_native_like_cpp(inventory, runtime, id, except_slot, sources)
}

#[test]
fn zero_and_missing_conditions_return_before_inventory_and_metadata_reads() {
    let mut inventory = PlayerInventoryStorage::default();
    inventory.items[0] = Some(ObjectGuid::create_item(1, 1));
    let runtime = PlayerInventoryRuntime::default();
    let mut sources = Sources {
        missing_condition: Some(41),
        failed_condition: Some(0),
        ..Default::default()
    };

    assert!(evaluate(&inventory, &runtime, 0, -1, &sources).unwrap());
    assert!(evaluate(&inventory, &runtime, 41, -1, &sources).unwrap());

    sources.failed_condition = Some(42);
    assert!(matches!(
        evaluate(
            &PlayerInventoryStorage::default(),
            &runtime,
            42,
            -1,
            &sources
        ),
        Err(GemConditionEvaluationError::Source(
            "condition source unavailable"
        ))
    ));
}

#[test]
fn missing_live_item_is_an_error_but_slot_nineteen_is_outside_equipment() {
    let mut sources = Sources::default();
    sources
        .conditions
        .insert(1, condition(1, [1; 5], [5; 5], [0; 5], [0; 5]));
    let mut inventory = PlayerInventoryStorage::default();
    inventory.items[0] = Some(ObjectGuid::create_item(1, 2));
    assert!(matches!(
        evaluate(
            &inventory,
            &PlayerInventoryRuntime::default(),
            1,
            -1,
            &sources
        ),
        Err(GemConditionEvaluationError::MissingLiveItem { slot: 0, .. })
    ));

    let mut inventory = PlayerInventoryStorage::default();
    let mut runtime = PlayerInventoryRuntime::default();
    install(
        &mut inventory,
        &mut runtime,
        EQUIPMENT_SLOT_END,
        item(9, 3, &[77], false),
    );
    sources.gem_colors.insert(77, 1);
    sources
        .conditions
        .insert(2, condition(2, [1, 0, 0, 0, 0], [5; 5], [0; 5], [1; 5]));
    assert!(!evaluate(&inventory, &runtime, 2, -1, &sources).unwrap());
}

#[test]
fn equipment_range_except_slot_and_broken_items_use_live_item_facts() {
    let mut inventory = PlayerInventoryStorage::default();
    let mut runtime = PlayerInventoryRuntime::default();
    install(&mut inventory, &mut runtime, 0, item(10, 10, &[1], false));
    install(&mut inventory, &mut runtime, 1, item(11, 11, &[2], true));
    install(&mut inventory, &mut runtime, 18, item(12, 12, &[3], false));
    install(&mut inventory, &mut runtime, 19, item(13, 13, &[4], false));

    let mut sources = Sources::default();
    sources.gem_colors.extend([(1, 1), (2, 2), (3, 2), (4, 1)]);
    sources
        .conditions
        .insert(1, condition(1, [1, 0, 0, 0, 0], [5; 5], [0; 5], [1; 5]));
    sources
        .conditions
        .insert(2, condition(2, [2, 0, 0, 0, 0], [5; 5], [0; 5], [1; 5]));
    sources
        .conditions
        .insert(3, condition(3, [1, 0, 0, 0, 0], [5; 5], [0; 5], [2; 5]));

    assert!(evaluate(&inventory, &runtime, 1, -1, &sources).unwrap());
    assert!(!evaluate(&inventory, &runtime, 1, 0, &sources).unwrap());
    assert!(evaluate(&inventory, &runtime, 2, -1, &sources).unwrap());
    assert!(!evaluate(&inventory, &runtime, 2, 18, &sources).unwrap());
    assert!(!evaluate(&inventory, &runtime, 3, -1, &sources).unwrap());
}

#[test]
fn every_gem_duplicate_and_full_width_item_id_contributes_independently() {
    let mut inventory = PlayerInventoryStorage::default();
    let mut runtime = PlayerInventoryRuntime::default();
    install(
        &mut inventory,
        &mut runtime,
        0,
        item(20, 20, &[7, 7, -1, 8, 9], false),
    );
    let mut sources = Sources::default();
    sources.gem_colors.extend([(7, 1), (u32::MAX, 2), (8, 4)]);
    sources
        .conditions
        .insert(1, condition(1, [1, 0, 0, 0, 0], [5; 5], [0; 5], [2; 5]));
    sources
        .conditions
        .insert(2, condition(2, [2, 0, 0, 0, 0], [5; 5], [0; 5], [1; 5]));
    sources
        .conditions
        .insert(3, condition(3, [3, 0, 0, 0, 0], [5; 5], [0; 5], [1; 5]));

    assert!(evaluate(&inventory, &runtime, 1, -1, &sources).unwrap());
    assert!(evaluate(&inventory, &runtime, 2, -1, &sources).unwrap());
    assert!(evaluate(&inventory, &runtime, 3, -1, &sources).unwrap());

    sources.failed_gem = Some(u32::MAX);
    assert!(matches!(
        evaluate(&inventory, &runtime, 1, -1, &sources),
        Err(GemConditionEvaluationError::Source(
            "gem source unavailable"
        ))
    ));
}

#[test]
fn color_bits_count_independently_and_u8_counters_wrap() {
    let mut inventory = PlayerInventoryStorage::default();
    let mut runtime = PlayerInventoryRuntime::default();
    install(&mut inventory, &mut runtime, 0, item(30, 30, &[1], false));
    let mut sources = Sources::default();
    sources.gem_colors.insert(1, 0x8000_000f);
    sources
        .conditions
        .insert(1, condition(1, [1, 2, 3, 4, 0], [5; 5], [0; 5], [1; 5]));
    assert!(evaluate(&inventory, &runtime, 1, -1, &sources).unwrap());

    let mut inventory = PlayerInventoryStorage::default();
    let mut runtime = PlayerInventoryRuntime::default();
    install(
        &mut inventory,
        &mut runtime,
        0,
        item(31, 31, &vec![2; 256], false),
    );
    sources.gem_colors = BTreeMap::from([(2, 1)]);
    sources
        .conditions
        .insert(2, condition(2, [1, 0, 0, 0, 0], [5; 5], [0; 5], [1; 5]));
    assert!(!evaluate(&inventory, &runtime, 2, -1, &sources).unwrap());
}

#[test]
fn comparisons_and_all_rows_preserve_operand_and_and_order() {
    let mut inventory = PlayerInventoryStorage::default();
    let mut runtime = PlayerInventoryRuntime::default();
    install(
        &mut inventory,
        &mut runtime,
        0,
        item(40, 40, &[4, 4, 5], false),
    );
    let mut sources = Sources::default();
    sources.gem_colors.extend([(4, 1), (5, 2)]);
    sources
        .conditions
        .insert(1, condition(1, [1, 0, 0, 0, 0], [2; 5], [0; 5], [3; 5]));
    sources
        .conditions
        .insert(2, condition(2, [1, 0, 0, 0, 0], [3; 5], [0; 5], [1; 5]));
    sources
        .conditions
        .insert(3, condition(3, [1, 0, 0, 0, 0], [5; 5], [0; 5], [2; 5]));
    assert!(evaluate(&inventory, &runtime, 1, -1, &sources).unwrap());
    assert!(evaluate(&inventory, &runtime, 2, -1, &sources).unwrap());
    assert!(evaluate(&inventory, &runtime, 3, -1, &sources).unwrap());

    sources.conditions.insert(
        4,
        condition(4, [1, 0, 0, 0, 0], [3; 5], [2, 0, 0, 0, 0], [255; 5]),
    );
    assert!(evaluate(&inventory, &runtime, 4, -1, &sources).unwrap());

    sources
        .conditions
        .insert(7, condition(7, [1; 5], [5; 5], [0; 5], [1, 2, 1, 2, 3]));
    assert!(!evaluate(&inventory, &runtime, 7, -1, &sources).unwrap());

    let left_zero = condition(8, [0; 5], [99; 5], [u8::MAX; 5], [0; 5]);
    sources.conditions.insert(8, left_zero);
    assert!(evaluate(&inventory, &runtime, 8, -1, &sources).unwrap());

    sources.conditions.insert(
        5,
        condition(5, [1, 5, 0, 0, 0], [2, 0, 0, 0, 0], [0; 5], [1; 5]),
    );
    assert!(matches!(
        evaluate(&inventory, &runtime, 5, -1, &sources),
        Err(GemConditionEvaluationError::InvalidOperandType {
            row: 1,
            side: GemConditionOperandSide::Left,
            operand_type: 5
        })
    ));

    sources.conditions.insert(
        6,
        condition(
            6,
            [1, 0, 0, 0, 0],
            [2, 0, 0, 0, 0],
            [255, 0, 0, 0, 0],
            [0; 5],
        ),
    );
    assert!(matches!(
        evaluate(&inventory, &runtime, 6, -1, &sources),
        Err(GemConditionEvaluationError::InvalidOperandType {
            row: 0,
            side: GemConditionOperandSide::Right,
            operand_type: 255
        })
    ));
}

#[test]
fn unknown_operators_resolve_operands_without_changing_the_result() {
    let inventory = PlayerInventoryStorage::default();
    let runtime = PlayerInventoryRuntime::default();
    let mut sources = Sources::default();
    sources.conditions.insert(
        1,
        condition(1, [1, 0, 0, 0, 0], [99; 5], [255, 0, 0, 0, 0], [0; 5]),
    );
    assert!(matches!(
        evaluate(&inventory, &runtime, 1, -1, &sources),
        Err(GemConditionEvaluationError::InvalidOperandType {
            row: 0,
            side: GemConditionOperandSide::Right,
            operand_type: 255
        })
    ));
    sources.conditions.insert(
        2,
        condition(2, [1, 0, 0, 0, 0], [99; 5], [2, 0, 0, 0, 0], [0; 5]),
    );
    assert!(evaluate(&inventory, &runtime, 2, -1, &sources).unwrap());
}
