//! Tests for the item_stats module.
//!
//! Separated from item_stats.rs under #687.

use super::*;

#[test]
fn test_base_stat_bonuses() {
    let entry = ItemStatEntry {
        armor: 0,
        resistances: [0; 7],
        stats: [
            (4, 100), // STR=100
            (7, 80),  // STA=80
            (3, 50),  // AGI=50
            (32, 40), // CritRating=40 (not a base stat)
            (-1, 0),
            (-1, 0),
            (-1, 0),
            (-1, 0),
            (-1, 0),
            (-1, 0),
        ],
    };
    let [str, agi, sta, int, spi] = entry.base_stat_bonuses();
    assert_eq!(str, 100);
    assert_eq!(agi, 50);
    assert_eq!(sta, 80);
    assert_eq!(int, 0);
    assert_eq!(spi, 0);
}

#[test]
fn test_attack_power_bonus() {
    let entry = ItemStatEntry {
        armor: 0,
        resistances: [0; 7],
        stats: [
            (38, 120), // AttackPower=120
            (4, 50),   // STR=50
            (-1, 0),
            (-1, 0),
            (-1, 0),
            (-1, 0),
            (-1, 0),
            (-1, 0),
            (-1, 0),
            (-1, 0),
        ],
    };
    assert_eq!(entry.attack_power_bonus(), 120);
    assert_eq!(entry.health_bonus(), 0);
}

#[test]
fn item_sparse_flags_are_exposed_like_cpp_extended_data() {
    let store = ItemStatsStore::from_parts(
        [],
        [(
            1,
            [
                ItemFlags::IS_BOUND_TO_ACCOUNT.bits() as u32,
                0x20000,
                0x400,
                0,
            ],
        )],
    );

    assert_eq!(store.raw_flags(1), Some([0x0800_0000, 0x20000, 0x400, 0]));
    assert!(
        store
            .item_flags(1)
            .is_some_and(|flags| flags.contains(ItemFlags::IS_BOUND_TO_ACCOUNT))
    );
    assert_eq!(store.item_flags(2), None);
}

#[test]
fn item_sparse_template_entry_matches_cpp_template_helpers() {
    let template = ItemSparseTemplateEntry {
        flags: [ItemFlags::IS_BOUND_TO_ACCOUNT.bits() as u32, 0, 0, 0],
        bag_family: 0x20,
        start_quest_id: 0,
        stackable: 20,
        max_count: 5,
        lock_id: 456,
        required_reputation_rank: 0,
        sell_price: 123,
        buy_price: 456,
        vendor_stack_count: 2,
        price_variance: 1.25,
        price_random_value: 0.75,
        other_faction_item_id: 999,
        content_tuning_id: 321,
        player_level_to_item_level_curve_id: 654,
        max_durability: 77,
        limit_category: 9,
        instance_bound: 11,
        zone_bound: [22, 33],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 4,
        bonding: 2,
        container_slots: 16,
        inventory_type: 18,
    };
    let store = ItemStatsStore::from_sparse_templates([(1, template)])
        .with_duration_in_inventory([(1, 45_000)]);

    let loaded = store.sparse_template(1).unwrap();
    assert_eq!(loaded.max_stack_size(), 20);
    assert_eq!(loaded.max_count, 5);
    assert_eq!(loaded.lock_id, 456);
    assert_eq!(loaded.required_reputation_rank, 0);
    assert_eq!(loaded.required_reputation_faction, 0);
    assert_eq!(loaded.sell_price, 123);
    assert_eq!(loaded.buy_price, 456);
    assert_eq!(loaded.vendor_stack_count, 2);
    assert_eq!(loaded.price_variance, 1.25);
    assert_eq!(loaded.price_random_value, 0.75);
    assert_eq!(loaded.other_faction_item_id_like_cpp(), 999);
    assert_eq!(loaded.scaling_stat_content_tuning_like_cpp(), 321);
    assert_eq!(loaded.player_level_to_item_level_curve_id_like_cpp(), 654);
    assert_eq!(loaded.instance_bound, 11);
    assert_eq!(loaded.zone_bound, [22, 33]);
    assert_eq!(loaded.required_expansion, 4);
    assert_eq!(loaded.container_slots, 16);
    assert_eq!(loaded.inventory_type, 18);
    assert_eq!(loaded.allowable_class, -1);
    assert!(loaded.item_flags().contains(ItemFlags::IS_BOUND_TO_ACCOUNT));
    assert_eq!(store.raw_flags(1), Some(template.flags));
    assert_eq!(store.duration_in_inventory(1), Some(45_000));

    let socket_template = ItemSocketTemplateEntry {
        socket_types: [1, 2, 4],
        required_skill_id: 755,
        required_skill_rank: 350,
    };
    let store = store
        .with_gem_properties([(1, 77)])
        .with_socket_templates([(1, socket_template)]);
    assert_eq!(store.gem_properties(1), Some(77));
    assert_eq!(store.gem_properties(2), None);
    assert_eq!(store.socket_template(1), Some(&socket_template));
    assert_eq!(store.socket_template(2), None);

    let unlimited = ItemSparseTemplateEntry {
        stackable: 0,
        ..template
    };
    assert_eq!(unlimited.max_stack_size(), 0x7FFF_FFFE);
}

#[test]
fn item_weapon_template_entry_exposes_cpp_apply_weapon_damage_inputs() {
    let weapon = ItemWeaponTemplateEntry {
        dmg_variance: 1.4,
        item_delay: 2600,
        min_damage: [120, 1, 2, 3, 4],
        max_damage: [180, 5, 6, 7, 8],
        damage_damage_type: 0,
    };
    let store = ItemStatsStore::from_weapon_templates([(100, weapon)]);

    let loaded = store.weapon_template(100).unwrap();
    assert_eq!(loaded.dmg_variance, 1.4);
    assert_eq!(loaded.item_delay, 2600);
    assert_eq!(loaded.min_damage[0], 120);
    assert_eq!(loaded.max_damage[0], 180);
    assert_eq!(loaded.damage_damage_type, 0);
    assert_eq!(store.weapon_template(101), None);
}

#[test]
fn test_load_item_stats_store() {
    let data_dir = "/home/server/woltk-server-core/Data";
    let locale = "esES";
    let path = std::path::Path::new(data_dir)
        .join("dbc")
        .join(locale)
        .join("ItemSparse.db2");
    if !path.exists() {
        eprintln!("Skipping test: ItemSparse.db2 not found");
        return;
    }

    let store = ItemStatsStore::load(data_dir, locale).expect("failed to load ItemStatsStore");

    eprintln!("ItemStatsStore: {} items with stats", store.len());
    assert!(
        store.len() > 1000,
        "expected >1000 items with stats, got {}",
        store.len()
    );

    // Check Shadowmourne (49623): should have STR, STA, CritRating
    if let Some(entry) = store.get(49623) {
        let [str, _agi, sta, _int, _spi] = entry.base_stat_bonuses();
        eprintln!("Shadowmourne: STR={str}, STA={sta}");
        eprintln!("  full stats: {:?}", entry.stats);
        assert!(str > 100, "Shadowmourne STR should be >100, got {str}");
        assert!(sta > 100, "Shadowmourne STA should be >100, got {sta}");
        let weapon = store
            .weapon_template(49623)
            .expect("Shadowmourne should expose ItemSparse weapon fields");
        assert!(weapon.item_delay > 0, "weapon delay should be loaded");
        assert!(
            weapon.min_damage[0] <= weapon.max_damage[0],
            "weapon damage range should be ordered"
        );
    } else {
        eprintln!("Shadowmourne (49623) not found in stats store");
    }

    // Hearthstone (6948) should NOT be in the store (no combat stats)
    assert!(
        store.get(6948).is_none(),
        "Hearthstone should have no stats"
    );
}
