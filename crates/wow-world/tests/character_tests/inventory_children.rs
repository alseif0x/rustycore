//! Child redirect validation over one real canonical Player inventory.

use super::item_fixture_rows::{basic_item_record, inventory_sparse_template};
use wow_world::session::{InventoryItem, WorldSession};
use wow_world::test_fixtures::{
    get_inventory_item_by_pos_for_test, insert_inventory_item_for_test,
    insert_inventory_item_object_for_test, make_inventory_item_object_for_test,
    mark_inventory_child_for_test, install_canonical_player_owner_for_test,
    inventory_items_for_test, inventory_swap_preflight_for_test,
    inventory_child_redirect_for_test, mutate_canonical_player_for_test,
    set_equipment_set_guid_generator_for_test,
};
use std::sync::Arc;
use wow_constants::{
    InventoryResult, InventoryType, ItemClass, ItemContext,
    ItemSubClassWeapon,
};
use wow_core::guid::HighGuid;
use wow_core::{EquipmentSetGuidGeneratorLikeCpp, ObjectGuid, ObjectGuidGenerator};
use wow_data::item::stats::{
    ItemModType, ItemSparseTemplateEntry, ItemStatEntry, ItemStatsStore,
};
use wow_entities::{
    CHILD_EQUIPMENT_SLOT_START, EQUIPMENT_SLOT_MAINHAND, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_ITEM_START, SwapItemPreflightResult,
};
use wow_packet::WorldPacket;

pub(super) fn make_session_with_send_capacity(
    capacity: usize,
) -> (WorldSession, flume::Receiver<Vec<u8>>) {
    let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
    let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
    let mut session = WorldSession::new(
        1,
        "TestAccount".into(),
        0,
        2,
        9,
        54261,
        vec![0u8; 40],
        "esES".into(),
        pkt_rx,
        send_tx,
    );
    session.set_item_guid_generator_like_cpp(Arc::new(ObjectGuidGenerator::new(
        HighGuid::Item,
        1,
    )));
    set_equipment_set_guid_generator_for_test(&mut session, Arc::new(
        EquipmentSetGuidGeneratorLikeCpp::new(1),
    ));
    (session, send_rx)
}

pub(super) fn install_equippable_item_fixture(
    session: &mut WorldSession,
    entry_id: u32,
    inventory_type: InventoryType,
    strength: Option<i16>,
) {
    session.set_item_store(Arc::new(wow_data::ItemStore::from_records([basic_item_record(
        entry_id,
        ItemClass::Weapon as u8,
        ItemSubClassWeapon::Sword as u8,
        inventory_type as i8,
    )])));
    let sparse = ItemSparseTemplateEntry {
        max_durability: 100,
        ..inventory_sparse_template(inventory_type as i8)
    };
    let stats = strength.into_iter().map(|amount| {
        (
            entry_id,
            ItemStatEntry {
                stats: std::array::from_fn(|index| {
                    if index == 0 {
                        (ItemModType::Strength as i8, amount)
                    } else {
                        (ItemModType::None as i8, 0)
                    }
                }),
                resistances: [0; 7],
                armor: 0,
            },
        )
    });
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_stats_sparse_and_random_property_templates(
            stats,
            [(entry_id, sparse)],
            [],
        ),
    ));
}

pub(super) fn insert_equippable_test_item(
    session: &mut WorldSession,
    bag: u8,
    slot: u8,
    entry_id: u32,
    db_guid: u64,
    inventory_type: InventoryType,
) -> ObjectGuid {
    let player_guid = session.player_guid().expect("test player");
    let item_guid = ObjectGuid::create_item(1, db_guid as i64);
    if bag == INVENTORY_SLOT_BAG_0 {
        insert_inventory_item_for_test(
            session,
            slot,
            InventoryItem {
                guid: item_guid,
                entry_id,
                db_guid,
                inventory_type: Some(inventory_type as u8),
            },
        );
    }
    let mut item = make_inventory_item_object_for_test(
        session,
        item_guid,
        entry_id,
        player_guid,
        1,
        0,
        ItemContext::None,
        slot,
    );
    if bag != INVENTORY_SLOT_BAG_0 {
        let bag_guid = inventory_items_for_test(session)
            .get(&bag)
            .expect("represented bag")
            .guid;
        item.set_container_guid_and_slot(bag_guid, bag);
    }
    insert_inventory_item_object_for_test(session, item);
    item_guid
}

pub(super) fn install_child_equipment_fixture(
    session: &mut WorldSession,
    parent_entry: u32,
    child_entry: u32,
    child_slot: u8,
) {
    session.set_item_child_equipment_store(Arc::new(
        wow_data::ItemChildEquipmentStore::from_entries([wow_data::ItemChildEquipmentEntry {
            id: 1,
            child_item_id: child_entry as i32,
            child_item_equip_slot: child_slot,
            parent_item_id: parent_entry,
        }]),
    ));
}

#[test]
fn child_redirect_validates_both_steps_without_mutating_runtime_like_upstream_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry_id = 121;
    session.set_player_guid(Some(player_guid));
    install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.set_player_alive_like_cpp(true);
    install_equippable_item_fixture(&mut session, entry_id, InventoryType::Weapon, None);

    let source_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        entry_id,
        71,
        InventoryType::Weapon,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        entry_id,
        72,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry_id,
        73,
        InventoryType::Weapon,
    );
    mark_inventory_child_for_test(&mut session, child_guid, parent_guid);


    mutate_canonical_player_for_test(&session, |player| {
        player.set_inventory_slot_count(wow_entities::INVENTORY_DEFAULT_SIZE);
        player.set_bank_bag_slot_count(0);
        for (slot, guid) in [
            (INVENTORY_SLOT_ITEM_START, source_guid),
            (wow_entities::EQUIPMENT_SLOT_OFFHAND, parent_guid),
            (EQUIPMENT_SLOT_MAINHAND, child_guid),
        ] {
            let _ = player.store_top_level_item(slot, guid);
        }
    })
    .expect("canonical inventory fixture");

    let source = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let destination =
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND);
    let redirect = inventory_swap_preflight_for_test(&session, source, destination)
        .expect("player inventory preflight");
    let SwapItemPreflightResult::ChildRedirect {
        first_src,
        first_dst,
        second_src,
        second_dst,
    } = redirect.result
    else {
        panic!("equipped child destination must redirect through its parent");
    };

    assert_eq!(
        inventory_child_redirect_for_test(&mut session, 
                INVENTORY_SLOT_BAG_0,
                EQUIPMENT_SLOT_MAINHAND,
                first_src,
                first_dst,
                second_src,
                second_dst,
            )
            .expect("both redirected moves are legal"),
        CHILD_EQUIPMENT_SLOT_START
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .map(|item| item.guid),
        Some(child_guid),
        "the validation overlay must restore the visible child slot"
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
        )
        .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            wow_entities::EQUIPMENT_SLOT_OFFHAND,
        )
        .map(|item| item.guid),
        Some(parent_guid)
    );
    assert!(
        get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            CHILD_EQUIPMENT_SLOT_START,
        )
        .is_none()
    );
}

#[test]
fn rejected_child_redirect_restores_validation_overlay_before_error_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(1);
    let player_guid = ObjectGuid::create_player(1, 42);
    let entry_id = 122;
    session.set_player_guid(Some(player_guid));
    install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.set_player_alive_like_cpp(false);
    install_equippable_item_fixture(&mut session, entry_id, InventoryType::Weapon, None);

    let source_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        INVENTORY_SLOT_ITEM_START,
        entry_id,
        74,
        InventoryType::Weapon,
    );
    let parent_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        wow_entities::EQUIPMENT_SLOT_OFFHAND,
        entry_id,
        75,
        InventoryType::Weapon,
    );
    let child_guid = insert_equippable_test_item(
        &mut session,
        INVENTORY_SLOT_BAG_0,
        EQUIPMENT_SLOT_MAINHAND,
        entry_id,
        76,
        InventoryType::Weapon,
    );
    mark_inventory_child_for_test(&mut session, child_guid, parent_guid);


    mutate_canonical_player_for_test(&session, |player| {
        player.set_inventory_slot_count(wow_entities::INVENTORY_DEFAULT_SIZE);
        player.set_bank_bag_slot_count(0);
        for (slot, guid) in [
            (INVENTORY_SLOT_ITEM_START, source_guid),
            (wow_entities::EQUIPMENT_SLOT_OFFHAND, parent_guid),
            (EQUIPMENT_SLOT_MAINHAND, child_guid),
        ] {
            let _ = player.store_top_level_item(slot, guid);
        }
    })
    .expect("canonical inventory fixture");

    let source = wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_START);
    let destination =
        wow_entities::make_item_pos(INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND);
    let redirect = inventory_swap_preflight_for_test(&session, source, destination)
        .expect("player inventory preflight");
    let SwapItemPreflightResult::ChildRedirect {
        first_src,
        first_dst,
        second_src,
        second_dst,
    } = redirect.result
    else {
        panic!("C++ examines child redirects before the player-dead gate");
    };

    assert_eq!(
        inventory_child_redirect_for_test(&mut session, 
            INVENTORY_SLOT_BAG_0,
            EQUIPMENT_SLOT_MAINHAND,
            first_src,
            first_dst,
            second_src,
            second_dst,
        ),
        Err(InventoryResult::PlayerDead)
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(&session, INVENTORY_SLOT_BAG_0, EQUIPMENT_SLOT_MAINHAND)
            .map(|item| item.guid),
        Some(child_guid),
        "a rejected redirected move must not persist the child in a hidden slot"
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            INVENTORY_SLOT_ITEM_START,
        )
        .map(|item| item.guid),
        Some(source_guid)
    );
    assert_eq!(
        get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            wow_entities::EQUIPMENT_SLOT_OFFHAND,
        )
        .map(|item| item.guid),
        Some(parent_guid)
    );
    assert!(
        get_inventory_item_by_pos_for_test(
            &session,
            INVENTORY_SLOT_BAG_0,
            CHILD_EQUIPMENT_SLOT_START,
        )
        .is_none()
    );
}
