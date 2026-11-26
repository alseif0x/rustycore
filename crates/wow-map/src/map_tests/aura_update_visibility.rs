//! MessageDistDeliverer observer routes for auras and melee combat.

use super::*;

fn add_indexed_player(
    map: &mut Map<RecordingTerrain, RecordingLifecycle>,
    mut player: Player,
) -> ObjectGuid {
    let guid = player.guid();
    player
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_player(player).unwrap())
        .unwrap();
    map.get_typed_player_mut(guid)
        .unwrap()
        .unit_mut()
        .world_mut()
        .object_mut()
        .add_to_world();
    guid
}

fn add_phased_player_at_x(
    map: &mut Map<RecordingTerrain, RecordingLifecycle>,
    counter: i64,
    x: f32,
    phase: u32,
) -> ObjectGuid {
    let mut player = test_player_for_viewpoint(counter);
    let world = player.unit_mut().world_mut();
    world.relocate(Position::xyz(x, 20.0, 30.0));
    world.phase_shift_mut().insert(phase);
    add_indexed_player(map, player)
}

fn add_indexed_creature_at_x(
    map: &mut Map<RecordingTerrain, RecordingLifecycle>,
    counter: i64,
    entry: u32,
    x: f32,
    phase: Option<u32>,
) -> ObjectGuid {
    let mut creature = Creature::new(true);
    let creature_guid = guid(HighGuid::Creature, counter);
    let world = creature.unit_mut().world_mut();
    world.object_mut().create(creature_guid);
    world.object_mut().set_entry(entry);
    world.set_map(map.map_id(), map.instance_id()).unwrap();
    world.relocate(Position::xyz(x, 20.0, 30.0));
    if let Some(phase) = phase {
        world.phase_shift_mut().insert(phase);
    }
    map.add_map_object_record_to_map_like_cpp(MapObjectRecord::new_creature(creature).unwrap())
        .unwrap();
    creature_guid
}

#[test]
fn corpse_birth_selects_owner_peer_and_shared_seer_with_exact_phase_and_range() {
    let mut map = test_map();
    let owner = add_indexed_player(&mut map, test_player_for_viewpoint(4_630_001));
    let peer = add_indexed_player(&mut map, test_player_for_viewpoint(4_630_002));
    let mut shared = test_player_for_viewpoint(4_630_003);
    shared
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(300.0, 300.0, 30.0));
    shared.set_farsight_object_like_cpp(peer);
    let shared = add_indexed_player(&mut map, shared);
    map.get_typed_player_mut(peer)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .control
        .shared_vision_guids
        .insert(shared);
    let mut wrong_phase = test_player_for_viewpoint(4_630_004);
    wrong_phase
        .unit_mut()
        .world_mut()
        .phase_shift_mut()
        .insert(77);
    let wrong_phase = add_indexed_player(&mut map, wrong_phase);
    let mut far = test_player_for_viewpoint(4_630_005);
    far.unit_mut()
        .world_mut()
        .relocate(Position::xyz(300.0, 300.0, 30.0));
    let far = add_indexed_player(&mut map, far);
    for guid in [owner, peer, shared] {
        map.get_typed_player_mut(guid)
            .unwrap()
            .unit_mut()
            .world_mut()
            .phase_shift_mut()
            .insert(10);
    }
    let guid = ObjectGuid::create_world_object(HighGuid::Corpse, 0, 1, 571, 0, 0, 4_630_001);
    let mut corpse = Corpse::new_at(CorpseType::ResurrectablePve, 1_000);
    corpse.world_mut().object_mut().create(guid);
    corpse
        .world_mut()
        .set_map(map.map_id(), map.instance_id())
        .unwrap();
    corpse.world_mut().relocate(
        map.get_typed_player(owner)
            .unwrap()
            .unit()
            .world()
            .position(),
    );
    corpse.set_owner_guid(owner);
    corpse.world_mut().phase_shift_mut().insert(10);
    map.add_corpse_to_map_like_cpp(corpse).unwrap();
    let recipients = map.corpse_birth_visibility_recipients_like_cpp(guid);
    assert_eq!(recipients.len(), 3);
    assert!(
        recipients.contains(&owner) && recipients.contains(&peer) && recipients.contains(&shared)
    );
    assert!(!recipients.contains(&wrong_phase) && !recipients.contains(&far));
    let bones = map
        .convert_corpse_to_bones_like_cpp(owner, true, 2_000)
        .unwrap()
        .bones_guid
        .unwrap();
    assert_eq!(
        map.corpse_birth_visibility_recipients_like_cpp(bones),
        recipients
    );
    map.get_typed_player_mut(owner)
        .unwrap()
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(300.0, 300.0, 30.0));
    let bones_recipients = map.corpse_birth_visibility_recipients_like_cpp(bones);
    assert!(
        !bones_recipients.contains(&owner),
        "far owner is not required for bones visibility"
    );
    assert!(bones_recipients.contains(&peer) && bones_recipients.contains(&shared));
    map.get_typed_player_mut(peer)
        .unwrap()
        .unit_mut()
        .world_mut()
        .object_mut()
        .remove_from_world();
    assert!(
        !map.corpse_birth_visibility_recipients_like_cpp(guid)
            .contains(&peer)
    );
}

#[test]
fn selected_aura_observers_use_source_range_phase_and_body_fallback_for_invalid_farsight() {
    let mut map = test_map();
    let source_guid = add_phased_player_at_x(&mut map, 4_610_001, 10.0, 10);
    let nearby_guid = add_phased_player_at_x(&mut map, 4_610_002, 12.0, 10);
    let invalid_farsight_guid = add_phased_player_at_x(&mut map, 4_610_003, 13.0, 10);
    map.get_typed_player_mut(invalid_farsight_guid)
        .unwrap()
        .set_farsight_object_like_cpp(guid(HighGuid::DynamicObject, 4_610_099));
    let wrong_phase_guid = add_phased_player_at_x(&mut map, 4_610_004, 10.0, 20);
    let out_of_range_guid = add_phased_player_at_x(&mut map, 4_610_005, 111.0, 10);

    let recipients = map.selected_aura_update_observers(source_guid);
    assert!(recipients.contains(&nearby_guid));
    assert!(recipients.contains(&invalid_farsight_guid));
    assert!(!recipients.contains(&wrong_phase_guid));
    assert!(!recipients.contains(&out_of_range_guid));

    let attacker_guid = add_indexed_creature_at_x(&mut map, 4_610_006, 299, 10.0, Some(10));
    let attacker = map
        .get_typed_creature(attacker_guid)
        .unwrap()
        .unit()
        .world();
    let melee_recipients = map.message_dist_observers_like_cpp(attacker);
    assert!(melee_recipients.contains(&nearby_guid));
    assert!(!melee_recipients.contains(&wrong_phase_guid));
    assert!(!melee_recipients.contains(&out_of_range_guid));
}

#[test]
fn selected_aura_observers_cover_shared_player_creature_dynamic_and_vehicle_routes() {
    let mut map = test_map();
    let source_guid = add_indexed_player(&mut map, test_player_for_viewpoint(4_620_001));

    let source_shared_viewer_guid = guid(HighGuid::Player, 4_620_010);
    map.get_typed_player_mut(source_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .control
        .shared_vision_guids
        .insert(source_shared_viewer_guid);
    let mut source_shared_viewer = test_player_for_viewpoint(4_620_010);
    source_shared_viewer
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(303.0, 300.0, 30.0));
    source_shared_viewer.set_farsight_object_like_cpp(source_guid);
    add_indexed_player(&mut map, source_shared_viewer);

    let shared_player_guid = guid(HighGuid::Player, 4_620_002);
    let mut shared_player = test_player_for_viewpoint(4_620_003);
    shared_player
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(12.0, 20.0, 30.0));
    shared_player
        .unit_mut()
        .subsystems_mut()
        .control
        .shared_vision_guids
        .insert(shared_player_guid);
    let shared_player_target_guid = add_indexed_player(&mut map, shared_player);
    let mut shared_player_viewer = test_player_for_viewpoint(4_620_002);
    shared_player_viewer
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(300.0, 300.0, 30.0));
    shared_player_viewer.set_farsight_object_like_cpp(shared_player_target_guid);
    add_indexed_player(&mut map, shared_player_viewer);

    let shared_creature_viewer_guid = guid(HighGuid::Player, 4_620_004);
    let creature_guid = add_indexed_creature_at_x(&mut map, 4_620_005, 42, 14.0, None);
    map.get_typed_creature_mut(creature_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .control
        .shared_vision_guids
        .insert(shared_creature_viewer_guid);
    let mut shared_creature_viewer = test_player_for_viewpoint(4_620_004);
    shared_creature_viewer
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(301.0, 300.0, 30.0));
    shared_creature_viewer.set_farsight_object_like_cpp(creature_guid);
    add_indexed_player(&mut map, shared_creature_viewer);

    let dynamic_viewer_guid = guid(HighGuid::Player, 4_620_006);
    let mut dynamic_viewer = test_player_for_viewpoint(4_620_006);
    dynamic_viewer
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(302.0, 300.0, 30.0));
    let mut dynamic = test_dynamic_object_for_viewpoint(4_620_007);
    dynamic.world_mut().object_mut().remove_from_world();
    dynamic
        .world_mut()
        .relocate(Position::xyz(16.0, 20.0, 30.0));
    let dynamic_guid = dynamic.world().guid();
    dynamic.set_caster_guid(dynamic_viewer_guid);
    map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_dynamic_object(dynamic).unwrap(),
    )
    .unwrap();
    map.get_typed_dynamic_object_mut(dynamic_guid)
        .unwrap()
        .world_mut()
        .object_mut()
        .add_to_world();
    dynamic_viewer.set_farsight_object_like_cpp(dynamic_guid);
    add_indexed_player(&mut map, dynamic_viewer);

    let vehicle_viewer_guid = guid(HighGuid::Player, 4_620_008);
    let mut vehicle_viewer = test_player_for_viewpoint(4_620_008);
    vehicle_viewer
        .unit_mut()
        .world_mut()
        .relocate(Position::xyz(18.0, 20.0, 30.0));
    vehicle_viewer.set_farsight_object_like_cpp(creature_guid);
    vehicle_viewer
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .enter_vehicle(guid(HighGuid::Creature, 4_620_009), Some(0));
    assert_eq!(
        add_indexed_player(&mut map, vehicle_viewer),
        vehicle_viewer_guid
    );

    let recipients = map.selected_aura_update_observers(source_guid);
    assert!(recipients.contains(&source_shared_viewer_guid));
    assert!(recipients.contains(&shared_player_guid));
    assert!(recipients.contains(&shared_creature_viewer_guid));
    assert!(recipients.contains(&dynamic_viewer_guid));
    assert!(recipients.contains(&vehicle_viewer_guid));

    map.get_typed_player_mut(vehicle_viewer_guid)
        .unwrap()
        .unit_mut()
        .subsystems_mut()
        .vehicle
        .exit_vehicle();
    assert!(
        !map.selected_aura_update_observers(source_guid)
            .contains(&vehicle_viewer_guid),
        "a far-sight viewer without a shared-vision edge is selected only while riding"
    );
}
