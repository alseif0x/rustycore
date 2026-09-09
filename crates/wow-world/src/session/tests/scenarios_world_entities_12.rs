//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn player_attack_creature_reputation_without_at_war_is_rejected_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 153);
    let victim = test_creature_guid(18_110);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_faction_store(Arc::new(
        wow_data::progression_rewards::FactionStore::from_entries([
            wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, 1),
        ]),
    ));
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            wow_data::progression_rewards::FactionTemplateEntry {
                id: 14,
                faction: 72,
                flags: 0,
                faction_group: 0,
                friend_group: 0,
                enemy_group: 0,
                enemies: [0; 8],
                friend: [0; 8],
            },
        ]),
    ));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_by_guid_like_cpp(player, |player| {
            player
                .gameplay_state_mut()
                .reputations
                .push(wow_entities::PlayerReputationRecord {
                    faction_id: 72,
                    standing: 0,
                    flags: 0,
                    ..Default::default()
                });
        })
        .unwrap();
    register_test_creature(&mut session, manager, victim, 40);

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    assert_eq!(
        map.get_typed_player(player).unwrap().unit().attacking(),
        None
    );
    assert!(
        !map.with_creature_like_cpp(victim, Clone::clone)
            .unwrap()
            .unit()
            .has_attacker_like_cpp(player)
    );
    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
}
#[test]
fn player_attack_creature_reputation_at_war_is_accepted_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 154);
    let victim = test_creature_guid(18_111);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_faction_store(Arc::new(
        wow_data::progression_rewards::FactionStore::from_entries([
            wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, 1),
        ]),
    ));
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            wow_data::progression_rewards::FactionTemplateEntry {
                id: 14,
                faction: 72,
                flags: 0,
                faction_group: 0,
                friend_group: 0,
                enemy_group: 0,
                enemies: [0; 8],
                friend: [0; 8],
            },
        ]),
    ));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_by_guid_like_cpp(player, |player| {
            player
                .gameplay_state_mut()
                .reputations
                .push(wow_entities::PlayerReputationRecord {
                    faction_id: 72,
                    standing: 0,
                    flags: wow_entities::REPUTATION_FLAG_AT_WAR_LIKE_CPP,
                    ..Default::default()
                });
        })
        .unwrap();
    register_test_creature(&mut session, manager, victim, 40);

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    assert_eq!(
        map.get_typed_player(player).unwrap().unit().attacking(),
        Some(victim)
    );
    assert!(
        map.with_creature_like_cpp(victim, Clone::clone)
            .unwrap()
            .unit()
            .has_attacker_like_cpp(player)
    );
    drop(guard);
    assert_eq!(
        session.resolved_combat_target_like_cpp(),
        Some(Some(victim))
    );
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(true));
}
#[test]
fn player_attack_creature_reputation_uses_faction_template_store_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 155);
    let victim = test_creature_guid(18_112);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            wow_data::progression_rewards::FactionTemplateEntry {
                id: 14,
                faction: 72,
                flags:
                    wow_data::progression_rewards::FACTION_TEMPLATE_FLAG_CONTESTED_GUARD_LIKE_CPP,
                faction_group: 0,
                friend_group: 0,
                enemy_group: 0,
                enemies: [0; 8],
                friend: [0; 8],
            },
        ]),
    ));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    session
        .mutate_canonical_player_by_guid_like_cpp(player, |player| {
            player.set_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
        })
        .unwrap();
    register_test_creature(&mut session, manager, victim, 40);

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    assert_eq!(
        map.get_typed_player(player).unwrap().unit().attacking(),
        Some(victim)
    );
    assert!(
        map.with_creature_like_cpp(victim, Clone::clone)
            .unwrap()
            .unit()
            .has_attacker_like_cpp(player)
    );
    drop(guard);
    assert_eq!(
        session.resolved_combat_target_like_cpp(),
        Some(Some(victim))
    );
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(true));
}
#[test]
fn player_attack_creature_non_reputation_faction_does_not_require_at_war_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 156);
    let victim = test_creature_guid(18_113);

    canonical.lock().unwrap().create_world_map(0, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_faction_store(Arc::new(
        wow_data::progression_rewards::FactionStore::from_entries([
            wow_data::progression_rewards::FactionEntry::for_test_like_cpp(72, -1),
        ]),
    ));
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            wow_data::progression_rewards::FactionTemplateEntry {
                id: 14,
                faction: 72,
                flags: 0,
                faction_group: 0,
                friend_group: 0,
                enemy_group: 0,
                enemies: [0; 8],
                friend: [0; 8],
            },
        ]),
    ));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 0,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        0,
        1,
        1,
        80,
        0,
    ));
    let _ = session.ensure_canonical_world_map_for_current_player_like_cpp();
    register_test_creature(&mut session, manager, victim, 40);

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(0, 0).unwrap().map();
    assert_eq!(
        map.get_typed_player(player).unwrap().unit().attacking(),
        Some(victim)
    );
    assert!(
        map.with_creature_like_cpp(victim, Clone::clone)
            .unwrap()
            .unit()
            .has_attacker_like_cpp(player)
    );
    drop(guard);
    assert_eq!(
        session.resolved_combat_target_like_cpp(),
        Some(Some(victim))
    );
    assert_eq!(session.resolved_in_combat_like_cpp(), Some(true));
}
#[test]
fn player_attack_evading_typed_creature_is_rejected_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let player = ObjectGuid::create_player(1, 54);
    let victim = test_creature_guid(18_010);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_map_manager(manager);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_COMMON,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player,
        "Warrior".to_string(),
        Position::new(10.0, 20.0, 30.0, 0.0),
        571,
        1,
        1,
        80,
        0,
    ));
    session.register_world_creature(
        571,
        Position::new(11.0, 20.0, 30.0, 0.0),
        test_creature_create_data(victim, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        None,
        0,
        0,
        0,
        0,
        -1,
    );
    session
        .mutate_world_creature(victim, |creature| {
            creature.creature.set_in_evade_mode_like_cpp(true);
        })
        .unwrap();

    session.start_player_attack_like_cpp(victim);

    let guard = canonical.lock().unwrap();
    let map = guard.find_map(571, 0).unwrap().map();
    assert_eq!(
        map.get_typed_player(player).unwrap().unit().attacking(),
        None
    );
    assert!(
        !map.with_creature_like_cpp(victim, Clone::clone)
            .unwrap()
            .unit()
            .has_attacker_like_cpp(player)
    );
    assert_eq!(session.combat_target, None);
    assert!(!session.in_combat);
}
#[test]
fn corpse_despawn_syncs_canonical_corpse_timer() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(18_005);
    register_test_creature(&mut session, manager.clone(), guid, 40);
    let despawn_at = Instant::now() + std::time::Duration::from_secs(30);

    session
        .mutate_world_creature(guid, |creature| {
            creature.take_damage(40);
            creature.set_corpse_despawn_at(Some(despawn_at));
        })
        .unwrap();

    let manager = manager.read().unwrap();
    let world_creature = manager.find_creature(0, 0, guid).unwrap();
    assert!(world_creature.corpse_despawn_at().is_some());
    assert_eq!(world_creature.current_hp(), 0);
}
#[tokio::test]
async fn spell_effect_grant_battle_pet_experience_requires_creature_target_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 220);
    let pet_guid = ObjectGuid::create_global(HighGuid::BattlePet, 0, 0x1A0);
    let creature_guid = ObjectGuid::create_global(HighGuid::Creature, 0, 0xCAFE);
    let spell_id = 77_286;

    session.set_player_guid(Some(player_guid));
    install_represented_battle_pet_stat_stores_like_cpp(&mut session);
    session.set_represented_battle_pet_xp_per_level_like_cpp(23, 100);
    session.add_represented_battle_pet_packet_info_like_cpp(
        pet_guid,
        RepresentedBattlePetDataLikeCpp {
            species: 11,
            breed: 7,
            level: 23,
            exp: 20,
            power: 10,
            health: 50,
            max_health: 100,
            speed: 20,
            quality: 3,
            save_info: RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            ..RepresentedBattlePetDataLikeCpp::minimal_like_cpp(
                0,
                RepresentedBattlePetSaveInfoLikeCpp::Unchanged,
            )
        },
    );
    session.send_battle_pet_journal_lock_status_like_cpp().await;
    let _ = drain_server_packet_bytes(&send_rx);

    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        spell_id,
        wow_data::SpellInfo {
            spell_id,
            cast_time_ms: 0,
            cooldown_ms: 0,
            recovery_time_ms: 0,
            effect_type: 0,
            effect_base_points: 0,
            effect_bonus_coefficient: 0.0,
            aura_type: None,
            display_flags: 0,
            requires_spell_focus: 0,
            power_costs: Vec::new(),
            effects: vec![wow_data::SpellEffectInfo {
                effect_index: 0,
                effect:
                    wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE,
                effect_base_points: 150,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            creature_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual::default(),
            SpellTargetData {
                flags: 0x2,
                unit: creature_guid,
                ..SpellTargetData::default()
            },
        )
        .await
        .expect("represented spell should execute as a no-op without unitTarget creature metadata");

    let pet = session
        .represented_battle_pet_like_cpp(pet_guid)
        .expect("unchanged pet");
    assert_eq!(pet.level, 23);
    assert_eq!(pet.exp, 20);
    assert!(
        session
            .represented_battle_pet_level_criteria_like_cpp()
            .is_empty()
    );
    assert_eq!(
        drain_server_opcodes(&send_rx),
        vec![ServerOpcodes::CastFailed]
    );
}
#[test]
fn far_sight_enable_gameobject_viewpoint_keeps_previous_seer_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 4256);
    let previous_seer = test_creature_guid(4257);
    let gameobject_guid = test_gameobject_guid(9002, 4258);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_player_guid(Some(player_guid));
    session.player_name = Some("FarSightGameObject".into());
    session.player_position = Some(Position::new(10.0, 10.0, 0.0, 0.0));
    session.current_map_id = 571;
    session.represented_seer_guid_like_cpp = Some(previous_seer);
    insert_session_player_into_canonical_map_like_cpp(&session, &canonical, 571, 0);
    add_canonical_test_gameobject(
        &canonical,
        gameobject_guid,
        9002,
        Position::new(11.0, 10.0, 0.0, 0.0),
    );
    set_canonical_player_farsight_object_like_cpp(&canonical, player_guid, gameobject_guid);

    session.apply_far_sight_like_cpp(true);

    assert_eq!(
        session.represented_seer_guid_like_cpp(),
        Some(previous_seer)
    );
    assert_eq!(
        canonical
            .lock()
            .unwrap()
            .find_map(571, 0)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap()
            .active_data()
            .farsight_object,
        gameobject_guid
    );
}
#[test]
fn gameobject_use_mover_guard_matches_cpp_remote_control_branch() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let controlled_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 571, 0, 123, 4);

    session.set_player_guid(Some(player_guid));
    assert!(session.represented_gameobject_use_allowed_by_mover_like_cpp(false));

    session.set_player_moved_unit_guid_like_cpp(controlled_guid);
    assert!(!session.represented_gameobject_use_allowed_by_mover_like_cpp(false));
    assert!(session.represented_gameobject_use_allowed_by_mover_like_cpp(true));

    session.set_player_mounted_like_cpp(true);
    assert!(session.represented_gameobject_use_allowed_by_mover_like_cpp(false));

    session.set_player_mounted_like_cpp(false);
    session.player_vehicle_seat_flags_like_cpp = Some(0);
    assert!(session.represented_gameobject_use_allowed_by_mover_like_cpp(false));
}
#[test]
fn creature_aggro_radius_uses_faction_template_neutral_to_all_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    session.set_faction_template_store(Arc::new(
        wow_data::progression_rewards::FactionTemplateStore::from_entries([
            faction_template_entry(10, 0, 0, 0, 0),
            faction_template_entry(11, 72, 0, 0, 0),
            faction_template_entry(12, 930, 0, 0, 72),
        ]),
    ));

    assert_eq!(
        session.creature_aggro_radius_for_faction_template_like_cpp(10, 15.0),
        0.0,
        "C++ WorldObject::IsNeutralToAll returns true when FactionTemplate::Faction is 0"
    );
    assert_eq!(
        session.creature_aggro_radius_for_faction_template_like_cpp(11, 15.0),
        0.0,
        "C++ falls through to FactionTemplate::IsNeutralToAll when raw Faction has no reputation row"
    );
    assert_eq!(
        session.creature_aggro_radius_for_faction_template_like_cpp(12, 15.0),
        15.0,
        "enemy relations are not neutral-to-all and must keep normal aggro"
    );

    session.set_faction_store(Arc::new(FactionStore::from_entries([
        FactionEntry::for_test_like_cpp(72, 1),
    ])));
    assert_eq!(
        session.creature_aggro_radius_for_faction_template_like_cpp(11, 15.0),
        15.0,
        "C++ treats faction rows with a reputation index as non-neutral"
    );
}
#[test]
fn gameobject_use_preamble_matches_cpp_player_branch() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 4);

    session.set_player_guid(Some(player_guid));
    session.set_player_mounted_like_cpp(true);
    assert!(
        session.apply_represented_gameobject_player_use_preamble_like_cpp(
            gameobject_guid,
            player_guid,
            false,
            false,
        )
    );
    assert!(!session.player_mounted_like_cpp);
    assert!(
        !session
            .player_unit_flags_like_cpp
            .contains(UnitFlags::MOUNT)
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::RemoveMountedAuras {
                gameobject_guid,
                player_guid,
            },
            RepresentedGameObjectUseEffect::ClearPlayerTalkMenus {
                gameobject_guid,
                player_guid,
            },
            RepresentedGameObjectUseEffect::GossipHelloAi {
                gameobject_guid,
                player_guid,
                handled: false,
            },
        ]
    );

    session.represented_gameobject_use_effects.clear();
    session
        .represented_gameobject_use_states
        .entry(gameobject_guid)
        .or_default()
        .gossip_hello_ai_returns_true = true;
    assert!(
        !session.apply_represented_gameobject_player_use_preamble_like_cpp(
            gameobject_guid,
            player_guid,
            true,
            false,
        )
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::ClearPlayerTalkMenus {
                gameobject_guid,
                player_guid,
            },
            RepresentedGameObjectUseEffect::GossipHelloAi {
                gameobject_guid,
                player_guid,
                handled: true,
            },
        ]
    );
}
#[test]
fn gameobject_use_preamble_rejects_damage_immune_player_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 5);

    session.set_player_guid(Some(player_guid));
    session.player_unit_flags_like_cpp.insert(UnitFlags::IMMUNE);
    assert!(
        !session.apply_represented_gameobject_player_use_preamble_like_cpp(
            gameobject_guid,
            player_guid,
            false,
            true,
        )
    );
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::UseRejectedNoDamageImmune {
            gameobject_guid,
            player_guid,
        },]
    );
}
#[test]
fn gameobject_use_cooldown_matches_cpp_template_gate() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 6);

    assert!(session.apply_represented_gameobject_cooldown_like_cpp(gameobject_guid, 0));
    assert!(session.represented_gameobject_use_effects.is_empty());

    assert!(session.apply_represented_gameobject_cooldown_like_cpp(gameobject_guid, 5));
    assert!(!session.apply_represented_gameobject_cooldown_like_cpp(gameobject_guid, 5));
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::CooldownStarted {
                gameobject_guid,
                cooldown_secs: 5,
            },
            RepresentedGameObjectUseEffect::CooldownRejected { gameobject_guid },
        ]
    );

    session
        .represented_gameobject_use_states
        .get_mut(&gameobject_guid)
        .unwrap()
        .cooldown_until = Some(Instant::now() - Duration::from_secs(1));
    assert!(session.apply_represented_gameobject_cooldown_like_cpp(gameobject_guid, 5));
}
#[test]
fn gameobject_use_door_or_button_matches_cpp_ready_gate_and_state_switch() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 7);

    assert!(session.use_represented_gameobject_door_or_button_like_cpp(
        gameobject_guid,
        player_guid,
        3000,
    ));
    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.go_state, Some(wow_entities::GoState::Active));
    assert_eq!(state.prev_go_state, Some(wow_entities::GoState::Ready));
    assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
    assert_eq!(state.loot_state_unit_guid, player_guid);
    assert_eq!(
        state.gameobject_flags & wow_entities::GO_FLAG_IN_USE,
        wow_entities::GO_FLAG_IN_USE
    );
    assert!(state.cooldown_until.is_some());
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![RepresentedGameObjectUseEffect::DoorOrButtonUsed {
            gameobject_guid,
            user_guid: player_guid,
            restore_time_ms: 3000,
            go_state: wow_entities::GoState::Active,
        }]
    );

    assert!(!session.use_represented_gameobject_door_or_button_like_cpp(
        gameobject_guid,
        player_guid,
        3000,
    ));
    assert_eq!(
        session.represented_gameobject_use_effects.last(),
        Some(&RepresentedGameObjectUseEffect::DoorOrButtonRejectedNotReady { gameobject_guid })
    );
}
#[test]
fn gameobject_door_or_button_tick_resets_after_cooldown_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 72);

    assert!(session.use_represented_gameobject_door_or_button_like_cpp(
        gameobject_guid,
        player_guid,
        3000,
    ));
    assert!(!session.tick_represented_gameobject_door_or_button_like_cpp(gameobject_guid));
    session
        .represented_gameobject_use_states
        .get_mut(&gameobject_guid)
        .unwrap()
        .cooldown_until = Some(Instant::now() - Duration::from_millis(1));

    assert!(session.tick_represented_gameobject_door_or_button_like_cpp(gameobject_guid));
    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(state.go_state, Some(wow_entities::GoState::Ready));
    assert_eq!(
        state.loot_state,
        Some(wow_entities::LootState::JustDeactivated)
    );
    assert_eq!(state.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 0);
    assert!(state.cooldown_until.is_none());
    assert_eq!(
        session.represented_gameobject_use_effects.last(),
        Some(&RepresentedGameObjectUseEffect::DoorOrButtonReset {
            gameobject_guid,
            go_state: wow_entities::GoState::Ready,
        })
    );
}
#[test]
fn gameobject_use_trap_matches_cpp_spell_cooldown_and_charges() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 99);
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 8);

    assert!(session.use_represented_gameobject_trap_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::TrapUseSource {
            spell_id: 1234,
            charges: 1,
            cooldown_secs: 0,
            ..Default::default()
        },
    ));
    let state = session
        .represented_gameobject_use_states
        .get(&gameobject_guid)
        .unwrap();
    assert_eq!(
        state.loot_state,
        Some(wow_entities::LootState::JustDeactivated)
    );
    assert!(state.cooldown_until.is_some());
    assert_eq!(
        session.represented_gameobject_use_effects,
        vec![
            RepresentedGameObjectUseEffect::CastSpell {
                gameobject_guid,
                player_guid,
                spell_id: 1234,
            },
            RepresentedGameObjectUseEffect::CooldownStarted {
                gameobject_guid,
                cooldown_secs: 4,
            },
        ]
    );

    assert!(!session.use_represented_gameobject_trap_like_cpp(
        gameobject_guid,
        player_guid,
        wow_entities::TrapUseSource {
            spell_id: 1234,
            charges: 1,
            cooldown_secs: 9,
            ..Default::default()
        },
    ));
    assert_eq!(
        session.represented_gameobject_use_effects.last(),
        Some(&RepresentedGameObjectUseEffect::CooldownRejected { gameobject_guid })
    );
}
