//! Phasing regressions.
//!
//! Moved out of phasing.rs under #683; every test is unchanged.

use super::*;

#[test]
fn reset_phase_shift_clears_active_and_suppressed_like_cpp() {
    let mut object = world_object();
    object
        .phase_shift_mut()
        .add_phase_like_cpp(10, PhaseFlags::NONE, 1);
    object
        .suppressed_phase_shift_mut()
        .add_phase_like_cpp(20, PhaseFlags::NONE, 1);

    reset_phase_shift_like_cpp(&mut object);

    assert!(
        object
            .phase_shift()
            .flags_like_cpp()
            .contains(PhaseShiftFlags::UNPHASED)
    );
    assert!(
        object
            .suppressed_phase_shift()
            .flags_like_cpp()
            .contains(PhaseShiftFlags::UNPHASED)
    );
    assert!(!object.phase_shift().has_phase_like_cpp(10));
    assert!(!object.suppressed_phase_shift().has_phase_like_cpp(20));
}

#[test]
fn controlled_unit_visitor_matches_cpp_selection_rules() {
    let owner_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 1);
    let controlled_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 2);
    let controlled_player = ObjectGuid::create_player(1, 50);
    let nested_vehicle_passenger =
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 3);
    let summon_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 4);
    let missing_summon = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 5);
    let vehicle_passenger = ObjectGuid::create_world_object(HighGuid::Creature, 0, 0, 571, 0, 1, 6);

    let mut unit = unit(owner_guid);
    unit.subsystems_mut()
        .control
        .add_controlled(controlled_guid);
    unit.subsystems_mut()
        .control
        .add_controlled(controlled_player);
    unit.subsystems_mut()
        .control
        .add_controlled(nested_vehicle_passenger);
    unit.subsystems_mut()
        .control
        .set_summon_slot(0, summon_guid);
    unit.subsystems_mut()
        .control
        .set_summon_slot(1, missing_summon);

    let mut visitor = ControlledUnitVisitor::new(owner_guid);
    let mut visited = Vec::new();
    visitor.visit_controlled_of_like_cpp(
        &unit,
        |guid| {
            if guid == controlled_guid {
                Some(ControlledUnitInfo::new(guid, TypeId::Unit, false))
            } else if guid == controlled_player {
                Some(ControlledUnitInfo::new(guid, TypeId::Player, false))
            } else if guid == nested_vehicle_passenger {
                Some(ControlledUnitInfo::new(guid, TypeId::Unit, true))
            } else {
                None
            }
        },
        |guid| guid == summon_guid,
        [vehicle_passenger, owner_guid, summon_guid],
        |guid| visited.push(guid),
    );

    visited.sort();
    assert_eq!(
        visited,
        vec![controlled_guid, summon_guid, vehicle_passenger]
    );
    assert!(visitor.was_visited(owner_guid));
}

#[test]
fn add_and_remove_phase_core_match_cpp_mutation() {
    let phase_store = phase_store();
    let mut object = world_object();
    let personal_guid = object.guid();

    let update = add_object_phase_like_cpp(&mut object, &phase_store, 20, true);

    assert_eq!(update, PhaseVisibilityUpdate::new(true, true));
    assert!(object.phase_shift().has_phase_like_cpp(20));
    assert_eq!(
        object
            .phase_shift()
            .phase_ref_like_cpp(20)
            .map(|phase| phase.flags()),
        Some(PhaseFlags::PERSONAL)
    );
    assert_eq!(object.phase_shift().personal_guid_like_cpp(), personal_guid);

    let update = add_object_phase_like_cpp(&mut object, &phase_store, 20, true);
    assert_eq!(update, PhaseVisibilityUpdate::new(true, false));

    let update = remove_phase_like_cpp(&mut object, 20, false);
    assert_eq!(update, PhaseVisibilityUpdate::new(false, false));
    assert!(object.phase_shift().has_phase_like_cpp(20));

    let update = remove_phase_like_cpp(&mut object, 20, false);
    assert_eq!(update, PhaseVisibilityUpdate::new(false, true));
    assert!(!object.phase_shift().has_phase_like_cpp(20));
}

#[test]
fn phase_group_core_uses_cpp_group_lookup_and_phase_flags() {
    let phase_store = phase_store();
    let phase_group_store = phase_group_store(&phase_store);
    let mut object = world_object();

    let update =
        add_object_phase_group_like_cpp(&mut object, &phase_store, &phase_group_store, 7, true);

    assert_eq!(update, Some(PhaseVisibilityUpdate::new(true, true)));
    assert!(object.phase_shift().has_phase_like_cpp(10));
    assert!(object.phase_shift().has_phase_like_cpp(20));
    assert!(!object.phase_shift().has_phase_like_cpp(99));

    let missing_group =
        add_object_phase_group_like_cpp(&mut object, &phase_store, &phase_group_store, 99, true);
    assert_eq!(missing_group, None);

    let update = remove_phase_group_like_cpp(&mut object, &phase_group_store, 7, false);
    assert_eq!(update, Some(PhaseVisibilityUpdate::new(false, true)));
    assert!(!object.phase_shift().has_phase_like_cpp(10));
    assert!(!object.phase_shift().has_phase_like_cpp(20));
}

#[test]
fn visible_map_id_core_updates_ui_map_phase_ids_like_cpp() {
    let terrain_swap_store = terrain_swap_store();
    let mut object = world_object();

    let update = add_visible_map_id_like_cpp(&mut object, &terrain_swap_store, 609);

    assert_eq!(update, Some(PhaseVisibilityUpdate::new(false, true)));
    assert!(object.phase_shift().has_visible_map_id_like_cpp(609));
    assert!(object.phase_shift().has_ui_map_phase_id_like_cpp(42));
    assert!(object.phase_shift().has_ui_map_phase_id_like_cpp(43));

    let update = add_visible_map_id_like_cpp(&mut object, &terrain_swap_store, 609);
    assert_eq!(update, Some(PhaseVisibilityUpdate::new(false, false)));

    let update = remove_visible_map_id_like_cpp(&mut object, &terrain_swap_store, 609);
    assert_eq!(update, Some(PhaseVisibilityUpdate::new(false, false)));
    assert!(object.phase_shift().has_visible_map_id_like_cpp(609));
    assert!(object.phase_shift().has_ui_map_phase_id_like_cpp(42));

    let update = remove_visible_map_id_like_cpp(&mut object, &terrain_swap_store, 609);
    assert_eq!(update, Some(PhaseVisibilityUpdate::new(false, true)));
    assert!(!object.phase_shift().has_visible_map_id_like_cpp(609));
    assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(42));
    assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(43));
}

#[test]
fn visible_map_id_core_ignores_missing_terrain_swap_info() {
    let terrain_swap_store = terrain_swap_store();
    let mut object = world_object();

    assert_eq!(
        add_visible_map_id_like_cpp(&mut object, &terrain_swap_store, 999),
        None
    );
    assert!(!object.phase_shift().has_visible_map_id_like_cpp(999));
}

#[test]
fn on_map_change_rebuilds_visible_ui_and_suppressed_swaps_like_cpp() {
    let terrain_swap_store = terrain_swap_store();
    let mut object = world_object();
    object.world_relocate(571, object.position());
    object.phase_shift_mut().add_visible_map_id_like_cpp(999, 1);
    object
        .phase_shift_mut()
        .add_ui_map_phase_id_like_cpp(999, 1);
    object
        .suppressed_phase_shift_mut()
        .add_visible_map_id_like_cpp(998, 1);

    let update = on_map_change_like_cpp(&mut object, &terrain_swap_store, |id, _| id != 609);

    assert_eq!(update, PhaseVisibilityUpdate::new(false, true));
    assert!(!object.phase_shift().has_visible_map_id_like_cpp(999));
    assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(999));
    assert!(
        !object
            .suppressed_phase_shift()
            .has_visible_map_id_like_cpp(998)
    );
    assert!(!object.phase_shift().has_visible_map_id_like_cpp(609));
    assert!(
        object
            .suppressed_phase_shift()
            .has_visible_map_id_like_cpp(609)
    );
    assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(42));
    assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(43));
    assert!(!object.phase_shift().has_visible_map_id_like_cpp(700));
    assert!(object.phase_shift().has_ui_map_phase_id_like_cpp(70));
}

#[test]
fn on_area_change_walks_parent_suppresses_and_reapplies_aura_phases_like_cpp() {
    let area_store = area_store();
    let phase_store = phase_store();
    let phase_group_store = phase_group_store(&phase_store);
    let phase_info_store = phase_info_store(&area_store, &phase_store);
    let mut object = world_object();
    let personal_guid = object.guid();
    object.set_zone_and_area(100, 101);
    object
        .phase_shift_mut()
        .add_phase_like_cpp(99, PhaseFlags::NONE, 1);
    object
        .suppressed_phase_shift_mut()
        .add_phase_like_cpp(98, PhaseFlags::NONE, 1);

    let update = on_area_change_like_cpp(
        &mut object,
        &area_store,
        &phase_store,
        &phase_group_store,
        &phase_info_store,
        |phase_id, _| phase_id != 20,
        [30],
        [7],
    );

    assert_eq!(update, PhaseVisibilityUpdate::new(true, true));
    assert!(!object.phase_shift().has_phase_like_cpp(99));
    assert!(!object.suppressed_phase_shift().has_phase_like_cpp(98));
    assert!(object.phase_shift().has_phase_like_cpp(10));
    assert!(object.suppressed_phase_shift().has_phase_like_cpp(20));
    assert!(object.phase_shift().has_phase_like_cpp(30));
    assert_eq!(
        object
            .phase_shift()
            .phase_ref_like_cpp(30)
            .map(|phase| phase.flags()),
        Some(PhaseFlags::COSMETIC)
    );
    assert_eq!(object.phase_shift().personal_guid_like_cpp(), personal_guid);
}

#[test]
fn on_condition_change_moves_phases_between_active_and_suppressed_like_cpp() {
    let phase_store = phase_store();
    let phase_group_store = phase_group_store(&phase_store);
    let terrain_swap_store = terrain_swap_store();
    let mut object = world_object();
    let personal_guid = object.guid();
    object
        .phase_shift_mut()
        .add_phase_like_cpp(10, PhaseFlags::NONE, 2);
    object
        .phase_shift_mut()
        .add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);
    object
        .suppressed_phase_shift_mut()
        .add_phase_like_cpp(30, PhaseFlags::COSMETIC, 1);

    let update = on_condition_change_like_cpp(
        &mut object,
        &phase_store,
        &phase_group_store,
        &terrain_swap_store,
        true,
        |phase_id, _| match phase_id {
            10 => Some(false),
            20 => None,
            _ => Some(true),
        },
        |phase_id, _| phase_id == 30,
        |_, _| true,
        [10],
        std::iter::empty(),
    );

    assert_eq!(update, PhaseVisibilityUpdate::new(true, true));
    assert_eq!(
        object
            .phase_shift()
            .phase_ref_like_cpp(10)
            .map(|phase| phase.references()),
        Some(1)
    );
    assert_eq!(
        object
            .suppressed_phase_shift()
            .phase_ref_like_cpp(10)
            .map(|phase| phase.references()),
        Some(1)
    );
    assert!(object.phase_shift().has_phase_like_cpp(20));
    assert_eq!(object.phase_shift().personal_guid_like_cpp(), personal_guid);
    assert!(object.phase_shift().has_phase_like_cpp(30));
    assert!(!object.suppressed_phase_shift().has_phase_like_cpp(30));
}

#[test]
fn on_condition_change_moves_visible_maps_and_ui_phase_ids_like_cpp() {
    let phase_store = phase_store();
    let phase_group_store = phase_group_store(&phase_store);
    let terrain_swap_store = terrain_swap_store();
    let mut object = world_object();
    object.phase_shift_mut().add_visible_map_id_like_cpp(609, 1);
    object.phase_shift_mut().add_ui_map_phase_id_like_cpp(42, 1);
    object.phase_shift_mut().add_ui_map_phase_id_like_cpp(43, 1);
    object
        .suppressed_phase_shift_mut()
        .add_visible_map_id_like_cpp(700, 1);

    let update = on_condition_change_like_cpp(
        &mut object,
        &phase_store,
        &phase_group_store,
        &terrain_swap_store,
        false,
        |_, _| None,
        |_, _| false,
        |visible_map_id, _| visible_map_id == 700,
        std::iter::empty(),
        std::iter::empty(),
    );

    assert_eq!(update, PhaseVisibilityUpdate::new(false, true));
    assert!(!object.phase_shift().has_visible_map_id_like_cpp(609));
    assert!(
        object
            .suppressed_phase_shift()
            .has_visible_map_id_like_cpp(609)
    );
    assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(42));
    assert!(!object.phase_shift().has_ui_map_phase_id_like_cpp(43));
    assert!(object.phase_shift().has_visible_map_id_like_cpp(700));
    assert!(
        !object
            .suppressed_phase_shift()
            .has_visible_map_id_like_cpp(700)
    );
    assert!(object.phase_shift().has_ui_map_phase_id_like_cpp(70));
}

#[test]
fn on_area_change_honors_parent_sub_area_exclusions_like_cpp() {
    let area_store = area_store();
    let phase_store = phase_store();
    let phase_group_store = phase_group_store(&phase_store);
    let phase_info_store = phase_info_store(&area_store, &phase_store);
    let mut object = world_object();
    object.set_zone_and_area(100, 101);

    on_area_change_like_cpp(
        &mut object,
        &area_store,
        &phase_store,
        &phase_group_store,
        &phase_info_store,
        |_, _| true,
        [],
        [],
    );

    assert!(object.phase_shift().has_phase_like_cpp(10));
    assert!(object.phase_shift().has_phase_like_cpp(20));
    assert!(!object.phase_shift().has_phase_like_cpp(30));
}

#[test]
fn inherit_phase_shift_copies_active_and_suppressed_like_cpp() {
    let mut source = world_object();
    let mut target = world_object();
    source
        .phase_shift_mut()
        .add_phase_like_cpp(10, PhaseFlags::NONE, 1);
    source
        .suppressed_phase_shift_mut()
        .add_phase_like_cpp(20, PhaseFlags::NONE, 1);

    inherit_phase_shift_like_cpp(&mut target, &source);

    assert!(target.phase_shift().has_phase_like_cpp(10));
    assert!(target.suppressed_phase_shift().has_phase_like_cpp(20));
}

#[test]
fn set_visibility_flags_match_cpp_and_report_update_request() {
    let mut object = world_object();

    let update = set_always_visible_like_cpp(&mut object, true, true);
    assert_eq!(update, PhaseVisibilityUpdate::new(true, true));
    assert!(
        object
            .phase_shift()
            .flags_like_cpp()
            .contains(PhaseShiftFlags::ALWAYS_VISIBLE)
    );

    let update = set_inversed_like_cpp(&mut object, true, false);
    assert_eq!(update, PhaseVisibilityUpdate::new(false, true));
    assert!(
        object
            .phase_shift()
            .flags_like_cpp()
            .contains(PhaseShiftFlags::INVERSE)
    );
    assert!(
        object
            .phase_shift()
            .flags_like_cpp()
            .contains(PhaseShiftFlags::INVERSE_UNPHASED)
    );
    assert!(
        !object
            .phase_shift()
            .flags_like_cpp()
            .contains(PhaseShiftFlags::UNPHASED)
    );
}

#[test]
fn init_db_phase_shift_uses_cpp_flags_phase_and_group_priority() {
    let phase_store = phase_store();
    let phase_group_store = phase_group_store(&phase_store);
    let mut phase_shift = PhaseShift::default();

    init_db_phase_shift_like_cpp(
        &mut phase_shift,
        &phase_store,
        &phase_group_store,
        PHASE_USE_FLAGS_ALWAYS_VISIBLE | PHASE_USE_FLAGS_INVERSE,
        20,
        7,
    );

    assert!(phase_shift.is_db_phase_shift_like_cpp());
    assert!(phase_shift.has_phase_like_cpp(20));
    assert!(!phase_shift.has_phase_like_cpp(10));
    assert_eq!(
        phase_shift.flags_like_cpp(),
        PhaseShiftFlags::ALWAYS_VISIBLE | PhaseShiftFlags::UNPHASED | PhaseShiftFlags::INVERSE
    );
}

#[test]
fn init_db_phase_shift_uses_group_and_unphased_fallback_like_cpp() {
    let phase_store = phase_store();
    let phase_group_store = phase_group_store(&phase_store);
    let mut phase_shift = PhaseShift::default();

    init_db_phase_shift_like_cpp(
        &mut phase_shift,
        &phase_store,
        &phase_group_store,
        PHASE_USE_FLAGS_INVERSE,
        0,
        7,
    );

    assert!(phase_shift.has_phase_like_cpp(10));
    assert!(phase_shift.has_phase_like_cpp(20));
    assert!(!phase_shift.has_phase_like_cpp(99));
    assert_eq!(phase_shift.flags_like_cpp(), PhaseShiftFlags::INVERSE);

    init_db_phase_shift_like_cpp(
        &mut phase_shift,
        &phase_store,
        &phase_group_store,
        PHASE_USE_FLAGS_INVERSE,
        0,
        0,
    );

    assert_eq!(
        phase_shift.flags_like_cpp(),
        PhaseShiftFlags::INVERSE | PhaseShiftFlags::INVERSE_UNPHASED
    );
}

#[test]
fn init_db_personal_ownership_stamps_personal_guid_like_cpp() {
    let phase_store = phase_store();
    let phase_group_store = phase_group_store(&phase_store);
    let personal_guid = ObjectGuid::create_player(1, 42);
    let mut phase_shift = PhaseShift::default();

    init_db_phase_shift_like_cpp(&mut phase_shift, &phase_store, &phase_group_store, 0, 20, 0);
    init_db_personal_ownership_like_cpp(&mut phase_shift, personal_guid);

    assert_eq!(phase_shift.personal_guid_like_cpp(), personal_guid);
}

#[test]
fn init_db_visible_map_id_resets_visible_maps_only_like_cpp() {
    let terrain_swap_store = terrain_swap_store();
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_visible_map_id_like_cpp(700, 1);
    phase_shift.add_ui_map_phase_id_like_cpp(70, 1);

    init_db_visible_map_id_like_cpp(&mut phase_shift, &terrain_swap_store, 609);

    assert!(!phase_shift.has_visible_map_id_like_cpp(700));
    assert!(phase_shift.has_visible_map_id_like_cpp(609));
    assert!(phase_shift.has_ui_map_phase_id_like_cpp(70));

    init_db_visible_map_id_like_cpp(&mut phase_shift, &terrain_swap_store, -1);
    assert_eq!(phase_shift.visible_map_id_count_like_cpp(), 0);
    assert!(phase_shift.has_ui_map_phase_id_like_cpp(70));
}

#[test]
fn phase_shift_change_packet_copies_phase_shift_like_cpp_send_to_player() {
    let player_guid = ObjectGuid::create_player(1, 42);
    let personal_guid = ObjectGuid::create_player(1, 99);
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_phase_like_cpp(10, PhaseFlags::COSMETIC, 1);
    phase_shift.add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);
    phase_shift.set_personal_guid_like_cpp(personal_guid);
    phase_shift.add_visible_map_id_like_cpp(609, 1);
    phase_shift.add_ui_map_phase_id_like_cpp(42, 1);

    let packet = phase_shift_change_for_player_like_cpp(player_guid, &phase_shift).unwrap();

    assert_eq!(packet.player_guid, player_guid);
    assert_eq!(
        packet.phase_shift_flags,
        phase_shift.flags_like_cpp().bits()
    );
    assert_eq!(packet.personal_guid, personal_guid);
    assert_eq!(
        packet.phases,
        vec![
            PhaseShiftDataPhase {
                phase_flags: PhaseFlags::COSMETIC.bits(),
                id: 10,
            },
            PhaseShiftDataPhase {
                phase_flags: PhaseFlags::PERSONAL.bits(),
                id: 20,
            },
        ]
    );
    assert_eq!(packet.visible_map_ids, vec![609]);
    assert!(packet.preload_map_ids.is_empty());
    assert_eq!(packet.ui_map_phase_ids, vec![42]);
}

#[test]
fn party_member_phase_states_copy_phase_shift_like_cpp() {
    let personal_guid = ObjectGuid::create_player(1, 99);
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_phase_like_cpp(10, PhaseFlags::COSMETIC, 1);
    phase_shift.add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);
    phase_shift.set_personal_guid_like_cpp(personal_guid);

    let states = party_member_phase_states_like_cpp(&phase_shift).unwrap();

    assert_eq!(
        states.phase_shift_flags,
        phase_shift.flags_like_cpp().bits()
    );
    assert_eq!(states.personal_guid, personal_guid);
    assert_eq!(
        states.phases,
        vec![
            PartyMemberPhase {
                flags: u32::from(PhaseFlags::COSMETIC.bits()),
                id: 10,
            },
            PartyMemberPhase {
                flags: u32::from(PhaseFlags::PERSONAL.bits()),
                id: 20,
            },
        ]
    );
}

#[test]
fn format_phases_keeps_cpp_comma_suffix_and_order() {
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);
    phase_shift.add_phase_like_cpp(10, PhaseFlags::COSMETIC, 1);

    assert_eq!(format_phases_like_cpp(&phase_shift), "10,20,");
}

#[test]
fn print_to_chat_snapshot_matches_cpp_argument_payloads() {
    let personal_guid = ObjectGuid::create_player(1, 99);
    let mut object = world_object();
    object
        .phase_shift_mut()
        .set_flags_like_cpp(PhaseShiftFlags::ALWAYS_VISIBLE);
    object
        .phase_shift_mut()
        .set_personal_guid_like_cpp(personal_guid);
    object
        .phase_shift_mut()
        .add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);
    object
        .phase_shift_mut()
        .add_phase_like_cpp(10, PhaseFlags::COSMETIC, 1);
    object.phase_shift_mut().add_visible_map_id_like_cpp(609, 1);
    object.phase_shift_mut().add_visible_map_id_like_cpp(571, 1);
    object.phase_shift_mut().add_ui_map_phase_id_like_cpp(42, 1);

    let snapshot = print_to_chat_snapshot_like_cpp(
        &object,
        |guid| (guid == personal_guid).then(|| String::from("Owner")),
        |phase_id| Some(format!("Phase {phase_id}")),
        "Cosmetic",
        "Personal",
    );

    assert_eq!(snapshot.flags, object.phase_shift().flags_like_cpp().bits());
    assert_eq!(snapshot.personal_guid, personal_guid);
    assert_eq!(snapshot.personal_owner_name, "Owner");
    assert_eq!(
        snapshot.phases,
        Some(String::from(
            "\r\n   10 (Phase 10) (Cosmetic)\r\n   20 (Phase 20) (Personal)"
        ))
    );
    assert_eq!(snapshot.visible_map_ids, Some(String::from("571, 609, ")));
    assert_eq!(snapshot.ui_map_phase_ids, Some(String::from("42, ")));
}

#[test]
fn print_to_chat_snapshot_uses_cpp_missing_owner_and_phase_name_fallbacks() {
    let mut object = world_object();
    object
        .phase_shift_mut()
        .add_phase_like_cpp(20, PhaseFlags::PERSONAL, 1);

    let snapshot =
        print_to_chat_snapshot_like_cpp(&object, |_| None, |_| None, "Cosmetic", "Personal");

    assert_eq!(snapshot.personal_owner_name, "N/A");
    assert_eq!(
        snapshot.phases,
        Some(String::from("\r\n   20 (Unknown Name) (Personal)"))
    );
    assert_eq!(snapshot.visible_map_ids, None);
    assert_eq!(snapshot.ui_map_phase_ids, None);
}

#[test]
fn phase_shift_change_packet_rejects_values_that_do_not_fit_cpp_wire_types() {
    let mut phase_shift = PhaseShift::default();
    phase_shift.add_phase_like_cpp(u16::MAX as u32 + 1, PhaseFlags::NONE, 1);

    let error = match phase_shift_change_for_player_like_cpp(ObjectGuid::EMPTY, &phase_shift) {
        Ok(_) => panic!("overflowing phase id must be rejected"),
        Err(error) => error,
    };
    assert_eq!(
        error,
        PhaseShiftPacketBuildError::PhaseIdOutOfRange(u16::MAX as u32 + 1)
    );
}
