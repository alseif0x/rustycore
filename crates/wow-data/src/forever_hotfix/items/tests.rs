use super::*;
use crate::forever_birth::{
    item_quantities::ItemEffectRelationRecord,
    item_records::ItemRecords,
    item_sparse::SparseItemStrings,
    item_specs::{GemPropertiesRecord, ItemSpecOverrideRecord, ItemSpecRecord, ItemSpecRecords},
};
use crate::forever_hotfix::{
    ForeverHotfixCatalog, ForeverTactKeys, RecordError, TACT_KEY_TABLE_HASH,
};
use crate::{Db2HotfixRemovalStoreLikeCpp, HotfixBlobCache};
use std::collections::BTreeMap;

fn v(field: usize, array: usize) -> u32 {
    match field {
        6 => 0x7FC0_1234, // source preserves NaN payload
        5..=34 => 0x8000_0000 | ((field as u32) << 8) | array as u32,
        35..=51 => 0x8000 | ((field as u32) << 4) | array as u32,
        _ => 0x80 | (field + array) as u32,
    }
}
fn sparse_row(id: u32) -> SparseItemRecord {
    let n = |field| v(field, 0);
    let f = |field| f32::from_bits(n(field));
    SparseItemRecord {
        id,
        strings: SparseItemStrings::for_locale(
            6,
            [
                b"description".to_vec(),
                b"\xff".to_vec(),
                vec![],
                b"\0hidden".to_vec(),
                b"name".to_vec(),
            ],
        ),
        expansion: n(5) as i32,
        damage_variance: f(6),
        limit_category: n(7) as i32,
        duration: n(8),
        quality_modifier: f(9),
        bag_family: n(10),
        start_quest: n(11) as i32,
        language: n(12) as i32,
        item_range: f(13),
        socket_percentage: std::array::from_fn(|i| f32::from_bits(v(14, i))),
        stat_percent: std::array::from_fn(|i| v(15, i) as i32),
        stat_bonus: std::array::from_fn(|i| v(16, i) as i32),
        stackable: n(17) as i32,
        max_count: n(18) as i32,
        min_reputation: n(19) as i32,
        required_ability: n(20),
        allowable_race: u64::from(v(21, 0)) | (u64::from(v(21, 1)) << 32),
        sell_price: n(22),
        buy_price: n(23),
        vendor_stack: n(24),
        price_variance: f(25),
        price_random: f(26),
        flags: std::array::from_fn(|i| v(27, i) as i32),
        faction_related: n(28) as i32,
        modified_crafting_reagent: n(29) as i32,
        content_tuning: n(30) as i32,
        player_level_curve: n(31) as i32,
        item_level_offset_curve: n(32) as i32,
        item_level_offset: n(33) as i32,
        squish_era: n(34) as i32,
        name_description: n(35) as u16,
        transmog_holiday: n(36) as u16,
        holiday: n(37) as u16,
        gem_properties: n(38) as u16,
        socket_enchantment: n(39) as u16,
        totem_category: n(40) as u16,
        instance_bound: n(41) as u16,
        zone_bound: [v(42, 0) as u16, v(42, 1) as u16],
        item_set: n(43) as u16,
        lock: n(44) as u16,
        page: n(45) as u16,
        delay: n(46) as u16,
        min_faction: n(47) as u16,
        required_skill_rank: n(48) as u16,
        required_skill: n(49) as u16,
        item_level: n(50) as u16,
        allowable_class: n(51) as i16,
        artifact: n(52) as u8,
        spell_weight: n(53) as u8,
        spell_weight_category: n(54) as u8,
        socket_type: std::array::from_fn(|i| v(55, i) as u8),
        sheathe: n(56) as u8,
        material: n(57) as u8,
        page_material: n(58) as u8,
        bonding: n(59) as u8,
        damage_type: n(60) as u8,
        container_slots: n(61) as u8,
        required_pvp_medal: n(62) as u8,
        required_pvp_rank: n(63) as i8,
        required_level: n(64) as i8,
        inventory_type: n(65) as i8,
        quality: n(66) as i8,
        ammunition: n(67) as u8,
    }
}

#[test]
fn sparse_metadata_order_all_arrays_raw_bits_cstrings_and_external_id_exclusion() {
    // Independent frozen Meta field counts/widths, not serializer helpers.
    let mut numeric = Vec::new();
    for field in 5..68 {
        let count = match field {
            14..=16 => 10,
            21 | 42 => 2,
            27 => 5,
            55 => 3,
            _ => 1,
        };
        let width = match field {
            5..=34 => 4,
            35..=51 => 2,
            _ => 1,
        };
        for array in 0..count {
            numeric.extend_from_slice(&v(field, array).to_le_bytes()[..width]);
        }
    }
    assert_eq!(numeric.len(), 302);
    let row = sparse_row(0x1234_5678);
    let expected = [
        b"description\0\xff\0\0\0name\0".as_slice(),
        numeric.as_slice(),
    ]
    .concat();
    assert_eq!(sparse(&row, 6), expected);
    assert_eq!(sparse(&row, 0), [vec![0; 5], numeric].concat()); // no enUS fallback
    let mut other_id = row.clone();
    other_id.id = u32::MAX;
    assert_eq!(sparse(&other_id, 6), sparse(&row, 6));
}

#[test]
fn basic_and_effect_exact_widths_and_signed_float_bits() {
    let row = ItemRecord {
        id: 0xDEAD_BEEF,
        class: -1,
        subclass: 1,
        material: 2,
        inventory_type: -128,
        sheathe: 3,
        pet_food: i32::MIN,
        sound_override: -1,
        icon_file: 0x0102_0304,
        group_sounds: u32::MAX,
        content_tuning: -2,
        modified_crafting_reagent: 1,
        unknown_1200: 255,
        crafting_quality: 2,
        squish_era: 3,
        recraft_reagent_percentage: f32::from_bits(0x8000_0000),
        order_source: 254,
    };
    assert_eq!(
        basic(&row),
        vec![
            255, 255, 255, 255, 1, 2, 128, 3, 0, 0, 0, 128, 255, 4, 3, 2, 1, 255, 255, 255, 255,
            254, 255, 255, 255, 1, 0, 0, 0, 255, 2, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 128, 254,
        ]
    );
    let row = ItemEffectRecord {
        id: 0xDEAD_BEEF,
        legacy_slot: 255,
        trigger: 128,
        charges: i16::MIN,
        cooldown: -1,
        category_cooldown: i32::MIN,
        spell_category: u16::MAX,
        spell: 0x0102_0304,
        specialization: 0x0506,
        player_condition: -2,
    };
    assert_eq!(
        effect(&row),
        vec![
            255, 128, 0, 128, 255, 255, 255, 255, 0, 0, 0, 128, 255, 255, 4, 3, 2, 1, 6, 5, 254,
            255, 255, 255
        ]
    );
}

fn stores() -> ItemHotfixStores {
    let removals =
        Db2HotfixRemovalStoreLikeCpp::from_status_rows_like_cpp([(ITEM_SPARSE_HASH, 8, 2)]);
    let data = ItemRecords {
        sparse: vec![sparse_row(7), sparse_row(8)],
        relations: vec![ItemEffectRelationRecord {
            id: 7,
            effect: -1,
            item: 0x0102_0304,
        }],
        ..Default::default()
    }
    .finish(Default::default(), Default::default(), &removals)
    .unwrap();
    let specs = ItemSpecRecords {
        specs: vec![ItemSpecRecord {
            id: 7,
            min_level: 1,
            max_level: 255,
            item_type: 2,
            primary: 3,
            secondary: 40,
            specialization: 0x0102,
        }],
        overrides: vec![ItemSpecOverrideRecord {
            id: 7,
            specialization: 0x0304,
            item: u32::MAX,
        }],
        gems: vec![GemPropertiesRecord {
            id: 7,
            enchantment: 0x0506,
            kind: i32::MIN,
        }],
    }
    .finish(Default::default(), Default::default(), &removals)
    .unwrap();
    ItemHotfixStores {
        data: Arc::new(data),
        specs: Arc::new(specs),
    }
}

#[test]
fn relationship_parent_fields_and_spec_gem_records_are_not_physical_db2_bytes() {
    let stores = stores();
    for (hash, expected) in [
        (ITEM_RELATION_HASH, vec![255, 255, 255, 255, 4, 3, 2, 1]),
        (ITEM_SPEC_HASH, vec![1, 255, 2, 3, 40, 2, 1]),
        (ITEM_SPEC_OVERRIDE_HASH, vec![4, 3, 255, 255, 255, 255]),
        (GEM_PROPERTIES_HASH, vec![6, 5, 0, 0, 0, 128]),
    ] {
        assert_eq!(stores.record(hash, 7, 6).unwrap(), expected);
        assert!(stores.record(hash, 99, 6).is_none());
    }
    assert!(stores.record(ITEM_SPARSE_HASH, 8, 6).is_none());
}

#[test]
fn delivery_requires_registration_shares_canonical_data_and_keeps_unported_fence() {
    let stores = stores();
    let mut metadata = HotfixBlobCache::new();
    metadata.register_typed_table(TACT_KEY_TABLE_HASH);
    let keys = || ForeverTactKeys {
        records: BTreeMap::from([(7, [1; 16])]),
    };
    let mut incomplete = HotfixBlobCache::new();
    incomplete.register_typed_table(TACT_KEY_TABLE_HASH);
    assert!(
        ForeverHotfixCatalog::new(keys(), incomplete)
            .unwrap()
            .with_item_stores(stores.data.clone(), stores.specs.clone())
            .is_err()
    );
    for hash in ITEM_TABLE_HASHES {
        metadata.register_typed_table(hash);
    }
    metadata.register_typed_table(123);
    let catalog = ForeverHotfixCatalog::new(keys(), metadata)
        .unwrap()
        .with_item_stores(stores.data.clone(), stores.specs.clone())
        .unwrap();
    assert!(Arc::ptr_eq(
        &catalog.item_stores.as_ref().unwrap().data,
        &stores.data
    ));
    for hash in ITEM_TABLE_HASHES {
        assert!(catalog.has_serializer(hash));
    }
    assert_eq!(
        catalog
            .write_record(ITEM_SPEC_HASH, 7, 6)
            .unwrap()
            .unwrap()
            .as_ref(),
        &[1, 255, 2, 3, 40, 2, 1]
    );
    assert!(
        catalog
            .write_record(ITEM_SPARSE_HASH, 8, 6)
            .unwrap()
            .is_none()
    );
    assert_eq!(
        catalog.write_record(123, 7, 6),
        Err(RecordError::UnportedStore)
    );
    assert_eq!(
        catalog.write_record(ITEM_SPEC_HASH, 7, 9),
        Err(RecordError::InvalidLocale)
    );
    assert!(catalog.write_record(99, 7, 6).unwrap().is_none());
    assert_eq!(catalog.metadata().total_blobs(), 0); // no serialized mirror
}
