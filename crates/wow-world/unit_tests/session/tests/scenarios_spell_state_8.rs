//! Session scenarios exercising the represented spell state responsibility.
//!
//! Split out of session_tests.rs under #626; assertions and registrations
//! are unchanged and the shared fixtures stay in the parent module.

use super::*;

#[test]
fn represented_mounted_aura_toggles_mount_flag_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    let other_guid = ObjectGuid::create_player(1, 43);
    let pet_guid =
        ObjectGuid::create_world_object(wow_core::guid::HighGuid::Pet, 0, 1, 0, 0, 500, 44);
    let registry = Arc::new(PlayerRegistry::default());
    let (other_tx, _other_rx) = flume::bounded(8);
    let (other_command_tx, other_command_rx) = flume::bounded(8);
    session.set_player_guid(Some(player_guid));
    session.set_player_registry(Arc::clone(&registry));
    session.set_player_position_like_cpp(Position::new(1.0, 2.0, 3.0, 0.5));
    session.client_visible_guids_like_cpp.insert(other_guid);
    session.set_represented_pet_mode_state_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_STAY_LIKE_CPP,
    );
    session.player_race = 1;
    session.player_gender = 0;
    registry.register_or_replace(
        player_guid,
        broadcast_info(player_guid, flume::bounded(1).0),
        Default::default(),
    );
    registry.register_or_replace(
        other_guid,
        broadcast_info_with_command(other_guid, other_tx, other_command_tx),
        Default::default(),
    );
    session.set_creature_template_mount_store(Arc::new(
        wow_data::CreatureTemplateMountStoreLikeCpp::from_entries([
            wow_data::CreatureTemplateMountEntryLikeCpp {
                entry: 1234,
                vehicle_id: 55,
                models: vec![wow_data::CreatureTemplateMountModelLikeCpp {
                    display_id: 4321,
                    display_scale: 1.0,
                    probability: 0.0,
                }],
            },
        ]),
    ));
    let native_display_id = crate::handlers::character::default_display_id(
        session.player_race_like_cpp(),
        session.player_gender_like_cpp(),
    );
    session.set_creature_display_info_store(Arc::new(
        wow_data::CreatureDisplayInfoStore::from_entries([
            wow_data::CreatureDisplayInfoEntry {
                id: native_display_id,
                model_id: 100,
                extended_display_info_id: 0,
                creature_model_scale: 1.2,
            },
            wow_data::CreatureDisplayInfoEntry {
                id: 4321,
                model_id: 200,
                extended_display_info_id: 0,
                creature_model_scale: 1.5,
            },
        ]),
    ));
    session.set_creature_model_data_store(Arc::new(
        wow_data::CreatureModelDataStore::from_entries([
            wow_data::CreatureModelDataEntry {
                id: 100,
                flags: 0,
                file_data_id: 0,
                collision_height: 2.0,
                hover_height: 0.75,
                model_scale: 1.1,
                mount_height: 0.0,
            },
            wow_data::CreatureModelDataEntry {
                id: 200,
                flags: 0,
                file_data_id: 0,
                collision_height: 0.0,
                hover_height: 1.25,
                model_scale: 1.0,
                mount_height: 4.0,
            },
        ]),
    ));
    session.set_vehicle_store(Arc::new(wow_data::VehicleStore::from_entries([
        wow_data::VehicleEntry {
            id: 55,
            flags: 0,
            flags_b: 0,
            seat_ids: [1000, 1001, 0, 0, 0, 0, 0, 0],
        },
    ])));
    session.set_vehicle_seat_store(Arc::new(wow_data::VehicleSeatStore::from_entries([
        wow_data::VehicleSeatEntry {
            id: 1000,
            attachment_offset_x: 0.0,
            attachment_offset_y: 0.0,
            attachment_offset_z: 0.0,
            flags: wow_data::VEHICLE_SEAT_FLAG_CAN_ENTER_OR_EXIT,
            flags_b: 0,
            flags_c: 0,
        },
        wow_data::VehicleSeatEntry {
            id: 1001,
            attachment_offset_x: 0.0,
            attachment_offset_y: 0.0,
            attachment_offset_z: 0.0,
            flags: 0,
            flags_b: wow_data::VEHICLE_SEAT_FLAG_B_USABLE_FORCED,
            flags_c: 0,
        },
    ])));
    session.set_vehicle_template_store(Arc::new(
        wow_data::VehicleTemplateStoreLikeCpp::from_entries([(
            1234,
            wow_entities::VehicleTemplate {
                despawn_delay_ms: 2500,
            },
        )]),
    ));
    session.set_vehicle_accessory_store(Arc::new(
        wow_data::VehicleAccessoryStoreLikeCpp::from_parts(
            std::iter::empty::<(u64, Vec<wow_entities::VehicleAccessory>)>(),
            [(
                1234,
                vec![
                    wow_entities::VehicleAccessory {
                        accessory_entry: 7001,
                        seat_id: 0,
                        is_minion: true,
                        summoned_type: 8,
                        summon_time_ms: 0,
                    },
                    wow_entities::VehicleAccessory {
                        accessory_entry: 7002,
                        seat_id: 1,
                        is_minion: false,
                        summoned_type: 6,
                        summon_time_ms: 5000,
                    },
                ],
            )],
        ),
    ));
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 1234,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_like_cpp(100, ObjectGuid::EMPTY, &effect)
        .unwrap();

    assert_eq!(session.player_mount_display_id_like_cpp, 4321);
    assert_eq!(session.player_mount_vehicle_id_like_cpp, 55);
    assert_eq!(session.player_mount_vehicle_seat_count_like_cpp, 2);
    assert_eq!(session.player_mount_vehicle_usable_seat_count_like_cpp, 1);
    let vehicle_kit = session.player_mount_vehicle_kit_like_cpp.as_ref().unwrap();
    assert_eq!(vehicle_kit.vehicle_id(), 55);
    assert_eq!(vehicle_kit.creature_entry(), 1234);
    assert_eq!(vehicle_kit.status(), wow_entities::VehicleStatus::Installed);
    assert_eq!(vehicle_kit.seats().len(), 2);
    assert_eq!(session.player_mount_vehicle_accessories_like_cpp.len(), 2);
    assert_eq!(
        session.player_mount_vehicle_accessories_like_cpp[0].accessory_entry,
        7001
    );
    assert_eq!(
        session.player_mount_vehicle_despawn_delay_ms_like_cpp(),
        2500
    );
    assert!(session.player_mounted_like_cpp);
    assert_eq!(session.mount_vehicle_create_requests_like_cpp, 1);
    assert_eq!(session.mount_vehicle_remove_requests_like_cpp, 0);
    assert_eq!(
        session.mount_cancel_expected_vehicle_aura_packets_like_cpp,
        1
    );
    assert_eq!(session.mount_pet_control_disable_requests_like_cpp, 1);
    assert_eq!(session.mount_pet_control_enable_requests_like_cpp, 0);
    assert_eq!(session.mount_pet_resummon_requests_like_cpp, 0);
    assert_eq!(session.mount_collision_height_update_requests_like_cpp, 1);
    assert_eq!(
        session.represented_pet_react_state_like_cpp,
        wow_packet::packets::pet::REACT_PASSIVE_LIKE_CPP
    );
    assert_eq!(
        session.represented_pet_command_state_like_cpp,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP
    );
    assert!((session.player_collision_height_like_cpp - 7.32).abs() < 0.0001);
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::MoveSetVehicleRecId));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::SetVehicleRecId));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::OnCancelExpectedRideVehicleAura));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::PetMode));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::MoveSetCollisionHeight));
    let command = other_command_rx.try_recv().unwrap();
    let broadcast = match command {
        SessionCommand::SendIfVisibleLikeCpp(command) => {
            wow_packet::WorldPacket::from_bytes(&command.packet_bytes)
        }
        other => {
            panic!("expected SendIfVisibleLikeCpp collision-height command, got {other:?}")
        }
    };
    assert_eq!(
        broadcast.server_opcode(),
        Some(wow_constants::ServerOpcodes::MoveUpdateCollisionHeight)
    );
    assert!(
        session
            .player_unit_flags_like_cpp
            .contains(UnitFlags::PLAYER_CONTROLLED | UnitFlags::MOUNT)
    );

    let slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| (aura.spell_id == 100).then_some(slot))
        .unwrap();
    session.remove_aura(slot).unwrap();

    assert_eq!(session.player_mount_display_id_like_cpp, 0);
    assert_eq!(session.player_mount_vehicle_id_like_cpp, 0);
    assert!(session.player_mount_vehicle_kit_like_cpp.is_none());
    assert!(session.player_mount_vehicle_accessories_like_cpp.is_empty());
    assert_eq!(session.player_mount_vehicle_despawn_delay_ms_like_cpp(), 1);
    assert_eq!(session.player_mount_vehicle_seat_count_like_cpp, 0);
    assert_eq!(session.player_mount_vehicle_usable_seat_count_like_cpp, 0);
    assert!(!session.player_mounted_like_cpp);
    assert_eq!(session.mount_vehicle_create_requests_like_cpp, 1);
    assert_eq!(session.mount_vehicle_remove_requests_like_cpp, 1);
    assert_eq!(session.mount_pet_control_disable_requests_like_cpp, 1);
    assert_eq!(session.mount_pet_control_enable_requests_like_cpp, 1);
    assert_eq!(session.mount_pet_resummon_requests_like_cpp, 1);
    assert_eq!(session.mount_collision_height_update_requests_like_cpp, 2);
    assert_eq!(
        session.represented_pet_react_state_like_cpp,
        wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP
    );
    assert_eq!(
        session.represented_pet_command_state_like_cpp,
        wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP
    );
    assert_eq!(session.temporary_mount_pet_react_state_like_cpp, None);
    assert!((session.player_collision_height_like_cpp - 2.64).abs() < 0.0001);
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::MoveSetVehicleRecId));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::SetVehicleRecId));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::PetMode));
    assert!(opcodes.contains(&wow_constants::ServerOpcodes::MoveSetCollisionHeight));
    let command = other_command_rx.try_recv().unwrap();
    let broadcast = match command {
        SessionCommand::SendIfVisibleLikeCpp(command) => {
            wow_packet::WorldPacket::from_bytes(&command.packet_bytes)
        }
        other => {
            panic!("expected SendIfVisibleLikeCpp collision-height command, got {other:?}")
        }
    };
    assert_eq!(
        broadcast.server_opcode(),
        Some(wow_constants::ServerOpcodes::MoveUpdateCollisionHeight)
    );
    assert!(
        session
            .player_unit_flags_like_cpp
            .contains(UnitFlags::PLAYER_CONTROLLED)
    );
    assert!(
        !session
            .player_unit_flags_like_cpp
            .contains(UnitFlags::MOUNT)
    );
}
#[test]
fn represented_mount_capability_applies_mounted_speed_aura_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_mount_capability_store(Arc::new(wow_data::MountCapabilityStore::from_entries([
        wow_data::MountCapabilityEntry {
            id: 77,
            flags: wow_data::MOUNT_CAPABILITY_FLAG_GROUND,
            req_riding_skill: 0,
            req_area_id: 0,
            req_spell_aura_id: 0,
            req_spell_known_id: 0,
            mod_spell_aura_id: 12_346,
            req_map_id: 0,
        },
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        12_346,
        wow_data::SpellInfo {
            spell_id: 12_346,
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_like_cpp(100, ObjectGuid::EMPTY, &effect)
        .unwrap();

    assert!(session.visible_auras.values().any(|aura| {
        aura.spell_id == 12_346
            && aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::MountedSpeed)
            && aura.represented_amount == 100
    }));
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 14.0).abs() < 0.0001
    );
    assert_eq!(
        session.forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run),
        1
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetRunSpeed),
        "C++ Unit::SetSpeedRate sends SMSG_MOVE_SET_RUN_SPEED when the mounted run rate changes"
    );
}
#[test]
fn represented_mounted_aura_recalculates_amount_from_mount_capability_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    install_canonical_player_owner_for_test(&mut session, 0, 0);
    session.set_mount_store(Arc::new(wow_data::MountStore::from_entries([
        wow_data::MountEntry {
            id: 1,
            mount_type_id: 7,
            flags: 0,
            source_type_enum: 0,
            source_spell_id: 100,
            player_condition_id: 0,
            mount_fly_ride_height: 0.0,
            ui_model_scene_id: 0,
        },
    ])));
    session.set_mount_capability_store(Arc::new(wow_data::MountCapabilityStore::from_entries([
        wow_data::MountCapabilityEntry {
            id: 77,
            flags: wow_data::MOUNT_CAPABILITY_FLAG_GROUND,
            req_riding_skill: 0,
            req_area_id: 0,
            req_spell_aura_id: 0,
            req_spell_known_id: 0,
            mod_spell_aura_id: 12_346,
            req_map_id: -1,
        },
    ])));
    session.set_mount_type_x_capability_store(Arc::new(
        wow_data::MountTypeXCapabilityStore::from_entries([wow_data::MountTypeXCapabilityEntry {
            id: 1,
            mount_type_id: 7,
            mount_capability_id: 77,
            order_index: 0,
        }]),
    ));
    session.set_area_table_store(Arc::new(wow_data::AreaTableStore::from_entries([])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        12_346,
        wow_data::SpellInfo {
            spell_id: 12_346,
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 0,
        effect_misc_value_1: 0,
        effect_misc_value_2: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_like_cpp(100, ObjectGuid::EMPTY, &effect)
        .unwrap();

    let visible_auras = session
        .resolved_player_visible_auras_like_cpp()
        .expect("canonical Player aura owner");
    assert!(visible_auras.values().any(|aura| {
        aura.spell_id == 100
            && aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Mounted)
            && aura.represented_amount == 77
    }));
    assert!(visible_auras.values().any(|aura| {
        aura.spell_id == 12_346
            && aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::MountedSpeed)
            && aura.represented_amount == 100
    }));
    assert!(
        (session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run) - 14.0).abs() < 0.0001
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetRunSpeed),
        "C++ AuraEffect::CalculateAmount turns SPELL_AURA_MOUNTED amount into MountCapabilityEntry::ID before HandleAuraMounted casts the speed aura"
    );
}
#[test]
fn canonical_pet_owns_live_mode_spell_and_speed_state_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let canonical = shared_canonical_map_manager();
    let player_guid = ObjectGuid::create_player(1, 5_582);
    let pet_guid = ObjectGuid::create_world_object(HighGuid::Pet, 0, 1, 571, 0, 7_001, 5_582);
    let position = Position::new(3700.0, 1500.0, 120.0, 0.0);

    session.set_canonical_map_manager(Arc::clone(&canonical));
    session.set_map_store(canonical_player_transfer_test_map_store_like_cpp());
    session.attach_player_controller_like_cpp(SessionPlayerController::new(
        player_guid,
        "CanonicalPetOwner".to_string(),
        position,
        571,
        1,
        1,
        80,
        0,
    ));
    session
        .ensure_canonical_world_map_for_current_player_like_cpp()
        .expect("initial world map");
    add_canonical_test_pet_with_number(
        &canonical,
        pet_guid,
        player_guid,
        7_001,
        position,
        42,
        9_001,
        0,
    );
    session.client_visible_guids_like_cpp.insert(pet_guid);
    session.set_represented_pet_mode_state_with_spell_like_cpp(
        Some(pet_guid),
        wow_packet::packets::pet::REACT_PASSIVE_LIKE_CPP,
        wow_packet::packets::pet::COMMAND_STAY_LIKE_CPP,
        9_002,
    );

    session.set_player_movement_speed_rate_and_notify_like_cpp(UnitMoveTypeLikeCpp::Run, 2.0);

    let manager = canonical.lock().unwrap();
    let pet = manager
        .find_map(571, 0)
        .unwrap()
        .map()
        .get_typed_pet(pet_guid)
        .expect("canonical pet");
    assert_eq!(pet.created_by_spell_id_like_cpp(), 9_002);
    assert_eq!(
        pet.creature().react_state(),
        wow_entities::ReactState::Passive
    );
    assert_eq!(
        pet.creature()
            .unit()
            .subsystems()
            .control
            .charm_info
            .as_ref()
            .unwrap()
            .command_state,
        wow_packet::packets::pet::COMMAND_STAY_LIKE_CPP
    );
    assert_eq!(
        pet.creature()
            .unit()
            .speed_rate_at_like_cpp(UnitMoveTypeLikeCpp::Run.index()),
        Some(2.0)
    );
    drop(manager);

    assert_eq!(session.represented_pet_speed_propagations_like_cpp(), 1);
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSplineSetRunSpeed),
        "C++ Pet::SetSpeedRate publishes the canonical Pet speed after mutation"
    );
}
#[tokio::test]
async fn represented_fly_aura_sets_can_fly_flags_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    let player_guid = ObjectGuid::create_player(1, 42);
    session.set_player_guid(Some(player_guid));
    session.set_player_map_position_like_cpp(571, Position::new(1.0, 2.0, 30.0, 0.0));
    let spell_id = 20_033;
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_FLY,
                effect_base_points: 0,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));

    session
        .execute_spell_with_visual_and_target_data(
            spell_id,
            player_guid,
            ObjectGuid::EMPTY,
            wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 725,
                script_visual_id: 0,
            },
            SpellTargetData::default(),
        )
        .await
        .expect("fly apply-aura row should execute");

    assert!(
        session
            .player_movement_flags_like_cpp()
            .contains(MovementFlag::CAN_FLY)
    );
    assert!(session.represented_can_swim_to_fly_transition_like_cpp());
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        opcodes.contains(&ServerOpcodes::MoveEnableTransitionBetweenSwimAndFly),
        "C++ HandleAuraAllowFlight enables swim-to-fly transition"
    );
    assert!(
        opcodes.contains(&ServerOpcodes::MoveSetCanFly),
        "C++ HandleAuraAllowFlight enables CanFly"
    );
}
#[test]
fn represented_fly_aura_removal_unsets_can_fly_and_resets_fall_info_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_map_position_like_cpp(571, Position::new(1.0, 2.0, 44.0, 0.0));
    session.set_fall_information_like_cpp(1_200, 80.0);
    let caster = ObjectGuid::create_player(1, 42);
    let fly = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_FLY,
        effect_base_points: 0,
        effect_index: 0,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_034,
            caster,
            &fly,
            RepresentedAuraEffectLikeCpp::Fly,
            30_000,
        )
        .unwrap();
    session.update_represented_flight_flags_for_flight_aura_like_cpp(true);
    let slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Fly)).then_some(slot)
        })
        .unwrap();
    let _ = drain_server_opcodes(&send_rx);

    session.remove_aura(slot).unwrap();

    assert!(
        !session
            .player_movement_flags_like_cpp()
            .contains(MovementFlag::CAN_FLY)
    );
    assert!(
        session
            .player_movement_flags_like_cpp()
            .contains(MovementFlag::FALLING),
        "C++ HandleAuraAllowFlight calls MotionMaster::MoveFall after the last flight source is removed"
    );
    assert!(!session.represented_can_swim_to_fly_transition_like_cpp());
    assert_eq!(session.fall_information_like_cpp(), (0, 44.0));
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        opcodes.contains(&ServerOpcodes::MoveDisableTransitionBetweenSwimAndFly),
        "C++ HandleAuraAllowFlight disables swim-to-fly transition on last flight source removal"
    );
    assert!(
        opcodes.contains(&ServerOpcodes::MoveUnsetCanFly),
        "C++ HandleAuraAllowFlight unsets CanFly and starts fall handling on last flight source removal"
    );
}
#[test]
fn represented_fly_aura_removal_skips_move_fall_when_gravity_disabled_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_player_map_position_like_cpp(571, Position::new(1.0, 2.0, 44.0, 0.0));
    session.set_player_movement_flags_like_cpp(MovementFlag::DISABLE_GRAVITY);
    session.set_fall_information_like_cpp(1_200, 80.0);
    let caster = ObjectGuid::create_player(1, 42);
    let fly = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_FLY,
        effect_base_points: 0,
        effect_index: 0,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_037,
            caster,
            &fly,
            RepresentedAuraEffectLikeCpp::Fly,
            30_000,
        )
        .unwrap();
    session.update_represented_flight_flags_for_flight_aura_like_cpp(true);
    let slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Fly)).then_some(slot)
        })
        .unwrap();
    let _ = drain_server_opcodes(&send_rx);

    session.remove_aura(slot).unwrap();

    assert!(
        session
            .player_movement_flags_like_cpp()
            .contains(MovementFlag::DISABLE_GRAVITY)
    );
    assert!(
        !session
            .player_movement_flags_like_cpp()
            .contains(MovementFlag::FALLING),
        "C++ HandleAuraAllowFlight skips MotionMaster::MoveFall while IsGravityDisabled"
    );
    assert_eq!(
        session.fall_information_like_cpp(),
        (0, 44.0),
        "C++ SetCanFly(false) still resets player fall information before the gravity guard"
    );
}
#[test]
fn represented_fly_aura_removal_preserves_flags_when_mounted_flight_remains_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    let caster = ObjectGuid::create_player(1, 42);
    let fly = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_FLY,
        effect_base_points: 0,
        effect_index: 0,
        ..Default::default()
    };
    let mounted_flight = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED,
        effect_base_points: 60,
        effect_index: 0,
        ..Default::default()
    };

    session
        .apply_represented_aura_modifier_like_cpp(
            20_035,
            caster,
            &fly,
            RepresentedAuraEffectLikeCpp::Fly,
            30_000,
        )
        .unwrap();
    session
        .apply_represented_aura_modifier_like_cpp(
            20_036,
            caster,
            &mounted_flight,
            RepresentedAuraEffectLikeCpp::MountedFlightSpeed,
            30_000,
        )
        .unwrap();
    session.update_represented_flight_flags_for_flight_aura_like_cpp(true);
    let fly_slot = session
        .visible_auras
        .iter()
        .find_map(|(&slot, aura)| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Fly)).then_some(slot)
        })
        .unwrap();
    let _ = drain_server_opcodes(&send_rx);

    session.remove_aura(fly_slot).unwrap();

    assert!(
        session
            .player_movement_flags_like_cpp()
            .contains(MovementFlag::CAN_FLY),
        "C++ keeps CanFly while a mounted-flight aura is still present"
    );
    assert!(session.represented_can_swim_to_fly_transition_like_cpp());
    let opcodes = drain_server_opcodes(&send_rx);
    assert!(
        !opcodes.contains(&ServerOpcodes::MoveUnsetCanFly),
        "C++ does not unset CanFly until the last SPELL_AURA_FLY/mounted-flight source is gone"
    );
    assert!(
        !opcodes.contains(&ServerOpcodes::MoveDisableTransitionBetweenSwimAndFly),
        "C++ keeps swim-to-fly transition while a mounted-flight aura remains"
    );
}
#[test]
fn represented_mount_removal_removes_capability_speed_aura_like_cpp() {
    let (mut session, _, send_rx) = make_session();
    session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
    session.set_mount_capability_store(Arc::new(wow_data::MountCapabilityStore::from_entries([
        wow_data::MountCapabilityEntry {
            id: 77,
            flags: wow_data::MOUNT_CAPABILITY_FLAG_GROUND,
            req_riding_skill: 0,
            req_area_id: 0,
            req_spell_aura_id: 0,
            req_spell_known_id: 0,
            mod_spell_aura_id: 12_346,
            req_map_id: 0,
        },
    ])));
    let mut spell_store = wow_data::SpellStore::new();
    spell_store.insert(
        12_346,
        wow_data::SpellInfo {
            spell_id: 12_346,
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
                effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
                effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED,
                effect_base_points: 100,
                ..Default::default()
            }],
        },
    );
    session.set_spell_store(Arc::new(spell_store));
    let effect = wow_data::SpellEffectInfo {
        effect: wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA,
        effect_aura: wow_data::spell::aura_types::SPELL_AURA_MOUNTED,
        effect_base_points: 77,
        effect_misc_value_1: 0,
        ..Default::default()
    };

    session
        .apply_represented_mounted_aura_like_cpp(100, ObjectGuid::EMPTY, &effect)
        .unwrap();
    let _ = drain_server_opcodes(&send_rx);
    let mounted_slot = session
        .visible_auras
        .values()
        .find_map(|aura| {
            (aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::Mounted))
                .then_some(aura.slot)
        })
        .unwrap();

    session.remove_aura(mounted_slot).unwrap();

    assert!(!session.visible_auras.values().any(|aura| {
        aura.spell_id == 12_346
            || aura.represented_effect == Some(RepresentedAuraEffectLikeCpp::MountedSpeed)
    }));
    assert_eq!(
        session.player_movement_speed_like_cpp(UnitMoveTypeLikeCpp::Run),
        7.0
    );
    assert_eq!(
        session.forced_speed_changes_like_cpp(UnitMoveTypeLikeCpp::Run),
        2
    );
    assert!(
        drain_server_opcodes(&send_rx).contains(&ServerOpcodes::MoveSetRunSpeed),
        "C++ Unit::SetSpeedRate sends SMSG_MOVE_SET_RUN_SPEED when dismount restores run speed"
    );
}
