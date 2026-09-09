//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn gameobject_use_new_flag_drop_records_owner_state_and_delete_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 37);
    let owner_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 36);
    let map_id = session.player_map_id_like_cpp();
    session.record_represented_gameobject_owner_guid_like_cpp(gameobject_guid, owner_guid);
    {
        let owner_state = session
            .represented_gameobject_use_states
            .entry(owner_guid)
            .or_default();
        owner_state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_NEW_FLAG as u8);
        owner_state.new_flag_state = Some(RepresentedNewFlagStateRequest::Dropped);
        owner_state.new_flag_return_on_defender_interact = Some(false);
        owner_state.new_flag_pickup_spell_id = Some(33);
        owner_state.new_flag_entry = Some(179_830);
    }

    assert!(session.use_represented_gameobject_new_flag_drop_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::NewFlagDropUseSource {
            spawn_vignette_id: 222,
        }
    ));
    let mut expected = (ServerOpcodes::GameObjectDespawn as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&gameobject_guid.to_raw_bytes());
    assert_eq!(send_rx.try_recv().unwrap(), expected);
    assert_eq!(
        send_rx.try_recv().unwrap(),
        wow_packet::packets::update::UpdateObject::destroy_objects(vec![gameobject_guid], map_id)
            .to_bytes()
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::NewFlagDropInteracted {
                gameobject_guid,
                player_guid,
                spawn_vignette_id: 222,
            },
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid: owner_guid,
                player_guid,
                gameobject_entry: 179_830,
                spell_id: 33,
                go_type: wow_entities::GAMEOBJECT_TYPE_NEW_FLAG,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: false,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                gameobject_guid: owner_guid,
                target_guid: player_guid,
                caster_guid: owner_guid,
                spell_id: 33,
                triggered: true,
                caster: RepresentedGameObjectSpellCaster::GameObject,
                spell_lookup_difficulty_id: 0,
            },
            RepresentedGameObjectUseEffect::GameObjectDeleted { gameobject_guid },
            RepresentedGameObjectUseEffect::NewFlagOwnerStateRequested {
                gameobject_guid: owner_guid,
                player_guid,
                state: RepresentedNewFlagStateRequest::Taken,
            },
        ]
    );
    let state = session
        .represented_gameobject_use_states
        .get(&owner_guid)
        .unwrap();
    assert_eq!(
        state.new_flag_state,
        Some(RepresentedNewFlagStateRequest::Taken)
    );
    assert_eq!(state.new_flag_carrier_guid, Some(player_guid));
    assert_eq!(state.new_flag_taken_from_base_game_time_ms, None);
}
#[test]
fn gameobject_use_new_flag_drop_returns_owner_flag_on_defender_interact_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 37);
    let owner_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 36);
    session.record_represented_gameobject_owner_guid_like_cpp(gameobject_guid, owner_guid);
    session.set_player_faction_template_like_cpp(1);
    session.record_represented_gameobject_faction_template_like_cpp(owner_guid, 2);
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(1, 10, 1, 0, 20),
            faction_template_entry(2, 20, 2, 0, 0),
        ]),
    ));
    {
        let owner_state = session
            .represented_gameobject_use_states
            .entry(owner_guid)
            .or_default();
        owner_state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_NEW_FLAG as u8);
        owner_state.new_flag_state = Some(RepresentedNewFlagStateRequest::Dropped);
        owner_state.new_flag_return_on_defender_interact = Some(true);
        owner_state.new_flag_pickup_spell_id = Some(33);
        owner_state.new_flag_entry = Some(179_830);
    }

    assert!(session.use_represented_gameobject_new_flag_drop_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::NewFlagDropUseSource {
            spawn_vignette_id: 222,
        }
    ));
    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::NewFlagOwnerStateRequested {
            gameobject_guid: owner_guid,
            player_guid,
            state: RepresentedNewFlagStateRequest::InBase,
        }
    ));
    let state = session
        .represented_gameobject_use_states
        .get(&owner_guid)
        .unwrap();
    assert_eq!(
        state.new_flag_state,
        Some(RepresentedNewFlagStateRequest::InBase)
    );
    assert_eq!(state.new_flag_carrier_guid, None);
}
#[test]
fn gameobject_use_new_flag_drop_deletes_without_taken_when_owner_cast_fails_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 37);
    let owner_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 36);
    session.record_represented_gameobject_owner_guid_like_cpp(gameobject_guid, owner_guid);
    session.set_spell_store(std::sync::Arc::new(wow_data::SpellStore::new()));
    {
        let owner_state = session
            .represented_gameobject_use_states
            .entry(owner_guid)
            .or_default();
        owner_state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_NEW_FLAG as u8);
        owner_state.new_flag_state = Some(RepresentedNewFlagStateRequest::Dropped);
        owner_state.new_flag_return_on_defender_interact = Some(false);
        owner_state.new_flag_pickup_spell_id = Some(33);
        owner_state.new_flag_entry = Some(179_830);
    }

    assert!(session.use_represented_gameobject_new_flag_drop_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::NewFlagDropUseSource {
            spawn_vignette_id: 222,
        }
    ));
    assert!(session.represented_gameobject_use_effects.contains(
        &RepresentedGameObjectUseEffect::GameObjectPostUseSpellMissing {
            gameobject_guid: owner_guid,
            player_guid,
            gameobject_entry: 179_830,
            spell_id: 33,
            go_type: wow_entities::GAMEOBJECT_TYPE_NEW_FLAG,
            spell_lookup_difficulty_id: 0,
        }
    ));
    assert!(
        !session
            .represented_gameobject_use_effects
            .iter()
            .any(|effect| {
                matches!(
                    effect,
                    RepresentedGameObjectUseEffect::NewFlagOwnerStateRequested {
                        gameobject_guid,
                        state: RepresentedNewFlagStateRequest::Taken,
                        ..
                    } if *gameobject_guid == owner_guid
                )
            })
    );
    let state = session
        .represented_gameobject_use_states
        .get(&owner_guid)
        .unwrap();
    assert_eq!(
        state.new_flag_state,
        Some(RepresentedNewFlagStateRequest::Dropped)
    );
}
#[test]
fn gameobject_use_new_flag_drop_without_owner_still_deletes_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 37);

    assert!(session.use_represented_gameobject_new_flag_drop_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::NewFlagDropUseSource {
            spawn_vignette_id: 222,
        }
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::NewFlagDropInteracted {
                gameobject_guid,
                player_guid,
                spawn_vignette_id: 222,
            },
            RepresentedGameObjectUseEffect::GameObjectDeleted { gameobject_guid },
        ]
    );
}
#[test]
fn new_flag_state_command_tracks_cpp_state_carrier_and_respawn_timer() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 38);

    assert!(session.apply_represented_new_flag_state_command_like_cpp(
        gameobject_guid,
        Some(player_guid),
        RepresentedNewFlagStateRequest::Taken,
        0,
    ));
    assert!(!session.apply_represented_new_flag_state_command_like_cpp(
        gameobject_guid,
        Some(player_guid),
        RepresentedNewFlagStateRequest::Taken,
        0,
    ));
    {
        let state = session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .unwrap();
        assert_eq!(
            state.new_flag_state,
            Some(RepresentedNewFlagStateRequest::Taken)
        );
        assert_eq!(state.new_flag_carrier_guid, Some(player_guid));
        assert!(state.new_flag_taken_from_base_game_time_ms.is_some());
        assert!(state.new_flag_respawn_until.is_none());
    }

    assert!(session.apply_represented_new_flag_state_command_like_cpp(
        gameobject_guid,
        Some(player_guid),
        RepresentedNewFlagStateRequest::Respawning,
        5000,
    ));
    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(
        state.new_flag_state,
        Some(RepresentedNewFlagStateRequest::Respawning)
    );
    assert_eq!(state.new_flag_carrier_guid, None);
    assert_eq!(state.new_flag_taken_from_base_game_time_ms, None);
    assert!(state.new_flag_respawn_until.is_some());
}
#[test]
fn gameobject_use_ritual_waits_until_required_unique_casters_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 18);

    assert!(!session.use_represented_gameobject_ritual_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::RitualUseSource {
            casters_required: 2,
            spell_id: 100,
            anim_spell_id: 200,
            persistent: false,
            caster_target_spell_id: 0,
            caster_target_spell_targets: 0,
            casters_grouped: true,
            no_target_check: true,
            allow_unfriendly_cross_faction_party: false,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::CastSpell {
                gameobject_guid,
                player_guid,
                spell_id: 200,
            },
            RepresentedGameObjectUseEffect::RitualWaitingForParticipants {
                gameobject_guid,
                player_guid,
                unique_user_count: 1,
                casters_required: 2,
            },
        ]
    );
}
#[test]
fn gameobject_use_ritual_validates_summoned_owner_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let owner_guid = ObjectGuid::create_player(1, 98);
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 18);
    let source = wow_entities::RitualUseSource {
        casters_required: 2,
        spell_id: 100,
        anim_spell_id: 200,
        persistent: false,
        caster_target_spell_id: 0,
        caster_target_spell_targets: 0,
        casters_grouped: true,
        no_target_check: true,
        allow_unfriendly_cross_faction_party: false,
    };
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .owner_guid = Some(owner_guid);

    assert!(!session.use_represented_gameobject_ritual_like_cpp(
        gameobject_guid,
        owner_guid,
        source,
    ));
    assert!(session.represented_gameobject_use_effects.is_empty());
    assert!(
        session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .unwrap()
            .unique_users
            .is_empty()
    );

    assert!(!session.use_represented_gameobject_ritual_like_cpp(
        gameobject_guid,
        player_guid,
        source,
    ));
    assert!(session.represented_gameobject_use_effects.is_empty());

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(owner_guid);
    group.add_member(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .owner_current_channeled_spell_active = Some(false);

    assert!(!session.use_represented_gameobject_ritual_like_cpp(
        gameobject_guid,
        player_guid,
        source,
    ));
    assert!(session.represented_gameobject_use_effects.is_empty());

    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .owner_current_channeled_spell_active = Some(true);

    assert!(!session.use_represented_gameobject_ritual_like_cpp(
        gameobject_guid,
        player_guid,
        source,
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::CastSpell {
                gameobject_guid,
                player_guid,
                spell_id: 200,
            },
            RepresentedGameObjectUseEffect::RitualWaitingForParticipants {
                gameobject_guid,
                player_guid,
                unique_user_count: 1,
                casters_required: 2,
            },
        ]
    );
}
#[test]
fn gameobject_use_ritual_completes_and_deactivates_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let other_player_guid = ObjectGuid::create_player(1, 100);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 18);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .unique_users = vec![other_player_guid];
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(other_player_guid);
    group.add_member(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    assert!(session.use_represented_gameobject_ritual_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::RitualUseSource {
            casters_required: 2,
            spell_id: 62330,
            anim_spell_id: 0,
            persistent: false,
            caster_target_spell_id: 333,
            caster_target_spell_targets: 1,
            casters_grouped: true,
            no_target_check: true,
            allow_unfriendly_cross_faction_party: false,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::RitualCasterTargetSpellRequested {
                gameobject_guid,
                player_guid,
                spell_id: 333,
                target_count: 1,
            },
            RepresentedGameObjectUseEffect::RitualCompleted {
                gameobject_guid,
                player_guid,
                final_spell_id: 61993,
                triggered: true,
                persistent: false,
                unique_user_count: 2,
            },
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid,
                player_guid,
                gameobject_entry: 777,
                spell_id: 61993,
                go_type: wow_entities::GAMEOBJECT_TYPE_RITUAL,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: false,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                gameobject_guid,
                target_guid: player_guid,
                caster_guid: other_player_guid,
                spell_id: 61993,
                triggered: true,
                caster: RepresentedGameObjectSpellCaster::User,
                spell_lookup_difficulty_id: 0,
            },
        ]
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .and_then(|state| state.loot_state),
        Some(wow_entities::LootState::JustDeactivated)
    );
}
#[test]
fn gameobject_use_ritual_casts_caster_target_spell_at_random_unique_users_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.seed_represented_runtime_rng_like_cpp(0xA141);
    let player_guid = ObjectGuid::create_player(1, 99);
    let other_player_guid = ObjectGuid::create_player(1, 100);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 18);
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .unique_users = vec![other_player_guid];
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(other_player_guid);
    group.add_member(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (player_tx, _player_rx) = flume::bounded(1);
    player_registry.register_or_replace(
        player_guid,
        broadcast_info(player_guid, player_tx),
        Default::default(),
    );
    let (other_tx, _other_rx) = flume::bounded(1);
    player_registry.register_or_replace(
        other_player_guid,
        broadcast_info(other_player_guid, other_tx),
        Default::default(),
    );
    session.set_player_registry(player_registry);

    assert!(session.use_represented_gameobject_ritual_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::RitualUseSource {
            casters_required: 2,
            spell_id: 100,
            anim_spell_id: 0,
            persistent: false,
            caster_target_spell_id: 333,
            caster_target_spell_targets: 4,
            casters_grouped: true,
            no_target_check: true,
            allow_unfriendly_cross_faction_party: false,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects.first(),
        Some(
            &RepresentedGameObjectUseEffect::RitualCasterTargetSpellRequested {
                gameobject_guid,
                player_guid,
                spell_id: 333,
                target_count: 4,
            }
        )
    );
    let target_casts: Vec<_> = session
        .represented_gameobject_use_effects
        .iter()
        .filter_map(|effect| {
            if let RepresentedGameObjectUseEffect::RitualCasterTargetSpellCast {
                gameobject_guid: cast_gameobject_guid,
                caster_guid,
                target_guid,
                spell_id,
                triggered,
            } = effect
            {
                Some((
                    *cast_gameobject_guid,
                    *caster_guid,
                    *target_guid,
                    *spell_id,
                    *triggered,
                ))
            } else {
                None
            }
        })
        .collect();
    let expected_targets: Vec<_> = {
        let unique_users = vec![other_player_guid, player_guid];
        let mut rng = StdRng::seed_from_u64(0xA141);
        (0..4)
            .map(|_| *unique_users.choose(&mut rng).expect("seeded target"))
            .collect()
    };
    assert_eq!(target_casts.len(), 4);
    assert_eq!(
        target_casts
            .iter()
            .map(|(_, _, target_guid, _, _)| *target_guid)
            .collect::<Vec<_>>(),
        expected_targets,
        "represented ritual random target selection should be driven by the session-owned StdRng, not thread-local entropy"
    );
    for (cast_gameobject_guid, caster_guid, target_guid, spell_id, triggered) in target_casts {
        assert_eq!(cast_gameobject_guid, gameobject_guid);
        assert_eq!(caster_guid, other_player_guid);
        assert!(target_guid == player_guid || target_guid == other_player_guid);
        assert_eq!(spell_id, 333);
        assert!(triggered);
    }
}
#[test]
fn gameobject_use_persistent_summoned_ritual_keeps_gameobject_owner_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let owner_guid = ObjectGuid::create_player(1, 98);
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 18);
    let state = session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default();
    state.owner_guid = Some(owner_guid);
    state.owner_current_channeled_spell_active = Some(true);
    state.unique_users = vec![owner_guid];
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(owner_guid);
    group.add_member(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    assert!(session.use_represented_gameobject_ritual_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::RitualUseSource {
            casters_required: 2,
            spell_id: 100,
            anim_spell_id: 0,
            persistent: true,
            caster_target_spell_id: 0,
            caster_target_spell_targets: 0,
            casters_grouped: true,
            no_target_check: true,
            allow_unfriendly_cross_faction_party: false,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::FinishChanneledSpell {
                player_guid: owner_guid,
            },
            RepresentedGameObjectUseEffect::RitualCompleted {
                gameobject_guid,
                player_guid,
                final_spell_id: 100,
                triggered: false,
                persistent: true,
                unique_user_count: 2,
            },
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid,
                player_guid,
                gameobject_entry: 777,
                spell_id: 100,
                go_type: wow_entities::GAMEOBJECT_TYPE_RITUAL,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: false,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                gameobject_guid,
                target_guid: player_guid,
                caster_guid: owner_guid,
                spell_id: 100,
                triggered: false,
                caster: RepresentedGameObjectSpellCaster::User,
                spell_lookup_difficulty_id: 0,
            },
        ]
    );
    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.owner_guid, Some(owner_guid));
    assert_eq!(state.ritual_owner_guid, None);
    assert!(state.unique_users.is_empty());
}
#[test]
fn gameobject_use_meeting_stone_maps_spell_by_entry_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let target_guid = ObjectGuid::create_player(1, 100);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 23);
    session.set_selection_guid_like_cpp(Some(target_guid));
    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(target_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, _target_rx) = flume::bounded(1);
    let mut target_info = broadcast_info(target_guid, target_tx);
    target_info.placement.level = 80;
    player_registry.register_or_replace(target_guid, target_info, Default::default());
    session.set_player_registry(player_registry);

    assert!(session.use_represented_gameobject_meeting_stone_like_cpp(
        gameobject_guid,
        player_guid,
        194097,
        wow_entities::MeetingStoneUseSource {
            area_id: 456,
            prevent_unfriendly_outside_instances: true,
            content_tuning_id: 0,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::MeetingStoneSummonRequested {
                gameobject_guid,
                player_guid,
                target_guid,
                gameobject_entry: 194097,
                spell_id: 61994,
                area_id: 456,
                prevent_unfriendly_outside_instances: true,
            },
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid,
                player_guid,
                gameobject_entry: 194097,
                spell_id: 61994,
                go_type: wow_entities::GAMEOBJECT_TYPE_MEETINGSTONE,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: false,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                gameobject_guid,
                target_guid: player_guid,
                caster_guid: player_guid,
                spell_id: 61994,
                triggered: false,
                caster: RepresentedGameObjectSpellCaster::User,
                spell_lookup_difficulty_id: 0,
            }
        ]
    );
}
#[test]
fn gameobject_use_meeting_stone_requires_selected_raid_target_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let target_guid = ObjectGuid::create_player(1, 100);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 23);
    let source = wow_entities::MeetingStoneUseSource {
        area_id: 456,
        prevent_unfriendly_outside_instances: true,
        content_tuning_id: 0,
    };

    assert!(!session.use_represented_gameobject_meeting_stone_like_cpp(
        gameobject_guid,
        player_guid,
        194097,
        source,
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::MeetingStoneTargetRejected {
            gameobject_guid,
            player_guid,
            target_guid: None,
        }]
    );
    session.represented_gameobject_use_effects.clear();
    session.set_selection_guid_like_cpp(Some(player_guid));

    assert!(!session.use_represented_gameobject_meeting_stone_like_cpp(
        gameobject_guid,
        player_guid,
        194097,
        source,
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::MeetingStoneTargetRejected {
            gameobject_guid,
            player_guid,
            target_guid: Some(player_guid),
        }]
    );
    session.represented_gameobject_use_effects.clear();
    session.set_selection_guid_like_cpp(Some(target_guid));

    assert!(!session.use_represented_gameobject_meeting_stone_like_cpp(
        gameobject_guid,
        player_guid,
        194097,
        source,
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::MeetingStoneTargetRejected {
            gameobject_guid,
            player_guid,
            target_guid: Some(target_guid),
        }]
    );
}
#[test]
fn gameobject_use_meeting_stone_checks_content_tuning_levels_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let target_guid = ObjectGuid::create_player(1, 100);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 23);
    session.set_player_guid(Some(player_guid));
    session.set_player_level_like_cpp(19);
    session.set_selection_guid_like_cpp(Some(target_guid));
    session.set_content_tuning_store(Arc::new(ContentTuningStore::from_entries([
        wow_data::progression_rewards::ContentTuningEntry {
            id: 55,
            min_level: 1,
            max_level: 20,
            flags: 0,
            expected_stat_mod_id: 0,
            difficulty_esm_id: 0,
        },
    ])));

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(target_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    let player_registry = Arc::new(PlayerRegistry::with_canonical_player_fixtures_like_cpp());
    let (target_tx, _target_rx) = flume::bounded(1);
    let mut target_info = broadcast_info(target_guid, target_tx);
    target_info.placement.level = 20;
    player_registry.register_or_replace(target_guid, target_info, Default::default());
    session.set_player_registry(player_registry);
    let source = wow_entities::MeetingStoneUseSource {
        area_id: 456,
        prevent_unfriendly_outside_instances: true,
        content_tuning_id: 55,
    };

    assert!(!session.use_represented_gameobject_meeting_stone_like_cpp(
        gameobject_guid,
        player_guid,
        194097,
        source,
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::MeetingStoneLevelRejected {
            gameobject_guid,
            player_guid,
            target_guid,
            player_level: 19,
            target_level: 20,
            required_level: 20,
        }]
    );

    session.represented_gameobject_use_effects.clear();
    session.set_player_level_like_cpp(20);
    assert!(session.use_represented_gameobject_meeting_stone_like_cpp(
        gameobject_guid,
        player_guid,
        194097,
        source,
    ));
}
