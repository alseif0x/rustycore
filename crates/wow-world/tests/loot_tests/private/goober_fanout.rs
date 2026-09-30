//! Preserved gameobject loot application scenarios.
use super::recovery_support::*;
use std::collections::HashMap;
use std::sync::Mutex;
use wow_data::{
    SpellInfo, SpellMiscEntry, SpellMiscStore, SpellRangeEntry, SpellRangeStore, SpellStore,
};
use wow_entities::GAMEOBJECT_TYPE_GOOBER;
use wow_loot::{
    LootStore, LootStoreItem, LootStoreKind, LootStores, LootTemplateRow, loot_is_looted_like_cpp,
};
use wow_world::session::mailbox::{
    SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand,
};
use wow_world::test_fixtures::loot::*;

#[test]
fn represented_goober_use_syncs_shared_state_to_same_map_viewers_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let other_map_guid = ObjectGuid::create_player(1, 88);
    let gameobject_guid = test_gameobject_guid(91_012);
    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (other_command_tx, other_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let (other_send_tx, _other_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    let mut other_info = broadcast_info(other_map_guid, other_send_tx);
    other_info.placement.map_id = 1;
    other_info.command_tx = other_command_tx;
    player_registry.register_or_replace(other_map_guid, other_info, Default::default());

    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);
    set_loot_linked_trap_for_test(&mut session, gameobject_guid, 190_015);

    assert!(use_loot_goober_for_test(
        &mut session,
        gameobject_guid,
        player_guid,
        777,
        wow_entities::GooberUseSource {
            auto_close_ms: 3_000,
            linked_trap_entry: 190_015,
            ..Default::default()
        },
    ));

    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected goober sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.go_type, GAMEOBJECT_TYPE_GOOBER as u8);
    assert_eq!(command.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 1);
    assert_eq!(
        command.loot_state,
        Some(wow_entities::LootState::Activated as u8)
    );
    assert_eq!(command.loot_state_unit_guid, player_guid);
    assert_eq!(command.go_state, Some(wow_entities::GoState::Active as i8));
    assert_eq!(command.linked_trap_entry, Some(190_015));
    assert!(other_command_rx.try_recv().is_err());
}
