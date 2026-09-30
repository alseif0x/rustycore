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
async fn represented_non_encounter_personal_chest_keeps_two_session_pools_independent_like_cpp() {
    let (mut first, _first_rx) = make_session_with_send_capacity(8);
    let (mut second, _second_rx) = make_session_with_send_capacity(8);
    let first_player = ObjectGuid::create_player(1, 42);
    let second_player = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_015);
    let personal_loot_id = 10_015;
    let item_id = 80_015;

    first.set_player_guid(Some(first_player));
    second.set_player_guid(Some(second_player));
    first.set_player_position_like_cpp(Position::ZERO);
    second.set_player_position_like_cpp(Position::ZERO);

    let gameobject =
        make_canonical_gameobject_for_session(&first, gameobject_guid, GAMEOBJECT_TYPE_CHEST as u8);
    attach_canonical_gameobject(&mut first, gameobject);
    share_loot_canonical_map_for_test(&first, &mut second);

    install_limited_test_item_template(&mut first, item_id, 0);
    install_limited_test_item_template(&mut second, item_id, 0);
    let mut gameobject_store = LootStore::for_kind_like_cpp(LootStoreKind::Gameobject);
    gameobject_store
        .load_rows_like_cpp(
            [LootTemplateRow {
                entry: personal_loot_id,
                item: LootStoreItem {
                    item_id,
                    reference: 0,
                    chance: 100.0,
                    needs_quest: false,
                    loot_mode: 0x01,
                    group_id: 0,
                    min_count: 1,
                    max_count: 1,
                },
            }],
            |_| true,
        )
        .unwrap();
    let mut stores = LootStores::new();
    stores.insert(LootStoreKind::Gameobject, gameobject_store);
    let stores = Arc::new(stores);
    first.set_loot_stores(Arc::clone(&stores));
    second.set_loot_stores(stores);

    let source = GameObjectLootSource {
        loot_id: 0,
        use_group_loot_rules: true,
        dungeon_encounter_id: 0,
        personal_loot_id,
        ..Default::default()
    };

    open_money_loot_normally_for_test(&mut first, gameobject_guid, source)
        .await;
    open_money_loot_normally_for_test(&mut second, gameobject_guid, source)
        .await;

    let authority = canonical_gameobject_snapshot(&first, gameobject_guid)
        .expect("canonical chest remains map-owned")
        .loot_authority_like_cpp()
        .clone();
    assert!(authority.shared_snapshot_like_cpp().is_none());
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 2);

    let first_pool = personal.get(&first_player).unwrap();
    let second_pool = personal.get(&second_player).unwrap();
    assert_ne!(first_pool.loot.loot_guid, second_pool.loot.loot_guid);
    assert_eq!(first_pool.loot.loot_method, 0);
    assert_eq!(second_pool.loot.loot_method, 0);
    for (player, pool) in [(first_player, first_pool), (second_player, second_pool)] {
        assert_eq!(pool.loot.allowed_looters, vec![player]);
        assert_eq!(pool.loot.items.len(), 1);
        assert_eq!(pool.loot.items[0].item_id, item_id);
        assert_eq!(pool.loot.items[0].allowed_looters, vec![player]);
    }

    let first_slot = first_pool.loot.items[0].loot_list_id;
    authority
        .reserve_item_like_cpp(first_player, first_slot)
        .await
        .unwrap()
        .commit_like_cpp()
        .unwrap();
    assert!(
        authority
            .snapshot_for_player_like_cpp(first_player)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
    assert!(
        !authority
            .snapshot_for_player_like_cpp(second_player)
            .unwrap()
            .loot
            .items[0]
            .taken
    );
}

#[tokio::test]
async fn represented_empty_non_encounter_personal_chest_still_opens_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    let player_guid = ObjectGuid::create_player(1, 141);
    let gameobject_guid = test_gameobject_guid(91_021);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    attach_canonical_gameobject(&mut session, gameobject);

    open_money_loot_normally_for_test(&mut session, 
            gameobject_guid,
            GameObjectLootSource {
                loot_id: 0,
                dungeon_encounter_id: 0,
                personal_loot_id: 10_021,
                ..Default::default()
            },
        )
        .await;

    let authority = canonical_gameobject_snapshot(&session, gameobject_guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    let pool = authority
        .snapshot_for_player_like_cpp(player_guid)
        .expect("C++ retains the empty non-encounter personal pool");
    assert!(loot_is_looted_like_cpp(&pool.loot));
    assert_eq!(pool.loot.allowed_looters, vec![player_guid]);
    assert!(is_active_loot_guid_for_test(&session, gameobject_guid));
    let _response = recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootResponse);
}
