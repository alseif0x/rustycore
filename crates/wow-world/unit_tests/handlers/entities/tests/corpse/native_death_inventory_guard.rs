use super::*;
use std::sync::{Arc, Mutex};
use wow_persistence::{
    PersistenceFutureLikeCpp, PersistenceOutcomeLikeCpp, PlayerInventoryPersistencePortLikeCpp,
    PlayerInventoryPersistenceRequestLikeCpp,
};

struct RetirePlayerAfterInventoryCommitLikeCpp {
    manager: Arc<Mutex<wow_map::MapManager>>,
    handle: wow_map::PlayerHandle,
    retired_player: Arc<Mutex<Option<Box<wow_entities::Player>>>>,
}

impl PlayerInventoryPersistencePortLikeCpp for RetirePlayerAfterInventoryCommitLikeCpp {
    fn persist_inventory_mutation_like_cpp(
        &self,
        _request: PlayerInventoryPersistenceRequestLikeCpp,
    ) -> PersistenceFutureLikeCpp<'_, PersistenceOutcomeLikeCpp> {
        let manager = Arc::clone(&self.manager);
        let handle = self.handle;
        let retired_player = Arc::clone(&self.retired_player);
        Box::pin(async move {
            let retired = manager.lock().unwrap().retire_player_like_cpp(handle);
            *retired_player.lock().unwrap() = retired;
            PersistenceOutcomeLikeCpp::Applied { rows: 1 }
        })
    }
}

#[tokio::test]
async fn native_quest_item_destroy_stops_after_player_retires_during_commit() {
    let (mut session, send, _realm, player_guid) = live_player(94_004);
    let manager = Arc::clone(
        session
            .core
            .canonical_map_manager
            .as_ref()
            .expect("canonical MapManager"),
    );
    let handle = session
        .core
        .player_handle_like_cpp
        .expect("canonical Player handle");
    let retired_player = Arc::new(Mutex::new(None));
    let inventory_slot = wow_entities::INVENTORY_SLOT_ITEM_START;
    let item_guid = ObjectGuid::create_item(1, 94_005);
    let item_entry = 35_001;

    assert!(session.set_player_inventory_slot_count_like_cpp(wow_entities::INVENTORY_DEFAULT_SIZE));
    assert_eq!(
        session.insert_inventory_item_like_cpp(
            inventory_slot,
            crate::session::InventoryItem {
                guid: item_guid,
                entry_id: item_entry,
                db_guid: 94_005,
                inventory_type: None,
            },
        ),
        None
    );
    let item_object = session.make_inventory_item_object(
        item_guid,
        item_entry,
        player_guid,
        1,
        0,
        wow_constants::ItemContext::None,
        inventory_slot,
    );
    session.insert_inventory_item_object(item_object);
    session.set_player_inventory_persistence_port_like_cpp(Arc::new(
        RetirePlayerAfterInventoryCommitLikeCpp {
            manager,
            handle,
            retired_player: Arc::clone(&retired_player),
        },
    ));
    while send.try_recv().is_ok() {}

    let result =
        wow_world_inventory::destroy_quest_item_count_like_cpp(&mut session, item_entry, 1).await;

    assert_eq!(
        result,
        Err("quest item full-stack destruction failed or is indeterminate")
    );
    let retired_player = retired_player
        .lock()
        .unwrap()
        .take()
        .expect("persistence callback retired the canonical Player");
    assert_eq!(
        retired_player
            .inventory_runtime_like_cpp()
            .item_objects()
            .get(&item_guid)
            .map(wow_entities::Item::count),
        Some(1),
        "stale native operation must not apply item destruction to the retired Player"
    );
    assert!(
        send.try_recv().is_err(),
        "stale native operation must not publish item removal"
    );
}
