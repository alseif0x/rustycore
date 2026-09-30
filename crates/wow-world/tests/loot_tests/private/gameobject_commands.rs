//! Preserved gameobject loot application scenarios.
use super::recovery_support::*;
use std::collections::HashMap;
use std::sync::Mutex;
use wow_world::test_fixtures::loot::*;
use wow_loot::{LootStore, LootStoreKind, LootStores, LootStoreItem, LootTemplateRow, loot_is_looted_like_cpp};
use wow_world::session::mailbox::{SyncChestGameobjectStateAndRefreshLikeCppCommand, SyncGooberGameobjectStateAndRefreshLikeCppCommand, SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand};
use wow_entities::{GAMEOBJECT_TYPE_GOOBER, GAMEOBJECT_TYPE_GATHERING_NODE, GoState};
use wow_data::{SpellStore, SpellInfo, SpellMiscStore, SpellMiscEntry, SpellRangeStore, SpellRangeEntry};

#[tokio::test]
async fn chest_state_sync_command_updates_receiver_before_refresh_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_011);
    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(
            SyncChestGameobjectStateAndRefreshLikeCppCommand {
                gameobject_guid,
                map_id: 571,
                instance_id: 0,
                go_type: wow_entities::GAMEOBJECT_TYPE_CHEST as u8,
                loot_state: Some(wow_entities::LootState::Activated as u8),
                loot_state_unit_guid: ObjectGuid::create_player(1, 42),
                chest_loot_id: 190_011,
                chest_personal_loot_id: 190_012,
                chest_push_loot_id: 190_013,
                chest_quest_id: 778,
                chest_restock_time_secs: 45,
                chest_consumable: false,
                linked_trap_entry: Some(190_014),
                linked_trap_guid: Some(test_gameobject_guid(91_014)),
            },
        ))
        .expect("command queued");

    process_loot_commands_for_test(&mut session)
        .await;

    let state = loot_gameobject_state_for_test(&session, gameobject_guid)
        .expect("synced chest state");
    assert_eq!(
        state.go_type(),
        Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8)
    );
    assert_eq!(state.loot_state(), Some(wow_entities::LootState::Activated));
    assert_eq!(state.chest_restock_time_secs(), Some(45));
    assert_eq!(state.chest_consumable(), Some(false));
    assert_eq!(state.chest_personal_loot_id(), Some(190_012));
    assert_eq!(state.linked_trap_entry(), Some(190_014));
    assert_eq!(state.linked_trap_guid(), Some(test_gameobject_guid(91_014)));
    let source = state.chest_loot_source().expect("synced chest source");
    assert_eq!(source.loot_id, 190_011);
    assert_eq!(source.personal_loot_id, 190_012);
    assert_eq!(source.push_loot_id, 190_013);
    assert_eq!(source.chest_quest_id, 778);
}

#[tokio::test]
async fn goober_state_sync_command_updates_receiver_before_refresh_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let owner_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_013);
    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(
            SyncGooberGameobjectStateAndRefreshLikeCppCommand {
                gameobject_guid,
                map_id: 571,
                instance_id: 0,
                go_type: GAMEOBJECT_TYPE_GOOBER as u8,
                gameobject_flags: wow_entities::GO_FLAG_IN_USE,
                loot_state: Some(wow_entities::LootState::Activated as u8),
                loot_state_unit_guid: owner_guid,
                go_state: Some(wow_entities::GoState::Active as i8),
                dynamic_flags: wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
                linked_trap_entry: Some(190_016),
                linked_trap_guid: Some(test_gameobject_guid(91_016)),
            },
        ))
        .expect("command queued");

    process_loot_commands_for_test(&mut session)
        .await;

    let state = loot_gameobject_state_for_test(&session, gameobject_guid)
        .expect("synced goober state");
    assert_eq!(state.go_type(), Some(GAMEOBJECT_TYPE_GOOBER as u8));
    assert_eq!(state.gameobject_flags() & wow_entities::GO_FLAG_IN_USE, 1);
    assert_eq!(state.loot_state(), Some(wow_entities::LootState::Activated));
    assert_eq!(state.loot_state_unit_guid(), owner_guid);
    assert_eq!(state.go_state(), Some(wow_entities::GoState::Active));
    assert_eq!(
        state.dynamic_flags() & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
        wow_entities::GO_DYNFLAG_LO_NO_INTERACT
    );
    assert_eq!(state.linked_trap_entry(), Some(190_016));
    assert_eq!(state.linked_trap_guid(), Some(test_gameobject_guid(91_016)));
    assert!(state.cooldown_until().is_none());
    assert!(!state.has_goober_source());
}

#[tokio::test]
async fn gathering_node_state_sync_command_updates_receiver_before_refresh_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_009);
    session.set_state(SessionState::LoggedIn);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);

    session
        .session_command_tx()
        .try_send(
            SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(
                SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand {
                    gameobject_guid,
                    map_id: 571,
                    instance_id: 0,
                    go_type: wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8,
                    loot_state: Some(wow_entities::LootState::Activated as u8),
                    loot_state_unit_guid: ObjectGuid::create_player(1, 42),
                    go_state: Some(wow_entities::GoState::Active as i8),
                    dynamic_flags: wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
                    gathering_node_loot_id: Some(190_009),
                    personal_loot_uses: 1,
                    linked_trap_entry: Some(191_009),
                    linked_trap_guid: Some(test_gameobject_guid(91_010)),
                },
            ),
        )
        .expect("command queued");

    process_loot_commands_for_test(&mut session)
        .await;

    let state = loot_gameobject_state_for_test(&session, gameobject_guid)
        .expect("synced gathering node state");
    assert_eq!(
        state.go_type(),
        Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8)
    );
    assert_eq!(state.loot_state(), Some(wow_entities::LootState::Activated));
    assert_eq!(state.go_state(), Some(wow_entities::GoState::Active));
    assert_eq!(
        state.dynamic_flags() & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
        wow_entities::GO_DYNFLAG_LO_NO_INTERACT
    );
    assert_eq!(state.gathering_node_loot_id(), Some(190_009));
    assert_eq!(state.personal_loot_uses(), 1);
    assert_eq!(state.linked_trap_entry(), Some(191_009));
    assert_eq!(state.linked_trap_guid(), Some(test_gameobject_guid(91_010)));
}

#[test]
fn partial_gathering_node_release_does_not_run_on_loot_release_state_like_cpp() {
    let mut session = make_session();
    let player_guid = ObjectGuid::create_player(1, 61_900);
    let gathering_node = test_gameobject_guid(61_901);
    record_represented_gameobject_runtime_state_for_test(&mut session, 
        0,
        gathering_node,
        gathering_node.entry(),
        Position::ZERO,
        GAMEOBJECT_TYPE_GATHERING_NODE as u8,
    );

    apply_cached_gameobject_loot_release_for_test(&mut session, 
        gathering_node,
        player_guid,
        false,
        false,
    );

    let state = loot_gameobject_state_for_test(&session, gathering_node)
        .unwrap();
    assert_ne!(state.go_state(), Some(GoState::Active));
    assert_eq!(state.loot_state(), Some(LootState::Activated));
}
