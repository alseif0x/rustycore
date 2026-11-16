use super::InventoryPlayerSnapshotLikeCpp;
use crate::Item;
use crate::ItemStorageTemplate;
use crate::player::{
    BANK_SLOT_BAG_START, CanBankItemArgs, CanEquipItemArgs, CanEquipUniqueItemArgs,
    CanStoreItemArgs, CanUnequipItemArgs, CanUseItemArgs, CanUseItemTemplateArgs,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START,
    ItemSearchCallbackResult, ItemSearchLocation, PLAYER_SLOT_END, Player, PlayerStorageError,
};
use wow_constants::{InventoryResult, InventoryType, ItemClass, ItemSubClassContainer};
use wow_core::ObjectGuid;

fn query_player() -> Player {
    let mut player = Player::new(Some(17), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(ObjectGuid::create_player(17, 0xC8B));
    player.set_inventory_slot_count(16);
    player.set_bank_bag_slot_count(0);
    player.set_primary_specialization(4);

    let bag_guid = ObjectGuid::create_item(17, 100);
    player
        .register_bag_storage(INVENTORY_SLOT_BAG_START, bag_guid, 2)
        .unwrap();
    let equipment_guid = ObjectGuid::create_item(17, 101);
    player.store_top_level_item(2, equipment_guid).unwrap();
    player.set_visible_item_slot(
        2,
        Some(crate::player::VisibleItemValues {
            item_id: 19_019,
            item_appearance_mod_id: 3,
            item_visual: 4,
        }),
    );
    player
        .store_top_level_item(INVENTORY_SLOT_BAG_START, bag_guid)
        .unwrap();
    let inventory_guid = ObjectGuid::create_item(17, 102);
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START + 1, inventory_guid)
        .unwrap();
    player
        .store_bag_item(
            INVENTORY_SLOT_BAG_START,
            0,
            ObjectGuid::create_item(17, 103),
        )
        .unwrap();
    player.set_inv_slot(
        usize::from(INVENTORY_SLOT_ITEM_START + 1),
        ObjectGuid::create_item(17, 199),
    );
    player
}

fn collect_guids(
    snapshot: &InventoryPlayerSnapshotLikeCpp,
    location: ItemSearchLocation,
) -> (bool, Vec<ObjectGuid>) {
    let mut visited = Vec::new();
    let completed = snapshot.for_each_item_guid(location, |guid| {
        visited.push(guid);
        ItemSearchCallbackResult::Continue
    });
    (completed, visited)
}

fn can_store_args(proto: &ItemStorageTemplate) -> CanStoreItemArgs<'_> {
    CanStoreItemArgs {
        bag: crate::player::NULL_BAG,
        slot: crate::player::NULL_SLOT,
        entry: proto.entry,
        count: 1,
        proto: Some(proto),
        source_item: None,
        source_is_not_empty_bag: false,
        source_bop_trade_allowed_for_player: false,
        swap: false,
        limit_category: None,
        slot_items: &[],
        stored_items: &[],
        bag_templates: &[],
    }
}

fn can_bank_bag_args<'a>(
    proto: &'a ItemStorageTemplate,
    source: &'a Item,
    slot: u8,
) -> CanBankItemArgs<'a> {
    CanBankItemArgs {
        bag: INVENTORY_SLOT_BAG_0,
        slot,
        proto: Some(proto),
        source_item: Some(source),
        source_is_not_empty_bag: false,
        source_is_bag: true,
        source_is_currency_token: false,
        source_bop_trade_allowed_for_player: false,
        swap: false,
        can_use_result: InventoryResult::Ok,
        limit_category: None,
        slot_items: &[],
        stored_items: &[],
        bag_templates: &[],
    }
}

fn can_use_args() -> CanUseItemArgs<'static> {
    let template_args = CanUseItemTemplateArgs {
        proto: None,
        skip_required_level_check: false,
        player_level: 70,
        team: 0,
        allowable_class_matches: true,
        allowable_race_matches: true,
        internal_item: false,
        faction_horde: false,
        faction_alliance: false,
        required_skill: 0,
        required_skill_rank: 0,
        required_skill_value: 0,
        required_spell: 0,
        has_required_spell: false,
        base_required_level: 0,
        holiday_id: 0,
        holiday_active: false,
        required_reputation_faction: 0,
        required_reputation_rank: 0,
        player_reputation_rank: 0,
        effect0_spell_id: None,
        effect1_spell_id: None,
        has_effect1_spell: false,
        artifact_specialization: None,
        primary_specialization: 0,
    };
    CanUseItemArgs {
        source_item: None,
        proto: None,
        not_loading: true,
        is_alive: true,
        player_level: 70,
        item_required_level: 0,
        source_bop_trade_allowed_for_player: false,
        template_args,
        item_skill: 0,
        item_skill_value: 0,
        has_item_skill: false,
        player_class: 1,
        proto_is_heirloom: false,
    }
}

fn can_equip_args() -> CanEquipItemArgs<'static> {
    CanEquipItemArgs {
        slot: 0,
        proto: None,
        source_item: None,
        source_bop_trade_allowed_for_player: false,
        swap: false,
        not_loading: true,
        is_stunned: false,
        is_charmed: false,
        is_in_combat: false,
        is_in_progress_arena: false,
        weapon_change_timer_active: false,
        current_generic_spell_allows_equip: None,
        current_channeled_spell_allows_equip: None,
        heirloom_required_level_failed: false,
        can_use_result: InventoryResult::Ok,
        can_equip_unique_result: InventoryResult::Ok,
        can_dual_wield: false,
        can_titan_grip: false,
        is_two_hand_used: false,
        proto_always_allow_dual_wield: false,
        has_required_profession_skill: false,
        profession_slot: None,
        offhand_can_unequip_result: InventoryResult::Ok,
        offhand_can_store_result: InventoryResult::Ok,
        limit_category: None,
        equipped_items: &[],
        stored_items: &[],
    }
}

fn can_equip_unique_args() -> CanEquipUniqueItemArgs<'static> {
    CanEquipUniqueItemArgs {
        source_item: None,
        proto: None,
        except_slot: crate::player::NULL_SLOT,
        limit_count: 1,
        unique_equippable: false,
        limit_category: None,
        equipped_items: &[],
        equipped_gems: &[],
        socketed_gems: &[],
    }
}

#[test]
fn inventory_player_snapshot_queries_use_frozen_topology_and_raw_counts() {
    let mut owner = query_player();
    let canonical = query_player();
    let mut snapshot = InventoryPlayerSnapshotLikeCpp::capture(&owner);

    assert_eq!(
        snapshot.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 15, true),
        canonical.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 15, true)
    );
    assert!(snapshot.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 15, true));
    assert!(!snapshot.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 16, true));
    assert_eq!(snapshot.primary_specialization_id_like_cpp(), 4);

    let (completed, visited) = collect_guids(&snapshot, ItemSearchLocation::EVERYWHERE);
    let mut canonical_visited = Vec::new();
    let canonical_completed =
        canonical.for_each_item_guid(ItemSearchLocation::EVERYWHERE, |guid| {
            canonical_visited.push(guid);
            ItemSearchCallbackResult::Continue
        });
    assert!(completed && canonical_completed);
    assert_eq!(visited, canonical_visited);
    assert_eq!(visited[0], ObjectGuid::create_item(17, 101));
    assert_eq!(visited[1], ObjectGuid::create_item(17, 100));
    assert_eq!(visited[2], ObjectGuid::create_item(17, 102));
    assert_eq!(visited[3], ObjectGuid::create_item(17, 103));
    assert_ne!(
        visited[2],
        snapshot.active_player_data.inv_slots[usize::from(INVENTORY_SLOT_ITEM_START + 1)]
    );

    let mut stopped_snapshot = Vec::new();
    let snapshot_completed = snapshot.for_each_item_guid(ItemSearchLocation::EVERYWHERE, |guid| {
        stopped_snapshot.push(guid);
        if guid == ObjectGuid::create_item(17, 100) {
            ItemSearchCallbackResult::Stop
        } else {
            ItemSearchCallbackResult::Continue
        }
    });
    let mut stopped_canonical = Vec::new();
    let canonical_completed =
        canonical.for_each_item_guid(ItemSearchLocation::EVERYWHERE, |guid| {
            stopped_canonical.push(guid);
            if guid == ObjectGuid::create_item(17, 100) {
                ItemSearchCallbackResult::Stop
            } else {
                ItemSearchCallbackResult::Continue
            }
        });
    assert!(!snapshot_completed && !canonical_completed);
    assert_eq!(stopped_snapshot, stopped_canonical);
    assert_eq!(stopped_snapshot, visited[..2]);

    let owner_addition = ObjectGuid::create_item(17, 104);
    owner
        .store_top_level_item(INVENTORY_SLOT_ITEM_START + 2, owner_addition)
        .unwrap();
    let mut topology_changed_owner_items = Vec::new();
    owner.for_each_item_guid(ItemSearchLocation::EVERYWHERE, |guid| {
        topology_changed_owner_items.push(guid);
        ItemSearchCallbackResult::Continue
    });
    assert_ne!(topology_changed_owner_items, visited);

    let bank_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        inventory_type: InventoryType::Bag,
        container_slots: 4,
        ..ItemStorageTemplate::regular_item(20_005, 1)
    };
    let mut bank_source = Item::default();
    bank_source
        .object_mut()
        .create(ObjectGuid::create_item(17, 105));
    bank_source.object_mut().set_entry(bank_proto.entry);
    bank_source.set_count(1);
    let bank_args = can_bank_bag_args(&bank_proto, &bank_source, BANK_SLOT_BAG_START + 1);
    assert_eq!(
        snapshot.can_bank_item(&mut Vec::new(), bank_args),
        InventoryResult::NoBankSlot
    );

    owner.set_inventory_slot_count(1);
    owner.set_bank_bag_slot_count(2);
    owner.set_primary_specialization(99);
    owner.set_can_titan_grip(true, 9_999);
    assert_eq!(
        snapshot.can_bank_item(&mut Vec::new(), bank_args),
        InventoryResult::NoBankSlot
    );
    assert_eq!(
        owner.can_bank_item(&mut Vec::new(), bank_args),
        InventoryResult::Ok
    );

    assert_eq!(
        snapshot.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 15, true),
        true
    );
    assert_eq!(
        owner.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 15, true),
        false
    );
    assert_eq!(snapshot.primary_specialization_id_like_cpp(), 4);
    assert_eq!(owner.primary_specialization_id_like_cpp(), 99);
    assert_eq!(
        collect_guids(&snapshot, ItemSearchLocation::EVERYWHERE).1,
        visited
    );
    let mut changed_owner_items = Vec::new();
    owner.for_each_item_guid(ItemSearchLocation::EVERYWHERE, |guid| {
        changed_owner_items.push(guid);
        ItemSearchCallbackResult::Continue
    });
    assert_ne!(changed_owner_items, visited);

    let two_hand_template = ItemStorageTemplate {
        class_id: ItemClass::Weapon,
        inventory_type: InventoryType::Weapon2Hand,
        ..ItemStorageTemplate::regular_item(20_001, 1)
    };
    assert!(snapshot.is_two_hand_used_template(Some(&two_hand_template)));
    let update_before_titan_overlay = snapshot.values_update(true);
    snapshot.set_can_titan_grip_like_cpp(true);
    assert!(!snapshot.is_two_hand_used_template(Some(&two_hand_template)));
    assert_eq!(snapshot.values_update(true), update_before_titan_overlay);
    assert_eq!(
        owner.is_two_hand_used_template(Some(&two_hand_template)),
        false
    );

    snapshot.set_inventory_slot_count(1);
    let mut canonical_overlay = query_player();
    canonical_overlay.set_inventory_slot_count(1);
    assert_eq!(
        snapshot.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, true),
        canonical_overlay.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, true)
    );
    assert_eq!(
        snapshot.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1, true),
        canonical_overlay.is_valid_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1, true)
    );
    let active_values = snapshot
        .values_update(true)
        .active_player_data
        .unwrap()
        .values;
    assert_eq!(active_values.num_backpack_slots, 1);
    assert_eq!(
        snapshot.values_update(true),
        canonical_overlay.values_update(true)
    );
}

#[test]
fn inventory_player_snapshot_delegates_storage_queries_and_bank_count() {
    let owner = query_player();
    let canonical = query_player();
    let snapshot = InventoryPlayerSnapshotLikeCpp::capture(&owner);
    let proto = ItemStorageTemplate::regular_item(20_010, 20);

    let mut snapshot_dest = Vec::new();
    let mut canonical_dest = Vec::new();
    assert_eq!(
        snapshot.can_store_item(&mut snapshot_dest, can_store_args(&proto)),
        canonical.can_store_item(&mut canonical_dest, can_store_args(&proto))
    );
    assert_eq!(snapshot_dest, canonical_dest);

    let mut source = Item::default();
    source.object_mut().create(ObjectGuid::create_item(17, 210));
    let bank_bag_proto = ItemStorageTemplate {
        class_id: ItemClass::Container,
        subclass_id: ItemSubClassContainer::Container as u32,
        inventory_type: InventoryType::Bag,
        container_slots: 4,
        ..ItemStorageTemplate::regular_item(20_011, 1)
    };
    source.object_mut().set_entry(bank_bag_proto.entry);
    source.set_count(1);
    let bank_slot = BANK_SLOT_BAG_START + 1;
    let bank_args = can_bank_bag_args(&bank_bag_proto, &source, bank_slot);
    let mut snapshot = snapshot;
    let mut canonical = canonical;
    assert_eq!(
        snapshot.can_bank_item(&mut Vec::new(), bank_args),
        InventoryResult::NoBankSlot
    );
    assert_eq!(
        canonical.can_bank_item(&mut Vec::new(), bank_args),
        InventoryResult::NoBankSlot
    );

    snapshot.set_bank_bag_slot_count(2);
    canonical.set_bank_bag_slot_count(2);
    assert_eq!(
        snapshot.can_bank_item(&mut Vec::new(), bank_args),
        InventoryResult::Ok
    );
    assert_eq!(
        canonical.can_bank_item(&mut Vec::new(), bank_args),
        InventoryResult::Ok
    );
    assert_eq!(snapshot.values_update(true), canonical.values_update(true));
    assert_eq!(
        snapshot
            .values_update(true)
            .player_data
            .unwrap()
            .values
            .num_bank_slots,
        canonical
            .values_update(true)
            .player_data
            .unwrap()
            .values
            .num_bank_slots
    );

    assert_eq!(
        snapshot.can_use_item(can_use_args()),
        canonical.can_use_item(can_use_args())
    );
    assert_eq!(
        snapshot.can_equip_item(can_equip_args()),
        canonical.can_equip_item(can_equip_args())
    );
    assert_eq!(
        snapshot.can_equip_unique_item(can_equip_unique_args()),
        canonical.can_equip_unique_item(can_equip_unique_args())
    );
    assert_eq!(
        snapshot.can_unequip_item(CanUnequipItemArgs {
            pos: 2,
            source_item: None,
            proto: None,
            swap: false,
            source_is_not_empty_bag: false,
            is_charmed: false,
            is_in_combat: false,
            is_in_progress_arena: false,
        }),
        canonical.can_unequip_item(CanUnequipItemArgs {
            pos: 2,
            source_item: None,
            proto: None,
            swap: false,
            source_is_not_empty_bag: false,
            is_charmed: false,
            is_in_combat: false,
            is_in_progress_arena: false,
        })
    );
    assert_eq!(
        snapshot.swap_item_preflight_plan(0, 1, true, None, None),
        canonical.swap_item_preflight_plan(0, 1, true, None, None)
    );
}

#[test]
fn inventory_player_snapshot_topology_mutations_match_player_order_and_masks() {
    let mut canonical = query_player();
    let snapshot_owner = query_player();
    let mut snapshot = InventoryPlayerSnapshotLikeCpp::capture(&snapshot_owner);
    let start = snapshot.values_update(true);

    let bad_slot = PLAYER_SLOT_END as u8;
    let bad_guid = ObjectGuid::create_item(17, 300);
    assert_eq!(
        snapshot.store_top_level_item(bad_slot, bad_guid),
        Err(PlayerStorageError::InvalidPlayerSlot(bad_slot))
    );
    assert_eq!(
        canonical.store_top_level_item(bad_slot, bad_guid),
        Err(PlayerStorageError::InvalidPlayerSlot(bad_slot))
    );
    assert_eq!(
        snapshot.remove_top_level_item(bad_slot),
        Err(PlayerStorageError::InvalidPlayerSlot(bad_slot))
    );
    assert_eq!(
        canonical.remove_top_level_item(bad_slot),
        Err(PlayerStorageError::InvalidPlayerSlot(bad_slot))
    );
    assert_eq!(snapshot.values_update(true), start);
    assert_eq!(snapshot.values_update(true), canonical.values_update(true));
    assert_eq!(
        snapshot.inventory.items[2],
        Some(ObjectGuid::create_item(17, 101))
    );
    assert_eq!(
        canonical.inventory.items[2],
        Some(ObjectGuid::create_item(17, 101))
    );
    assert!(
        snapshot
            .inventory
            .bag_at(INVENTORY_SLOT_BAG_START)
            .is_some()
    );
    assert!(
        canonical
            .inventory
            .bag_at(INVENTORY_SLOT_BAG_START)
            .is_some()
    );

    let replace_slot = 2;
    let replacement = ObjectGuid::create_item(17, 301);
    snapshot
        .store_top_level_item(replace_slot, replacement)
        .unwrap();
    canonical
        .store_top_level_item(replace_slot, replacement)
        .unwrap();
    assert_eq!(
        snapshot.inventory.items[replace_slot as usize],
        Some(replacement)
    );
    assert_eq!(
        canonical.inventory.items[replace_slot as usize],
        Some(replacement)
    );

    assert_eq!(
        snapshot.remove_top_level_item(replace_slot),
        Ok(Some(replacement))
    );
    assert_eq!(
        canonical.remove_top_level_item(replace_slot),
        Ok(Some(replacement))
    );
    assert_eq!(snapshot.inventory.items[replace_slot as usize], None);
    assert_eq!(canonical.inventory.items[replace_slot as usize], None);
    assert_eq!(
        snapshot.active_player_data.inv_slots[replace_slot as usize],
        ObjectGuid::EMPTY
    );
    assert_eq!(
        canonical.active_data.inv_slots[replace_slot as usize],
        ObjectGuid::EMPTY
    );
    assert_eq!(
        snapshot.player_data.visible_items[replace_slot as usize],
        crate::player::VisibleItemValues::default()
    );
    assert_eq!(
        canonical.data.visible_items[replace_slot as usize],
        crate::player::VisibleItemValues::default()
    );
    assert_eq!(snapshot.values_update(true), canonical.values_update(true));

    let visible_without_native_item = crate::player::VisibleItemValues {
        item_id: 22_222,
        item_appearance_mod_id: 6,
        item_visual: 7,
    };
    snapshot.set_visible_item_slot(replace_slot, Some(visible_without_native_item));
    canonical.set_visible_item_slot(replace_slot, Some(visible_without_native_item));
    assert_eq!(snapshot.inventory.items[replace_slot as usize], None);
    assert_eq!(canonical.inventory.items[replace_slot as usize], None);
    assert_eq!(snapshot.remove_top_level_item(replace_slot), Ok(None));
    assert_eq!(canonical.remove_top_level_item(replace_slot), Ok(None));
    assert_eq!(
        snapshot.player_data.visible_items[replace_slot as usize],
        crate::player::VisibleItemValues::default()
    );
    assert_eq!(
        canonical.data.visible_items[replace_slot as usize],
        crate::player::VisibleItemValues::default()
    );
    assert_eq!(snapshot.values_update(true), canonical.values_update(true));

    let bag_slot = INVENTORY_SLOT_BAG_START;
    let bag_guid = ObjectGuid::create_item(17, 302);
    let child_guid = ObjectGuid::create_item(17, 303);
    assert_eq!(
        snapshot.register_bag_storage(bag_slot, bag_guid, 2),
        canonical.register_bag_storage(bag_slot, bag_guid, 2)
    );
    snapshot.store_bag_item(bag_slot, 0, child_guid).unwrap();
    canonical.store_bag_item(bag_slot, 0, child_guid).unwrap();
    assert_eq!(
        snapshot.remove_bag_item(bag_slot, 0),
        canonical.remove_bag_item(bag_slot, 0)
    );
    assert_eq!(
        snapshot.inventory.bag_at(bag_slot).unwrap().item_by_pos(0),
        None
    );
    snapshot.store_bag_item(bag_slot, 0, child_guid).unwrap();
    canonical.store_bag_item(bag_slot, 0, child_guid).unwrap();

    let old_top_level = ObjectGuid::create_item(17, 100);
    assert_eq!(
        snapshot.remove_top_level_item(bag_slot),
        Ok(Some(old_top_level))
    );
    assert_eq!(
        canonical.remove_top_level_item(bag_slot),
        Ok(Some(old_top_level))
    );
    assert!(snapshot.inventory.bag_at(bag_slot).is_none());
    assert!(canonical.inventory.bag_at(bag_slot).is_none());

    assert_eq!(snapshot.remove_top_level_item(40), Ok(None));
    assert_eq!(canonical.remove_top_level_item(40), Ok(None));
    assert_eq!(
        snapshot.values_update(false),
        canonical.values_update(false)
    );
    assert_eq!(snapshot.values_update(true), canonical.values_update(true));
}
