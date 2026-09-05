//! Skill authority transitions execute under the generation-checked Player owner.
use super::*;

#[test]
fn occupied_skill_slot_authority_matches_previous_route_for_active_and_detached_player() {
    let (mut session, _, _) = make_session();
    install_canonical_player_owner_for_test(&mut session, 571, 0);
    for detached in [false, true] {
        if detached {
            assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
        }
        for loaded in [false, true] {
            for complete in [false, true] {
                for ids in [
                    vec![],
                    vec![333],
                    vec![333, 333, 70000],
                    (1..=256).collect(),
                    (1..=257).collect(),
                ] {
                    for slots in [0, 1, 2, 256, 257] {
                        let prepare = |player: &mut Player| {
                            player.replace_skill_records_like_cpp(
                                ids.iter()
                                    .map(|id| wow_entities::PlayerSkillRecord {
                                        skill_line_id: *id,
                                        current_value: 0,
                                        max_value: 0,
                                        step: 0,
                                        profession_slot: -1,
                                        state: wow_entities::PlayerSkillLoadState::Deleted,
                                    })
                                    .collect(),
                                loaded,
                                complete,
                                Some(7),
                                BTreeSet::from([333]),
                            );
                        };
                        let projection = |player: &Player| {
                            (
                                player.skill_records_like_cpp().to_vec(),
                                player.skill_records_loaded_like_cpp(),
                                player.skill_records_complete_like_cpp(),
                                player.occupied_skill_slots_like_cpp(),
                                player.non_durable_skill_tombstones_like_cpp().clone(),
                            )
                        };
                        session.with_owned_player_mut_like_cpp(prepare).unwrap();
                        let expected_return =
                            session.fixture_set_player_skill_occupied_slots_like_cpp(slots);
                        let expected = session.with_owned_player_like_cpp(projection);
                        session.with_owned_player_mut_like_cpp(prepare).unwrap();
                        assert_eq!(
                            session.set_player_skill_occupied_slots_like_cpp(slots),
                            expected_return
                        );
                        assert_eq!(session.with_owned_player_like_cpp(projection), expected);
                    }
                }
            }
        }
    }
}

#[test]
fn occupied_skill_slot_authority_rejects_stale_and_missing_owner() {
    let (mut session, _, _) = make_session();
    let guid = install_canonical_player_owner_for_test(&mut session, 571, 0);
    let manager = Arc::clone(session.canonical_map_manager.as_ref().unwrap());
    assert!(session.remove_current_player_from_canonical_current_map_like_cpp());
    let mut replacement = Box::new(Player::new(Some(1), false));
    replacement.unit_mut().world_mut().object_mut().create(guid);
    replacement.replace_skill_records_like_cpp(vec![], true, true, Some(7), BTreeSet::from([333]));
    let handle = manager
        .lock()
        .unwrap()
        .install_detached_player_like_cpp(replacement)
        .unwrap();
    for missing in [false, true] {
        if missing {
            session.canonical_map_manager = None;
        }
        assert!(!session.set_player_skill_occupied_slots_like_cpp(0));
        assert_eq!(
            manager
                .lock()
                .unwrap()
                .with_player_like_cpp(handle, |player| (
                    player.occupied_skill_slots_like_cpp(),
                    player.non_durable_skill_tombstones_like_cpp().clone()
                )),
            Some((Some(7), BTreeSet::from([333])))
        );
    }
}
