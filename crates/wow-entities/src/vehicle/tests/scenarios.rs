//! Vehicle regressions.
//!
//! Moved out of vehicle.rs under #683; every test is unchanged.

use super::*;

#[test]
fn vehicle_constructor_matches_cpp_base_state() {
    let vehicle = vehicle();

    assert_eq!(vehicle.base_guid(), base_guid());
    assert_eq!(vehicle.base_type_id(), TypeId::Unit);
    assert_eq!(vehicle.vehicle_id(), 123);
    assert_eq!(vehicle.creature_entry(), 456);
    assert_eq!(vehicle.status(), VehicleStatus::None);
    assert_eq!(vehicle.usable_seat_num(), 1);
    assert_eq!(vehicle.seats().len(), 3);
    assert!(vehicle.has_empty_seat(0));
    assert!(!vehicle.has_empty_seat(99));
    assert_eq!(vehicle.available_seat_count(), 2);
    assert!(vehicle.is_controllable_vehicle());
    assert!(!vehicle.is_vehicle_in_use());
}

#[test]
fn install_uninstall_and_passengers_follow_cpp_shape() {
    let mut vehicle = vehicle();
    vehicle.install();
    assert_eq!(vehicle.status(), VehicleStatus::Installed);

    assert!(vehicle.add_vehicle_passenger(passenger_guid(1), 0));
    assert_eq!(vehicle.passenger(0), Some(passenger_guid(1)));
    assert!(vehicle.is_vehicle_in_use());
    assert!(!vehicle.add_vehicle_passenger(passenger_guid(2), 0));
    assert_eq!(vehicle.remove_passenger(passenger_guid(1)), Some(0));
    assert!(vehicle.passenger(0).is_none());

    vehicle.add_pending_event(passenger_guid(3), 0);
    assert!(vehicle.has_pending_event_for_seat(0));
    assert!(!vehicle.has_empty_seat(0));
    vehicle.remove_pending_events_for_passenger(passenger_guid(3));
    assert!(vehicle.has_empty_seat(0));

    vehicle.add_vehicle_passenger(passenger_guid(4), 0);
    vehicle.add_pending_event(passenger_guid(5), 2);
    vehicle.uninstall();
    assert_eq!(vehicle.status(), VehicleStatus::Uninstalling);
    assert!(!vehicle.is_vehicle_in_use());
    assert!(!vehicle.has_pending_event_for_seat(2));
}

#[test]
fn remove_passenger_plan_restores_cpp_side_effects() {
    let mut vehicle = vehicle();
    let passenger = passenger_guid(1);
    vehicle.seats.get_mut(&0).unwrap().seat_info.can_control = true;
    vehicle
        .seats
        .get_mut(&0)
        .unwrap()
        .seat_info
        .disables_gravity = true;
    vehicle
        .seats
        .get_mut(&0)
        .unwrap()
        .seat_info
        .passenger_not_selectable = true;
    assert!(vehicle.add_vehicle_passenger(passenger, 0));
    vehicle.usable_seat_num = 0;

    let plan = vehicle
        .remove_passenger_plan_like_cpp(passenger, TypeId::Player, true, false, true, true)
        .expect("boarded passenger is removable");

    assert_eq!(
        plan,
        VehiclePassengerRemovePlan {
            seat_id: 0,
            set_vehicle_none: true,
            restore_gravity: true,
            restore_interactible: true,
            restore_npc_flag: true,
            remove_charm: true,
            transport_reset: VehiclePassengerTransportReset::Reset,
            cast_parachute: true,
            call_ai_passenger_boarded: true,
            call_on_remove_passenger_script: true,
        }
    );
    assert_eq!(vehicle.usable_seat_num(), 1);
    assert!(vehicle.passenger(0).is_none());
}

#[test]
fn remove_passenger_plan_preserves_existing_passenger_flags_like_cpp() {
    let mut vehicle = vehicle();
    let passenger = passenger_guid(1);
    let seat = vehicle.seats.get_mut(&0).unwrap();
    seat.seat_info.disables_gravity = true;
    seat.seat_info.passenger_not_selectable = true;
    seat.passenger = PassengerInfo {
        guid: passenger,
        is_uninteractible: true,
        is_gravity_disabled: true,
    };

    let plan = vehicle
        .remove_passenger_plan_like_cpp(passenger, TypeId::Unit, true, true, false, false)
        .expect("boarded passenger is removable");

    assert!(!plan.restore_gravity);
    assert!(!plan.restore_interactible);
    assert_eq!(
        plan.transport_reset,
        VehiclePassengerTransportReset::InheritBaseTransport
    );
    assert!(!plan.remove_charm);
    assert!(!plan.cast_parachute);
}

#[test]
fn passenger_seat_lookups_match_cpp_helpers() {
    let mut vehicle = vehicle();
    let passenger = passenger_guid(1);
    assert!(vehicle.add_vehicle_passenger(passenger, 0));

    assert_eq!(vehicle.seat_id_for_passenger_like_cpp(passenger), Some(0));
    assert_eq!(
        vehicle.seat_info_for_passenger_like_cpp(passenger),
        Some(vehicle.seats().get(&0).unwrap().seat_info)
    );
    assert_eq!(
        vehicle.seat_addon_for_passenger_like_cpp(passenger),
        Some(vehicle.seats().get(&0).unwrap().seat_addon)
    );
    assert_eq!(
        vehicle.seat_id_for_passenger_like_cpp(passenger_guid(2)),
        None
    );
    assert_eq!(
        vehicle.seat_info_for_passenger_like_cpp(passenger_guid(2)),
        None
    );
    assert_eq!(
        vehicle.seat_addon_for_passenger_like_cpp(passenger_guid(2)),
        None
    );
}

#[test]
fn install_all_accessories_plan_filters_like_cpp() {
    let all = [
        VehicleAccessory {
            accessory_entry: 10,
            is_minion: true,
            summon_time_ms: 100,
            seat_id: 0,
            summoned_type: 8,
        },
        VehicleAccessory {
            accessory_entry: 20,
            is_minion: false,
            summon_time_ms: 200,
            seat_id: 1,
            summoned_type: 6,
        },
    ];

    let player_plan = vehicle_accessory_install_plan_like_cpp(TypeId::Player, true, &all);
    assert!(player_plan.remove_all_passengers);
    assert_eq!(player_plan.accessories, vec![all[0]]);

    let creature_normal = vehicle_accessory_install_plan_like_cpp(TypeId::Unit, false, &all);
    assert!(creature_normal.remove_all_passengers);
    assert_eq!(creature_normal.accessories, all);

    let creature_evading = vehicle_accessory_install_plan_like_cpp(TypeId::Unit, true, &all);
    assert!(!creature_evading.remove_all_passengers);
    assert_eq!(creature_evading.accessories, vec![all[0]]);
}

#[test]
fn install_accessory_plan_matches_cpp_status_and_minion_paths() {
    let accessory = VehicleAccessory {
        accessory_entry: 10,
        is_minion: true,
        summon_time_ms: 100,
        seat_id: 2,
        summoned_type: 8,
    };
    let mut vehicle = vehicle();

    assert_eq!(
        vehicle.install_accessory_plan_like_cpp(accessory),
        Some(VehicleAccessorySummonPlan {
            accessory,
            add_accessory_unit_mask: true,
            handle_spell_click_seat_id: 2,
        })
    );

    vehicle.status = VehicleStatus::Uninstalling;
    assert_eq!(vehicle.install_accessory_plan_like_cpp(accessory), None);
}

#[test]
fn vehicle_join_execute_plan_matches_cpp_success_path() {
    let mut vehicle = vehicle();
    let passenger = passenger_guid(1);
    vehicle.seats.get_mut(&0).unwrap().seat_info.can_control = true;
    vehicle
        .seats
        .get_mut(&0)
        .unwrap()
        .seat_info
        .disables_gravity = true;
    vehicle
        .seats
        .get_mut(&0)
        .unwrap()
        .seat_info
        .attachment_offset = Position::new(4.0, 5.0, 6.0, 0.0);
    vehicle
        .seats
        .get_mut(&0)
        .unwrap()
        .seat_addon
        .seat_orientation_offset = 0.75;
    vehicle.add_pending_event(passenger, 0);

    let plan = vehicle
        .vehicle_join_execute_plan_like_cpp(
            passenger,
            0,
            true,
            true,
            TypeId::Player,
            true,
            false,
            true,
            true,
            true,
        )
        .expect("known seat executes");

    assert_eq!(plan.abort, None);
    assert!(plan.exit_existing_vehicle);
    assert_eq!(
        plan.passenger_info,
        Some(PassengerInfo {
            guid: passenger,
            is_uninteractible: true,
            is_gravity_disabled: false,
        })
    );
    assert!(plan.remove_npc_flag);
    assert!(plan.player_drop_battleground_flag);
    assert!(plan.player_unsummon_temporary_pet);
    assert!(plan.set_disable_gravity);
    assert_eq!(
        plan.transport_position,
        Some(Position::new(4.0, 5.0, 6.0, 0.75))
    );
    assert!(plan.set_vehicle_charm);
    assert!(plan.send_clear_target);
    assert!(plan.set_root_controlled);
    assert!(plan.launch_transport_enter_spline);
    assert!(plan.transfer_threat_to_vehicle);
    assert!(plan.call_ai_passenger_boarded);
    assert!(plan.call_on_add_passenger_script);
    assert!(plan.call_on_install_accessory_script);
    assert_eq!(vehicle.passenger(0), Some(passenger));
    assert_eq!(vehicle.usable_seat_num(), 0);
}

#[test]
fn vehicle_join_execute_plan_aborts_dead_passenger_like_cpp() {
    let mut vehicle = vehicle();
    let passenger = passenger_guid(1);
    vehicle.add_pending_event(passenger, 0);

    let plan = vehicle
        .vehicle_join_execute_plan_like_cpp(
            passenger,
            0,
            false,
            false,
            TypeId::Unit,
            false,
            false,
            false,
            true,
            false,
        )
        .expect("known seat executes");

    assert_eq!(
        plan.abort,
        Some(VehicleJoinAbortPlan {
            remove_pending_event: true,
            remove_control_vehicle_aura: true,
            despawn_accessory: true,
        })
    );
    assert!(plan.passenger_info.is_none());
    assert!(vehicle.passenger(0).is_none());
    assert!(!vehicle.has_pending_event_for_seat(0));
}

#[test]
fn vehicle_join_execute_plan_keeps_pet_when_cpp_flag_is_set() {
    let mut vehicle = vehicle();
    let passenger = passenger_guid(1);
    vehicle.seats.get_mut(&0).unwrap().seat_info.keep_pet = true;

    let plan = vehicle
        .vehicle_join_execute_plan_like_cpp(
            passenger,
            0,
            true,
            false,
            TypeId::Player,
            false,
            false,
            false,
            false,
            false,
        )
        .expect("known seat executes");

    assert!(!plan.player_unsummon_temporary_pet);
}

#[test]
fn vehicle_base_movement_flags_match_cpp_init_movement_info() {
    let flags = VehicleFlag::NoStrafe as u32
        | VehicleFlag::NoJumping as u32
        | VehicleFlag::FullSpeedTurning as u32
        | VehicleFlag::AllowPitching as u32
        | VehicleFlag::FullSpeedPitching as u32
        | VehicleFlag::FixedPosition as u32;

    assert_eq!(
        vehicle_base_movement_flags_like_cpp(flags),
        MovementFlag2::NO_STRAFE
            | MovementFlag2::NO_JUMPING
            | MovementFlag2::FULL_SPEED_TURNING
            | MovementFlag2::ALWAYS_ALLOW_PITCHING
            | MovementFlag2::FULL_SPEED_PITCHING
    );
    assert!(vehicle_base_movement_flags_like_cpp(0).is_empty());
}

#[test]
fn apply_all_immunities_plan_matches_cpp_cases() {
    let generic = vehicle_immunity_plan_like_cpp(1, false, false);
    assert!(!generic.root);
    assert_eq!(
        generic.immunities,
        vec![
            VehicleSpellImmunity {
                kind: VehicleSpellImmunityKind::Effect,
                spell_or_mechanic: SPELL_EFFECT_KNOCK_BACK_LIKE_CPP,
                apply: true,
            },
            VehicleSpellImmunity {
                kind: VehicleSpellImmunityKind::Effect,
                spell_or_mechanic: SPELL_EFFECT_KNOCK_BACK_DEST_LIKE_CPP,
                apply: true,
            },
        ]
    );

    let mechanical = vehicle_immunity_plan_like_cpp(1, true, false);
    assert!(mechanical.immunities.contains(&VehicleSpellImmunity {
        kind: VehicleSpellImmunityKind::Effect,
        spell_or_mechanic: SPELL_EFFECT_HEAL_LIKE_CPP,
        apply: true,
    }));
    assert!(mechanical.immunities.contains(&VehicleSpellImmunity {
        kind: VehicleSpellImmunityKind::State,
        spell_or_mechanic: SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN_LIKE_CPP,
        apply: true,
    }));

    let world_boss = vehicle_immunity_plan_like_cpp(1, true, true);
    assert_eq!(world_boss.immunities, generic.immunities);

    let rooted_cannon = vehicle_immunity_plan_like_cpp(160, false, false);
    assert!(rooted_cannon.root);
    assert!(rooted_cannon.immunities.contains(&VehicleSpellImmunity {
        kind: VehicleSpellImmunityKind::State,
        spell_or_mechanic: SPELL_AURA_MOD_DECREASE_SPEED_LIKE_CPP,
        apply: true,
    }));

    let salvaged_chopper = vehicle_immunity_plan_like_cpp(335, false, false);
    assert!(salvaged_chopper.immunities.contains(&VehicleSpellImmunity {
        kind: VehicleSpellImmunityKind::State,
        spell_or_mechanic: SPELL_AURA_MOD_DAMAGE_PERCENT_TAKEN_LIKE_CPP,
        apply: false,
    }));
}

#[test]
fn reset_plan_matches_cpp_unit_alive_and_dead_paths() {
    let accessories = [
        VehicleAccessory {
            accessory_entry: 10,
            is_minion: false,
            summon_time_ms: 0,
            seat_id: 0,
            summoned_type: 1,
        },
        VehicleAccessory {
            accessory_entry: 11,
            is_minion: true,
            summon_time_ms: 100,
            seat_id: 1,
            summoned_type: 2,
        },
    ];

    let mut alive = vehicle();
    assert!(alive.add_vehicle_passenger(passenger_guid(1), 0));
    let plan = alive
        .reset_plan_like_cpp(false, true, true, false, &accessories)
        .expect("unit vehicles reset in C++");
    assert!(plan.call_on_reset_script);
    assert!(
        plan.immunity_plan
            .immunities
            .contains(&VehicleSpellImmunity {
                kind: VehicleSpellImmunityKind::Effect,
                spell_or_mechanic: SPELL_EFFECT_HEAL_LIKE_CPP,
                apply: true,
            })
    );
    assert_eq!(
        plan.accessory_install_plan,
        Some(VehicleAccessoryInstallPlan {
            remove_all_passengers: true,
            accessories: accessories.to_vec(),
        })
    );
    assert!(alive.passenger(0).is_none());

    let mut dead = vehicle();
    assert!(dead.add_vehicle_passenger(passenger_guid(2), 0));
    let plan = dead
        .reset_plan_like_cpp(false, false, false, false, &accessories)
        .expect("dead unit vehicles still reapply immunities in C++");
    assert!(plan.accessory_install_plan.is_none());
    assert_eq!(dead.passenger(0), Some(passenger_guid(2)));
}

#[test]
fn reset_plan_noops_for_non_unit_like_cpp() {
    let mut player_vehicle = Vehicle::new(
        base_guid(),
        TypeId::Player,
        Position::new(10.0, 20.0, 30.0, 1.0),
        123,
        456,
        [(0, seat(1000, true), VehicleSeatAddon::default())],
    );

    assert_eq!(
        player_vehicle.reset_plan_like_cpp(false, true, true, false, &[]),
        None
    );
}

#[test]
fn remove_all_passengers_plan_matches_cpp_pending_and_current_passengers() {
    let mut vehicle = vehicle();
    let boarded = passenger_guid(1);
    let pending = passenger_guid(2);
    assert!(vehicle.add_vehicle_passenger(boarded, 0));
    vehicle.add_pending_event(pending, 1);

    let plan = vehicle.remove_all_passengers_plan_like_cpp();

    assert_eq!(
        plan,
        VehicleRemoveAllPassengersPlan {
            pending_join_aborts: vec![VehiclePendingJoinAbort {
                passenger: pending,
                seat_id: 1,
                target_vehicle_available: true,
            }],
            remove_control_vehicle_auras: true,
            forced_exit_passengers: vec![boarded],
        }
    );
    assert!(vehicle.passenger(0).is_none());
    assert!(!vehicle.has_pending_event_for_seat(1));
}

#[test]
fn remove_all_passengers_plan_marks_uninstalling_abort_target_like_cpp() {
    let mut vehicle = vehicle();
    let pending = passenger_guid(3);
    vehicle.add_pending_event(pending, 1);
    vehicle.status = VehicleStatus::Uninstalling;

    let plan = vehicle.remove_all_passengers_plan_like_cpp();

    assert_eq!(
        plan.pending_join_aborts,
        vec![VehiclePendingJoinAbort {
            passenger: pending,
            seat_id: 1,
            target_vehicle_available: false,
        }]
    );
}

#[test]
fn add_vehicle_passenger_plan_selects_empty_seat_and_blocks_pending_like_cpp() {
    let mut vehicle = vehicle();
    vehicle.add_pending_event(passenger_guid(1), 0);

    assert_eq!(vehicle.available_seat_count(), 1);
    let plan = vehicle.add_vehicle_passenger_plan_like_cpp(passenger_guid(2), -1);

    assert_eq!(
        plan,
        VehiclePassengerAddPlan {
            accepted: true,
            seat_id: Some(2),
            scheduled_abort: false,
            displaced_passenger: None,
        }
    );
    assert!(vehicle.has_pending_event_for_seat(2));
    assert!(vehicle.passenger(2).is_none());
}

#[test]
fn add_vehicle_passenger_plan_aborts_when_no_seat_or_uninstalling_like_cpp() {
    let mut full = vehicle();
    full.add_pending_event(passenger_guid(1), 0);
    full.add_pending_event(passenger_guid(2), 2);

    assert_eq!(
        full.add_vehicle_passenger_plan_like_cpp(passenger_guid(3), -1),
        VehiclePassengerAddPlan {
            accepted: false,
            seat_id: None,
            scheduled_abort: true,
            displaced_passenger: None,
        }
    );
    assert_eq!(
        full.add_vehicle_passenger_plan_like_cpp(passenger_guid(4), 99),
        VehiclePassengerAddPlan {
            accepted: false,
            seat_id: None,
            scheduled_abort: true,
            displaced_passenger: None,
        }
    );

    full.status = VehicleStatus::Uninstalling;
    assert_eq!(
        full.add_vehicle_passenger_plan_like_cpp(passenger_guid(5), 0),
        VehiclePassengerAddPlan {
            accepted: false,
            seat_id: None,
            scheduled_abort: false,
            displaced_passenger: None,
        }
    );
}

#[test]
fn add_vehicle_passenger_plan_displaces_specific_occupied_seat_like_cpp() {
    let mut vehicle = vehicle();
    let old_passenger = passenger_guid(1);
    let new_passenger = passenger_guid(2);
    assert!(vehicle.add_vehicle_passenger(old_passenger, 0));

    let plan = vehicle.add_vehicle_passenger_plan_like_cpp(new_passenger, 0);

    assert_eq!(
        plan,
        VehiclePassengerAddPlan {
            accepted: true,
            seat_id: Some(0),
            scheduled_abort: false,
            displaced_passenger: Some(old_passenger),
        }
    );
    assert!(vehicle.passenger(0).is_none());
    assert!(vehicle.has_pending_event_for_seat(0));
}

#[test]
fn remove_pending_events_for_seat_schedules_abort_like_cpp() {
    let mut vehicle = vehicle();
    let first = passenger_guid(1);
    let second = passenger_guid(2);
    vehicle.add_pending_event(first, 0);
    vehicle.add_pending_event(second, 1);

    let plan = vehicle.remove_pending_events_for_seat_plan_like_cpp(0);

    assert_eq!(
        plan,
        VehiclePendingEventRemovalPlan {
            scheduled_aborts: vec![VehiclePendingJoinAbort {
                passenger: first,
                seat_id: 0,
                target_vehicle_available: true,
            }],
        }
    );
    assert!(!vehicle.has_pending_event_for_seat(0));
    assert!(vehicle.has_pending_event_for_seat(1));
}

#[test]
fn remove_pending_events_for_passenger_schedules_abort_like_cpp() {
    let mut vehicle = vehicle();
    let passenger = passenger_guid(1);
    vehicle.add_pending_event(passenger, 2);

    let plan = vehicle.remove_pending_events_for_passenger_plan_like_cpp(passenger);

    assert_eq!(
        plan,
        VehiclePendingEventRemovalPlan {
            scheduled_aborts: vec![VehiclePendingJoinAbort {
                passenger,
                seat_id: 2,
                target_vehicle_available: true,
            }],
        }
    );
    assert!(!vehicle.has_pending_event_for_seat(2));
    assert_eq!(
        vehicle.remove_pending_events_for_passenger_plan_like_cpp(passenger),
        VehiclePendingEventRemovalPlan::default()
    );
}

#[test]
fn next_empty_seat_skips_occupied_pending_and_unusable() {
    let mut vehicle = vehicle();
    assert!(vehicle.add_vehicle_passenger(passenger_guid(1), 0));
    vehicle.add_pending_event(passenger_guid(2), 2);

    assert_eq!(vehicle.next_empty_seat(0, true), None);
    vehicle.remove_pending_events_for_seat(2);
    assert_eq!(vehicle.next_empty_seat(0, true), Some(2));
}

#[test]
fn relocate_passengers_plan_matches_cpp_transport_offsets() {
    let mut vehicle = vehicle();
    let first = passenger_guid(1);
    let second = passenger_guid(2);
    assert!(vehicle.add_vehicle_passenger(first, 0));
    assert!(vehicle.add_vehicle_passenger(second, 1));

    let first_offset = Position::new(1.0, 2.0, 3.0, 0.25);
    let ignored = passenger_guid(3);
    let relocations = vehicle
        .relocate_passengers_plan_like_cpp(&[(first, first_offset), (ignored, first_offset)]);

    assert_eq!(
        relocations,
        vec![VehiclePassengerRelocation {
            passenger: first,
            position: vehicle.calculate_passenger_position(first_offset),
            set_home_position: false,
        }]
    );
}

#[test]
fn debug_info_matches_cpp_shape() {
    let mut vehicle = vehicle();
    let passenger = passenger_guid(1);
    let pending = passenger_guid(2);
    assert!(vehicle.add_vehicle_passenger(passenger, 0));
    vehicle.add_pending_event(pending, 2);

    let debug = vehicle.debug_info_like_cpp();

    assert!(debug.starts_with("Vehicle seats:\n"));
    assert!(debug.contains(&format!("seat 0: {passenger}\n")));
    assert!(debug.contains("seat 1: empty\n"));
    assert!(debug.contains(&format!("Vehicle pending events:\nseat 2: {pending}\n")));

    vehicle.remove_pending_events_for_passenger(pending);
    assert!(
        vehicle
            .debug_info_like_cpp()
            .ends_with("Vehicle pending events: none")
    );
}

#[test]
fn transport_position_transforms_match_cpp_formula() {
    let vehicle = vehicle();
    let offset = Position::new(2.0, 3.0, 4.0, 0.5);

    let global = vehicle.calculate_passenger_position(offset);
    let roundtrip = vehicle.calculate_passenger_offset(global);

    assert!((roundtrip.x - offset.x).abs() < 0.0001);
    assert!((roundtrip.y - offset.y).abs() < 0.0001);
    assert!((roundtrip.z - offset.z).abs() < 0.0001);
    assert!((roundtrip.orientation - offset.orientation).abs() < 0.0001);
}
