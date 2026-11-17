use super::*;
use std::sync::{Arc, Mutex};
use wow_core::{ObjectGuid, Position};
use wow_entities::{
    EQUIPMENT_SLOT_CHEST, EQUIPMENT_SLOT_OFFHAND, INVENTORY_SLOT_ITEM_START, Item,
    ItemValuesSnapshot, Player, PlayerInventoryItem,
};
use wow_map::{MIN_GRID_DELAY_MS, MapKey, MapManager, PlayerHandle};

const MAP: MapKey = MapKey::new(571, 0);
const CHEST_ENTRY: u32 = 71_101;
const WEAPON_ENTRY: u32 = 71_102;
const TWO_HAND_ENTRY: u32 = 71_103;

fn player_guid() -> ObjectGuid {
    ObjectGuid::create_player(1, 71_100)
}

fn record(guid: ObjectGuid, entry_id: u32, inventory_type: InventoryType) -> PlayerInventoryItem {
    PlayerInventoryItem {
        guid,
        entry_id,
        db_guid: guid.counter() as u64,
        inventory_type: Some(inventory_type as u8),
    }
}

fn item(guid: ObjectGuid, entry_id: u32, slot: u8, level: u16) -> Item {
    let mut item = Item::new(0);
    item.object_mut().create(guid);
    item.object_mut().set_entry(entry_id);
    item.set_owner_guid(player_guid());
    item.set_contained_in(player_guid());
    item.set_slot(slot);
    item.set_count(1);
    item.set_debug_item_level(level);
    item
}

fn empty_player() -> Player {
    let mut player = Player::new(Some(80), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid());
    player.set_inventory_slot_count(16);
    player
}

fn install_owner(
    session: &mut WorldSession,
    player: Player,
    active: bool,
) -> (Arc<Mutex<MapManager>>, PlayerHandle) {
    let mut manager = MapManager::new(MIN_GRID_DELAY_MS, 10);
    if active {
        manager.create_world_map(MAP.map_id, MAP.instance_id);
    }
    let handle = manager
        .install_detached_player_like_cpp(Box::new(player))
        .expect("real Player should install in the owner manager");
    if active {
        manager
            .attach_player_like_cpp(handle, MAP, Position::ZERO)
            .expect("real Player should attach to the current map");
    }
    let manager = Arc::new(Mutex::new(manager));
    session.set_player_guid(Some(player_guid()));
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.core.current_map_id = MAP.map_id as u16;
    session.core.player_handle_like_cpp = Some(handle);
    (manager, handle)
}

fn install_catalogs(session: &mut WorldSession) {
    session.set_item_store(Arc::new(ItemStore::from_records([
        represented_test_item_record_like_cpp(
            CHEST_ENTRY,
            InventoryType::Chest,
            ItemClass::Armor,
            ItemSubClassArmor::Cloth as u8,
        ),
        represented_test_item_record_like_cpp(
            WEAPON_ENTRY,
            InventoryType::Weapon,
            ItemClass::Weapon,
            ItemSubClassWeapon::Axe2 as u8,
        ),
        represented_test_item_record_like_cpp(
            TWO_HAND_ENTRY,
            InventoryType::Weapon2Hand,
            ItemClass::Weapon,
            ItemSubClassWeapon::Axe2 as u8,
        ),
    ])));
    session.set_item_stats_store(Arc::new(
        ItemStatsStore::from_sparse_and_random_property_templates(
            [
                (
                    CHEST_ENTRY,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Chest, 0),
                ),
                (
                    WEAPON_ENTRY,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Weapon, 0),
                ),
                (
                    TWO_HAND_ENTRY,
                    sparse_template_for_inventory_type_like_cpp(InventoryType::Weapon2Hand, 0),
                ),
            ],
            [
                (
                    CHEST_ENTRY,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 100,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                ),
                (
                    WEAPON_ENTRY,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 80,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Weapon as i8,
                    },
                ),
                (
                    TWO_HAND_ENTRY,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 120,
                        quality: ItemQuality::Epic as i8,
                        inventory_type: InventoryType::Weapon2Hand as i8,
                    },
                ),
            ],
        ),
    ));
}

fn seed_item(
    player: &mut Player,
    slot: u8,
    entry_id: u32,
    inventory_type: InventoryType,
    guid: ObjectGuid,
    level: u16,
) {
    player
        .store_top_level_item(slot, guid)
        .expect("item should occupy a valid canonical slot");
    let runtime = player.inventory_runtime_mut_like_cpp();
    runtime.store_item_in_slot_like_cpp(slot, record(guid, entry_id, inventory_type));
    runtime.store_item_object_like_cpp(item(guid, entry_id, slot, level));
}

fn poison_fixture(session: &mut WorldSession, level: u16) -> ObjectGuid {
    let guid = ObjectGuid::create_item(1, 71_199);
    session.inventory.insert_inventory_item_for_test_like_cpp(
        EQUIPMENT_SLOT_CHEST,
        record(guid, CHEST_ENTRY, InventoryType::Chest),
    );
    session
        .inventory
        .insert_inventory_item_object_for_test_like_cpp(
            guid,
            item(guid, CHEST_ENTRY, EQUIPMENT_SLOT_CHEST, level),
        );
    guid
}

fn averages(session: &WorldSession) -> Option<(f32, f32)> {
    let owned = session.represented_player_condition_context_like_cpp()?;
    let context = owned.as_context(session)?;
    Some((context.avg_item_level, context.avg_equipped_item_level))
}

fn owner_facts(
    session: &WorldSession,
    guids: [ObjectGuid; 2],
    slots: [u8; 2],
) -> (
    [ItemValuesSnapshot; 2],
    wow_entities::PlayerValuesUpdate,
    [Option<ObjectGuid>; 2],
    [Option<PlayerInventoryItem>; 2],
) {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let runtime = player.inventory_runtime_like_cpp();
            let items = guids.map(|guid| {
                ItemValuesSnapshot::capture(
                    runtime
                        .item_objects()
                        .get(&guid)
                        .expect("canonical runtime should own the captured item"),
                )
            });
            let records = slots.map(|slot| runtime.inventory_items().get(&slot).cloned());
            let topology = slots.map(|slot| player.top_level_item_guid(slot));
            (items, player.values_update(true), topology, records)
        })
        .expect("current Player owner should remain bound")
}

fn can_equip_candidate(
    session: &WorldSession,
    guid: ObjectGuid,
) -> wow_entities::CanEquipItemOutcome {
    let (record, values) = session
        .core
        .with_owned_player_like_cpp(|player| {
            let runtime = player.inventory_runtime_like_cpp();
            (
                runtime.inventory_items()[&INVENTORY_SLOT_ITEM_START].clone(),
                ItemValuesSnapshot::capture(&runtime.item_objects()[&guid]),
            )
        })
        .expect("current Player owner should provide the candidate");
    let projection = session.player_condition_projection_cx_like_cpp();
    wow_world_player_conditions::can_equip_inventory_item_like_cpp(
        &projection,
        &record,
        values.read_view(),
        EQUIPMENT_SLOT_CHEST,
        false,
        true,
        false,
        false,
        false,
    )
}

fn can_equip_two_hand_candidate(
    session: &WorldSession,
    guid: ObjectGuid,
) -> wow_entities::CanEquipItemOutcome {
    let (record, values) = session
        .core
        .with_owned_player_like_cpp(|player| {
            let runtime = player.inventory_runtime_like_cpp();
            (
                runtime.inventory_items()[&INVENTORY_SLOT_ITEM_START].clone(),
                ItemValuesSnapshot::capture(&runtime.item_objects()[&guid]),
            )
        })
        .expect("current Player owner should provide the two-hand candidate");
    let projection = session.player_condition_projection_cx_like_cpp();
    wow_world_player_conditions::can_equip_inventory_item_like_cpp(
        &projection,
        &record,
        values.read_view(),
        EQUIPMENT_SLOT_MAINHAND,
        false,
        true,
        false,
        false,
        false,
    )
}

#[test]
fn valuation_and_equip_use_active_then_detached_owner_without_mutation_or_packets() {
    let (mut session, _, send_rx) = make_session();
    install_catalogs(&mut session);
    let candidate_guid = ObjectGuid::create_item(1, 71_110);
    let offhand_guid = ObjectGuid::create_item(1, 71_111);
    let mut player = empty_player();
    seed_item(
        &mut player,
        INVENTORY_SLOT_ITEM_START,
        CHEST_ENTRY,
        InventoryType::Chest,
        candidate_guid,
        160,
    );
    seed_item(
        &mut player,
        EQUIPMENT_SLOT_OFFHAND,
        WEAPON_ENTRY,
        InventoryType::Weapon,
        offhand_guid,
        80,
    );
    let (manager, handle) = install_owner(&mut session, player, true);
    let poison_guid = poison_fixture(&mut session, 320);
    let guids = [candidate_guid, offhand_guid];
    let slots = [INVENTORY_SLOT_ITEM_START, EQUIPMENT_SLOT_OFFHAND];
    let before = owner_facts(&session, guids, slots);

    let active_averages = averages(&session).expect("active canonical Player should project");
    assert_eq!(active_averages, (15.0, 5.0));
    let active_equip = can_equip_candidate(&session, candidate_guid);
    assert_eq!(active_equip.result, InventoryResult::Ok);
    assert_eq!(owner_facts(&session, guids, slots), before);
    assert!(drain_server_opcodes(&send_rx).is_empty());

    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    assert_eq!(
        manager.lock().unwrap().player_residence_like_cpp(handle),
        Some(wow_map::PlayerResidenceLikeCpp::Detached)
    );
    let _ = drain_server_opcodes(&send_rx);
    assert_eq!(averages(&session), Some(active_averages));
    let detached_equip = can_equip_candidate(&session, candidate_guid);
    assert_eq!(detached_equip.result, active_equip.result);
    assert_eq!(detached_equip.dest, active_equip.dest);
    assert_eq!(owner_facts(&session, guids, slots), before);
    assert!(drain_server_opcodes(&send_rx).is_empty());

    assert!(
        session
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(1))
            .is_some()
    );
    let full_before = owner_facts(&session, guids, slots);
    let full_equip = can_equip_candidate(&session, candidate_guid);
    assert_eq!(full_equip.result, InventoryResult::Ok);
    assert_eq!(full_equip.dest, active_equip.dest);
    assert_eq!(owner_facts(&session, guids, slots), full_before);
    assert!(drain_server_opcodes(&send_rx).is_empty());

    manager
        .lock()
        .unwrap()
        .retire_player_like_cpp(handle)
        .expect("detached owner generation should retire");
    let mut replacement = empty_player();
    replacement
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(player_guid());
    let current_handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(Box::new(replacement))
        .expect("same-GUID replacement should install");
    assert_ne!(current_handle, handle);

    session.core.player_handle_like_cpp = Some(handle);
    assert!(
        averages(&session).is_none(),
        "stale handle must not read same-GUID replacement or poisoned fixture"
    );
    session.core.player_handle_like_cpp = Some(current_handle);
    assert_eq!(averages(&session), Some((0.0, 0.0)));
    session.core.player_handle_like_cpp = None;
    let fixture_averages = averages(&session).expect("absent handle retains fixture fallback");
    assert_eq!(fixture_averages, (20.0, 20.0));
    assert!(poison_guid != candidate_guid);
    assert!(drain_server_opcodes(&send_rx).is_empty());
}

#[test]
fn two_hand_offhand_storage_uses_selected_owner_capacity() {
    let (mut session, _, send_rx) = make_session();
    install_catalogs(&mut session);
    let candidate_guid = ObjectGuid::create_item(1, 71_120);
    let offhand_guid = ObjectGuid::create_item(1, 71_121);
    let mut player = empty_player();
    seed_item(
        &mut player,
        INVENTORY_SLOT_ITEM_START,
        TWO_HAND_ENTRY,
        InventoryType::Weapon2Hand,
        candidate_guid,
        120,
    );
    seed_item(
        &mut player,
        EQUIPMENT_SLOT_OFFHAND,
        WEAPON_ENTRY,
        InventoryType::Weapon,
        offhand_guid,
        80,
    );
    let (_manager, _handle) = install_owner(&mut session, player, false);

    assert_eq!(
        can_equip_two_hand_candidate(&session, candidate_guid).result,
        InventoryResult::Ok,
        "free backpack capacity lets the offhand move before equipping"
    );
    assert!(
        session
            .core
            .with_owned_player_mut_like_cpp(|player| player.set_inventory_slot_count(1))
            .is_some()
    );
    assert_eq!(
        can_equip_two_hand_candidate(&session, candidate_guid).result,
        InventoryResult::InvFull,
        "a full backpack blocks the same two-hand equip"
    );
    assert!(drain_server_opcodes(&send_rx).is_empty());
}
