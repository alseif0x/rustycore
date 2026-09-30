//! Shared overworld personal-loot fixtures and assertions.

use std::sync::Arc;
use wow_constants::{InventoryType, ItemBondingType, ItemClass, ItemFlags2};
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_data::{ItemRecord, ItemSparseTemplateEntry, ItemStatsStore, ItemStore};
use wow_loot::{
    LootStore, LootStoreItem, LootStoreKind, LootStores, LootTemplateRow, OwnedLootAuthority,
};

use super::{
    LOOT_MODE_DEFAULT_LIKE_CPP, attach_loot_guid_allocator_for_owner, broadcast_info, make_session,
    register_test_creature_like_cpp, test_creature, test_creature_guid,
};
use crate::session::WorldSession;
use crate::session::directory::PlayerRegistry;

pub(super) struct OverworldPersonalLootTestFixtureLikeCpp {
    pub(super) session: WorldSession,
    pub(super) owner_guid: ObjectGuid,
    pub(super) first_tapper: ObjectGuid,
    pub(super) second_tapper: ObjectGuid,
    pub(super) disconnected_tapper: ObjectGuid,
    pub(super) normal_item_id: u32,
    pub(super) alliance_item_id: u32,
}

pub(super) fn overworld_personal_loot_test_fixture_like_cpp()
-> OverworldPersonalLootTestFixtureLikeCpp {
    let mut session = make_session();
    let first_tapper = ObjectGuid::create_player(1, 42);
    let second_tapper = ObjectGuid::create_player(1, 43);
    let disconnected_tapper = ObjectGuid::create_player(1, 44);
    let owner_guid = test_creature_guid(19_098);
    let loot_id = 90_001;
    let normal_item_id = 80_101;
    let alliance_item_id = 80_102;

    session.set_player_guid(Some(first_tapper));
    session.set_loaded_player_identity_like_cpp(0, 1, 1, 10, 0);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));

    let registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (second_tx, _second_rx) = flume::bounded(1);
    let mut second = broadcast_info(second_tapper, second_tx);
    second.identity.race = 2;
    registry.register_or_replace(second_tapper, second, Default::default());
    let (disconnected_tx, _disconnected_rx) = flume::bounded(1);
    let mut disconnected = broadcast_info(disconnected_tapper, disconnected_tx);
    disconnected.placement.is_in_world = false;
    registry.register_or_replace(disconnected_tapper, disconnected, Default::default());
    session.set_player_registry(registry);

    let item_record = |id| ItemRecord {
        id,
        class_id: ItemClass::Consumable as u8,
        subclass_id: 0,
        material: 0,
        inventory_type: InventoryType::NonEquip as i8,
        sheathe_type: 0,
        random_select: 0,
        random_suffix_group_id: 0,
        scaling_stat_distribution_id: 0,
        scaling_stat_value: 0,
    };
    let sparse_template = |flags2| ItemSparseTemplateEntry {
        flags: [0, flags2, 0, 0],
        bag_family: 0,
        start_quest_id: 0,
        stackable: 20,
        max_count: 0,
        lock_id: 0,
        required_reputation_rank: 0,
        sell_price: 0,
        buy_price: 0,
        vendor_stack_count: 1,
        price_variance: 1.0,
        price_random_value: 1.0,
        max_durability: 0,
        other_faction_item_id: 0,
        content_tuning_id: 0,
        player_level_to_item_level_curve_id: 0,
        limit_category: 0,
        instance_bound: 0,
        zone_bound: [0, 0],
        required_reputation_faction: 0,
        allowable_class: -1,
        required_expansion: 0,
        bonding: ItemBondingType::None as u8,
        container_slots: 0,
        inventory_type: InventoryType::NonEquip as i8,
    };
    session.set_item_store(Arc::new(ItemStore::from_records([
        item_record(normal_item_id),
        item_record(alliance_item_id),
    ])));
    session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([
        (normal_item_id, sparse_template(0)),
        (
            alliance_item_id,
            sparse_template(ItemFlags2::FactionAlliance as u32),
        ),
    ])));

    let mut creature_store = LootStore::for_kind_like_cpp(LootStoreKind::Creature);
    creature_store
        .load_rows_like_cpp(
            [normal_item_id, alliance_item_id].map(|item_id| LootTemplateRow {
                entry: loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }),
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Creature, creature_store);
    session.set_loot_stores(Arc::new(stores));

    let mut creature = test_creature(owner_guid, false);
    creature.entry = 9_001;
    creature.level = 10;
    creature.loot_id = loot_id;
    creature.gold_min = 7;
    creature.gold_max = 7;
    register_test_creature_like_cpp(&mut session, creature);
    session.mutate_world_creature(owner_guid, |world_creature| {
        world_creature
            .creature
            .set_tapped_by_player(disconnected_tapper, &[first_tapper, second_tapper]);
    });
    attach_loot_guid_allocator_for_owner(&mut session, owner_guid);

    OverworldPersonalLootTestFixtureLikeCpp {
        session,
        owner_guid,
        first_tapper,
        second_tapper,
        disconnected_tapper,
        normal_item_id,
        alliance_item_id,
    }
}

pub(super) fn assert_overworld_personal_loot_generation_like_cpp(
    authority: &OwnedLootAuthority,
    fixture: &OverworldPersonalLootTestFixtureLikeCpp,
) -> (u8, u8) {
    assert!(authority.shared_snapshot_like_cpp().is_none());
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 2);
    assert!(!personal.contains_key(&fixture.disconnected_tapper));

    let first = &personal[&fixture.first_tapper].loot;
    let second = &personal[&fixture.second_tapper].loot;
    assert_ne!(first.loot_guid, second.loot_guid);
    for (tapper, loot) in [
        (fixture.first_tapper, first),
        (fixture.second_tapper, second),
    ] {
        assert_eq!(loot.loot_guid.high_type(), HighGuid::LootObject);
        assert_eq!(loot.coins, 7);
        assert_eq!(loot.loot_method, 0);
        assert_eq!(loot.allowed_looters, vec![tapper]);
        assert!(
            loot.items
                .iter()
                .all(|item| item.allowed_looters == vec![tapper])
        );
    }
    assert!(
        first
            .items
            .iter()
            .any(|item| item.item_id == fixture.normal_item_id)
    );
    assert!(
        first
            .items
            .iter()
            .any(|item| item.item_id == fixture.alliance_item_id)
    );
    assert_eq!(
        second
            .items
            .iter()
            .map(|item| item.item_id)
            .collect::<Vec<_>>(),
        vec![fixture.normal_item_id],
        "FillLoot eligibility must run for the Horde tapper instead of cloning the first pool"
    );

    let first_normal_slot = first
        .items
        .iter()
        .find(|item| item.item_id == fixture.normal_item_id)
        .unwrap()
        .loot_list_id;
    (first_normal_slot, second.items[0].loot_list_id)
}

pub(super) async fn assert_overworld_personal_loot_claims_are_independent_like_cpp(
    authority: &OwnedLootAuthority,
    first_tapper: ObjectGuid,
    first_normal_slot: u8,
    second_tapper: ObjectGuid,
    second_normal_slot: u8,
) {
    assert!(
        authority
            .reserve_item_like_cpp(first_tapper, first_normal_slot)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
    assert!(
        !authority
            .personal_snapshot_like_cpp(second_tapper)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert!(
        authority
            .reserve_money_like_cpp(first_tapper)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
    assert_eq!(
        authority
            .personal_snapshot_like_cpp(second_tapper)
            .unwrap()
            .loot
            .coins,
        7
    );
    assert!(
        authority
            .reserve_item_like_cpp(second_tapper, second_normal_slot)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
    assert!(
        authority
            .reserve_money_like_cpp(second_tapper)
            .await
            .unwrap()
            .commit_like_cpp()
            .unwrap()
    );
}
