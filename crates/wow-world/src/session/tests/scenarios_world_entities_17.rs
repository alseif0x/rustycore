//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[tokio::test]
async fn gameobject_use_spellcaster_casts_and_ticks_charges_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 20);

    session.set_state(SessionState::LoggedIn);
    assert!(session.use_represented_gameobject_spellcaster_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::SpellcasterUseSource {
            spell_id: 3456,
            charges: 1,
            party_only: false,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::RemoveMountedAuras {
                gameobject_guid,
                player_guid,
            },
            RepresentedGameObjectUseEffect::GameObjectUseCountIncremented {
                gameobject_guid,
                use_count: 1,
            },
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid,
                player_guid,
                gameobject_entry: 777,
                spell_id: 3456,
                go_type: wow_entities::GAMEOBJECT_TYPE_SPELLCASTER,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: false,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                gameobject_guid,
                target_guid: player_guid,
                caster_guid: player_guid,
                spell_id: 3456,
                triggered: false,
                caster: RepresentedGameObjectSpellCaster::User,
                spell_lookup_difficulty_id: 0,
            },
        ]
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .unwrap()
            .use_count,
        1
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .unwrap()
            .max_charges,
        Some(1)
    );
    session.represented_gameobject_use_effects.clear();

    session.process_pending().await;

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::GameObjectChargesDepleted {
            gameobject_guid,
            max_charges: 1,
            loot_state: wow_entities::LootState::JustDeactivated,
        }]
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .unwrap()
            .use_count,
        0
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .unwrap()
            .loot_state,
        Some(wow_entities::LootState::JustDeactivated)
    );
}
#[test]
fn gameobject_use_spellcaster_party_only_fails_without_owner_context_like_cpp_guard() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 21);

    assert!(!session.use_represented_gameobject_spellcaster_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::SpellcasterUseSource {
            spell_id: 3456,
            charges: 1,
            party_only: true,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::SpellcasterPartyOnlyRejected {
                gameobject_guid,
                player_guid,
            }
        ]
    );
}
#[test]
fn gameobject_use_spellcaster_party_only_requires_owner_raid_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let owner_guid = ObjectGuid::create_player(1, 98);
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 22);
    let source = wow_entities::SpellcasterUseSource {
        spell_id: 3456,
        charges: 0,
        party_only: true,
    };
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .owner_guid = Some(owner_guid);

    assert!(!session.use_represented_gameobject_spellcaster_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        source,
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::SpellcasterPartyOnlyRejected {
                gameobject_guid,
                player_guid,
            }
        ]
    );
    session.represented_gameobject_use_effects.clear();

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(owner_guid);
    group.add_member(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    assert!(session.use_represented_gameobject_spellcaster_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        source,
    ));
    assert!(
        session
            .represented_gameobject_use_effects
            .iter()
            .any(|effect| matches!(
                effect,
                RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                    caster_guid,
                    spell_id: 3456,
                    ..
                } if *caster_guid == player_guid
            ))
    );
}
#[test]
fn gameobject_use_spellcaster_party_only_accepts_canonical_created_by_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let owner_guid = ObjectGuid::create_player(1, 98);
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 23);
    let canonical = shared_canonical_map_manager();
    let source = wow_entities::SpellcasterUseSource {
        spell_id: 3456,
        charges: 0,
        party_only: true,
    };

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.current_map_id = 571;
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::ZERO,
        wow_entities::GAMEOBJECT_TYPE_SPELLCASTER as u8,
    );
    {
        let mut canonical = canonical.lock().unwrap();
        canonical
            .find_map_mut(571, 0)
            .unwrap()
            .map_mut()
            .get_typed_game_object_mut(gameobject_guid)
            .unwrap()
            .set_created_by(owner_guid);
    }

    assert!(!session.use_represented_gameobject_spellcaster_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        source,
    ));
    session.represented_gameobject_use_effects.clear();

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(owner_guid);
    group.add_member(player_guid);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    assert!(session.use_represented_gameobject_spellcaster_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        source,
    ));
    assert!(
        session
            .represented_gameobject_use_effects
            .iter()
            .any(|effect| matches!(
                effect,
                RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                    caster_guid,
                    spell_id: 3456,
                    ..
                } if *caster_guid == player_guid
            ))
    );
}
#[test]
fn gameobject_use_spell_focus_triggers_linked_trap_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 8);

    assert!(session.use_represented_gameobject_spell_focus_like_cpp(
        gameobject_guid,
        player_guid,
        555,
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::TriggerLinkedTrap {
            gameobject_guid,
            player_guid,
            trap_entry: 555,
        }]
    );

    session.represented_gameobject_use_effects.clear();
    assert!(session.use_represented_gameobject_spell_focus_like_cpp(
        gameobject_guid,
        player_guid,
        0,
    ));
    assert!(session.represented_gameobject_use_effects.is_empty());
}
#[test]
fn gameobject_use_camera_records_cinematic_and_event_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 8);

    assert!(session.use_represented_gameobject_camera_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::CameraUseSource {
            cinematic_id: 444,
            event_id: 55,
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TriggerCinematic {
                gameobject_guid,
                player_guid,
                cinematic_id: 444,
            },
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 55,
            },
        ]
    );
    let mut expected = (ServerOpcodes::TriggerCinematic as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&444_u32.to_le_bytes());
    expected.extend_from_slice(&ObjectGuid::EMPTY.to_raw_bytes());
    assert_eq!(send_rx.try_recv().unwrap(), expected);
    assert_eq!(session.represented_cinematic_like_cpp(), None);

    session.set_cinematic_sequences_store(Arc::new(
        wow_data::CinematicSequencesStore::from_entries([wow_data::CinematicSequencesEntry {
            id: 444,
            sound_id: 0,
            camera: [0; 8],
        }]),
    ));
    assert!(session.use_represented_gameobject_camera_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::CameraUseSource {
            cinematic_id: 444,
            event_id: 0,
        },
    ));
    assert_eq!(session.represented_cinematic_like_cpp(), Some(444));
    let mut expected = (ServerOpcodes::TriggerCinematic as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&444_u32.to_le_bytes());
    expected.extend_from_slice(&ObjectGuid::EMPTY.to_raw_bytes());
    assert_eq!(send_rx.try_recv().unwrap(), expected);

    session.represented_gameobject_use_effects.clear();
    assert!(session.use_represented_gameobject_camera_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::CameraUseSource {
            cinematic_id: 0,
            event_id: 0,
        },
    ));
    assert!(session.represented_gameobject_use_effects.is_empty());
}
#[tokio::test]
async fn gameobject_use_goober_records_player_preamble_hooks_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 9);
    let position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(position);

    assert!(
        session
            .use_represented_gameobject_goober_preamble_like_cpp(
                gameobject_guid,
                777,
                position,
                player_guid,
                wow_entities::GooberUseSource {
                    page_id: 123,
                    gossip_id: 456,
                    event_id: 55,
                    linked_trap_entry: 999,
                    ..Default::default()
                },
            )
            .await
    );

    let mut expected = (ServerOpcodes::PageText as u16).to_le_bytes().to_vec();
    expected.extend_from_slice(&gameobject_guid.to_raw_bytes());
    assert_eq!(send_rx.try_recv().unwrap(), expected);
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::ShowPageText {
                gameobject_guid,
                player_guid,
                page_id: 123,
            },
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 55,
            },
            RepresentedGameObjectUseEffect::KillCreditGo {
                gameobject_guid,
                player_guid,
                entry: 777,
            },
            RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                gameobject_guid,
                player_guid,
                trap_entry: 999,
            },
        ]
    );

    session.represented_gameobject_use_effects.clear();
    assert!(
        session
            .use_represented_gameobject_goober_preamble_like_cpp(
                gameobject_guid,
                777,
                position,
                player_guid,
                wow_entities::GooberUseSource {
                    gossip_id: 456,
                    ..Default::default()
                },
            )
            .await
    );
    assert_eq!(
        session.represented_gameobject_use_effects.first(),
        Some(&RepresentedGameObjectUseEffect::SendGossip {
            gameobject_guid,
            player_guid,
            gossip_id: 456,
        })
    );
}
#[tokio::test]
async fn gameobject_use_goober_tracking_event_objective_auto_rewards_like_cpp() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 779, 9);
    let quest_id = 12_502;
    let gameobject_entry = 779;
    let mut quest = test_quest_template(quest_id);
    quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
    quest.objectives.push(wow_data::quest::QuestObjective {
        id: quest_id * 10,
        quest_id,
        obj_type: 2, // C++ QUEST_OBJECTIVE_GAMEOBJECT.
        order: 0,
        storage_index: 0,
        object_id: gameobject_entry as i32,
        amount: 1,
        flags: 0,
        flags2: 0,
        progress_bar_weight: 0.0,
        description: String::new(),
    });
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(Position::ZERO);
    session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
        [quest],
    )));
    session.player_quests.insert(
        quest_id,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![0],
            slot: 0,
        },
    );

    assert!(
        session
            .use_represented_gameobject_goober_preamble_like_cpp(
                gameobject_guid,
                gameobject_entry,
                Position::ZERO,
                player_guid,
                wow_entities::GooberUseSource::default(),
            )
            .await
    );

    assert!(!session.player_quests.contains_key(&quest_id));
    assert!(session.rewarded_quests.contains(&quest_id));
    assert_eq!(
        session.represented_quest_complete_status_updates_like_cpp(),
        &[RepresentedQuestCompleteStatusUpdateLikeCpp {
            quest_id,
            old_status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            new_status: crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP,
            send_quest_update_called: true,
            quest_slot_state_complete_represented: true,
            quest_slot_state_live_update_unrepresented: true,
            visible_gameobjects_or_spellclicks_refresh_unrepresented: true,
            spell_area_runtime_unrepresented: true,
            tracking_event_auto_reward_unrepresented: false,
            quest_tracker_complete_time_unrepresented: true,
            script_status_change_unrepresented: true,
        }]
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![
            ServerOpcodes::QuestUpdateAddCredit,
            ServerOpcodes::UpdateObject,
            ServerOpcodes::QuestGiverQuestComplete,
            ServerOpcodes::QuestUpdateComplete,
        ]
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::KillCreditGo {
            gameobject_guid,
            player_guid,
            entry: gameobject_entry,
        }]
    );
}
#[tokio::test]
async fn gameobject_use_goober_quest_gate_matches_cpp_incomplete_requirement() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 10);
    let mut quest_store = wow_data::quest::QuestStore::new();
    quest_store.quests.insert(200, test_quest_template(200));
    session.set_quest_store(Arc::new(quest_store));
    session.set_player_guid(Some(player_guid));

    assert!(
        !session
            .use_represented_gameobject_goober_preamble_like_cpp(
                gameobject_guid,
                777,
                Position::ZERO,
                player_guid,
                wow_entities::GooberUseSource {
                    quest_id: 200,
                    event_id: 55,
                    linked_trap_entry: 999,
                    ..Default::default()
                },
            )
            .await
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TriggerGameEvent {
                gameobject_guid,
                player_guid,
                event_id: 55,
            },
            RepresentedGameObjectUseEffect::GooberQuestGateRejected {
                gameobject_guid,
                player_guid,
                quest_id: 200,
            },
        ]
    );

    session.represented_gameobject_use_effects.clear();
    session.player_quests.insert(
        200,
        crate::handlers::quest::PlayerQuestStatus {
            quest_id: 200,
            status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            explored: false,
            accept_time_secs: 0,
            end_time_secs: 0,
            objective_counts: vec![],
            slot: 0,
        },
    );
    assert!(
        session
            .use_represented_gameobject_goober_preamble_like_cpp(
                gameobject_guid,
                777,
                Position::ZERO,
                player_guid,
                wow_entities::GooberUseSource {
                    quest_id: 200,
                    linked_trap_entry: 999,
                    ..Default::default()
                },
            )
            .await
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::KillCreditGo {
                gameobject_guid,
                player_guid,
                entry: 777,
            },
            RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                gameobject_guid,
                player_guid,
                trap_entry: 999,
            },
        ]
    );
}
#[tokio::test]
async fn gameobject_use_goober_kill_credit_filters_group_reward_distance_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let near_member = ObjectGuid::create_player(1, 100);
    let far_member = ObjectGuid::create_player(1, 101);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 11);
    let gameobject_position = Position::new(10.0, 0.0, 0.0, 0.0);
    session.set_player_guid(Some(player_guid));
    session.set_player_position_like_cpp(gameobject_position);

    let player_registry = Arc::new(PlayerRegistry::default());
    let (near_tx, _near_rx) = flume::bounded(1);
    let (far_tx, _far_rx) = flume::bounded(1);
    let mut near_info = broadcast_info(near_member, near_tx);
    near_info.placement.position = Position::new(20.0, 0.0, 0.0, 0.0);
    let mut far_info = broadcast_info(far_member, far_tx);
    far_info.placement.position = Position::new(10_000.0, 0.0, 0.0, 0.0);
    player_registry.register_or_replace(near_member, near_info, Default::default());
    player_registry.register_or_replace(far_member, far_info, Default::default());

    let group_registry = Arc::new(GroupRegistry::default());
    let mut group = GroupInfo::new(player_guid);
    group.add_member(near_member);
    group.add_member(far_member);
    let group_guid = group.group_guid;
    group_registry.register_group_like_cpp(group_guid, group);
    session.group_guid = Some(group_guid);
    session.set_player_registry(player_registry);
    session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));

    assert!(
        session
            .use_represented_gameobject_goober_preamble_like_cpp(
                gameobject_guid,
                777,
                gameobject_position,
                player_guid,
                wow_entities::GooberUseSource::default(),
            )
            .await
    );

    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::KillCreditGo {
                gameobject_guid,
                player_guid,
                entry: 777,
            },
            RepresentedGameObjectUseEffect::KillCreditGo {
                gameobject_guid,
                player_guid: near_member,
                entry: 777,
            },
        ]
    );
}
#[test]
fn gameobject_use_goober_state_branch_matches_cpp_global_use() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 12);

    assert!(session.use_represented_gameobject_goober_state_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::GooberUseSource {
            auto_close_ms: 3_000,
            spell_id: 7777,
            player_cast: true,
            ..Default::default()
        },
    ));

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 1);
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, player_guid);
    assert_eq!(state.go_state, Some(wow_entities::GoState::Active));
    assert!(state.cooldown_until.is_some());
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::GooberUsed {
                gameobject_guid,
                user_guid: player_guid,
                custom_anim: 0,
                auto_close_ms: 3_000,
                go_state: Some(wow_entities::GoState::Active),
            },
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid,
                player_guid,
                gameobject_entry: 777,
                spell_id: 7777,
                go_type: wow_entities::GAMEOBJECT_TYPE_GOOBER,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: false,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                gameobject_guid,
                target_guid: player_guid,
                caster_guid: player_guid,
                spell_id: 7777,
                triggered: false,
                caster: RepresentedGameObjectSpellCaster::User,
                spell_lookup_difficulty_id: 0,
            },
        ]
    );
}
#[test]
fn gameobject_use_goober_state_branch_matches_cpp_custom_anim_and_go_cast() {
    let (mut session, _pkt_tx, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let other_map_guid = ObjectGuid::create_player(1, 88);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 13);
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
    session.record_represented_gameobject_anim_progress_like_cpp(gameobject_guid, 123);

    assert!(session.use_represented_gameobject_goober_state_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        wow_entities::GooberUseSource {
            custom_anim: 2,
            spell_id: 8888,
            player_cast: false,
            ..Default::default()
        },
    ));

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.go_state, None);
    let mut expected = (ServerOpcodes::GameObjectCustomAnim as u16)
        .to_le_bytes()
        .to_vec();
    expected.extend_from_slice(&gameobject_guid.to_raw_bytes());
    expected.extend_from_slice(&123_u32.to_le_bytes());
    expected.push(0x00);
    assert_eq!(send_rx.try_recv().unwrap(), expected);
    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SendIfVisibleLikeCpp(command)) => command,
        other => panic!("expected SendIfVisibleLikeCpp custom anim command, got {other:?}"),
    };
    assert_eq!(command.source_guid, gameobject_guid);
    assert_eq!(command.map_id, 571);
    assert_eq!(command.instance_id, 0);
    assert_eq!(command.packet_bytes, expected);
    assert!(other_command_rx.try_recv().is_err());
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::GooberUsed {
                gameobject_guid,
                user_guid: player_guid,
                custom_anim: 2,
                auto_close_ms: 0,
                go_state: None,
            },
            RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                gameobject_guid,
                player_guid,
                gameobject_entry: 777,
                spell_id: 8888,
                go_type: wow_entities::GAMEOBJECT_TYPE_GOOBER,
                spell_lookup_difficulty_id: 0,
                spell_info_missing: false,
            },
            RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                gameobject_guid,
                target_guid: player_guid,
                caster_guid: gameobject_guid,
                spell_id: 8888,
                triggered: false,
                caster: RepresentedGameObjectSpellCaster::GameObject,
                spell_lookup_difficulty_id: 0,
            },
        ]
    );
}
