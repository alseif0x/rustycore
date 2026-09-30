//! Original encounter scenarios backed by real resident Players and shared locks.
use super::creature_pool_setup::overworld_personal_loot_test_fixture_like_cpp;
use super::interaction_support::*;
use wow_world::test_fixtures::loot::attach_loot_allocator_for_test as attach_loot_guid_allocator_for_owner;

#[tokio::test]
async fn represented_gameobject_personal_encounter_loot_skips_locked_tappers_like_cpp() {
    let mut session = make_session();
    let locked_tapper = ObjectGuid::create_player(1, 42);
    let open_tapper = ObjectGuid::create_player(1, 77);
    let gameobject_guid = test_gameobject_guid(91_010);
    attach_loot_guid_allocator_for_owner(&mut session, gameobject_guid);
    set_loot_gameobject_tappers_for_test(
        &mut session,
        gameobject_guid,
        vec![locked_tapper, open_tapper],
    );
    prepare_encounter_loot_players_for_test(
        &mut session,
        733,
        &[locked_tapper, open_tapper],
        &[locked_tapper],
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

    let loot =
        generate_chest_loot_for_test(&mut session, gameobject_guid, locked_tapper, source, &[])
            .await
            .expect("canonical owner map allocates a LootObject");

    assert_eq!(loot.allowed_looters, vec![open_tapper]);
    assert!(
        loot.items
            .iter()
            .all(|entry| entry.allowed_looters == vec![open_tapper])
    );
}

#[tokio::test]
async fn dungeon_encounter_builds_independent_unlocked_personal_pools_like_cpp() {
    let mut fixture = overworld_personal_loot_test_fixture_like_cpp();
    fixture
        .session
        .set_map_store(Arc::new(wow_data::MapStore::from_entries([
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
    let encounter_id = 733;
    mutate_loot_creature_for_test(&mut fixture.session, fixture.owner_guid, |creature| {
        creature.creature.ai_ownership_mut().dungeon_encounter_id = encounter_id;
    });
    prepare_money_player_residence_for_test(&mut fixture.session);
    prepare_encounter_loot_players_for_test(
        &mut fixture.session,
        encounter_id,
        &[fixture.first_tapper, fixture.second_tapper],
        &[fixture.second_tapper],
    );

    ensure_creature_kill_loot_for_test(&mut fixture.session, fixture.owner_guid).await;

    let authority =
        loot_recovery_authority_for_test(&mut fixture.session, fixture.owner_guid).unwrap();
    let personal = authority.personal_snapshots_like_cpp();
    assert_eq!(personal.len(), 1);
    assert!(personal.contains_key(&fixture.first_tapper));
    assert!(!personal.contains_key(&fixture.second_tapper));
    assert!(!personal.contains_key(&fixture.disconnected_tapper));
    let first = &personal[&fixture.first_tapper].loot;
    assert_eq!(first.dungeon_encounter_id, encounter_id);
    assert_eq!(first.allowed_looters, vec![fixture.first_tapper]);
}

#[tokio::test]
async fn represented_personal_encounter_locked_or_empty_late_player_does_not_install_like_cpp() {
    let (mut first, _first_rx) = make_session_with_send_capacity(8);
    let (mut locked, locked_rx) = make_session_with_send_capacity(8);
    let (mut empty, empty_rx) = make_session_with_send_capacity(8);
    let first_player = ObjectGuid::create_player(1, 242);
    let locked_player = ObjectGuid::create_player(1, 277);
    let empty_player = ObjectGuid::create_player(1, 288);
    let gameobject_guid = test_gameobject_guid(91_019);
    let personal_loot_id = 10_019;
    let item_id = 80_019;
    let encounter_id = 734;

    for (session, player) in [
        (&mut first, first_player),
        (&mut locked, locked_player),
        (&mut empty, empty_player),
    ] {
        session.set_player_guid(Some(player));
        session.set_player_position_like_cpp(Position::ZERO);
    }
    let gameobject =
        make_canonical_gameobject_for_session(&first, gameobject_guid, GAMEOBJECT_TYPE_CHEST as u8);
    attach_canonical_gameobject(&mut first, gameobject);
    share_loot_canonical_map_for_test(&first, &mut locked);
    share_loot_canonical_map_for_test(&first, &mut empty);

    install_limited_test_item_template(&mut first, item_id, 0);
    install_limited_test_item_template(&mut locked, item_id, 0);
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
                    loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
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
    locked.set_loot_stores(stores);
    prepare_money_player_residence_for_test(&mut first);
    prepare_money_player_residence_for_test(&mut locked);
    prepare_money_player_residence_for_test(&mut empty);
    prepare_encounter_loot_players_for_test(
        &mut first,
        encounter_id,
        &[first_player, locked_player, empty_player],
        &[locked_player],
    );
    share_encounter_loot_catalogs_for_test(&first, &mut locked);
    share_encounter_loot_catalogs_for_test(&first, &mut empty);

    let source = GameObjectLootSource {
        loot_id: 0,
        dungeon_encounter_id: encounter_id,
        personal_loot_id,
        ..Default::default()
    };
    open_money_loot_normally_for_test(&mut first, gameobject_guid, source).await;
    let authority = canonical_gameobject_snapshot(&first, gameobject_guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    let first_before = authority
        .snapshot_for_player_like_cpp(first_player)
        .unwrap();

    open_money_loot_normally_for_test(&mut locked, gameobject_guid, source).await;
    open_money_loot_normally_for_test(&mut empty, gameobject_guid, source).await;

    assert_eq!(authority.personal_snapshots_like_cpp().len(), 1);
    assert_eq!(
        authority
            .snapshot_for_player_like_cpp(first_player)
            .unwrap(),
        first_before
    );
    assert!(
        authority
            .snapshot_for_player_like_cpp(locked_player)
            .is_none()
    );
    assert!(
        authority
            .snapshot_for_player_like_cpp(empty_player)
            .is_none()
    );
    assert!(!is_active_loot_guid_for_test(&locked, gameobject_guid));
    assert!(!is_active_loot_guid_for_test(&empty, gameobject_guid));
    assert!(locked_rx.try_recv().is_err());
    assert!(empty_rx.try_recv().is_err());
}
