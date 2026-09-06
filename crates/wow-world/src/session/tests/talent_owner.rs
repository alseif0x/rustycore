//! Talent/glyph mutations stay inside one canonical Player access.

use super::*;

#[test]
fn talent_reset_price_reads_active_and_detached_owner_without_mutation() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    let month = 30 * 24 * 60 * 60;
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        session
            .mutate_player_talent_runtime_like_cpp(|runtime| {
                runtime.reset_talents_cost = 500_000;
                runtime.reset_talents_time_secs = month;
                runtime.talent_groups[0].insert(42, 2);
            })
            .unwrap();
        let before = session.player_talent_runtime_snapshot_like_cpp().unwrap();
        assert_eq!(
            session.represented_next_reset_talents_cost_like_cpp(2 * month),
            Some(450_000)
        );
        assert_eq!(
            session.represented_talent_reset_cost_like_cpp(),
            Some(500_000)
        );
        assert_eq!(
            session.represented_talent_reset_time_secs_like_cpp(),
            Some(month)
        );
        assert_eq!(
            session.player_talent_runtime_snapshot_like_cpp().unwrap(),
            before
        );
        assert!(manager.try_lock().is_ok());
    }
    session.canonical_map_manager = None;
    assert_eq!(
        session.represented_next_reset_talents_cost_like_cpp(month),
        None
    );
    assert_eq!(session.represented_talent_reset_cost_like_cpp(), None);
    assert_eq!(session.represented_talent_reset_time_secs_like_cpp(), None);
}

#[test]
fn talent_reset_price_rejects_stale_generation_instead_of_pricing_replacement() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.gameplay_state_mut().talents.reset_talents_cost = 500_000;
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    assert_eq!(
        session.represented_next_reset_talents_cost_like_cpp(0),
        None
    );
    assert_eq!(session.represented_talent_reset_cost_like_cpp(), None);
    assert_eq!(session.represented_talent_reset_time_secs_like_cpp(), None);
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| {
                player
                    .talent_runtime_like_cpp()
                    .next_reset_talents_cost_like_cpp(0)
            }),
        Some(500_000)
    );
}

#[test]
fn talent_points_refresh_reads_active_group_and_rewards_from_active_and_detached_owner() {
    let (mut session, _, send_rx) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_player_level_like_cpp(80);
    session.set_player_class_like_cpp(1);
    session.set_num_talents_at_level_store(Arc::new(
        wow_data::progression_rewards::NumTalentsAtLevelStore::from_entries([
            wow_data::progression_rewards::NumTalentsAtLevelEntry {
                id: 80,
                num_talents: 71,
                num_talents_death_knight: 61,
                num_talents_demon_hunter: 51,
            },
        ]),
    ));
    session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries([
        test_talent_entry_like_cpp(101, 2, 50_101),
        test_talent_entry_like_cpp(102, 1, 50_102),
    ])));
    let mut spells = wow_data::SpellStore::new();
    spells.insert(50_101, test_spell_info_like_cpp(50_101));
    session.set_spell_store(Arc::new(spells));
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        session
            .mutate_player_talent_runtime_like_cpp(|runtime| {
                runtime.active_group = 1;
                runtime.talent_groups[0].insert(101, 2);
                runtime.talent_groups[1].insert(101, 2);
                runtime.talent_groups[1].insert(102, 1); // Missing SpellInfo: not counted.
                runtime.talent_groups[1].insert(999, 8); // Missing talent: not counted.
            })
            .unwrap();
        session
            .with_owned_player_mut_like_cpp(|player| {
                player.gameplay_state_mut().quest_rewarded_talent_points = 5;
            })
            .unwrap();
        session.refresh_represented_talent_points_like_cpp();
        assert_eq!(session.player_character_points_like_cpp(), 73);
        assert!(
            manager.try_lock().is_ok(),
            "no owner guard survives refresh"
        );
        assert!(
            send_rx.is_empty(),
            "refresh must not publish talents itself"
        );
    }
}

#[test]
fn talent_points_refresh_without_level_catalog_saturates_and_clamps_canonical_rewards() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    session.set_talent_store(Arc::new(wow_data::TalentStore::from_entries([
        test_talent_entry_like_cpp(101, 2, 50_101),
    ])));
    session
        .mutate_player_talent_runtime_like_cpp(|runtime| {
            runtime.talent_groups[0].insert(101, 2);
        })
        .unwrap();
    for (rewarded, expected) in [(2, 0), (5, 2), (u32::MAX, i32::MAX)] {
        session
            .with_owned_player_mut_like_cpp(|player| {
                player.gameplay_state_mut().quest_rewarded_talent_points = rewarded;
            })
            .unwrap();
        session.refresh_represented_talent_points_like_cpp();
        assert_eq!(session.player_character_points_like_cpp(), expected);
    }
}

#[test]
fn talent_points_refresh_rejects_stale_and_missing_owner_without_fixture_fallback() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.set_character_points_like_cpp(123);
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    session.player_character_points_like_cpp = 456;
    session.refresh_represented_talent_points_like_cpp();
    assert_eq!(session.player_character_points_like_cpp, 456);
    session.canonical_map_manager = None;
    session.refresh_represented_talent_points_like_cpp();
    assert_eq!(session.player_character_points_like_cpp, 456);
    assert_eq!(
        manager
            .lock()
            .unwrap()
            .with_player_like_cpp(handle, |player| { player.active_data().character_points }),
        Some(123)
    );
}

#[test]
fn talent_mutation_runs_on_active_and_detached_player_without_writeback() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        let calls = std::cell::Cell::new(0);
        assert_eq!(
            session.mutate_player_talent_runtime_like_cpp(|runtime| {
                calls.set(calls.get() + 1);
                assert!(matches!(
                    manager.try_lock(),
                    Err(std::sync::TryLockError::WouldBlock)
                ));
                runtime.talent_groups[0].insert(42, 1);
                runtime.glyph_groups[0][2] = 700;
                runtime.reset_talents_cost = 100_000;
                runtime.talents_loaded = true;
                detached
            }),
            Some(detached)
        );
        assert_eq!(calls.get(), 1);
        assert!(
            manager.try_lock().is_ok(),
            "no guard survives the operation"
        );
        // The normal loaded-state setter must preserve every unrelated value.
        session.mark_represented_glyphs_loaded_like_cpp();
        let state = session.player_talent_runtime_snapshot_like_cpp().unwrap();
        assert_eq!(state.talent_groups[0].get(&42), Some(&1));
        assert_eq!(state.glyph_groups[0][2], 700);
        assert_eq!(state.reset_talents_cost, 100_000);
        assert!(state.talents_loaded && state.glyphs_loaded);
    }
    session.canonical_map_manager = None;
    assert_eq!(
        session.mutate_player_talent_runtime_like_cpp(|_| panic!("missing owner")),
        None::<()>
    );
}
