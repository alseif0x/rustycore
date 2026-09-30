use super::*;
use crate::{LOOT_MODE_DEFAULT_LIKE_CPP, LootStoreItem, LootStoreKind, NotNormalLootItem};
use wow_constants::ItemContext;

#[test]
fn creature_generated_loot_entry_uses_item_template_addon_follow_loot_rules_like_cpp() {
    let generated = GeneratedLootItem {
        item_id: 25,
        count: 1,
        loot_list_id: 7,
        random_properties_id: -77,
        random_properties_seed: 456,
        context: ItemContext::DungeonNormal as u8,
        store_item_context: LootStoreItemContext {
            store_kind: LootStoreKind::Creature,
            entry: 100,
            item: LootStoreItem {
                item_id: 25,
                reference: 0,
                chance: 100.0,
                needs_quest: true,
                loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                group_id: 0,
                min_count: 1,
                max_count: 1,
            },
        },
        free_for_all: false,
        follow_loot_rules: false,
        needs_quest: true,
        is_looted: false,
        is_blocked: false,
        is_under_threshold: false,
        is_counted: false,
    };

    let default_entry = loot_entry_from_generated(
        generated,
        false,
    );
    assert!(!default_entry.flags.follow_loot_rules);
    assert_eq!(default_entry.loot_list_id, 7);
    assert_eq!(default_entry.random_properties_id, -77);
    assert_eq!(default_entry.random_properties_seed, 456);
    assert_eq!(default_entry.item_context, ItemContext::DungeonNormal as u8);

    let follow_entry = loot_entry_from_generated(
        generated,
        true,
    );
    assert!(follow_entry.flags.follow_loot_rules);
    assert!(follow_entry.flags.needs_quest);
}

#[test]
fn shared_gameobject_item_evaluates_each_looter_after_roll_like_cpp() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let store_item_context = LootStoreItemContext {
        store_kind: LootStoreKind::Reference,
        entry: 900,
        item: LootStoreItem {
            item_id: 25,
            reference: 0,
            chance: 100.0,
            needs_quest: false,
            loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 0,
            min_count: 1,
            max_count: 1,
        },
    };
    let generated = GeneratedLootItem {
        item_id: 25,
        count: 1,
        loot_list_id: 0,
        random_properties_id: 0,
        random_properties_seed: 0,
        context: ItemContext::None as u8,
        store_item_context,
        free_for_all: false,
        follow_loot_rules: true,
        needs_quest: false,
        is_looted: false,
        is_blocked: false,
        is_under_threshold: false,
        is_counted: false,
    };
    let mut evaluated = Vec::new();

    let entry = shared_loot_entry_from_generated(
        generated,
        false,
        &[first, second],
        |context, looter| {
            evaluated.push((context, looter));
            looter == second
        },
    );

    assert_eq!(
        evaluated,
        vec![(store_item_context, first), (store_item_context, second)]
    );
    assert_eq!(entry.item_id, 25, "the rolled candidate remains present");
    assert_eq!(entry.allowed_looters, vec![second]);
}

fn pool_guid(counter: i64) -> ObjectGuid {
    ObjectGuid::create_world_object(
        wow_core::guid::HighGuid::LootObject, 0, 1, 0, 0, 0, counter,
    )
}

fn pool_entry(loot_list_id: u8, allowed_looters: Vec<ObjectGuid>) -> LootEntry {
    LootEntry {
        loot_list_id,
        item_id: 25 + u32::from(loot_list_id),
        quantity: 3,
        random_properties_id: -77,
        random_properties_seed: 456,
        item_context: ItemContext::DungeonNormal as u8,
        flags: LootEntryFlags {
            follow_loot_rules: true,
            ..Default::default()
        },
        allowed_looters,
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: false,
    }
}

fn pool_fixture() -> CreatureLoot {
    CreatureLoot {
        loot_guid: pool_guid(100),
        coins: 99,
        unlooted_count: 17,
        loot_type: 1,
        dungeon_encounter_id: 733,
        loot_method: crate::LOOT_METHOD_GROUP_LIKE_CPP,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: Vec::new(),
        items: Vec::new(),
        looted_by_player: false,
    }
}

#[test]
fn shared_pool_returns_unchanged_without_resolving() {
    let player = ObjectGuid::create_player(1, 42);
    let mut loot = pool_fixture();
    loot.allowed_looters = vec![player, player];
    loot.players_looting = vec![player];
    loot.items.push(pool_entry(7, vec![player]));
    let mut expected = pool_fixture();
    expected.allowed_looters = vec![player, player];
    expected.players_looting = vec![player];
    expected.items.push(pool_entry(7, vec![player]));

    let (shared, personal) = materialize_loot_pools(loot, player, false, |_, _| {
        panic!("shared pools must not resolve GUIDs or money")
    }).unwrap();

    assert_eq!(shared, Some(expected));
    assert!(personal.is_empty());
}

#[test]
fn empty_personal_owners_and_empty_fallback_return_empty_topology() {
    let (shared, personal) = materialize_loot_pools(
        pool_fixture(),
        ObjectGuid::EMPTY,
        true,
        |_, _| panic!("an empty ownership set must not resolve a pool"),
    ).unwrap();

    assert!(shared.is_none());
    assert!(personal.is_empty());
}

#[test]
fn personal_fallback_reuses_existing_guid_and_resolved_money() {
    let player = ObjectGuid::create_player(1, 42);
    let mut calls = Vec::new();
    let (shared, personal) = materialize_loot_pools(
        pool_fixture(),
        player,
        true,
        |looter, existing_guid| {
            calls.push((looter, existing_guid));
            Some((existing_guid.expect("the first pool reuses its GUID"), 41))
        },
    ).unwrap();

    assert!(shared.is_none());
    assert_eq!(calls, vec![(player, Some(pool_guid(100)))]);
    assert_eq!(personal.len(), 1);
    let pool = personal.get(&player).unwrap();
    assert_eq!(pool.loot_guid, pool_guid(100));
    assert_eq!(pool.coins, 41);
    assert_eq!(pool.allowed_looters, vec![player]);
    assert_eq!(pool.unlooted_count, 0);
}

#[test]
fn personal_resolution_sorts_deduplicates_and_allocates_before_money() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let third = ObjectGuid::create_player(1, 99);
    let mut loot = pool_fixture();
    loot.allowed_looters = vec![third, second, first, third, first];
    let mut events = Vec::new();
    let mut next_guid = 101;
    let (_, personal) = materialize_loot_pools(loot, ObjectGuid::EMPTY, true, |looter, existing| {
        let guid = match existing {
            Some(guid) => {
                events.push((looter, "reuse"));
                guid
            }
            None => {
                events.push((looter, "allocate"));
                let guid = pool_guid(next_guid);
                next_guid += 1;
                guid
            }
        };
        events.push((looter, "money"));
        Some((guid, looter.counter() as u32))
    }).unwrap();

    assert_eq!(
        events,
        vec![
            (first, "reuse"), (first, "money"),
            (second, "allocate"), (second, "money"),
            (third, "allocate"), (third, "money"),
        ]
    );
    assert_eq!(personal.len(), 3);
    assert_eq!(personal[&first].loot_guid, pool_guid(100));
    assert_eq!(personal[&second].loot_guid, pool_guid(101));
    assert_eq!(personal[&third].loot_guid, pool_guid(102));
    assert_eq!(personal[&first].coins, 42);
    assert_eq!(personal[&second].coins, 77);
    assert_eq!(personal[&third].coins, 99);
    assert_eq!(next_guid, 103);
}

#[test]
fn failed_guid_resolution_returns_none_before_money_or_later_players() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let third = ObjectGuid::create_player(1, 99);
    let mut loot = pool_fixture();
    loot.allowed_looters = vec![third, first, second];
    let mut events = Vec::new();
    let result = materialize_loot_pools(loot, first, true, |looter, existing| {
        let guid = match existing {
            Some(guid) => guid,
            None => {
                events.push((looter, "allocation_failed"));
                return None;
            }
        };
        events.push((looter, "money"));
        Some((guid, 41))
    });

    assert!(result.is_none());
    assert_eq!(events, vec![(first, "money"), (second, "allocation_failed")]);
}

#[test]
fn personal_partition_filters_permissions_and_viewers_without_renumbering() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let outsider = ObjectGuid::create_player(1, 99);
    let mut loot = pool_fixture();
    loot.allowed_looters = vec![second, first];
    loot.players_looting = vec![second, first, outsider, first];
    loot.loot_master = second;
    loot.round_robin_player = first;
    loot.items = vec![
        pool_entry(7, Vec::new()),
        pool_entry(2, vec![first]),
        pool_entry(7, vec![second]),
        pool_entry(9, vec![outsider]),
    ];
    let (_, personal) = materialize_loot_pools(loot, outsider, true, |looter, existing| {
        Some((existing.unwrap_or(pool_guid(101)), looter.counter() as u32))
    }).unwrap();

    assert_eq!(personal.len(), 2);
    assert!(!personal.contains_key(&outsider));
    let first_pool = &personal[&first];
    let second_pool = &personal[&second];
    assert_eq!(first_pool.allowed_looters, vec![first]);
    assert_eq!(second_pool.allowed_looters, vec![second]);
    assert_eq!(first_pool.players_looting, vec![first, first]);
    assert_eq!(second_pool.players_looting, vec![second]);
    assert_eq!(
        first_pool.items.iter().map(|entry| entry.loot_list_id).collect::<Vec<_>>(),
        vec![7, 2]
    );
    assert_eq!(
        second_pool.items.iter().map(|entry| entry.loot_list_id).collect::<Vec<_>>(),
        vec![7, 7]
    );
    for (player, pool) in [(first, first_pool), (second, second_pool)] {
        for entry in &pool.items {
            assert_eq!(entry.allowed_looters, vec![player]);
            assert_eq!(entry.quantity, 3);
            assert_eq!(entry.random_properties_id, -77);
            assert_eq!(entry.random_properties_seed, 456);
            assert_eq!(entry.item_context, ItemContext::DungeonNormal as u8);
        }
        assert_eq!(pool.unlooted_count, 2);
        assert_eq!(pool.loot_master, second);
        assert_eq!(pool.round_robin_player, first);
        assert_eq!(pool.loot_method, crate::LOOT_METHOD_GROUP_LIKE_CPP);
        assert_eq!(pool.dungeon_encounter_id, 733);
    }
}

#[test]
fn personal_partition_preserves_consumed_items_and_player_ffa_counts() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let mut loot = pool_fixture();
    loot.allowed_looters = vec![first, second];
    let ordinary = pool_entry(5, vec![first, second]);
    let mut taken = pool_entry(6, vec![first, second]);
    taken.taken = true;
    let mut ffa = pool_entry(7, vec![first, second]);
    ffa.flags.freeforall = true;
    ffa.flags.counted = true;
    ffa.ffa_looted_by = vec![first];
    loot.items = vec![ordinary, taken, ffa];
    loot.player_ffa_items = vec![(first, vec![NotNormalLootItem {
        loot_list_id: 7,
        is_looted: false,
    }])];

    let (_, personal) = materialize_loot_pools(loot, first, true, |_, existing| {
        Some((existing.unwrap_or(pool_guid(101)), 0))
    }).unwrap();

    let first_pool = &personal[&first];
    let second_pool = &personal[&second];
    assert_eq!(first_pool.unlooted_count, 1);
    assert_eq!(second_pool.unlooted_count, 2);
    for pool in [first_pool, second_pool] {
        assert_eq!(pool.items.len(), 3);
        assert!(pool.items[0].flags.counted);
        assert!(pool.items[1].flags.counted);
        assert!(pool.items[1].taken);
        assert!(!pool.items[2].flags.counted);
        assert_eq!(pool.items[2].ffa_looted_by, vec![first]);
        assert_eq!(pool.player_ffa_items.len(), 1);
    }
    assert_eq!(first_pool.player_ffa_items, vec![(first, vec![NotNormalLootItem {
        loot_list_id: 7,
        is_looted: true,
    }])]);
    assert_eq!(second_pool.player_ffa_items, vec![(second, vec![NotNormalLootItem {
        loot_list_id: 7,
        is_looted: false,
    }])]);
}

#[test]
fn shared_entry_keeps_callback_order_and_duplicate_evaluations() {
    let first = ObjectGuid::create_player(1, 42);
    let second = ObjectGuid::create_player(1, 77);
    let generated = GeneratedLootItem {
        item_id: 25,
        count: 0,
        loot_list_id: 257,
        random_properties_id: -77,
        random_properties_seed: -456,
        context: ItemContext::DungeonNormal as u8,
        store_item_context: LootStoreItemContext {
            store_kind: LootStoreKind::Reference,
            entry: 900,
            item: LootStoreItem {
                item_id: 25,
                reference: 0,
                chance: 100.0,
                needs_quest: false,
                loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                group_id: 0,
                min_count: 1,
                max_count: 1,
            },
        },
        free_for_all: true,
        follow_loot_rules: false,
        needs_quest: false,
        is_looted: true,
        is_blocked: true,
        is_under_threshold: true,
        is_counted: true,
    };
    let mut evaluated = Vec::new();
    let entry = shared_loot_entry_from_generated(
        generated, false, &[second, first, second], |context, looter| {
            evaluated.push((context, looter));
            looter == second
        },
    );

    assert_eq!(evaluated, vec![
        (generated.store_item_context, second),
        (generated.store_item_context, first),
        (generated.store_item_context, second),
    ]);
    assert_eq!(entry.allowed_looters, vec![second]);
    assert_eq!(entry.loot_list_id, 1);
    assert_eq!(entry.item_id, 25);
    assert_eq!(entry.quantity, 0);
    assert_eq!(entry.random_properties_id, -77);
    assert_eq!(entry.random_properties_seed, -456);
    assert_eq!(entry.item_context, ItemContext::DungeonNormal as u8);
    assert!(entry.flags.follow_loot_rules);
    assert!(entry.flags.freeforall);
    assert!(entry.flags.blocked);
    assert!(entry.flags.counted);
    assert!(entry.flags.under_threshold);
    assert!(!entry.flags.needs_quest);
    assert!(entry.taken);
    assert!(entry.roll_winner.is_empty());
    assert!(entry.ffa_looted_by.is_empty());
}
