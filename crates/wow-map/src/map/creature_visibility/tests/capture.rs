use super::*;
use wow_constants::{UnitFlags, UnitFlags2, UnitFlags3};

#[test]
fn compatible_create_keeps_ai_overrides_distinct_from_unit_flags() {
    let mut source = creature(201, true);
    source
        .unit_mut()
        .set_unit_flags_like_cpp(UnitFlags::from_bits_retain(0x10));
    source
        .unit_mut()
        .set_unit_flags2_like_cpp(UnitFlags2::from_bits_retain(0x20));
    source
        .unit_mut()
        .set_unit_flags3_like_cpp(UnitFlags3::from_bits_retain(0x40));
    source.unit_mut().set_npc_flags_like_cpp(0x80);
    let ai = source.ai_ownership_mut();
    ai.npc_flags = 0x1234;
    ai.npc_flags2 = 0x5678;
    ai.unit_flags = 0x100;
    ai.unit_flags2 = 0x200;
    ai.unit_flags3 = 0x400;
    let captured = CreatureVisibilityCandidate::compatible(&source);
    let data = captured.create().create_data();
    assert_eq!(data.npc_flags, 0x5678_0000_1234);
    assert_eq!(data.unit_flags, 0x100);
    assert_eq!(data.unit_flags2, 0x200);
    assert_eq!(data.unit_flags3, 0x400);
    assert!(captured.create().active_move_spline().is_none());
    assert_eq!(source.unit().data().flags, 0x10);
}

#[test]
fn compatible_capture_keeps_u64_model_health_and_u32_app_override_separate() {
    let mut source = creature(202, true);
    source.unit_mut().set_max_health(u64::MAX);
    source.unit_mut().set_health(u64::MAX);
    let captured = CreatureVisibilityCandidate::compatible(&source);
    assert_eq!(captured.create().create_data().health, i64::MAX);
    assert_eq!(captured.create().create_data().max_health, i64::MAX);
    assert_eq!(captured.create().current_hp(), u32::MAX);
    assert_eq!(captured.create().max_hp(), u32::MAX);
    source.unit_mut().set_health(1);
    assert_eq!(captured.create().current_hp(), u32::MAX);
    assert_eq!(captured.create().level(), 12);
}

#[test]
fn legacy_keeps_stored_create_data_area_and_spline_independent_of_later_writes() {
    let mut source = legacy(203);
    source.create_data.scale = 1.75;
    source.create_data.current_area_id = 77;
    source.create_data.npc_flags = 999;
    source.create_data.guid = ObjectGuid::EMPTY;
    source.create_data.entry = 9999;
    source.creature.ai_ownership_mut().npc_flags = 123;
    source
        .begin_move_spline_like_cpp(Position::xyz(20.0, 10.0, 0.0))
        .unwrap();
    let original_spline_id = source.active_move_spline_like_cpp().unwrap().id();
    let captured = source.capture_visibility_candidate();
    source.create_data.scale = 2.0;
    source.create_data.current_area_id = 88;
    source.creature.unit_mut().set_health(20);
    source
        .creature
        .set_ai_position(Position::xyz(100.0, 100.0, 0.0));
    source
        .begin_move_spline_like_cpp(Position::xyz(110.0, 100.0, 0.0))
        .unwrap();
    assert_eq!(captured.create().create_data().scale, 1.75);
    assert_eq!(captured.create().create_data().current_area_id, 77);
    assert_eq!(captured.create().create_data().npc_flags, 999);
    assert_eq!(captured.create().create_data().guid, ObjectGuid::EMPTY);
    assert_eq!(captured.create().create_data().entry, 9999);
    assert_eq!(captured.guid(), source.guid());
    assert_eq!(captured.create().entry(), 42);
    assert_eq!(captured.create().npc_flags_mask(), 123);
    assert_eq!(captured.create().current_hp(), 75);
    assert_eq!(captured.create().position(), Position::xyz(10.0, 10.0, 0.0));
    assert_eq!(
        captured.create().active_move_spline().unwrap().id(),
        original_spline_id
    );
}

#[test]
fn compatible_actor_ignores_stored_presentation_and_spline_without_changing_motor() {
    let mut source = legacy(204);
    source.create_data.scale = 9.0;
    source.create_data.current_area_id = 777;
    let guid = source.guid();
    let mut map = Map::new(571, 7, 1, 1000);
    let entry = map.creature_actor_entry(source).unwrap();
    map.add_object_entry_to_map(entry).unwrap();
    let mut rng = map
        .creature_actor_mut(guid)
        .unwrap()
        .seed_actor_storage_runtime(true);
    let actor_pointer = map.creature_actor(guid).unwrap() as *const WorldCreature;
    let witness = map.creature_actor_witness(guid).unwrap();
    let revision = map
        .creature_actor(guid)
        .unwrap()
        .creature
        .unit()
        .health_state_revision_like_cpp();
    let captured = collect(&map);
    assert_eq!(captured.len(), 1);
    assert_eq!(captured[0].guid(), guid);
    assert_eq!(captured[0].create().create_data().scale, 1.0);
    assert_eq!(captured[0].create().create_data().current_area_id, 0);
    assert!(captured[0].create().active_move_spline().is_none());
    assert_eq!(
        map.creature_actor(guid).unwrap() as *const WorldCreature,
        actor_pointer
    );
    assert!(witness.same_actor(&map.creature_actor_witness(guid).unwrap()));
    assert_eq!(
        map.creature_actor(guid)
            .unwrap()
            .creature
            .unit()
            .health_state_revision_like_cpp(),
        revision
    );
    assert_eq!(
        map.capture_compatible_creature_message_source(guid)
            .unwrap()
            .guid(),
        guid
    );
    map.creature_actor_mut(guid)
        .unwrap()
        .assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn candidate_target_facts_are_evaluated_against_later_seer_without_rereading_source() {
    let mut source = legacy(205);
    source.creature.unit_mut().set_invisibility_like_cpp(37, 8);
    let captured = source.capture_visibility_candidate();
    source.creature.unit_mut().set_invisibility_like_cpp(37, 0);
    let mut seer = Unit::new(true);
    assert!(!seer.can_see_or_detect_target(captured.visibility_target(), false, true, false));
    seer.set_invisibility_detect_like_cpp(37, 8);
    assert!(seer.can_see_or_detect_target(captured.visibility_target(), false, true, false));
}

#[test]
fn message_capture_keeps_position_phase_and_override_without_alive_gate() {
    let mut source = legacy(206);
    source
        .creature
        .unit_mut()
        .world_mut()
        .set_visibility_distance_override_like_cpp(
            wow_entities::VisibilityDistanceTypeLikeCpp::Tiny,
        );
    *source.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([10]);
    source.creature.unit_mut().set_health(0);
    let captured = source.capture_message_source();
    source
        .creature
        .set_ai_position(Position::xyz(90.0, 90.0, 0.0));
    *source.creature.unit_mut().world_mut().phase_shift_mut() = PhaseShift::from_phases([20]);
    assert_eq!(captured.guid(), source.guid());
    assert_eq!(captured.map_id(), 571);
    assert_eq!(captured.instance_id(), 7);
    assert_eq!(captured.position(), Position::xyz(10.0, 10.0, 0.0));
    assert_eq!(captured.visibility_range(), 25.0);
    assert!(PhaseShift::from_phases([10]).can_see(captured.phase_shift()));
    assert!(!PhaseShift::from_phases([20]).can_see(captured.phase_shift()));
}
