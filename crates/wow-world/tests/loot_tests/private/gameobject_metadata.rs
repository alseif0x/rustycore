//! Preserved gameobject loot application scenarios.
use super::recovery_support::*;
use std::collections::HashMap;
use std::sync::Mutex;
use wow_world::test_fixtures::loot::*;
use wow_loot::{LootStore, LootStoreKind, LootStores, LootStoreItem, LootTemplateRow, loot_is_looted_like_cpp};
use wow_world::session::mailbox::{SyncChestGameobjectStateAndRefreshLikeCppCommand, SyncGooberGameobjectStateAndRefreshLikeCppCommand, SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand};
use wow_entities::GAMEOBJECT_TYPE_GOOBER;
use wow_data::{SpellStore, SpellInfo, SpellMiscStore, SpellMiscEntry, SpellRangeStore, SpellRangeEntry};

#[tokio::test]
async fn represented_gameobject_chest_loot_carries_cpp_source_metadata() {
    let mut session = make_session();
    let gameobject_guid = test_gameobject_guid(91_001);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: false,
        dungeon_encounter_id: 733,
        personal_loot_id: 10_001,
        push_loot_id: 0,
        triggered_event_id: 0,
        linked_trap_entry: 0,
        ..Default::default()
    };

    let loot = generate_chest_loot_for_test(&mut session, 
            gameobject_guid,
            ObjectGuid::create_player(1, 42),
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.loot_type, LOOT_TYPE_CHEST_LIKE_CPP);
    assert_eq!(loot.dungeon_encounter_id, 733);
    assert_eq!(loot.loot_method, 0);
}

#[test]
fn chest_allowed_looters_ignore_range_only_in_same_dungeon_instance_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send();
    let player_guid = ObjectGuid::create_player(1, 42);
    let member_guid = ObjectGuid::create_player(1, 43);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let registry = Arc::new(PlayerRegistry::default());
    let (member_tx, _member_rx) = flume::bounded(1);
    let mut member = broadcast_info(member_guid, member_tx.clone());
    member.placement.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    registry.register_or_replace(member_guid, member, Default::default());
    session.set_player_registry(Arc::clone(&registry));

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
    let canonical = Arc::new(Mutex::new(wow_map::MapManager::default()));
    session.set_canonical_map_manager(canonical);
    attach_money_player_controller_for_test(&mut session,
        player_guid,
        "LootOwner".to_string(),
        Position::ZERO,
        0,
        1,
        1,
        80,
        0,
    );
    ensure_money_player_map_for_test(&mut session)
        .expect("canonical loot owner map");
    install_group_loot_group(&mut session, player_guid, member_guid);
    assert_eq!(
        chest_reward_looters_for_test(&session, player_guid),
        vec![player_guid]
    );

    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    assert_eq!(
        chest_reward_looters_for_test(&session, player_guid),
        vec![player_guid, member_guid]
    );

    let mut wrong_instance = broadcast_info(member_guid, member_tx);
    wrong_instance.placement.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    wrong_instance.placement.instance_id = 1;
    registry.register_or_replace(member_guid, wrong_instance, Default::default());
    assert_eq!(
        chest_reward_looters_for_test(&session, player_guid),
        vec![player_guid]
    );
}
