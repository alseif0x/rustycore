//! Session scenarios exercising the represented world entities responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn creature_kill_reputation_applies_generic_and_faction_auras_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let creature_guid = test_creature_guid(69_501);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    configure_single_creature_kill_reputation_for_test(&mut session);
    session.visible_auras.insert(
        1,
        reputation_aura_for_test(1, RepresentedAuraEffectLikeCpp::ModReputationGain, 20, None),
    );
    session.visible_auras.insert(
        2,
        reputation_aura_for_test(
            2,
            RepresentedAuraEffectLikeCpp::ModFactionReputationGain,
            30,
            Some(7),
        ),
    );
    session.visible_auras.insert(
        3,
        reputation_aura_for_test(
            3,
            RepresentedAuraEffectLikeCpp::ModFactionReputationGain,
            200,
            Some(8),
        ),
    );

    session.reward_reputation_from_creature_kill_like_cpp(9001, creature_guid, 80, 1.0);

    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(5)
            .unwrap()
            .standing,
        375
    );
    assert_eq!(
        session
            .represented_creature_kill_events_like_cpp()
            .iter()
            .filter(|event| matches!(
                event,
                RepresentedCreatureKillEventLikeCpp::CreatureKillReputationAwarded {
                    creature_guid: guid,
                    faction_id: 7,
                    reputation: 375,
                    spillover_only: false,
                } if *guid == creature_guid
            ))
            .count(),
        1
    );
}
#[test]
fn creature_kill_reputation_uses_championing_faction_in_wrath_non_raid_dungeon_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let creature_guid = test_creature_guid(69301);
    session.set_loaded_player_identity_like_cpp(571, 1, 1, 80, 0);
    session.set_championing_faction_like_cpp(99);
    session.set_map_store(Arc::new(wow_data::MapStore::from_entries([
        wow_data::MapEntry {
            id: 571,
            instance_type: wow_data::map::MAP_INSTANCE,
            expansion_id: 0,
            parent_map_id: -1,
            cosmetic_parent_map_id: -1,
            flags1: 0,
            flags2: 0,
        },
    ])));
    session.set_lfg_dungeons_store(Arc::new(wow_data::LfgDungeonsStore::from_entries([
        lfg_dungeon_entry_for_test(1, 571, 0, WRATH_OF_THE_LICH_KING_MAX_LEVEL_LIKE_CPP),
    ])));

    let mut table_faction = FactionEntry::for_test_like_cpp(7, 5);
    table_faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let mut champion_faction = FactionEntry::for_test_like_cpp(99, 9);
    champion_faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let faction_store = FactionStore::from_entries([table_faction, champion_faction]);
    let creature_template_store = creature_template_lifecycle_store_for_test([9001]);
    let (onkill_store, report) =
        wow_data::reputation::CreatureOnKillReputationStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::CreatureOnKillReputationRowLikeCpp {
                creature_id: 9001,
                entry: wow_data::reputation::CreatureOnKillReputationEntryLikeCpp {
                    rep_faction_1: 7,
                    rep_faction_2: 0,
                    reputation_max_cap_1: wow_data::reputation::ReputationRankLikeCpp::Exalted
                        .as_u8(),
                    rep_value_1: 250,
                    reputation_max_cap_2: 0,
                    rep_value_2: 0,
                    is_team_award_1: false,
                    is_team_award_2: false,
                    team_dependent: false,
                },
            }],
            &creature_template_store,
            &faction_store,
        );
    assert_eq!(report.loaded, 1);

    session.set_faction_store(Arc::new(faction_store));
    session.set_creature_onkill_reputation_store(Arc::new(onkill_store));

    session.reward_reputation_from_creature_kill_like_cpp(9001, creature_guid, 80, 1.0);

    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(5)
            .unwrap()
            .standing,
        0
    );
    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(9)
            .unwrap()
            .standing,
        250
    );
    assert_eq!(
        session.represented_creature_kill_events_like_cpp(),
        &[
            RepresentedCreatureKillEventLikeCpp::CreatureKillReputationAwarded {
                creature_guid,
                faction_id: 99,
                reputation: 250,
                spillover_only: false,
            }
        ]
    );
}
#[test]
fn creature_kill_reputation_respects_team_dependent_horde_branch_like_cpp() {
    let (mut session, _pkt_tx, _send_rx) = make_session();
    let creature_guid = test_creature_guid(69202);
    session.set_loaded_player_identity_like_cpp(571, 2, 1, 80, 0);

    let mut alliance_faction = FactionEntry::for_test_like_cpp(7, 5);
    alliance_faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let mut horde_faction = FactionEntry::for_test_like_cpp(8, 6);
    horde_faction.reputation_flags[0] = ReputationFlagsLikeCpp::VISIBLE.bits();
    let faction_store = FactionStore::from_entries([alliance_faction, horde_faction]);
    let creature_template_store = creature_template_lifecycle_store_for_test([9001]);
    let (onkill_store, report) =
        wow_data::reputation::CreatureOnKillReputationStoreLikeCpp::from_rows_like_cpp(
            [wow_data::reputation::CreatureOnKillReputationRowLikeCpp {
                creature_id: 9001,
                entry: wow_data::reputation::CreatureOnKillReputationEntryLikeCpp {
                    rep_faction_1: 7,
                    rep_faction_2: 8,
                    reputation_max_cap_1: wow_data::reputation::ReputationRankLikeCpp::Exalted
                        .as_u8(),
                    rep_value_1: 250,
                    reputation_max_cap_2: wow_data::reputation::ReputationRankLikeCpp::Exalted
                        .as_u8(),
                    rep_value_2: 400,
                    is_team_award_1: false,
                    is_team_award_2: false,
                    team_dependent: true,
                },
            }],
            &creature_template_store,
            &faction_store,
        );
    assert_eq!(report.loaded, 1);

    session.set_faction_store(Arc::new(faction_store));
    session.set_creature_onkill_reputation_store(Arc::new(onkill_store));

    session.reward_reputation_from_creature_kill_like_cpp(9001, creature_guid, 80, 1.0);

    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(5)
            .unwrap()
            .standing,
        0
    );
    assert_eq!(
        session
            .reputation_mgr_like_cpp()
            .get_state(6)
            .unwrap()
            .standing,
        400
    );
}
#[test]
fn register_world_creature_mirrors_existing_canonical_map_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(611);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_map_manager(manager);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.current_map_id = 571;
    session.register_world_creature(
        571,
        Position::new(10.0, 20.0, 30.0, 1.0),
        test_creature_create_data(guid, 9001, 25),
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

    let guard = canonical.lock().unwrap();
    let creature = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_creature(guid)
        .expect("creature inserted into canonical map");
    assert_eq!(creature.guid(), guid);
    assert_eq!(creature.object().entry(), 9001);
    assert_eq!(creature.position(), Position::new(10.0, 20.0, 30.0, 1.0));
    assert!(creature.object().is_in_world());
    let typed = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .expect("creature stored as typed Creature entity");
    assert_eq!(typed.unit().world().object().entry(), 9001);
    assert_eq!(typed.current_health(), 25);
}
#[test]
fn register_world_creature_preserves_create_state_in_canonical_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(614);
    let mut create_data = test_creature_create_data(guid, 9001, 25);
    create_data.display_id = 1234;
    create_data.native_display_id = 5678;
    create_data.display_scale = 1.25;
    create_data.native_x_display_scale = 0.75;
    create_data.bounding_radius = 0.91;
    create_data.combat_reach = 2.75;
    create_data.display_power = wow_constants::unit::PowerType::Energy as u8;
    create_data.power[0] = 42;
    create_data.max_power[0] = 100;
    create_data.power[3] = 42;
    create_data.max_power[3] = 100;
    create_data.base_mana = 600;
    create_data.base_attack_time = 1_750;
    create_data.ranged_attack_time = 2_250;
    create_data.mount_display_id = 321;
    create_data.stand_state = wow_constants::unit::UnitStandStateType::Kneel as u8;
    create_data.vis_flags = 0x02;
    create_data.anim_tier = 3;
    create_data.sheathe_state = wow_constants::unit::SheathState::Ranged as u8;
    create_data.pvp_flags = wow_constants::unit::UnitPvpFlags::FFA_PVP.bits();

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_map_manager(manager);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.current_map_id = 571;
    session.register_world_creature(
        571,
        Position::new(10.0, 20.0, 30.0, 1.0),
        create_data.clone(),
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

    let guard = canonical.lock().unwrap();
    let creature = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .expect("creature inserted into canonical map");
    let reconstructed =
        crate::map_manager::WorldCreature::create_data_from_canonical_like_cpp(&creature);

    assert_eq!(reconstructed.display_id, create_data.display_id);
    assert_eq!(
        reconstructed.native_display_id,
        create_data.native_display_id
    );
    assert_eq!(reconstructed.display_scale, create_data.display_scale);
    assert_eq!(
        reconstructed.native_x_display_scale,
        create_data.native_x_display_scale
    );
    assert_eq!(reconstructed.bounding_radius, create_data.bounding_radius);
    assert_eq!(reconstructed.combat_reach, create_data.combat_reach);
    assert_eq!(reconstructed.display_power, create_data.display_power);
    assert_eq!(reconstructed.power[0], create_data.power[0]);
    assert_eq!(reconstructed.max_power[0], create_data.max_power[0]);
    assert_eq!(reconstructed.power[3], create_data.power[3]);
    assert_eq!(reconstructed.max_power[3], create_data.max_power[3]);
    assert_eq!(reconstructed.base_mana, create_data.base_mana);
    assert_eq!(
        creature.get_power_index(wow_constants::unit::PowerType::Energy),
        Some(0)
    );
    assert_eq!(
        creature
            .unit()
            .get_power(wow_constants::unit::PowerType::Energy),
        create_data.power[0]
    );
    assert_eq!(
        creature.unit().get_create_mana_like_cpp(),
        create_data.base_mana
    );
    assert_eq!(reconstructed.base_attack_time, create_data.base_attack_time);
    assert_eq!(
        reconstructed.ranged_attack_time,
        create_data.ranged_attack_time
    );
    assert_eq!(reconstructed.mount_display_id, create_data.mount_display_id);
    assert_eq!(reconstructed.stand_state, create_data.stand_state);
    assert_eq!(reconstructed.vis_flags, create_data.vis_flags);
    assert_eq!(reconstructed.anim_tier, create_data.anim_tier);
    assert_eq!(reconstructed.sheathe_state, create_data.sheathe_state);
    assert_eq!(reconstructed.pvp_flags, create_data.pvp_flags);
}
#[test]
fn register_world_creature_hydrates_db_waypoint_path_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(613);
    let path = wow_movement::WaypointPath::new(
        77_001,
        vec![wow_movement::WaypointNode::new(1, 12.0, 20.0, 30.0)],
    );

    session.set_map_manager(Arc::clone(&manager));
    session.set_waypoint_path_resolver_like_cpp(Arc::new(move |path_id| {
        (path_id == 77_001).then_some(path.clone())
    }));
    session.current_map_id = 571;
    session.register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
        571,
        Position::new(10.0, 20.0, 30.0, 1.0),
        test_creature_create_data(guid, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        wow_entities::DEFAULT_RESPAWN_DELAY_SECS,
        6,
        -1,
        "npc_db_waypoint".to_string(),
        Some("spawn-string".to_string()),
        None,
        None,
        0,
        0,
        0,
        0,
        -1,
        0,
        wow_constants::CreatureGroundMovementType::Run as u8,
        true,
        0,
        false,
        wow_constants::CreatureChaseMovementType::Run as u8,
        wow_constants::CreatureRandomMovementType::Walk as u8,
        wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        0.0,
        wow_entities::MovementGeneratorType::Waypoint,
        77_001,
    );

    let guard = manager.read().unwrap();
    let creature = guard
        .find_creature(571, 0, guid)
        .expect("creature inserted into legacy map");
    assert_eq!(
        creature.creature.waypoint_path_id_like_cpp(),
        77_001,
        "C++ Creature::LoadCreaturesAddon copies nonzero PathId into _waypointPathId"
    );
    assert_eq!(
        creature.creature.equipment_id(),
        6,
        "C++ Creature::LoadEquipment stores the selected equipment id on the creature"
    );
    assert_eq!(
        creature.creature.original_equipment_id(),
        -1,
        "C++ Creature::InitEntry preserves DB equipment_id separately from the selected equipment template"
    );
    assert_eq!(
        creature.creature.lifecycle_metadata().script_name,
        "npc_db_waypoint",
        "C++ ObjectMgr::LoadCreatures stores creature.ScriptName on CreatureData for AI selection"
    );
    assert_eq!(
        creature.creature.lifecycle_metadata().string_id.as_deref(),
        Some("spawn-string"),
        "C++ Creature::LoadFromDB copies CreatureData::StringId into m_stringIds[1]"
    );
    assert!(
        creature.active_waypoint_generator_like_cpp().is_some(),
        "C++ WaypointMovementGenerator::DoInitialize resolves owner GetWaypointPathId for DB-loaded waypoint movement"
    );
}
#[test]
fn register_world_creature_applies_addon_lifecycle_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let guid = test_creature_guid(614);
    let path = wow_movement::WaypointPath::new(
        77_002,
        vec![wow_movement::WaypointNode::new(1, 14.0, 22.0, 31.0)],
    );

    session.set_map_manager(Arc::clone(&manager));
    session.set_waypoint_path_resolver_like_cpp(Arc::new(move |path_id| {
        (path_id == 77_002).then_some(path.clone())
    }));
    session.current_map_id = 571;
    session.register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
        571,
        Position::new(10.0, 20.0, 30.0, 1.0),
        test_creature_create_data(guid, 9001, 25),
        3,
        5,
        20.0,
        0,
        0,
        0,
        0,
        wow_entities::DEFAULT_RESPAWN_DELAY_SECS,
        0,
        0,
        String::new(),
        None,
        Some(CreatureAddonLifecycleRecordLikeCpp {
            path_id: 77_002,
            visibility_distance_type: wow_entities::VisibilityDistanceTypeLikeCpp::Large,
            auras: vec![70_010],
            ..CreatureAddonLifecycleRecordLikeCpp::default()
        }),
        None,
        0,
        0,
        0,
        0,
        -1,
        0,
        wow_constants::CreatureGroundMovementType::Run as u8,
        true,
        0,
        false,
        wow_constants::CreatureChaseMovementType::Run as u8,
        wow_constants::CreatureRandomMovementType::Walk as u8,
        wow_entities::DEFAULT_CREATURE_INTERACTION_PAUSE_TIMER_MS_LIKE_CPP,
        0.0,
        wow_entities::MovementGeneratorType::Waypoint,
        0,
    );

    let guard = manager.read().unwrap();
    let creature = guard
        .find_creature(571, 0, guid)
        .expect("creature inserted into legacy map");
    assert_eq!(
        creature.creature.waypoint_path_id_like_cpp(),
        77_002,
        "C++ Creature::LoadCreaturesAddon copies addon PathId into _waypointPathId"
    );
    assert!(
        creature.active_waypoint_generator_like_cpp().is_some(),
        "C++ waypoint movement initializes from the addon-owned PathId"
    );
    assert!(
        creature
            .creature
            .unit()
            .world()
            .visibility_distance_override_like_cpp()
            .is_some(),
        "C++ Creature::LoadCreaturesAddon applies non-normal visibility overrides"
    );
    assert!(
        creature
            .creature
            .unit()
            .unit_flags2_like_cpp()
            .contains(UnitFlags2::LARGE_AOI),
        "C++ SetVisibilityDistanceOverride sets the matching AOI flag"
    );
    assert!(
        creature
            .creature
            .unit()
            .subsystems()
            .auras
            .has_aura_spell_like_cpp(70_010),
        "C++ Creature::LoadCreaturesAddon applies addon auras"
    );
}
#[test]
fn represented_gameobject_owner_syncs_to_canonical_created_by_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 64);
    let owner_guid = ObjectGuid::create_player(1, 99);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.current_map_id = 571;
    session.record_represented_gameobject_owner_guid_like_cpp(gameobject_guid, owner_guid);
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::ZERO,
        wow_entities::GAMEOBJECT_TYPE_FISHING_NODE as u8,
    );

    let guard = canonical.lock().unwrap();
    let game_object = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_game_object(gameobject_guid)
        .expect("gameobject inserted into canonical map");
    assert_eq!(game_object.owner_guid(), owner_guid);
}
#[test]
fn represented_gameobject_owner_update_mutates_existing_canonical_created_by_like_cpp() {
    let (mut session, _, _) = make_session();
    let canonical = shared_canonical_map_manager();
    let gameobject_guid =
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 571, 0, 777, 65);
    let owner_guid = ObjectGuid::create_player(1, 99);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.current_map_id = 571;
    session.record_represented_gameobject_runtime_state_like_cpp(
        571,
        gameobject_guid,
        777,
        Position::ZERO,
        wow_entities::GAMEOBJECT_TYPE_FISHING_NODE as u8,
    );
    session.record_represented_gameobject_owner_guid_like_cpp(gameobject_guid, owner_guid);

    let guard = canonical.lock().unwrap();
    let game_object = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_game_object(gameobject_guid)
        .expect("gameobject inserted into canonical map");
    assert_eq!(game_object.owner_guid(), owner_guid);
}
#[test]
fn mutate_world_creature_relocates_canonical_map_object_like_cpp() {
    let (mut session, _, _) = make_session();
    let manager = shared_map_manager();
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(612);

    canonical.lock().unwrap().create_world_map(571, 0);
    session.set_map_manager(manager);
    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.current_map_id = 571;
    session.register_world_creature(
        571,
        Position::new(10.0, 20.0, 30.0, 1.0),
        test_creature_create_data(guid, 9001, 25),
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

    let moved = Position::new(15.0, 25.0, 35.0, 2.0);
    session.mutate_world_creature(guid, |creature| {
        creature.creature.set_ai_position(moved);
    });

    let guard = canonical.lock().unwrap();
    let creature = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_creature(guid)
        .expect("creature remains in canonical map");
    assert_eq!(creature.position(), moved);
    let typed = guard
        .find_map(571, 0)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .expect("creature remains a typed Creature entity");
    assert_eq!(typed.position(), moved);
}
#[test]
fn canonical_creature_sync_helpers_use_explicit_map_and_instance_like_cpp() {
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(613);
    let player_guid = ObjectGuid::create_player(1, 613);
    let initial = Position::new(10.0, 20.0, 30.0, 1.0);
    add_canonical_test_creature_on_map(&canonical, guid, 9001, initial, 0, 609, 7);
    add_canonical_test_player_on_map(&canonical, player_guid, initial, 609, 7);
    {
        let mut manager = canonical.lock().unwrap();
        let map = manager.find_map_mut(609, 7).unwrap().map_mut();
        let threat_ref = {
            let creature = map.get_typed_creature_mut(guid).unwrap();
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .set_threat(player_guid, 25.0);
            *creature
                .unit()
                .subsystems()
                .combat
                .threat_ref(player_guid)
                .unwrap()
        };
        map.get_typed_player_mut(player_guid)
            .unwrap()
            .unit_mut()
            .subsystems_mut()
            .combat
            .put_threatened_by_me_ref(guid, threat_ref);
    }

    let relocated = Position::new(15.0, 25.0, 35.0, 2.0);
    relocate_canonical_creature_map_object_on_map_like_cpp(&canonical, 609, 7, guid, relocated);
    {
        let guard = canonical.lock().unwrap();
        let creature = guard
            .find_map(609, 7)
            .unwrap()
            .map()
            .get_creature(guid)
            .expect("creature remains in explicit map instance");
        assert_eq!(creature.position(), relocated);
    }
    let authority_before = canonical
        .lock()
        .unwrap()
        .find_map(609, 7)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .unwrap()
        .loot_authority_like_cpp()
        .clone();

    let synced = Position::new(17.0, 27.0, 37.0, 3.0);
    let mut creature = canonical
        .lock()
        .unwrap()
        .find_map(609, 7)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .unwrap()
        .clone();
    creature.unit_mut().world_mut().relocate(synced);
    creature.unit_mut().set_level(33);
    creature.unit_mut().subsystems_mut().combat.end_all_combat();

    let selected_authority =
        sync_canonical_creature_entity_on_map_like_cpp(&canonical, 609, 7, creature)
            .expect("canonical sync keeps a typed creature authority");
    assert!(authority_before.shares_storage_like_cpp(&selected_authority));

    let guard = canonical.lock().unwrap();
    let typed = guard
        .find_map(609, 7)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .expect("typed creature remains in explicit map instance");
    assert_eq!(typed.position(), synced);
    assert_eq!(typed.level(), 33);
    assert!(
        authority_before.shares_storage_like_cpp(typed.loot_authority_like_cpp()),
        "a whole-entity sync must not replace the canonical loot authority"
    );
    assert!(
        !guard
            .find_map(609, 7)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&guid),
        "syncing an evaded legacy creature must purge the canonical reverse threat ref"
    );
    drop(guard);

    let mut reengaged = canonical
        .lock()
        .unwrap()
        .find_map(609, 7)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .unwrap()
        .clone();
    reengaged
        .unit_mut()
        .subsystems_mut()
        .combat
        .add_threat(player_guid, 30.0);
    sync_canonical_creature_entity_on_map_like_cpp(&canonical, 609, 7, reengaged)
        .expect("reengaged creature sync");
    assert!(
        canonical
            .lock()
            .unwrap()
            .find_map(609, 7)
            .unwrap()
            .map()
            .get_typed_player(player_guid)
            .unwrap()
            .unit()
            .subsystems()
            .combat
            .threatened_by_me_owner_guids()
            .contains(&guid),
        "a proximity-acquired forward threat ref must create the player's inverse ref"
    );
}
#[test]
fn canonical_creature_sync_quarantines_distinct_active_loot_authorities_like_cpp() {
    let canonical = shared_canonical_map_manager();
    let guid = test_creature_guid(61_399);
    let player_guid = ObjectGuid::create_player(1, 61_398);
    let position = Position::new(1.0, 2.0, 3.0, 0.5);
    add_canonical_test_creature_on_map(&canonical, guid, 9_399, position, 0, 609, 7);

    let loot = |coins| CreatureLoot {
        loot_guid: ObjectGuid::create_world_object(
            HighGuid::LootObject,
            0,
            0,
            609,
            0,
            0,
            i64::from(coins),
        ),
        coins,
        unlooted_count: 0,
        loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
        dungeon_encounter_id: 0,
        loot_method: 0,
        loot_master: ObjectGuid::EMPTY,
        round_robin_player: ObjectGuid::EMPTY,
        player_ffa_items: Vec::new(),
        players_looting: Vec::new(),
        allowed_looters: vec![player_guid],
        items: Vec::new(),
        looted_by_player: false,
    };

    let canonical_authority = {
        let mut manager = canonical.lock().unwrap();
        let creature = manager
            .find_map_mut(609, 7)
            .unwrap()
            .map_mut()
            .get_typed_creature_mut(guid)
            .unwrap();
        creature.initialize_shared_loot_authority_like_cpp(loot(10));
        creature.loot_authority_like_cpp().clone()
    };

    let mut incoming = wow_entities::Creature::new(false);
    incoming.unit_mut().world_mut().object_mut().create(guid);
    incoming
        .unit_mut()
        .world_mut()
        .object_mut()
        .set_entry(9_399);
    incoming.unit_mut().world_mut().set_map(609, 7).unwrap();
    incoming.unit_mut().world_mut().relocate(position);
    incoming.initialize_shared_loot_authority_like_cpp(loot(20));
    let incoming_authority = incoming.loot_authority_like_cpp().clone();
    assert!(!canonical_authority.shares_storage_like_cpp(&incoming_authority));

    let selected = sync_canonical_creature_entity_on_map_like_cpp(&canonical, 609, 7, incoming)
        .expect("conflicting mirrors collapse onto a canonical tombstone");
    assert_eq!(
        canonical_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Detached,
        "the displaced canonical allocation is terminally invalidated"
    );
    assert_eq!(
        incoming_authority.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Active,
        "the incoming value is only a transport snapshot; its live legacy owner performs its own CAS"
    );
    assert_eq!(
        selected.lifecycle_like_cpp(),
        OwnedLootAuthorityLifecycle::Quarantined
    );
    assert!(!canonical_authority.shares_storage_like_cpp(&selected));
    assert!(!incoming_authority.shares_storage_like_cpp(&selected));
    let manager = canonical.lock().unwrap();
    let stored = manager
        .find_map(609, 7)
        .unwrap()
        .map()
        .with_creature_like_cpp(guid, Clone::clone)
        .unwrap();
    assert!(selected.shares_storage_like_cpp(stored.loot_authority_like_cpp()));
}
