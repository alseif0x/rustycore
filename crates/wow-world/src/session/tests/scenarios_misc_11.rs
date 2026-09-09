//! Session scenarios exercising the represented misc responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_get_reaction_player_controlled_owner_branches_match_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(1, 72, 0, 0, 0),
            faction_template_entry(2, 930, 0, 0, 0),
        ]),
    ));

    let mut input = represented_get_reaction_input_like_cpp();
    input.same_player_owner = true;
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Friendly
    );

    input.same_player_owner = false;
    input.duel_in_progress = true;
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Hostile
    );

    input.duel_in_progress = false;
    input.same_raid = true;
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Friendly
    );

    input.same_raid = false;
    input.self_ffa_pvp = true;
    input.target_ffa_pvp = true;
    assert_eq!(
        session.represented_get_reaction_to_like_cpp(input),
        wow_data::reputation::ReputationRankLikeCpp::Hostile
    );
}
#[test]
fn battleground_object_use_guard_matches_cpp_faction_and_player_state() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 39);

    session.set_player_faction_template_like_cpp(1);
    session.record_represented_gameobject_faction_template_like_cpp(gameobject_guid, 2);
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(1, 10, 1, 0, 20),
            faction_template_entry(2, 20, 2, 0, 0),
        ]),
    ));
    assert!(
        !session
            .represented_player_can_use_battleground_object_like_cpp(gameobject_guid, player_guid,)
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::UnfriendlyFaction,
            }
        ]
    );

    session.represented_gameobject_use_effects.clear();
    session.record_represented_gameobject_faction_template_like_cpp(gameobject_guid, 0);
    session.player_unit_flags_like_cpp.insert(UnitFlags::IMMUNE);
    assert!(
        !session
            .represented_player_can_use_battleground_object_like_cpp(gameobject_guid, player_guid,)
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::DamageImmune,
            }
        ]
    );

    session.represented_gameobject_use_effects.clear();
    session.player_unit_flags_like_cpp.remove(UnitFlags::IMMUNE);
    session.apply_aura(50_327, player_guid, 30_000, 0).unwrap();
    assert!(
        !session
            .represented_player_can_use_battleground_object_like_cpp(gameobject_guid, player_guid,)
    );
    assert_eq!(
        session.represented_gameobject_use_effects.last(),
        Some(
            &RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::RecentlyDroppedFlag,
            }
        )
    );

    session.visible_auras.clear();
    session.represented_gameobject_use_effects.clear();
    session.set_player_alive_like_cpp(false);
    assert!(
        !session
            .represented_player_can_use_battleground_object_like_cpp(gameobject_guid, player_guid,)
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::BattlegroundObjectUseRejected {
                gameobject_guid,
                player_guid,
                reason: RepresentedBattlegroundObjectUseRejection::Dead,
            }
        ]
    );
}
#[tokio::test]
async fn process_pending_ticks_expired_door_or_button_like_cpp_update() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let door_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 73);
    let trap_guid = ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 778, 74);

    session.set_state(SessionState::LoggedIn);
    assert!(session.use_represented_gameobject_door_or_button_like_cpp(
        door_guid,
        player_guid,
        3000,
    ));
    {
        let state = session
            .represented_gameobject_use_states
            .get_mut(&door_guid)
            .unwrap();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_DOOR as u8);
        state.cooldown_until = Some(Instant::now() - Duration::from_millis(1));
    }
    {
        let state = session
            .represented_gameobject_use_states
            .entry(trap_guid)
            .or_default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
        state.loot_state = Some(wow_entities::LootState::Activated);
        state.cooldown_until = Some(Instant::now() - Duration::from_millis(1));
        state.gameobject_flags = wow_entities::GO_FLAG_IN_USE;
    }

    session.process_pending().await;

    let door_state = session
        .represented_gameobject_use_states
        .get(&door_guid)
        .unwrap();
    assert_eq!(
        door_state.loot_state,
        Some(wow_entities::LootState::JustDeactivated)
    );
    assert_eq!(
        door_state.gameobject_flags & wow_entities::GO_FLAG_IN_USE,
        0
    );
    assert!(door_state.cooldown_until.is_none());
    assert!(
        session
            .represented_gameobject_use_effects
            .iter()
            .any(|effect| matches!(
                effect,
                RepresentedGameObjectUseEffect::DoorOrButtonReset {
                    gameobject_guid,
                    go_state: wow_entities::GoState::Ready,
                } if *gameobject_guid == door_guid
            ))
    );

    let trap_state = session
        .represented_gameobject_use_states
        .get(&trap_guid)
        .unwrap();
    assert_eq!(
        trap_state.loot_state,
        Some(wow_entities::LootState::Activated)
    );
    assert_eq!(
        trap_state.gameobject_flags & wow_entities::GO_FLAG_IN_USE,
        wow_entities::GO_FLAG_IN_USE
    );
    assert!(trap_state.cooldown_until.is_some());
}
#[tokio::test]
async fn process_pending_ticks_bomb_trap_update_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 75);

    session.set_state(SessionState::LoggedIn);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
        state.loot_state = Some(wow_entities::LootState::NotReady);
        state.trap_use_source = Some(wow_entities::TrapUseSource {
            spell_id: 1234,
            charges: 2,
            cooldown_secs: 0,
            ..Default::default()
        });
    }

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Ready));
    assert!(state.cooldown_until.is_some_and(|cooldown_until| {
        cooldown_until > Instant::now() + Duration::from_secs(9)
    }));
    assert!(session.represented_gameobject_use_effects.is_empty());

    session
        .represented_gameobject_use_states
        .get_mut(&gameobject_guid)
        .unwrap()
        .cooldown_until = Some(Instant::now() - Duration::from_millis(1));
    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
    assert!(session.represented_gameobject_use_effects.is_empty());

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(
        state.loot_state,
        Some(wow_entities::LootState::JustDeactivated)
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::TrapBombSpellCast {
            gameobject_guid,
            spell_id: 1234,
        }]
    );
}
#[tokio::test]
async fn process_pending_ticks_non_bomb_trap_not_ready_start_delay_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let owner_guid = ObjectGuid::create_player(1, 99);
    let combat_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 76);
    let idle_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 77);
    for (guid, owner_in_combat) in [(combat_trap_guid, true), (idle_trap_guid, false)] {
        let state = session
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
        state.loot_state = Some(wow_entities::LootState::NotReady);
        state.owner_guid = Some(owner_guid);
        state.owner_in_combat = Some(owner_in_combat);
        state.trap_use_source = Some(wow_entities::TrapUseSource {
            charges: 1,
            start_delay_secs: 3,
            ..Default::default()
        });
    }

    session.process_pending().await;

    let combat_state = session
        .represented_gameobject_use_states
        .get(&combat_trap_guid)
        .unwrap();
    assert_eq!(
        combat_state.loot_state,
        Some(wow_entities::LootState::Ready)
    );
    assert!(combat_state.cooldown_until.is_some_and(|cooldown_until| {
        cooldown_until > Instant::now() + Duration::from_secs(2)
    }));

    let idle_state = session
        .represented_gameobject_use_states
        .get(&idle_trap_guid)
        .unwrap();
    assert_eq!(idle_state.loot_state, Some(wow_entities::LootState::Ready));
    assert!(idle_state.cooldown_until.is_none());
}
#[tokio::test]
async fn process_pending_ticks_non_bomb_trap_target_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let player_guid = ObjectGuid::create_player(1, 99);
    let target_guid = ObjectGuid::create_player(1, 100);
    let owner_guid = ObjectGuid::create_player(1, 101);
    let environmental_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 83);
    let owned_trap_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 84);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::new(3.0, 0.0, 0.0, 0.0));
    {
        let state = session
            .represented_gameobject_use_states
            .entry(environmental_trap_guid)
            .or_default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
        state.map_id = Some(571);
        state.position = Some(Position::ZERO);
        state.loot_state = Some(wow_entities::LootState::Ready);
        state.trap_use_source = Some(wow_entities::TrapUseSource {
            radius: 8,
            spell_id: 333,
            charges: 0,
            cooldown_secs: 9,
            ..Default::default()
        });
    }
    {
        let state = session
            .represented_gameobject_use_states
            .entry(owned_trap_guid)
            .or_default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_TRAP as u8);
        state.loot_state = Some(wow_entities::LootState::Ready);
        state.owner_guid = Some(owner_guid);
        state.trap_target_guid = Some(target_guid);
        state.trap_use_source = Some(wow_entities::TrapUseSource {
            radius: 12,
            spell_id: 444,
            charges: 1,
            cooldown_secs: 0,
            check_all_units: true,
            ..Default::default()
        });
    }

    session.process_pending().await;

    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&environmental_trap_guid)
            .unwrap()
            .loot_state,
        Some(wow_entities::LootState::Activated)
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&environmental_trap_guid)
            .unwrap()
            .loot_state_unit_guid,
        player_guid
    );
    assert_eq!(
        session
            .represented_gameobject_use_states
            .get(&owned_trap_guid)
            .unwrap()
            .loot_state_unit_guid,
        target_guid
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TrapTargetActivated {
                gameobject_guid: environmental_trap_guid,
                target_guid: player_guid,
            },
            RepresentedGameObjectUseEffect::TrapTargetActivated {
                gameobject_guid: owned_trap_guid,
                target_guid,
            },
        ]
    );

    session.process_pending().await;

    let environmental_state = session
        .represented_gameobject_use_states
        .get(&environmental_trap_guid)
        .unwrap();
    assert_eq!(
        environmental_state.loot_state,
        Some(wow_entities::LootState::Ready)
    );
    assert!(
        environmental_state
            .cooldown_until
            .is_some_and(|cooldown_until| {
                cooldown_until > Instant::now() + Duration::from_secs(8)
            })
    );

    let owned_state = session
        .represented_gameobject_use_states
        .get(&owned_trap_guid)
        .unwrap();
    assert_eq!(
        owned_state.loot_state,
        Some(wow_entities::LootState::JustDeactivated)
    );
    assert!(owned_state.cooldown_until.is_some_and(|cooldown_until| {
        cooldown_until > Instant::now() + Duration::from_secs(3)
    }));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::TrapTargetActivated {
                gameobject_guid: environmental_trap_guid,
                target_guid: player_guid,
            },
            RepresentedGameObjectUseEffect::TrapTargetActivated {
                gameobject_guid: owned_trap_guid,
                target_guid,
            },
            RepresentedGameObjectUseEffect::TrapTargetSpellCast {
                gameobject_guid: environmental_trap_guid,
                target_guid: player_guid,
                spell_id: 333,
                original_caster_guid: ObjectGuid::EMPTY,
            },
            RepresentedGameObjectUseEffect::TrapTargetSpellCast {
                gameobject_guid: owned_trap_guid,
                target_guid,
                spell_id: 444,
                original_caster_guid: owner_guid,
            },
        ]
    );
}
#[tokio::test]
async fn process_pending_ticks_fishing_bobber_ready_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let player_guid = ObjectGuid::create_player(1, 99);
    let ready_bobber_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 81);
    let waiting_bobber_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 82);
    for (guid, ready_at) in [
        (ready_bobber_guid, Instant::now() - Duration::from_secs(1)),
        (waiting_bobber_guid, Instant::now() + Duration::from_secs(5)),
    ] {
        let state = session
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_FISHING_NODE as u8);
        state.loot_state = Some(wow_entities::LootState::NotReady);
        state.owner_guid = Some(player_guid);
        state.fishing_bobber_ready_at = Some(ready_at);
    }

    session.process_pending().await;

    let ready_state = session
        .represented_gameobject_use_states
        .get(&ready_bobber_guid)
        .unwrap();
    assert_eq!(ready_state.loot_state, Some(wow_entities::LootState::Ready));
    assert_eq!(ready_state.fishing_bobber_ready_at, None);

    let waiting_state = session
        .represented_gameobject_use_states
        .get(&waiting_bobber_guid)
        .unwrap();
    assert_eq!(
        waiting_state.loot_state,
        Some(wow_entities::LootState::NotReady)
    );
    assert!(waiting_state.fishing_bobber_ready_at.is_some());
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::FishingBobberReady {
            gameobject_guid: ready_bobber_guid,
            owner_guid: player_guid,
        }]
    );
}
#[tokio::test]
async fn process_pending_ticks_capture_point_assault_timer_like_cpp_update() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 434);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.capture_point_state = Some(RepresentedCapturePointStateLikeCpp::ContestedHorde);
        state.capture_point_source = Some(wow_entities::CapturePointUseSource {
            capture_broadcast_horde: 55,
            capture_event_horde: 66,
            world_state_id: 77,
            spell_visual_ids: [1, 2, 3, 4, 5],
            ..Default::default()
        });
        state.capture_point_assault_until = Some(Instant::now() - Duration::from_secs(1));
    }

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(
        state.capture_point_state,
        Some(RepresentedCapturePointStateLikeCpp::HordeCaptured)
    );
    assert_eq!(state.capture_point_last_team_capture, Team::Horde);
    assert_eq!(state.capture_point_assault_until, None);
    assert!(
        session
            .represented_gameobject_use_effects
            .iter()
            .any(|effect| matches!(
                effect,
                RepresentedGameObjectUseEffect::CapturePointUpdated {
                    gameobject_guid: updated_guid,
                    state: RepresentedCapturePointStateLikeCpp::HordeCaptured,
                    broadcast_text_id: 55,
                    event_id: 66,
                    world_state_id: 77,
                    spell_visual_id: 4,
                    custom_anim: 3,
                    assault_timer_ms: 0,
                } if *updated_guid == gameobject_guid
            ))
    );
}
#[tokio::test]
async fn process_pending_ticks_guardpost_charges_like_cpp_get_charges() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_state(SessionState::LoggedIn);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 212);
    {
        let state = session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_GUARDPOST as u8);
        state.use_count = 2;
        state.max_charges = Some(2);
    }

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.use_count, 0);
    assert_eq!(
        state.loot_state,
        Some(wow_entities::LootState::JustDeactivated)
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::GameObjectChargesDepleted {
            gameobject_guid,
            max_charges: 2,
            loot_state: wow_entities::LootState::JustDeactivated,
        }]
    );
}
#[tokio::test]
async fn process_pending_ticks_goober_autoclose_then_cleanup_like_cpp_update() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let same_map_guid = ObjectGuid::create_player(1, 77);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 17);
    let source = wow_entities::GooberUseSource {
        lock_id: 12,
        auto_close_ms: 3_000,
        ..Default::default()
    };

    session.set_state(SessionState::LoggedIn);
    assert!(session.use_represented_gameobject_goober_state_like_cpp(
        gameobject_guid,
        player_guid,
        777,
        source,
    ));
    {
        let state = session
            .represented_gameobject_use_states
            .get_mut(&gameobject_guid)
            .unwrap();
        state.go_type = Some(wow_entities::GAMEOBJECT_TYPE_GOOBER as u8);
        state.cooldown_until = Some(Instant::now() - Duration::from_millis(1));
    }
    session.represented_gameobject_use_effects.clear();

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(
        state.loot_state,
        Some(wow_entities::LootState::JustDeactivated)
    );
    assert_eq!(state.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 0);
    assert!(state.cooldown_until.is_none());
    assert_eq!(state.goober_use_source, Some(source));
    assert!(session.represented_gameobject_use_effects.is_empty());

    let (same_command_tx, same_command_rx) = flume::bounded(2);
    let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
    let player_registry = Arc::new(PlayerRegistry::default());
    let mut same_info = broadcast_info(same_map_guid, same_send_tx);
    same_info.placement.map_id = 571;
    same_info.command_tx = same_command_tx;
    player_registry.register_or_replace(same_map_guid, same_info, Default::default());
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::ZERO);
    session.set_player_registry(player_registry);

    session.process_pending().await;

    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Ready));
    assert_eq!(state.go_state, Some(wow_entities::GoState::Ready));
    assert_eq!(state.goober_use_source, None);
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::GooberCleared {
            gameobject_guid,
            loot_state: wow_entities::LootState::Ready,
            go_state: Some(wow_entities::GoState::Ready),
        }]
    );
    let command = match same_command_rx.try_recv() {
        Ok(SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command)) => command,
        other => panic!("expected final goober state sync command, got {other:?}"),
    };
    assert_eq!(command.gameobject_guid, gameobject_guid);
    assert_eq!(
        command.loot_state,
        Some(wow_entities::LootState::Ready as u8)
    );
    assert_eq!(command.go_state, Some(wow_entities::GoState::Ready as i8));
    assert_eq!(command.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 0);
}
/// A poisoned legacy lock must not hand the tick back to the session while the
/// global loop still believes it owns it.
///
/// The two readers used to disagree: the session read the owner as
/// `mm.read().ok()` and fell back to `Session` on poison, while every tick body
/// reads through the poison with `unwrap_or_else(|p| p.into_inner())`. Under a
/// poisoned lock that told both of them they owned the creature tick, and it
/// resolved twice (#28).
#[test]
fn poisoned_legacy_lock_does_not_hand_the_tick_back_to_the_session_like_cpp() {
    use crate::map_manager::{RuntimeTickOwner, shared_runtime_tick_owner_like_cpp};

    let manager = shared_map_manager();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);

    // Poison the lock the way a panicking tick would.
    let poisoning = Arc::clone(&manager);
    let _ = std::thread::spawn(move || {
        let _guard = poisoning.write().unwrap();
        panic!("poison the legacy map manager");
    })
    .join();
    assert!(manager.is_poisoned(), "the fixture must actually poison it");

    // What every tick body sees, reading through the poison.
    let tick_body_owner = manager
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .tick_owner();
    assert_eq!(tick_body_owner, RuntimeTickOwner::GlobalLegacy);

    // What the shared reader — and therefore the session — sees. Before #28
    // this returned `Session`, so both sides ticked the same creature.
    assert_eq!(
        shared_runtime_tick_owner_like_cpp(&manager),
        tick_body_owner,
        "both readers must agree about who owns the tick, poisoned or not"
    );
}
#[test]
fn runtime_tick_owner_default_is_session() {
    use crate::map_manager::RuntimeTickOwner;
    let manager = shared_map_manager();
    let owner = manager.read().unwrap().tick_owner();
    assert_eq!(owner, RuntimeTickOwner::Session);
}
#[test]
fn legacy_turret_ai_raw_max_attempt_resets_swing_before_min_range_failure_like_cpp() {
    use crate::map_manager::RuntimeTickOwner;

    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let (mut session, _, _) = make_session();
    let creature_guid = test_creature_guid(91_302);
    let victim_guid = ObjectGuid::create_player(1, 91_303);
    let spell_id = 70_101_i32;
    add_canonical_creature_spell_test_pair_like_cpp(&canonical, creature_guid, victim_guid);
    register_test_creature_mirrored_like_cpp(
        &mut session,
        manager.clone(),
        &canonical,
        creature_guid,
        25,
    );
    session
        .mutate_world_creature(creature_guid, |creature| {
            creature
                .creature
                .set_ai_identity_names_runtime_like_cpp("TurretAI", String::new());
            creature
                .creature
                .set_spell(0, u32::try_from(spell_id).unwrap());
            creature.enter_combat(victim_guid);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
            assert!(creature.can_swing());
        })
        .unwrap();
    manager
        .write()
        .unwrap()
        .set_tick_owner(RuntimeTickOwner::GlobalLegacy);
    let mut config = creature_ai_spell_test_config_like_cpp(
        creature_ai_test_spell_info_like_cpp(spell_id, 6, 0),
        false,
        30.0,
    );
    config.spell_range_store = Some(Arc::new(wow_data::SpellRangeStore::from_entries([
        spell_range_entry_like_cpp(71, 5.0, 30.0),
    ])));

    let rejected =
        run_legacy_creature_spell_tick_once_like_cpp(&manager, Some(&canonical), &config);

    assert_eq!(rejected.casts_ready, 0, "spell tick outcome: {rejected:?}");
    assert_eq!(rejected.spell_range_rejections, 1);
    assert_eq!(rejected.canonical_cast_preconditions_passed, 0);
    assert!(rejected.plan.events.is_empty());
    assert!(
        !session
            .mutate_world_creature(creature_guid, |creature| creature.can_swing())
            .unwrap(),
        "TurretAI must reset BASE_ATTACK after its raw-max CastSpell attempt"
    );
}
#[test]
fn run_tick_returns_output_without_sending() {
    // run_creatures_tick and run_combat_tick must return a RuntimeOutput
    // containing the packets but NOT send anything to the channel until
    // flush_runtime_output is called. This catches any residual direct send.
    let manager = shared_map_manager();

    // ── creatures tick ─────────────────────────────────────────────────
    let (mut session_c, _, recv_c) = make_session();
    session_c.set_mmap_runtime_config_like_cpp(MMapRuntimeConfigLikeCpp {
        enabled: false,
        ..Default::default()
    });
    let guid_c = test_creature_guid(90_005);
    register_test_creature(&mut session_c, manager.clone(), guid_c, 25);
    session_c.client_visible_guids_like_cpp.insert(guid_c);
    session_c
        .mutate_world_creature(guid_c, |creature| {
            creature
                .creature
                .set_default_movement_type_runtime_like_cpp(
                    wow_entities::MovementGeneratorType::Random,
                );
            let ai = creature.creature.ai_ownership_mut();
            ai.wander_delay_ms = 0;
            ai.move_start_ms = 0;
            ai.wander_radius = 3.0;
            creature.seed_runtime_rng_like_cpp(0x9005);
        })
        .unwrap();

    let output_c = (0..32)
        .find_map(|_| {
            let output = session_c.run_creatures_tick();
            assert!(
                recv_c.try_recv().is_err(),
                "run_creatures_tick must not send before flush"
            );
            (!output.packets.is_empty()).then_some(output)
        })
        .expect("run_creatures_tick must eventually return at least one packet");
    // Flush and confirm the channel is now populated.
    session_c.flush_runtime_output(output_c);
    assert!(
        recv_c.try_recv().is_ok(),
        "channel must have packets after flush"
    );

    // ── combat tick ───────────────────────────────────────────────────
    let (mut session_m, _, recv_m) = make_session();
    let guid_m = test_creature_guid(90_006);
    let player_m = ObjectGuid::create_player(1, 90_006);
    session_m.player_guid = Some(player_m);
    session_m.combat_target = Some(guid_m);
    session_m.in_combat = true;
    session_m.client_visible_guids_like_cpp.insert(guid_m);
    register_test_creature(&mut session_m, manager.clone(), guid_m, 40);
    session_m
        .mutate_world_creature(guid_m, |creature| {
            creature.enter_combat(player_m);
            creature.creature.ai_ownership_mut().last_swing_ms = 0;
            creature.creature.ai_ownership_mut().swing_timer_ms = 0;
        })
        .unwrap();

    let output_m = session_m.run_combat_tick();
    assert!(
        recv_m.try_recv().is_err(),
        "run_combat_tick must not send before flush"
    );
    assert!(
        !output_m.packets.is_empty(),
        "run_combat_tick must return at least one packet"
    );
    session_m.flush_runtime_output(output_m);
    assert!(
        recv_m.try_recv().is_ok(),
        "channel must have packets after flush"
    );
}
