use super::*;

use std::sync::{Arc, Mutex};

use wow_constants::ItemFieldFlags;
use wow_core::Position;
use wow_entities::{
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_BAG_START, INVENTORY_SLOT_ITEM_START, Item,
    ItemValuesMapSnapshot, ItemValuesSnapshot, Player, PlayerInventoryItem,
    PlayerQuestStatusRecord,
};
use wow_map::{MapKey, MapManager, PlayerHandle};
use wow_persistence::{
    InventoryGraphDestroyPersistenceLikeCpp, PersistenceOutcomeLikeCpp,
    PlayerInventoryPersistenceRequestLikeCpp,
};

const MAP: MapKey = MapKey::new(571, 0);
const ROOT_BAG_SLOT: u8 = INVENTORY_SLOT_BAG_START;
const CHILD_SLOT: u8 = 0;
const QUEST_ID: u32 = 91_701;
const ROOT_ENTRY: u32 = 600;
const CHILD_ENTRY: u32 = 700;

fn player_guid() -> ObjectGuid {
    ObjectGuid::create_player(1, 91_700)
}

fn root_guid() -> ObjectGuid {
    ObjectGuid::create_item(1, 91_701)
}

fn child_guid() -> ObjectGuid {
    ObjectGuid::create_item(1, 91_702)
}

fn inventory_record(
    guid: ObjectGuid,
    entry_id: u32,
    inventory_type: InventoryType,
) -> PlayerInventoryItem {
    PlayerInventoryItem {
        guid,
        entry_id,
        db_guid: guid.counter() as u64,
        inventory_type: Some(inventory_type as u8),
    }
}

fn item_values(
    guid: ObjectGuid,
    entry_id: u32,
    owner_guid: ObjectGuid,
    slot: u8,
    count: u32,
    container: Option<(ObjectGuid, u8)>,
) -> Item {
    let mut item = Item::new(0);
    item.object_mut().create(guid);
    item.object_mut().set_entry(entry_id);
    item.set_owner_guid(owner_guid);
    item.set_contained_in(owner_guid);
    item.set_slot(slot);
    item.set_count(count);
    if let Some((container_guid, container_slot)) = container {
        item.set_container_guid_and_slot(container_guid, container_slot);
    }
    item
}

fn install_owner(
    session: &mut WorldSession,
    player: Player,
    active: bool,
) -> (Arc<Mutex<MapManager>>, PlayerHandle) {
    let guid = player.guid();
    let mut manager = MapManager::new(wow_map::MIN_GRID_DELAY_MS, 10);
    if active {
        manager.create_world_map(MAP.map_id, MAP.instance_id);
    }
    let handle = manager
        .install_detached_player_like_cpp(Box::new(player))
        .expect("real Player should install in the owner manager");
    if active {
        manager
            .attach_player_like_cpp(handle, MAP, Position::ZERO)
            .expect("real Player should attach to its map");
    }
    let manager = Arc::new(Mutex::new(manager));
    session.set_player_guid(Some(guid));
    session.set_canonical_map_manager(Arc::clone(&manager));
    session.core.current_map_id = MAP.map_id as u16;
    session.core.player_handle_like_cpp = Some(handle);
    (manager, handle)
}

fn seed_quest_status(player: &mut Player, objective_counts: Vec<i32>) {
    player.gameplay_state_mut().quests.insert_status_like_cpp(
        QUEST_ID,
        PlayerQuestStatusRecord {
            quest_id: QUEST_ID,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts,
            slot: 0,
        },
    );
    player
        .gameplay_state_mut()
        .quests
        .set_status_authority_complete_like_cpp(true);
}

fn install_graph_quest(session: &mut WorldSession) {
    let mut quest = quest_template(QUEST_ID);
    quest.objectives = [CHILD_ENTRY, ROOT_ENTRY]
        .into_iter()
        .enumerate()
        .map(|(index, entry_id)| QuestObjective {
            id: QUEST_ID * 10 + index as u32,
            quest_id: QUEST_ID,
            obj_type: 1,
            order: index as u8,
            storage_index: index as i8,
            object_id: entry_id as i32,
            amount: 10,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        })
        .collect();
    session.set_quest_store(Arc::new(QuestStore::from_quests_like_cpp([quest])));
}

fn graph_owner(
    active: bool,
) -> (
    WorldSession,
    flume::Receiver<Vec<u8>>,
    PlayerInventoryItem,
    ItemValuesSnapshot,
) {
    let (mut session, send_rx) = make_session_with_send_capacity(4);
    let owner_guid = player_guid();
    let root_guid = root_guid();
    let child_guid = child_guid();
    let survivor_guid = ObjectGuid::create_item(1, 91_704);
    session.set_player_guid(Some(owner_guid));
    install_graph_quest(&mut session);

    // Contradictory fixture values make a missing canonical owner read visible.
    session.insert_inventory_item_like_cpp(
        ROOT_BAG_SLOT,
        PlayerInventoryItem {
            guid: root_guid,
            entry_id: 999,
            db_guid: 999_001,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    let fixture_root = item_values(root_guid, 999, owner_guid, ROOT_BAG_SLOT, 1, None);
    session.insert_inventory_item_object(fixture_root);
    let fixture_child = item_values(
        child_guid,
        998,
        owner_guid,
        CHILD_SLOT,
        1,
        Some((root_guid, ROOT_BAG_SLOT)),
    );
    session.insert_inventory_item_object(fixture_child);

    let mut player = Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(owner_guid);
    player.set_inventory_slot_count(16);
    player
        .store_top_level_item(ROOT_BAG_SLOT, root_guid)
        .expect("bag slot should be valid");
    player
        .register_bag_storage(ROOT_BAG_SLOT, root_guid, 4)
        .expect("bag storage should register");
    player
        .store_bag_item(ROOT_BAG_SLOT, CHILD_SLOT, child_guid)
        .expect("child should occupy the real bag slot");
    player
        .store_top_level_item(INVENTORY_SLOT_ITEM_START, survivor_guid)
        .expect("backpack slot should be valid");
    seed_quest_status(&mut player, vec![8, 1]);

    let root_record = inventory_record(root_guid, ROOT_ENTRY, InventoryType::Bag);
    let survivor_record = inventory_record(survivor_guid, CHILD_ENTRY, InventoryType::NonEquip);
    let mut root = item_values(root_guid, ROOT_ENTRY, owner_guid, ROOT_BAG_SLOT, 1, None);
    root.set_item_flag(ItemFieldFlags::REFUNDABLE);
    let root_snapshot = ItemValuesSnapshot::capture(&root);
    let survivor = item_values(
        survivor_guid,
        CHILD_ENTRY,
        owner_guid,
        INVENTORY_SLOT_ITEM_START,
        5,
        None,
    );
    let child = item_values(
        child_guid,
        CHILD_ENTRY,
        owner_guid,
        CHILD_SLOT,
        3,
        Some((root_guid, ROOT_BAG_SLOT)),
    );
    {
        let runtime = player.inventory_runtime_mut_like_cpp();
        runtime.store_item_in_slot_like_cpp(ROOT_BAG_SLOT, root_record.clone());
        runtime.store_item_in_slot_like_cpp(INVENTORY_SLOT_ITEM_START, survivor_record);
        runtime.store_item_object_like_cpp(root);
        runtime.store_item_object_like_cpp(survivor);
        runtime.store_item_object_like_cpp(child);
    }
    let _ = install_owner(&mut session, player, active);
    (session, send_rx, root_record, root_snapshot)
}

fn canonical_inventory_facts(
    session: &WorldSession,
) -> (
    std::collections::HashMap<u8, PlayerInventoryItem>,
    Vec<(ObjectGuid, ItemValuesSnapshot)>,
    Option<ObjectGuid>,
    Option<ObjectGuid>,
) {
    session
        .core
        .with_owned_player_like_cpp(|player| {
            let runtime = player.inventory_runtime_like_cpp();
            (
                runtime.inventory_items().clone(),
                ItemValuesMapSnapshot::capture(runtime.item_objects())
                    .iter()
                    .map(|(key, value)| (*key, value.clone()))
                    .collect(),
                player.top_level_item_guid(ROOT_BAG_SLOT),
                player.get_item_by_pos(ROOT_BAG_SLOT, CHILD_SLOT),
            )
        })
        .expect("current Player owner should remain bound")
}

fn graph_request(
    requests: &Arc<Mutex<Vec<PlayerInventoryPersistenceRequestLikeCpp>>>,
) -> InventoryGraphDestroyPersistenceLikeCpp {
    let requests = requests
        .lock()
        .expect("persistence requests should not be poisoned");
    assert_eq!(requests.len(), 1);
    match &requests[0] {
        PlayerInventoryPersistenceRequestLikeCpp::GraphDestroy(request) => request.clone(),
        other => panic!("expected GraphDestroy request, got {other:?}"),
    }
}

#[tokio::test]
async fn graph_destroy_uses_canonical_child_counts_and_preserves_failed_owners() {
    for (outcome, active) in [
        (
            PersistenceOutcomeLikeCpp::Failed {
                reason: "graph destroy failed".into(),
            },
            false,
        ),
        (
            PersistenceOutcomeLikeCpp::Unknown {
                reason: "graph destroy outcome unknown".into(),
            },
            true,
        ),
    ] {
        let (mut session, send_rx, root_record, root_snapshot) = graph_owner(active);
        let (port, requests) = PlayerInventoryPersistencePortFixtureLikeCpp::new_like_cpp(outcome);
        session.set_player_inventory_persistence_port_like_cpp(port);
        let before = canonical_inventory_facts(&session);

        assert_eq!(root_snapshot.read_view().count(), 1);
        assert!(root_snapshot.read_view().is_refundable());
        assert!(!session.inventory_item_objects_like_cpp()[&root_guid()].is_refundable());
        let removed = wow_world_inventory::destroy_inventory_full_stack_by_pos_like_cpp(
            &mut session,
            INVENTORY_SLOT_BAG_0,
            ROOT_BAG_SLOT,
            root_record,
            Some(root_snapshot),
            "K32 canonical graph test",
        )
        .await;

        assert!(!removed, "Failed and Unknown must not apply the graph");
        let request = graph_request(&requests);
        assert_eq!(request.owner_guid, player_guid().counter() as u64);
        assert_eq!(
            request
                .nodes
                .iter()
                .map(|node| node.item_guid)
                .collect::<Vec<_>>(),
            vec![child_guid().counter() as u64, root_guid().counter() as u64]
        );
        assert!(
            request
                .nodes
                .iter()
                .all(|node| node.expected_owner_db_guid.is_none())
        );
        assert_eq!(request.quest_statuses.len(), 1);
        assert_eq!(request.quest_statuses[0].quest_id, QUEST_ID);
        assert_eq!(
            request.quest_statuses[0]
                .objectives
                .iter()
                .map(|objective| (objective.objective_index, objective.count))
                .collect::<Vec<_>>(),
            vec![(0, 5)],
            "pre-COMMIT quest rows must retain the untouched canonical CHILD_ENTRY count"
        );
        assert_eq!(canonical_inventory_facts(&session), before);
        assert_eq!(
            inventory_failure_result(&send_rx.try_recv().expect("failed graph packet")),
            InventoryResult::InternalBagError as i32
        );
        assert!(
            send_rx.try_recv().is_err(),
            "no graph success packet is published"
        );
    }
}

#[tokio::test]
async fn destroy_item_partial_count_uses_canonical_item_values() {
    let (mut session, send_rx) = make_session_with_send_capacity(2);
    let owner_guid = player_guid();
    let item_guid = ObjectGuid::create_item(1, 91_703);
    let slot = INVENTORY_SLOT_ITEM_START;
    let entry_id = 120;
    session.set_player_guid(Some(owner_guid));
    install_bank_move_item_fixture(&mut session, entry_id, 20);

    session.insert_inventory_item_like_cpp(
        slot,
        PlayerInventoryItem {
            guid: item_guid,
            entry_id,
            db_guid: item_guid.counter() as u64,
            inventory_type: Some(InventoryType::NonEquip as u8),
        },
    );
    session
        .insert_inventory_item_object(item_values(item_guid, entry_id, owner_guid, slot, 1, None));

    let mut player = Player::new(Some(1), false);
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .create(owner_guid);
    player.set_inventory_slot_count(16);
    player
        .store_top_level_item(slot, item_guid)
        .expect("backpack slot should be valid");
    let item_record = inventory_record(item_guid, entry_id, InventoryType::NonEquip);
    let canonical_item = item_values(item_guid, entry_id, owner_guid, slot, 9, None);
    {
        let runtime = player.inventory_runtime_mut_like_cpp();
        runtime.store_item_in_slot_like_cpp(slot, item_record);
        runtime.store_item_object_like_cpp(canonical_item);
    }
    let _ = install_owner(&mut session, player, false);

    let (port, requests) = PlayerInventoryPersistencePortFixtureLikeCpp::new_like_cpp(
        PersistenceOutcomeLikeCpp::Failed {
            reason: "partial destroy rejected".into(),
        },
    );
    session.set_player_inventory_persistence_port_like_cpp(port);

    wow_world_inventory::handle_destroy_item(
        &mut session,
        wow_packet::packets::item::DestroyItemPkt {
            count: 2,
            container_id: INVENTORY_SLOT_BAG_0,
            slot_num: slot,
        },
    )
    .await;

    let requests = requests
        .lock()
        .expect("persistence requests should not be poisoned");
    assert_eq!(requests.len(), 1);
    match &requests[0] {
        PlayerInventoryPersistenceRequestLikeCpp::PartialDestroy(request) => {
            assert_eq!(request.item_guid, item_guid.counter() as u64);
            assert_eq!(
                request.new_count, 7,
                "count comes from canonical stack count 9"
            );
        }
        other => panic!("expected PartialDestroy request, got {other:?}"),
    }
    drop(requests);

    assert_eq!(
        session.core.with_owned_player_like_cpp(|player| {
            player
                .inventory_runtime_like_cpp()
                .item_objects()
                .get(&item_guid)
                .expect("failed request keeps canonical item")
                .count()
        }),
        Some(9)
    );
    assert_eq!(
        inventory_failure_result(&send_rx.try_recv().expect("failed partial packet")),
        InventoryResult::InternalBagError as i32
    );
    assert!(send_rx.try_recv().is_err());
}
