//! Original personal-encounter pool lifecycle cases.
//! Empty immutable encounter metadata makes legacy unspecified encounters known-unlocked
//! through the real InstanceLock lookup; it does not enable the cfg(test) fallback.
use super::recovery_support::*;
use std::collections::HashMap;
use wow_world::test_fixtures::loot::*;
use wow_world::test_fixtures::loot::attach_loot_allocator_for_test as attach_loot_guid_allocator_for_owner;
use wow_loot::{LootStore, LootStores, LootStoreKind, LootStoreItem, LootTemplateRow, loot_is_looted_like_cpp};

fn install_unknown_encounter_catalog(session: &mut WorldSession) {
    session.set_dungeon_encounter_store(Arc::new(wow_data::DungeonEncounterStore::from_entries([])));
}

#[tokio::test]
async fn represented_personal_encounter_late_session_without_canonical_tap_list_fails_closed() {
    let (mut first, _first_rx) = make_session_with_send_capacity(8);
    let (mut second, second_rx) = make_session_with_send_capacity(8);
    let first_player = ObjectGuid::create_player(1, 142);
    let second_player = ObjectGuid::create_player(1, 177);
    let gameobject_guid = test_gameobject_guid(91_018);
    let personal_loot_id = 10_018;
    let item_id = 80_018;

    install_unknown_encounter_catalog(&mut first);
    install_unknown_encounter_catalog(&mut second);
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
        dungeon_encounter_id: 733,
        personal_loot_id,
        ..Default::default()
    };
    open_money_loot_normally_for_test(&mut first, gameobject_guid, source)
        .await;
    let authority = canonical_gameobject_snapshot(&first, gameobject_guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    let first_before = authority
        .snapshot_for_player_like_cpp(first_player)
        .expect("the first opener owns the initial encounter pool");

    open_money_loot_normally_for_test(&mut second, gameobject_guid, source)
        .await;

    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 1);
    let first_after = personal.get(&first_player).unwrap();
    assert_eq!(first_after, &first_before);
    assert!(
        authority
            .snapshot_for_player_like_cpp(second_player)
            .is_none(),
        "without canonical GameObject::GetTapList state Rust must not fabricate outsider loot"
    );
    assert!(!is_active_loot_guid_for_test(&second, gameobject_guid));
    assert!(second_rx.try_recv().is_err());
}

#[tokio::test]
async fn represented_empty_personal_encounter_chest_does_not_install_or_open_like_cpp() {
    let (mut session, send_rx) = make_session_with_send_capacity(8);
    install_unknown_encounter_catalog(&mut session);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_016);
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
                dungeon_encounter_id: 733,
                personal_loot_id: 10_016,
                ..Default::default()
            },
        )
        .await;

    let gameobject = canonical_gameobject_snapshot(&session, gameobject_guid).unwrap();
    assert!(gameobject.loot_authority_like_cpp().is_pristine_like_cpp());
    assert!(
        gameobject
            .loot_authority_like_cpp()
            .personal_snapshots_like_cpp()
            .is_empty()
    );
    assert!(!has_loot_for_test(&session, gameobject_guid));
    assert!(
        !is_personal_loot_owner_for_test(&session, gameobject_guid)
    );
    assert!(
        !has_personal_loot_money_entry_for_test(&session, gameobject_guid, player_guid)
    );
    assert!(!is_active_loot_guid_for_test(&session, gameobject_guid));
    assert!(send_rx.try_recv().is_err());
}

#[tokio::test]
async fn represented_nonempty_personal_encounter_chest_keeps_live_canonical_pool_like_cpp() {
    let (mut session, _send_rx) = make_session_with_send_capacity(8);
    install_unknown_encounter_catalog(&mut session);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_017);
    let personal_loot_id = 10_017;
    let item_id = 80_017;
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    let gameobject = make_canonical_gameobject_for_session(
        &session,
        gameobject_guid,
        GAMEOBJECT_TYPE_CHEST as u8,
    );
    attach_canonical_gameobject(&mut session, gameobject);

    install_limited_test_item_template(&mut session, item_id, 0);
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
    session.set_loot_stores(Arc::new(stores));

    open_money_loot_normally_for_test(&mut session, 
            gameobject_guid,
            GameObjectLootSource {
                loot_id: 0,
                dungeon_encounter_id: 733,
                personal_loot_id,
                ..Default::default()
            },
        )
        .await;

    let gameobject = canonical_gameobject_snapshot(&session, gameobject_guid).unwrap();
    let pool = gameobject
        .loot_authority_like_cpp()
        .snapshot_for_player_like_cpp(player_guid)
        .expect("nonempty encounter pool remains map-owned");
    assert_eq!(pool.loot.allowed_looters, vec![player_guid]);
    assert_eq!(pool.loot.items.len(), 1);
    assert_eq!(pool.loot.items[0].item_id, item_id);
    assert!(!loot_is_looted_like_cpp(&pool.loot));
    assert!(is_active_loot_guid_for_test(&session, gameobject_guid));
}

#[tokio::test]
async fn represented_gameobject_personal_encounter_loot_uses_current_player_when_no_tap_list_like_cpp()
 {
    let mut session = make_session();
    install_unknown_encounter_catalog(&mut session);
    let player_guid = ObjectGuid::create_player(1, 42);
    let gameobject_guid = test_gameobject_guid(91_008);
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
            player_guid,
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.allowed_looters, vec![player_guid]);
    assert!(
        loot.items
            .iter()
            .all(|entry| entry.allowed_looters == vec![player_guid])
    );
    assert_eq!(loot.coins, 0);
    assert!(
        is_personal_loot_owner_for_test(&session, gameobject_guid)
    );
    assert!(
        has_personal_loot_money_entry_for_test(&session, gameobject_guid, player_guid)
    );
}

#[tokio::test]
async fn represented_gameobject_personal_encounter_loot_uses_tap_list_like_cpp() {
    let mut session = make_session();
    install_unknown_encounter_catalog(&mut session);
    let first_tapper = ObjectGuid::create_player(1, 42);
    let second_tapper = ObjectGuid::create_player(1, 77);
    let non_player_tapper = ObjectGuid::create_item(1, 900);
    let gameobject_guid = test_gameobject_guid(91_009);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    set_loot_gameobject_tappers_for_test(&mut session, 
        gameobject_guid,
        vec![
            second_tapper,
            non_player_tapper,
            first_tapper,
            second_tapper,
        ],
    );
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
            first_tapper,
            source,
            &[],
        )
        .await
        .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.allowed_looters, vec![first_tapper, second_tapper]);
    assert!(
        loot.items
            .iter()
            .all(|entry| entry.allowed_looters == vec![first_tapper, second_tapper])
    );
    assert_eq!(loot.coins, 0);
    assert!(
        is_personal_loot_owner_for_test(&session, gameobject_guid)
    );
    assert!(
        has_personal_loot_money_entry_for_test(&session, gameobject_guid, first_tapper)
    );
    assert!(
        has_personal_loot_money_entry_for_test(&session, gameobject_guid, second_tapper)
    );
}
