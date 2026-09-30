//! Represented AddToWorld stages and deferred visibility, using the sole lifecycle body.

use super::*;
use wow_entities::{
    AppliedAuraRef, CreatureAddToWorldVehicleResetContextLikeCpp, CreatureFormationInfoLikeCpp,
    ObjectNotifyFlags, Player, SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP,
    VehicleSeatAddon, VehicleSeatInfo,
};

#[test]
fn fresh_lifecycle_indexes_before_unit_add_and_consumes_formation_vehicle_and_aura_stages() {
    let mut map = map();
    let (mut incoming, mut rng) = actor(309, 3090, true);
    let guid = incoming.guid();
    incoming.creature.set_formation_info_like_cpp(Some(CreatureFormationInfoLikeCpp {
        leader_spawn_id: 900309,
        follow_dist: 8.0,
        follow_angle_radians: 0.75,
        group_ai: 4,
        leader_waypoint_ids: [21, 22],
    }));
    incoming.creature.set_add_to_world_vehicle_reset_context_like_cpp(Some(
        CreatureAddToWorldVehicleResetContextLikeCpp {
            is_mechanical_creature: false,
            is_world_boss: false,
            accessories: Vec::new(),
        },
    ));
    let position = incoming.creature.unit().world().position();
    let entry = incoming.creature.unit().world().object().entry();
    incoming.creature.unit_mut().subsystems_mut().vehicle.create_vehicle_kit_like_cpp(
        guid, position, Some(9309), entry, true,
        Some(vec![(0, VehicleSeatInfo {
            id: 100,
            attachment_offset: Position::default(),
            can_enter_or_exit: true,
            usable_by_override: false,
            can_control: false,
            can_switch_from_seat: false,
            ejectable: false,
            disables_gravity: false,
            passenger_not_selectable: false,
            keep_pet: false,
        }, VehicleSeatAddon::default())]),
    );
    let aura = AppliedAuraRef::new(47_510, ObjectGuid::create_player(1, 99), 1, 0x1);
    incoming.creature.unit_mut().subsystems_mut().auras.register_applied_aura(
        aura, None, SPELL_AURA_INTERRUPT_FLAG_ENTER_WORLD_LIKE_CPP, 0,
    );
    let outcome = inserted(&mut map, incoming);
    assert_eq!(outcome.creature_store_inserted_before_add_to_world, Some(true));
    assert_eq!(outcome.creature_spawn_indexed_before_add_to_world, Some(true));
    let unit = outcome.creature_unit_add_to_world.unwrap();
    assert!(unit.is_in_world_after);
    assert_eq!(unit.removed_enter_world_auras, vec![aura]);
    let formation = outcome.creature_search_formation.unwrap();
    assert_eq!(formation.spawn_id, 3090);
    assert_eq!(formation.leader_spawn_id, Some(900309));
    assert!(formation.add_to_group_requested);
    assert!(map.creature_group_holder_contains_like_cpp(900309, guid));
    assert_eq!(map.creature_group_holder_member_count_like_cpp(900309), 1);
    let aim = outcome.creature_aim_initialize.unwrap();
    assert!(aim.aim_create_represented && aim.ai_initialize_represented);
    assert!(aim.vehicle_reset_expected);
    assert_eq!(outcome.creature_vehicle_reset.unwrap().kit_id, 9309);
    assert!(outcome.creature_vehicle_install.unwrap().had_kit);
    let zone = outcome.creature_zone_script_create.unwrap();
    assert!(zone.represented_callback);
    assert!(!zone.script_dispatch_represented);
    assert!(outcome.add_to_map_tail.unwrap().initialize_object_represented);
    let stored = map.creature_actor_mut(guid).unwrap();
    assert!(!stored.creature.unit().subsystems().auras.has_applied(aura));
    assert!(stored.creature.unit().subsystems().vehicle.kit.as_ref().unwrap().installed());
    assert_eq!(stored.creature.unit().world().current_cell(),
        Some((Cell::from_world(1.0, 2.0).cell_x(), Cell::from_world(1.0, 2.0).cell_y())));
    // Unit's local represented AddToWorld helper runs, but the complete actor
    // motor (generator, spline, schedule, clocks and RNG) is not reconstructed.
    stored.assert_actor_storage_runtime(&mut rng, true);
}

#[test]
fn fresh_admission_marks_existing_nearby_player_for_deferred_visibility() {
    let mut map = map();
    let player_guid = ObjectGuid::create_player(1, 310);
    let mut player = Player::new(Some(7), false);
    player.unit_mut().world_mut().object_mut().create(player_guid);
    player.unit_mut().world_mut().set_map(571, 7).unwrap();
    player.unit_mut().world_mut().relocate(Position::xyz(2.0, 3.0, 3.0));
    let player_add = map.add_map_object_record_to_map_like_cpp(
        MapObjectRecord::new_player(player).unwrap(),
    ).unwrap();
    // Reset setup notification before observing the fresh admission alone.
    map.get_typed_player_mut(player_guid).unwrap().unit_mut().world_mut().object_mut()
        .reset_all_notifies();
    let (incoming, mut rng) = actor(310, 3100, false);
    let guid = incoming.guid();
    let outcome = inserted(&mut map, incoming);
    assert_eq!(outcome.grid, player_add.grid);
    assert!(!outcome.grid_created);
    assert!(!outcome.grid_loaded);
    assert!(map.map_object(player_guid).unwrap().object()
        .is_need_notify(ObjectNotifyFlags::VISIBILITY_CHANGED));
    assert!(outcome.add_to_map_tail.unwrap().update_object_visibility_on_create_runtime_gap);
    map.creature_actor_mut(guid).unwrap().assert_actor_storage_runtime(&mut rng, false);
}

#[test]
fn wrong_kind_same_guid_record_is_retained_without_lifecycle_or_actor_promotion() {
    let mut map = map();
    let (incoming, mut rng) = actor(311, 3110, true);
    let guid = incoming.guid();
    let mut object = incoming.creature.unit().world().clone();
    object.object_mut().create(ObjectGuid::create_world_object(
        HighGuid::GameObject, 0, 1, 571, 7, 42, 311,
    ));
    let mut record = MapObjectRecord::new(AccessorObjectKind::GameObject, object).unwrap();
    // Existing record mutation can represent this mismatch without new fields
    // or an admission-only test capability.
    record.object_mut().object_mut().create(guid);
    map.insert_map_object_record(record).unwrap();
    let (error, mut returned) = map.admit_fresh_creature_actor(incoming).unwrap_err();
    assert_eq!(error, FreshCreatureActorAdmissionError::NotExactCreature {
        guid, actual_kind: AccessorObjectKind::GameObject,
    });
    returned.assert_actor_storage_runtime(&mut rng, true);
    assert_eq!(map.entity_world.kind(guid), Some(AccessorObjectKind::GameObject));
    assert!(map.creature_actor(guid).is_none());
    assert!(map.grids.iter().all(Option::is_none));
    assert_eq!(map.terrain.loads, 0);
    assert_eq!(map.lifecycle.loads, 0);
}
