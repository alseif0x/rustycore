use super::InventoryPlayerSnapshotLikeCpp;
use crate::Item;
use crate::player::{
    BANK_SLOT_BAG_START, CanBankItemArgs, CanEquipItemArgs, CanEquipUniqueItemArgs,
    CanStoreItemArgs, CanUnequipItemArgs, CanUseItemArgs, CanUseItemTemplateArgs,
    EQUIPMENT_SLOT_FINGER1, EQUIPMENT_SLOT_FINGER2, EQUIPMENT_SLOT_HEAD, EquippedGemRef,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START,
    ITEM_LIMIT_CATEGORY_MODE_HAVE, ItemLimitCategoryTemplate, ItemSearchCallbackResult,
    ItemSearchLocation, ItemSlotRef, ItemStorageRef, PLAYER_SLOT_END, Player, PlayerStorageError,
    SocketedGemUniqueRef,
};
use crate::{ItemReadView, ItemStorageTemplate, ItemValuesSnapshot};
use wow_constants::{
    InventoryResult, InventoryType, ItemClass, ItemFieldFlags, ItemSubClassContainer,
};
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
    can_use_args_with_source(None, None)
}

fn can_use_args_with_source<'a>(
    proto: Option<&'a ItemStorageTemplate>,
    source_item: Option<ItemReadView<'a>>,
) -> CanUseItemArgs<'a> {
    let template_args = CanUseItemTemplateArgs {
        proto,
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
        source_item,
        proto,
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

fn can_equip_args_with_source<'a>(
    slot: u8,
    proto: &'a ItemStorageTemplate,
    source_item: ItemReadView<'a>,
    swap: bool,
    can_equip_unique_result: InventoryResult,
    equipped_items: &'a [ItemSlotRef<'a>],
    stored_items: &'a [ItemStorageRef<'a>],
) -> CanEquipItemArgs<'a> {
    CanEquipItemArgs {
        slot,
        proto: Some(proto),
        source_item: Some(source_item),
        source_bop_trade_allowed_for_player: false,
        swap,
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
        can_equip_unique_result,
        can_dual_wield: false,
        can_titan_grip: false,
        is_two_hand_used: false,
        proto_always_allow_dual_wield: false,
        has_required_profession_skill: false,
        profession_slot: None,
        offhand_can_unequip_result: InventoryResult::Ok,
        offhand_can_store_result: InventoryResult::Ok,
        limit_category: None,
        equipped_items,
        stored_items,
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

fn can_equip_unique_args_with_source<'a>(
    source_item: ItemReadView<'a>,
    proto: &'a ItemStorageTemplate,
    limit_category: Option<&'a ItemLimitCategoryTemplate>,
    equipped_gems: &'a [EquippedGemRef],
    socketed_gems: &'a [SocketedGemUniqueRef<'a>],
) -> CanEquipUniqueItemArgs<'a> {
    CanEquipUniqueItemArgs {
        source_item: Some(source_item),
        proto: Some(proto),
        except_slot: crate::player::NULL_SLOT,
        limit_count: 1,
        unique_equippable: false,
        limit_category,
        equipped_items: &[],
        equipped_gems,
        socketed_gems,
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
fn captured_item_read_view_keeps_usability_binding_facts() {
    let player = query_player();
    let snapshot = InventoryPlayerSnapshotLikeCpp::capture(&player);
    let proto = ItemStorageTemplate::regular_item(20_012, 20);
    let owner_guid = ObjectGuid::create_player(17, 0xC8B);
    let mut source = Item::default();
    source.set_item_flag(ItemFieldFlags::SOULBOUND);
    source.set_owner_guid(owner_guid);
    let captured_source = ItemValuesSnapshot::capture(&source);

    source.set_owner_guid(ObjectGuid::create_player(17, 0xC8C));

    let captured_view = captured_source.read_view();
    let captured_args = can_use_args_with_source(Some(&proto), Some(captured_view));
    assert_eq!(snapshot.can_use_item(captured_args), InventoryResult::Ok);
    assert_eq!(player.can_use_item(captured_args), InventoryResult::Ok);

    let live_args = can_use_args_with_source(Some(&proto), Some(source.read_view()));
    assert_eq!(snapshot.can_use_item(live_args), InventoryResult::NotOwner);
    assert_eq!(player.can_use_item(live_args), InventoryResult::NotOwner);

    let mut missing_source_args = can_use_args_with_source(Some(&proto), None);
    missing_source_args.is_alive = false;
    assert_eq!(
        snapshot.can_use_item(missing_source_args),
        InventoryResult::ItemNotFound
    );

    let mut dead_before_template_args = can_use_args_with_source(None, Some(source.read_view()));
    dead_before_template_args.is_alive = false;
    assert_eq!(
        player.can_use_item(dead_before_template_args),
        InventoryResult::PlayerDead
    );

    let mut loading_args = can_use_args_with_source(Some(&proto), Some(source.read_view()));
    loading_args.is_alive = false;
    loading_args.not_loading = false;
    loading_args.item_required_level = 71;
    assert_eq!(
        snapshot.can_use_item(loading_args),
        InventoryResult::NotOwner
    );

    let mut level_args = can_use_args_with_source(Some(&proto), Some(captured_source.read_view()));
    level_args.item_required_level = 71;
    assert_eq!(
        player.can_use_item(level_args),
        InventoryResult::CantEquipLevelI
    );
}

#[test]
fn equipment_identity_branches_compare_read_referents() {
    let player = query_player();
    let snapshot = InventoryPlayerSnapshotLikeCpp::capture(&player);
    let finger = ItemStorageTemplate {
        inventory_type: InventoryType::Finger,
        ..ItemStorageTemplate::regular_item(20_020, 1)
    };
    let shared_guid = ObjectGuid::create_item(17, 520);
    let mut ring_source = Item::default();
    ring_source.object_mut().create(shared_guid);
    ring_source.set_count(1);
    let mut equal_but_distinct_ring = Item::default();
    equal_but_distinct_ring.object_mut().create(shared_guid);
    equal_but_distinct_ring.set_count(1);
    assert_eq!(ring_source.data(), equal_but_distinct_ring.data());

    let shared_paired_ring = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_FINGER2,
        &ring_source,
    )];
    let shared_args = can_equip_args_with_source(
        EQUIPMENT_SLOT_FINGER1,
        &finger,
        ring_source.read_view(),
        true,
        InventoryResult::ItemUniqueEquippable,
        &shared_paired_ring,
        &[],
    );
    assert_eq!(
        player.can_equip_item(shared_args).unique_ignore_slot,
        Some(EQUIPMENT_SLOT_FINGER2)
    );
    assert_eq!(
        snapshot.can_equip_item(shared_args).unique_ignore_slot,
        Some(EQUIPMENT_SLOT_FINGER2)
    );

    let distinct_paired_ring = [ItemSlotRef::new(
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_FINGER2,
        &equal_but_distinct_ring,
    )];
    let distinct_args = can_equip_args_with_source(
        EQUIPMENT_SLOT_FINGER1,
        &finger,
        ring_source.read_view(),
        true,
        InventoryResult::ItemUniqueEquippable,
        &distinct_paired_ring,
        &[],
    );
    assert_eq!(
        player.can_equip_item(distinct_args).unique_ignore_slot,
        Some(EQUIPMENT_SLOT_FINGER1)
    );
    assert_eq!(
        snapshot.can_equip_item(distinct_args).unique_ignore_slot,
        Some(EQUIPMENT_SLOT_FINGER1)
    );

    let empty_player = Player::new(None, false);
    let empty_snapshot = InventoryPlayerSnapshotLikeCpp::capture(&empty_player);
    let quiver = ItemStorageTemplate {
        class_id: ItemClass::Quiver,
        subclass_id: crate::player::ItemSubClassQuiver::AmmoPouch as u32,
        inventory_type: InventoryType::Bag,
        ..ItemStorageTemplate::regular_item(20_021, 1)
    };
    let quiver_guid = ObjectGuid::create_item(17, 521);
    let mut quiver_source = Item::default();
    quiver_source.object_mut().create(quiver_guid);
    quiver_source.set_count(1);
    let mut equal_but_distinct_quiver = Item::default();
    equal_but_distinct_quiver.object_mut().create(quiver_guid);
    equal_but_distinct_quiver.set_count(1);
    assert_eq!(quiver_source.data(), equal_but_distinct_quiver.data());

    let shared_quiver_storage = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START,
        &quiver_source,
        Some(&quiver),
    )];
    let shared_quiver_args = can_equip_args_with_source(
        crate::player::NULL_SLOT,
        &quiver,
        quiver_source.read_view(),
        false,
        InventoryResult::Ok,
        &[],
        &shared_quiver_storage,
    );
    assert_eq!(
        empty_player.can_equip_item(shared_quiver_args).result,
        InventoryResult::Ok
    );
    assert_eq!(
        empty_snapshot.can_equip_item(shared_quiver_args).result,
        InventoryResult::Ok
    );

    let distinct_quiver_storage = [ItemStorageRef::new(
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_BAG_START,
        &equal_but_distinct_quiver,
        Some(&quiver),
    )];
    let distinct_quiver_args = can_equip_args_with_source(
        crate::player::NULL_SLOT,
        &quiver,
        quiver_source.read_view(),
        false,
        InventoryResult::Ok,
        &[],
        &distinct_quiver_storage,
    );
    assert_eq!(
        empty_player.can_equip_item(distinct_quiver_args).result,
        InventoryResult::OnlyOneAmmo
    );
    assert_eq!(
        empty_snapshot.can_equip_item(distinct_quiver_args).result,
        InventoryResult::OnlyOneAmmo
    );
}

#[test]
fn captured_item_read_view_drives_equipment_and_unique_queries() {
    let player = query_player();
    let snapshot = InventoryPlayerSnapshotLikeCpp::capture(&player);
    let head = ItemStorageTemplate {
        class_id: ItemClass::Armor,
        inventory_type: InventoryType::Head,
        ..ItemStorageTemplate::regular_item(20_022, 1)
    };
    let player_guid = ObjectGuid::create_player(17, 0xC8B);
    let mut source = Item::default();
    source.set_count(1);
    source.set_item_flag(ItemFieldFlags::SOULBOUND);
    source.set_owner_guid(player_guid);
    let captured_source = ItemValuesSnapshot::capture(&source);
    source.set_owner_guid(ObjectGuid::create_player(17, 0xC8C));

    let captured_args = can_equip_args_with_source(
        EQUIPMENT_SLOT_HEAD,
        &head,
        captured_source.read_view(),
        false,
        InventoryResult::Ok,
        &[],
        &[],
    );
    assert_eq!(
        player.can_equip_item(captured_args).result,
        InventoryResult::Ok
    );
    assert_eq!(
        snapshot.can_equip_item(captured_args).result,
        InventoryResult::Ok
    );

    let live_args = can_equip_args_with_source(
        EQUIPMENT_SLOT_HEAD,
        &head,
        source.read_view(),
        false,
        InventoryResult::Ok,
        &[],
        &[],
    );
    assert_eq!(
        player.can_equip_item(live_args).result,
        InventoryResult::NotOwner
    );
    assert_eq!(
        snapshot.can_equip_item(live_args).result,
        InventoryResult::NotOwner
    );

    let mut unique_source = Item::default();
    unique_source.set_slot(EQUIPMENT_SLOT_HEAD);
    let captured_unique_source = ItemValuesSnapshot::capture(&unique_source);
    unique_source.set_slot(INVENTORY_SLOT_ITEM_START);
    let source_proto = ItemStorageTemplate::regular_item(20_023, 1);
    let gem_proto = ItemStorageTemplate {
        item_limit_category: 77,
        ..ItemStorageTemplate::regular_item(20_024, 1)
    };
    let gem_limit_category = ItemLimitCategoryTemplate {
        id: 77,
        quantity: 2,
        flags: ITEM_LIMIT_CATEGORY_MODE_HAVE,
    };
    let equipped_gems = [EquippedGemRef::new(EQUIPMENT_SLOT_FINGER1, 20_024, 77)];
    let socketed_gems = [SocketedGemUniqueRef::new(
        Some(&gem_proto),
        false,
        Some(&gem_limit_category),
        3,
    )];
    let captured_unique_args = can_equip_unique_args_with_source(
        captured_unique_source.read_view(),
        &source_proto,
        None,
        &equipped_gems,
        &socketed_gems,
    );
    assert_eq!(
        player.can_equip_unique_item(captured_unique_args),
        InventoryResult::Ok
    );
    assert_eq!(
        snapshot.can_equip_unique_item(captured_unique_args),
        InventoryResult::Ok
    );

    let live_unique_args = can_equip_unique_args_with_source(
        unique_source.read_view(),
        &source_proto,
        None,
        &equipped_gems,
        &socketed_gems,
    );
    assert_eq!(
        player.can_equip_unique_item(live_unique_args),
        InventoryResult::ItemMaxLimitCategoryEquippedExceededIs
    );
    assert_eq!(
        snapshot.can_equip_unique_item(live_unique_args),
        InventoryResult::ItemMaxLimitCategoryEquippedExceededIs
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
