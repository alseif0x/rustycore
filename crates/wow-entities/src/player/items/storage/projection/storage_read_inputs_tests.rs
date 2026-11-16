use super::{InventoryPlayerSnapshotLikeCpp, query_player};
use crate::player::{
    BANK_SLOT_ITEM_START, CanBankItemArgs, CanStoreItemArgs, CanUnequipItemArgs,
    EQUIPMENT_SLOT_HEAD, INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START, ItemPosCount,
    ItemSlotRef, ItemStorageRef, NULL_BAG, NULL_SLOT, make_item_pos,
};
use crate::{Item, ItemReadView, ItemStorageTemplate, ItemValuesSnapshot};
use wow_constants::{InventoryResult, ItemFieldFlags};
use wow_core::ObjectGuid;

fn bound_source(guid: ObjectGuid, owner: ObjectGuid, entry: u32) -> Item {
    let mut item = Item::default();
    item.object_mut().create(guid);
    item.object_mut().set_entry(entry);
    item.set_count(6);
    item.set_item_flag(ItemFieldFlags::SOULBOUND);
    item.set_owner_guid(owner);
    item
}

fn store_args<'a>(
    source: ItemReadView<'a>,
    proto: &'a ItemStorageTemplate,
    slot_items: &'a [ItemSlotRef<'a>],
    stored_items: &'a [ItemStorageRef<'a>],
) -> CanStoreItemArgs<'a> {
    CanStoreItemArgs {
        bag: NULL_BAG,
        slot: NULL_SLOT,
        entry: proto.entry,
        count: source.count(),
        proto: Some(proto),
        source_item: Some(source),
        source_is_not_empty_bag: false,
        source_bop_trade_allowed_for_player: false,
        swap: false,
        limit_category: None,
        slot_items,
        stored_items,
        bag_templates: &[],
    }
}

fn bank_args<'a>(
    source: ItemReadView<'a>,
    proto: &'a ItemStorageTemplate,
) -> CanBankItemArgs<'a> {
    CanBankItemArgs {
        bag: INVENTORY_SLOT_BAG_0,
        slot: BANK_SLOT_ITEM_START,
        proto: Some(proto),
        source_item: Some(source),
        source_is_not_empty_bag: false,
        source_is_bag: false,
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

fn unequip_args<'a>(
    source_item: ItemReadView<'a>,
    proto: &'a ItemStorageTemplate,
) -> CanUnequipItemArgs<'a> {
    CanUnequipItemArgs {
        pos: make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_HEAD),
        source_item: Some(source_item),
        proto: Some(proto),
        swap: false,
        source_is_not_empty_bag: false,
        is_charmed: false,
        is_in_combat: false,
        is_in_progress_arena: false,
    }
}

#[test]
fn captured_storage_bank_and_unequip_inputs_keep_item_facts_and_guid_skip() {
    let owner = query_player();
    let snapshot = InventoryPlayerSnapshotLikeCpp::capture(&owner);
    let canonical = query_player();
    let owner_guid = ObjectGuid::create_player(17, 0xC8B);
    let other_guid = ObjectGuid::create_player(17, 0xC8C);
    let proto = ItemStorageTemplate::regular_item(20_030, 20);

    let source_guid = ObjectGuid::create_item(17, 8_000);
    let mut snapshot_source = bound_source(source_guid, owner_guid, proto.entry);
    let mut player_source = bound_source(source_guid, owner_guid, proto.entry);
    let snapshot_source_facts = ItemValuesSnapshot::capture(&snapshot_source);
    let player_source_facts = ItemValuesSnapshot::capture(&player_source);
    snapshot_source.set_count(1);
    snapshot_source.set_owner_guid(other_guid);
    player_source.set_count(1);
    player_source.set_owner_guid(other_guid);

    let mut matching_stack = Item::default();
    matching_stack
        .object_mut()
        .create(ObjectGuid::create_item(17, 8_001));
    matching_stack.object_mut().set_entry(proto.entry);
    matching_stack.set_count(18);
    let matching_stack_facts = ItemValuesSnapshot::capture(&matching_stack);
    matching_stack.set_count(1);
    let slot_items = [
        ItemSlotRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            snapshot_source.read_view(),
        ),
        ItemSlotRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            matching_stack_facts.read_view(),
        ),
    ];
    let stored_items = [
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
            snapshot_source.read_view(),
            Some(&proto),
        ),
        ItemStorageRef::new(
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START + 1,
            matching_stack_facts.read_view(),
            Some(&proto),
        ),
    ];

    let snapshot_source_view = snapshot_source_facts.read_view();
    let player_source_view = player_source_facts.read_view();
    assert_eq!(snapshot_source_view.count(), 6);
    assert!(!snapshot_source_view.is_binded_not_with(owner_guid, &proto, false));
    assert!(snapshot_source.read_view().is_binded_not_with(owner_guid, &proto, false));
    assert!(!player_source_view.is_binded_not_with(owner_guid, &proto, false));
    assert!(player_source.read_view().is_binded_not_with(owner_guid, &proto, false));

    let mut snapshot_dest = Vec::new();
    let snapshot_outcome = snapshot.can_store_item(
        &mut snapshot_dest,
        store_args(snapshot_source_view, &proto, &slot_items, &stored_items),
    );
    let mut player_dest = Vec::new();
    let player_outcome = canonical.can_store_item(
        &mut player_dest,
        store_args(player_source_view, &proto, &slot_items, &stored_items),
    );
    assert_eq!(snapshot_outcome, player_outcome);
    assert_eq!(snapshot_outcome.result, InventoryResult::Ok);
    assert_eq!(snapshot_dest, player_dest);
    assert_eq!(
        snapshot_dest,
        vec![
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START + 1),
                2,
            ),
            ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START),
                4,
            ),
        ]
    );
    assert_eq!(
        canonical
            .can_store_item(
                &mut Vec::new(),
                store_args(
                    snapshot_source.read_view(),
                    &proto,
                    &slot_items,
                    &stored_items,
                ),
            )
            .result,
        InventoryResult::NotOwner
    );

    let mut snapshot_bank_source = Item::default();
    snapshot_bank_source
        .object_mut()
        .create(ObjectGuid::create_item(17, 8_002));
    snapshot_bank_source.object_mut().set_entry(proto.entry);
    snapshot_bank_source.set_count(7);
    let snapshot_bank_facts = ItemValuesSnapshot::capture(&snapshot_bank_source);
    snapshot_bank_source.set_count(2);
    let mut player_bank_source = Item::default();
    player_bank_source
        .object_mut()
        .create(ObjectGuid::create_item(17, 8_003));
    player_bank_source.object_mut().set_entry(proto.entry);
    player_bank_source.set_count(7);
    let player_bank_facts = ItemValuesSnapshot::capture(&player_bank_source);
    player_bank_source.set_count(2);

    let mut snapshot_bank_dest = Vec::new();
    let mut player_bank_dest = Vec::new();
    assert_eq!(
        snapshot.can_bank_item(
            &mut snapshot_bank_dest,
            bank_args(snapshot_bank_facts.read_view(), &proto),
        ),
        InventoryResult::Ok
    );
    assert_eq!(
        canonical.can_bank_item(
            &mut player_bank_dest,
            bank_args(player_bank_facts.read_view(), &proto),
        ),
        InventoryResult::Ok
    );
    assert_eq!(snapshot_bank_dest, player_bank_dest);
    assert_eq!(
        snapshot_bank_dest,
        vec![ItemPosCount::new(
            make_item_pos(INVENTORY_SLOT_BAG_0, BANK_SLOT_ITEM_START),
            7,
        )]
    );

    snapshot_source.set_loot_generated(true);
    player_source.set_loot_generated(true);
    assert_eq!(
        snapshot.can_unequip_item(unequip_args(snapshot_source_facts.read_view(), &proto)),
        InventoryResult::Ok
    );
    assert_eq!(
        canonical.can_unequip_item(unequip_args(player_source_facts.read_view(), &proto)),
        InventoryResult::Ok
    );
    assert_eq!(
        snapshot.can_unequip_item(unequip_args(snapshot_source.read_view(), &proto)),
        InventoryResult::LootGone
    );
    assert_eq!(
        canonical.can_unequip_item(unequip_args(player_source.read_view(), &proto)),
        InventoryResult::LootGone
    );
}
