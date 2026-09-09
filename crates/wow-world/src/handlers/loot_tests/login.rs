//! Login scenarios for [`super`].
//!
//! Split out of loot_tests.rs under #628; assertions and
//! registrations are unchanged and shared fixtures stay in the parent module.

use super::*;

#[test]
fn retired_object_authority_releases_every_session_window_like_cpp() {
    let (mut first, first_rx, mut second, second_rx, owner, first_guid, second_guid) =
        two_sessions_with_authoritative_creature_loot_like_cpp(authoritative_test_loot_like_cpp(
            9, true,
        ));
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner)
        .unwrap();
    let _ = drain_server_opcodes_like_cpp(&first_rx);
    let _ = drain_server_opcodes_like_cpp(&second_rx);

    authority.retire_like_cpp();
    first.close_retired_active_loot_windows_like_cpp(first_guid);
    second.close_retired_active_loot_windows_like_cpp(second_guid);

    for (session, owner_guid) in [(&first, owner), (&second, owner)] {
        assert!(!session.active_loot_view_owners.contains(&owner_guid));
        assert!(!session.loot_table.contains_key(&owner_guid));
    }
    for rx in [&first_rx, &second_rx] {
        assert_eq!(
            drain_server_opcodes_like_cpp(rx),
            vec![wow_constants::ServerOpcodes::LootRelease as u16]
        );
    }
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

    first.set_player_guid(Some(first_player));
    second.set_player_guid(Some(second_player));
    first.set_player_position_like_cpp(Position::ZERO);
    second.set_player_position_like_cpp(Position::ZERO);
    let gameobject =
        make_canonical_gameobject_for_session(&first, gameobject_guid, GAMEOBJECT_TYPE_CHEST as u8);
    attach_canonical_gameobject(&mut first, gameobject);
    second.set_canonical_map_manager(Arc::clone(
        first
            .canonical_map_manager
            .as_ref()
            .expect("both sessions share the canonical map owner"),
    ));

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
    second.set_loot_stores(stores);

    let source = GameObjectLootSource {
        loot_id: 0,
        dungeon_encounter_id: 733,
        personal_loot_id,
        ..Default::default()
    };
    first
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
        .await;
    let authority = canonical_gameobject_snapshot(&first, gameobject_guid)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();
    let first_before = authority
        .snapshot_for_player_like_cpp(first_player)
        .expect("the first opener owns the initial encounter pool");

    second
        .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
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
    assert!(!second.is_active_loot_guid(gameobject_guid));
    assert!(second_rx.try_recv().is_err());
}
#[tokio::test]
async fn authoritative_partial_release_clears_round_robin_for_all_sessions_and_forces_dynflags_like_cpp()
 {
    let first_guid = ObjectGuid::create_player(1, 61_890);
    let second_guid = ObjectGuid::create_player(1, 61_891);
    let mut loot = authoritative_test_loot_like_cpp(7, false);
    loot.round_robin_player = first_guid;
    loot.allowed_looters = vec![first_guid, second_guid];
    let (mut first, _first_rx, mut second, _second_rx, owner_guid, _, _) =
        two_sessions_with_authoritative_creature_loot_like_cpp(loot);
    // The helper uses its own deterministic player ids; install the exact
    // current round-robin holder from the opened first session.
    let opened_first = first.player_guid().unwrap();
    let opened_second = second.player_guid().unwrap();
    let authority = first
        .represented_owned_loot_authority_like_cpp(owner_guid)
        .unwrap();
    let generation = authority
        .snapshot_for_player_like_cpp(opened_first)
        .unwrap()
        .generation;
    let mut replacement = authority
        .snapshot_for_player_like_cpp(opened_first)
        .unwrap()
        .loot;
    replacement.round_robin_player = opened_first;
    authority.replace_like_cpp(Some(replacement), HashMap::new());
    let replacement_generation = authority
        .snapshot_for_player_like_cpp(opened_first)
        .unwrap()
        .generation;
    assert_ne!(generation, replacement_generation);
    authority.add_viewer_like_cpp(opened_first).unwrap();
    first
        .active_loot_view_generations_like_cpp
        .insert(owner_guid, replacement_generation);
    first
        .active_loot_view_authorities_like_cpp
        .insert(owner_guid, authority.clone());
    assert!(first.reconcile_represented_loot_cache_like_cpp(owner_guid, opened_first));
    let _ = first.mutate_world_creature(owner_guid, |creature| {
        creature
            .creature
            .unit_mut()
            .world_mut()
            .object_mut()
            .clear_update_mask(false);
    });

    assert!(
        first
            .do_loot_release_owner_like_cpp(owner_guid, opened_first)
            .await
    );

    assert!(
        authority
            .snapshot_for_player_like_cpp(opened_second)
            .unwrap()
            .loot
            .round_robin_player
            .is_empty()
    );
    assert!(second.reconcile_represented_loot_cache_like_cpp(owner_guid, opened_second));
    assert!(
        second
            .loot_table
            .get(&owner_guid)
            .unwrap()
            .round_robin_player
            .is_empty()
    );
    assert!(
        first
            .mutate_world_creature(owner_guid, |creature| {
                creature
                    .creature
                    .unit()
                    .world()
                    .object()
                    .changed_fields()
                    .contains(ObjectChangedFields::DYNAMIC_FLAGS)
            })
            .unwrap()
    );
}
